//! A forest file to a WAV, streamed: any length, in constant memory. The WAV carries the forest
//! file (with the render settings used) in a `knpy` chunk, so it can be opened and rendered again
//! exactly. Same file, same seed: the same WAV, bit for bit.

use crate::files::{ForestFile, RenderFile};
use crate::life::DT;
use crate::wav::WavWriter;
use crate::{amp_to_db, VERSION};
use std::io::{Seek, Write};
use std::time::Instant;

pub const SAMPLE_RATE: u32 = 48_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderReport {
    pub seconds: f64,
    pub cpu_seconds: f64,
    pub peak_dbfs: f32,
    pub rms_dbfs: f32,
    pub calls: u64,
    pub ear_start_db: f64,
    pub ear_end_db: f64,
    /// Most any limiter took off, dB (0 = never acted).
    pub limited_db: f32,
}

/// The LIST/INFO chunk naming the software.
fn info_chunk() -> Vec<u8> {
    let mut sw = format!("kanopy {VERSION}").into_bytes();
    sw.push(0);
    if sw.len() % 2 == 1 {
        sw.push(0);
    }
    let mut b = b"INFO".to_vec();
    b.extend_from_slice(b"ISFT");
    b.extend_from_slice(&(sw.len() as u32).to_le_bytes());
    b.extend_from_slice(&sw);
    b
}

/// A gentle (raised-cosine) fade: 0 at the start, 1 after `len` samples.
fn ramp(i: u64, len: u64) -> f32 {
    if len == 0 || i >= len {
        1.0
    } else {
        (0.5 - 0.5 * (std::f64::consts::PI * i as f64 / len as f64).cos()) as f32
    }
}

/// Render `forest` with `opts` (overriding the file's own render settings) to `out`.
/// `progress` gets the seconds done, about once a second of audio.
pub fn render<W: Write + Seek>(
    forest: &ForestFile,
    opts: &RenderFile,
    out: W,
    mut progress: impl FnMut(f64),
) -> Result<(W, RenderReport), String> {
    let bytes = opts.minutes.max(0.0)
        * 60.0
        * SAMPLE_RATE as f64
        * 2.0
        * if opts.bits == 16 { 2.0 } else { 3.0 };
    if bytes > u32::MAX as f64 - 1e6 {
        let most = (u32::MAX as f64 - 1e6)
            / (60.0 * SAMPLE_RATE as f64 * 2.0 * if opts.bits == 16 { 2.0 } else { 3.0 });
        return Err(format!("a WAV can't be over 4 GB: at {} bits that's {:.0} minutes at most; render it in parts with different seeds, or use --bits 16", if opts.bits == 16 { 16 } else { 24 }, most.floor()));
    }
    let mut recorded = forest.clone();
    recorded.render = opts.clone();
    let json = serde_json::to_vec(&recorded).map_err(|e| e.to_string())?;
    let bits = if opts.bits == 16 { 16 } else { 24 };
    let mut wav = WavWriter::with_chunks(
        out,
        SAMPLE_RATE,
        bits,
        &[(*b"LIST", info_chunk()), (*b"knpy", json)],
    )
    .map_err(|e| e.to_string())?;

    let mut scene = forest.scene()?.with_sound(SAMPLE_RATE as f32);
    let n = scene.step_samples();
    let steps = (opts.minutes.max(0.0) * 60.0 / DT).round() as u64;
    let total = steps * n as u64;
    let fade_in = (opts.fade_in.max(0.0) * SAMPLE_RATE as f64) as u64;
    let fade_out = (opts.fade_out.max(0.0) * SAMPLE_RATE as f64) as u64;
    let (mut l, mut r) = (vec![0f32; n], vec![0f32; n]);
    let (mut peak, mut power) = (0f32, 0f64);
    let started = Instant::now();
    let ear_start = scene
        .mixer
        .as_ref()
        .and_then(|m| m.master.as_ref())
        .map_or(0.0, |m| m.start_db);
    let per_second = (1.0 / DT).round() as u64;
    for k in 0..steps {
        l.fill(0.0);
        r.fill(0.0);
        scene.step_audio(&mut l, &mut r);
        let base = k * n as u64;
        if base < fade_in || base + n as u64 > total.saturating_sub(fade_out) {
            for i in 0..n {
                let s = base + i as u64;
                let g = ramp(s, fade_in) * ramp(total - 1 - s, fade_out);
                l[i] *= g;
                r[i] *= g;
            }
        }
        for (a, b) in l.iter().zip(&r) {
            peak = peak.max(a.abs()).max(b.abs());
            power += (*a as f64).powi(2) + (*b as f64).powi(2);
        }
        wav.write(&l, &r).map_err(|e| e.to_string())?;
        if (k + 1) % per_second == 0 {
            progress((k + 1) as f64 * DT);
        }
    }
    let out = wav.finish().map_err(|e| e.to_string())?;
    let master = scene.mixer.as_ref().and_then(|m| m.master.as_ref());
    let limited = master.map_or(1.0, |m| {
        m.limiters
            .iter()
            .map(|x| x.min_gain)
            .fold(m.last.min_gain, f32::min)
    });
    let report = RenderReport {
        seconds: steps as f64 * DT,
        cpu_seconds: started.elapsed().as_secs_f64(),
        peak_dbfs: amp_to_db(peak),
        rms_dbfs: (10.0 * (power / (2 * total.max(1)) as f64).max(1e-20).log10()) as f32,
        calls: scene.stats.calls,
        ear_start_db: ear_start,
        ear_end_db: master.map_or(0.0, |m| m.gain_db),
        limited_db: amp_to_db(limited),
    };
    Ok((out, report))
}

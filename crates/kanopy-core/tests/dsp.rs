//! Wavetables, the body filter and the voice.

mod common;
use common::*;
use kanopy_core::filter::{OnePole, Svf};
use kanopy_core::rng::Rng;
use kanopy_core::voice::{note_to_hz, UnitSpec, Voice};
use kanopy_core::wavetable::*;

fn table_harmonic(t: &[f32], h: usize) -> f32 {
    let n = TABLE_LEN;
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for i in 0..n {
        let a = std::f64::consts::TAU * (h * i) as f64 / n as f64;
        re += t[i] as f64 * a.cos();
        im += t[i] as f64 * a.sin();
    }
    (2.0 * (re * re + im * im).sqrt() / n as f64) as f32
}

#[test]
fn shapes_share_rms() {
    let wt = Wavetables::new(SR);
    for lvl in 0..wt.levels() {
        for s in 0..SHAPES {
            let r = rms(&wt.table(lvl, s)[..TABLE_LEN]);
            assert!(
                (r - std::f32::consts::FRAC_1_SQRT_2).abs() < 2e-3,
                "level {lvl} shape {s} rms {r}"
            );
        }
    }
}

#[test]
fn no_harmonic_passes_nyquist() {
    let wt = Wavetables::new(SR);
    for lvl in 3..wt.levels() {
        let fmax = 20.0 * 2f32.powi(lvl as i32);
        let hmax = (((SR / 2.0) / fmax).floor() as usize).max(1);
        for s in [SAW, SQUARE] {
            let t = wt.table(lvl, s);
            for h in (hmax + 1)..(hmax + 6).min(TABLE_LEN / 2) {
                assert!(
                    table_harmonic(t, h) < 1e-4,
                    "level {lvl} shape {s} harmonic {h}"
                );
            }
        }
    }
}

#[test]
fn shapes_have_the_right_harmonics() {
    let wt = Wavetables::new(SR);
    let lvl = 5;
    let sine = wt.table(lvl, SINE);
    assert!(table_harmonic(sine, 2) < 1e-5 && table_harmonic(sine, 3) < 1e-5);
    let sq = wt.table(lvl, SQUARE);
    let r3 = table_harmonic(sq, 3) / table_harmonic(sq, 1);
    assert!((r3 - 1.0 / 3.0).abs() < 1e-3 && table_harmonic(sq, 2) < 1e-5);
    let tri = wt.table(lvl, TRIANGLE);
    let r3 = table_harmonic(tri, 3) / table_harmonic(tri, 1);
    assert!((r3 - 1.0 / 9.0).abs() < 1e-3);
    let saw = wt.table(lvl, SAW);
    let r2 = table_harmonic(saw, 2) / table_harmonic(saw, 1);
    assert!((r2 - 0.5).abs() < 1e-3);
}

#[test]
fn shape_crossfade_never_cancels() {
    // fundamentals share phase, so a blend's fundamental lies between its two ends
    let wt = Wavetables::new(SR);
    let lvl = 5;
    let blend = |s: f32| -> Vec<f32> {
        (0..TABLE_LEN)
            .map(|i| wt.sample(lvl, s, i as f32 / TABLE_LEN as f32))
            .collect()
    };
    for s in [0.5f32, 1.5, 2.5] {
        let a = table_harmonic(&blend(s.floor()), 1);
        let b = table_harmonic(&blend(s.ceil()), 1);
        let m = table_harmonic(&blend(s), 1);
        assert!(m >= a.min(b) - 1e-4, "shape {s}: {m} vs {a}, {b}");
    }
}

#[test]
fn level_covers_the_frequency() {
    let wt = Wavetables::new(SR);
    for f in [30.0f32, 440.0, 3000.0, 12000.0] {
        let lvl = wt.level_for(f);
        assert!(20.0 * 2f32.powi(lvl as i32) >= f);
        if lvl > 0 {
            assert!(20.0 * 2f32.powi(lvl as i32 - 1) < f);
        }
    }
}

fn hp_gain(f: f32, cutoff: f32, res: f32) -> f32 {
    let mut svf = Svf::new(cutoff, res, SR);
    let x = sine(f, 48_000);
    let y: Vec<f32> = x.iter().map(|v| svf.highpass(*v)).collect();
    db(tone(&y[24_000..], f, SR) / tone(&x[24_000..], f, SR))
}

#[test]
fn body_is_a_12db_highpass() {
    assert!((hp_gain(1000.0, 1000.0, 0.0) + 3.0).abs() < 0.5);
    let g = hp_gain(250.0, 1000.0, 0.0);
    assert!((g + 24.1).abs() < 1.5, "two octaves below: {g}");
    assert!(hp_gain(8000.0, 1000.0, 0.0).abs() < 0.3);
}

#[test]
fn resonance_peaks_at_cutoff() {
    let g = hp_gain(1000.0, 1000.0, 1.0);
    assert!(g > 22.0, "peak {g}");
}

#[test]
fn body_survives_jumping_cutoff() {
    let mut rng = Rng::new(3);
    let mut svf = Svf::new(1000.0, 1.0, SR);
    let mut peak = 0f32;
    for blk in 0..2000 {
        svf.set(rng.range(10.0, 1e6) as f32, 1.0, SR); // includes far past the top
        for i in 0..64 {
            let x = (((blk * 64 + i) as f32) * 0.05).sin();
            let y = svf.highpass(x);
            assert!(y.is_finite());
            peak = peak.max(y.abs());
        }
    }
    assert!(peak < 200.0, "peak {peak}");
}

#[test]
fn absorption_lowpass_cutoff() {
    let mut lp = OnePole::default();
    let a = OnePole::coeff(2000.0, SR);
    let x = sine(2000.0, 48_000);
    let y: Vec<f32> = x.iter().map(|v| lp.lowpass(*v, a)).collect();
    let g = db(tone(&y[24_000..], 2000.0, SR));
    assert!((g + 3.0).abs() < 0.5, "{g}");
}

fn render(spec: UnitSpec) -> Vec<f32> {
    let wt = Wavetables::new(SR);
    let mut v = Voice::new(spec, SR);
    let mut out = Vec::new();
    let mut buf = [0f32; 64];
    while !v.finished() {
        let n = v.render(&wt, &mut buf, 1.0);
        out.extend_from_slice(&buf[..n]);
        if n < 64 {
            break;
        }
    }
    out
}

#[test]
fn voice_pitch_length_and_soft_start() {
    let spec = UnitSpec {
        note_start: 69.0,
        note_end: 69.0,
        dur: 1.0,
        shape_start: 0.0,
        shape_end: 0.0,
        ..Default::default()
    };
    let y = render(spec);
    assert!((y.len() as i64 - 48_000).abs() <= 1, "length {}", y.len());
    let f = zc_freq(&y[4800..43_200], SR);
    assert!((f - 440.0).abs() < 0.2, "freq {f}");
    // no click at the start: tiny first sample, small first steps
    assert!(y[0].abs() < 0.01);
    let step = y[..480]
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0f32, f32::max);
    assert!(step < 0.06, "step {step}");
}

#[test]
fn voice_glides_in_semitones() {
    let spec = UnitSpec {
        note_start: 60.0,
        note_end: 72.0,
        dur: 2.0,
        ..Default::default()
    };
    let y = render(spec);
    let early = zc_freq(&y[..4800], SR); // first 0.1 s: notes 60–60.6
    let late = zc_freq(&y[91_200..96_000], SR); // last 0.1 s
    assert!((early - note_to_hz(60.3)).abs() < 4.0, "early {early}");
    assert!((late - note_to_hz(71.7)).abs() < 8.0, "late {late}");
}

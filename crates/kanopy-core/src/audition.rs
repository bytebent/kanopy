//! Auditioning one species on its own (design §9, Designer): dry and close.
//!
//! The creature as heard from 1 m: no distance, no reverb, no Ear. Its body moves with the
//! forest's world when Bodies follow is on, since that's how it sounds there; its loudness is
//! the world's after medium compensation, so the same as on Earth.
//!
//! - `call`: a fresh call. `signature`: its signature as a group repeats it (with Variation).
//! - `render`: a motif to mono samples. 80 dB SPL at 1 m is full scale (the forest's reference).
//! - `check`: how much quieter the world's body shift makes it (the below-hearing flag).
use crate::calls::{self, Motif};
use crate::filter::cutoff_max;
use crate::master::REFERENCE_DB;
use crate::rng::Rng;
use crate::scale::Key;
use crate::species::Species;
use crate::voice::Voice;
use crate::wavetable::Wavetables;
use crate::{db_to_amp, BLOCK, FULL_SCALE_SPL};

/// Silence after the last unit, seconds, so releases ring out.
pub const TAIL: f64 = 0.15;

/// A fresh call; `take` counts presses so each one is new.
pub fn call(sp: &Species, key: Option<&Key>, seed: u64, take: u64) -> Motif {
    let mut rng = Rng::stream(seed, 0xA0D0_0000_0000 + take);
    calls::fresh(sp, 0, key, &mut rng)
}

/// The species' signature (the same for the same species, key and seed), repeated with its
/// Variation; `take` counts presses.
pub fn signature(sp: &Species, key: Option<&Key>, seed: u64, take: u64) -> Motif {
    let mut rng = Rng::stream(seed, 0xA0D1_0000_0000);
    let sig = calls::fresh(sp, 0, key, &mut rng);
    let mut rng = Rng::stream(seed, 0xA0D2_0000_0000 + take);
    calls::vary(&sig, sp, sp.calls.variation, sp.calls.units, key, &mut rng)
}

/// The motif as mono samples, from 1 m. `body_ratio` moves every body cutoff (1 = as designed).
pub fn render(m: &Motif, body_ratio: f32, sample_rate: f32) -> Vec<f32> {
    let wt = Wavetables::new(sample_rate);
    render_with(&wt, m, body_ratio, sample_rate)
}

fn render_with(wt: &Wavetables, m: &Motif, body_ratio: f32, sample_rate: f32) -> Vec<f32> {
    let sr = sample_rate as f64;
    let len = ((m.duration() + TAIL) * sr).ceil() as usize;
    let mut out = vec![0f32; len];
    let mut block = [0f32; BLOCK];
    for (at, mut spec) in m.specs() {
        spec.cutoff_start *= body_ratio;
        spec.cutoff_end *= body_ratio;
        let amp = db_to_amp(spec.level_db - FULL_SCALE_SPL + REFERENCE_DB as f32);
        let mut v = Voice::new(spec, sample_rate);
        let mut i = (at * sr).round() as usize;
        while i < len {
            let n = BLOCK.min(len - i);
            let got = v.render(wt, &mut block[..n], 1.0);
            for (o, x) in out[i..i + n].iter_mut().zip(&block[..n]) {
                *o += x * amp;
            }
            i += n;
            if got < n || v.finished() {
                break;
            }
        }
    }
    out
}

/// What the world's body shift does to a species.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Check {
    /// How much the body moves, as a ratio (1 = not at all).
    pub ratio: f64,
    /// Lowest and highest body cutoff there, Hz (before the filter's own limit).
    pub cutoff: (f64, f64),
    /// How much quieter its calls get, dB (0 = no change).
    pub loss_db: f64,
    /// The body sits at the filter's top end for every call.
    pub pinned: bool,
}

/// Compare a few calls with and without the body shift.
pub fn check(sp: &Species, ratio: f64, key: Option<&Key>, seed: u64) -> Check {
    let sr = 48_000.0f32;
    let lo = sp.voice.cutoff.0.min(sp.voice.cutoff.1) * ratio;
    let hi = sp.voice.cutoff.0.max(sp.voice.cutoff.1) * ratio;
    let mut loss_db = 0.0;
    if (ratio - 1.0).abs() > 1e-6 {
        let wt = Wavetables::new(sr);
        let (mut p0, mut p1) = (0.0f64, 0.0f64);
        for take in 0..6 {
            let m = call(sp, key, seed, 1000 + take);
            let pw = |r: f32| {
                render_with(&wt, &m, r, sr)
                    .iter()
                    .map(|x| (*x as f64).powi(2))
                    .sum::<f64>()
            };
            p0 += pw(1.0);
            p1 += pw(ratio as f32);
        }
        loss_db = if p0 > 0.0 {
            (10.0 * (p0 / p1.max(1e-30)).log10()).clamp(0.0, 120.0)
        } else {
            0.0
        };
    }
    Check {
        ratio,
        cutoff: (lo, hi),
        loss_db,
        pinned: lo >= cutoff_max(sr) as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amp_to_db;
    use crate::propagation::Medium;
    use crate::scale::Scale;
    use crate::species::demo;

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |a, v| a.max(v.abs()))
    }

    #[test]
    fn a_call_is_its_own_length_and_level() {
        let sp = &demo(1.0)[5]; // warblers, 65–78 dB
        let m = call(sp, None, 1, 0);
        let x = render(&m, 1.0, 48_000.0);
        assert_eq!(x.len(), ((m.duration() + TAIL) * 48_000.0).ceil() as usize);
        // 80 dB SPL at 1 m is full scale: a 65–78 dB call peaks around −15…−2 dBFS, a little
        // more where the body's resonance peaks (the live path limits it)
        let db = amp_to_db(peak(&x));
        assert!((-20.0..=3.0).contains(&db), "peak {db} dBFS");
        // silent in the tail
        let tail = &x[x.len() - 200..];
        assert!(peak(tail) < 1e-3);
    }

    #[test]
    fn presses_differ_and_repeat() {
        let sp = &demo(1.0)[2];
        let k = Key::new(2, Scale::Minor);
        assert_ne!(call(sp, Some(&k), 1, 0), call(sp, Some(&k), 1, 1));
        assert_eq!(call(sp, Some(&k), 1, 3), call(sp, Some(&k), 1, 3));
        // every note in key and in range
        for u in call(sp, Some(&k), 1, 4).units {
            assert!((sp.voice.pitch.0..=sp.voice.pitch.1).contains(&u.note));
            assert!((k.snap(u.note) - u.note).abs() < 1e-9);
        }
    }

    #[test]
    fn signature_stays_itself_at_variation_zero() {
        let mut sp = demo(1.0)[3].clone();
        sp.calls.variation = 0.0;
        let a = signature(&sp, None, 7, 0);
        assert_eq!(a, signature(&sp, None, 7, 1));
        sp.calls.variation = 1.0;
        let changed = (0..8).filter(|&t| signature(&sp, None, 7, t) != a).count();
        assert!(
            changed >= 6,
            "only {changed} of 8 repeats changed at Variation 1"
        );
    }

    #[test]
    fn water_flags_a_high_body_and_spares_a_low_one() {
        let w = Medium::preset("water").unwrap().body_ratio();
        // a pure tone near 500 Hz with its body at 2–3 kHz: in water the body moves to about
        // 9–13 kHz and takes another 20 dB or more off the tone
        let mut whistler = demo(1.0)[4].clone();
        whistler.voice.shape = (0.0, 0.1);
        whistler.voice.pitch = (70.0, 74.0);
        whistler.voice.cutoff = (2000.0, 3000.0);
        whistler.voice.resonance = (0.1, 0.2);
        // a body far below the pitch: moved ×4.3 it's still below it
        let mut low = whistler.clone();
        low.voice.cutoff = (20.0, 40.0);
        let a = check(&whistler, w, None, 1);
        let b = check(&low, w, None, 1);
        assert!(a.loss_db > 20.0, "whistler loses {}", a.loss_db);
        assert!(b.loss_db < 1.0, "low body loses {}", b.loss_db);
        assert_eq!(check(&whistler, 1.0, None, 1).loss_db, 0.0);
        let swifts = &demo(1.0)[4];
        let mut pinned = swifts.clone();
        pinned.voice.cutoff = (8000.0, 9000.0);
        assert!(check(&pinned, w, None, 1).pinned);
    }
}

#![allow(dead_code)]

pub const SR: f32 = 48_000.0;

/// Amplitude of the component at `f` Hz (a unit sine gives 1.0).
pub fn tone(x: &[f32], f: f32, sr: f32) -> f32 {
    let w = std::f64::consts::TAU * f as f64 / sr as f64;
    let (mut re, mut im) = (0.0f64, 0.0f64);
    let n = x.len();
    for (i, &v) in x.iter().enumerate() {
        // Hann window to keep neighbours out
        let h = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos();
        re += v as f64 * h * (w * i as f64).cos();
        im += v as f64 * h * (w * i as f64).sin();
    }
    (2.0 * (re * re + im * im).sqrt() / (0.5 * n as f64)) as f32
}

pub fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt() as f32
}

pub fn db(a: f32) -> f32 {
    20.0 * a.max(1e-12).log10()
}

/// Frequency from upward zero crossings (good for clean periodic signals).
pub fn zc_freq(x: &[f32], sr: f32) -> f32 {
    let mut first = None;
    let mut last = 0.0;
    let mut count = 0;
    for i in 1..x.len() {
        if x[i - 1] <= 0.0 && x[i] > 0.0 {
            let t = (i - 1) as f32 + x[i - 1] / (x[i - 1] - x[i]);
            if first.is_none() {
                first = Some(t);
            } else {
                count += 1;
            }
            last = t;
        }
    }
    count as f32 * sr / (last - first.unwrap_or(0.0))
}

/// Sine through a process, steady-state amplitude.
pub fn sine(f: f32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| (std::f32::consts::TAU * f * i as f32 / SR).sin())
        .collect()
}

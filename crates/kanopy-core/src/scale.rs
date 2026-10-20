//! Keys (design §6): In key snaps anchor notes to a scale; Free leaves them alone. Also the
//! consonant intervals Harmonise picks from: scale steps in key, simple ratios when Free.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    Major,
    Minor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    PentatonicMajor,
    PentatonicMinor,
    WholeTone,
    Chromatic,
}

impl Scale {
    pub fn steps(self) -> &'static [i32] {
        match self {
            Scale::Major => &[0, 2, 4, 5, 7, 9, 11],
            Scale::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Scale::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Scale::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            Scale::Lydian => &[0, 2, 4, 6, 7, 9, 11],
            Scale::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            Scale::PentatonicMajor => &[0, 2, 4, 7, 9],
            Scale::PentatonicMinor => &[0, 3, 5, 7, 10],
            Scale::WholeTone => &[0, 2, 4, 6, 8, 10],
            Scale::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        }
    }
}

/// A root (0 = C … 11 = B) and a scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub root: i32,
    pub scale: Scale,
}

impl Key {
    pub const fn new(root: i32, scale: Scale) -> Self {
        Self { root, scale }
    }

    /// The note of scale degree `d` (degree 0 = the root in MIDI octave −1, i.e. note `root`).
    pub fn note(&self, d: i64) -> f64 {
        let s = self.scale.steps();
        let n = s.len() as i64;
        let (oct, i) = (d.div_euclid(n), d.rem_euclid(n));
        (self.root as i64 + 12 * oct + s[i as usize] as i64) as f64
    }

    /// The nearest scale degree to a note (ties go down).
    pub fn degree(&self, note: f64) -> i64 {
        let s = self.scale.steps();
        let n = s.len() as i64;
        let approx = ((note - self.root as f64) / 12.0 * n as f64).floor() as i64;
        let mut best = approx;
        let mut best_d = f64::INFINITY;
        for d in approx - n - 1..=approx + n + 1 {
            let dist = (self.note(d) - note).abs();
            if dist < best_d - 1e-9 {
                best_d = dist;
                best = d;
            }
        }
        best
    }

    pub fn snap(&self, note: f64) -> f64 {
        self.note(self.degree(note))
    }

    /// Snap, staying inside lo…hi when the scale has a note there.
    pub fn snap_within(&self, note: f64, lo: f64, hi: f64) -> f64 {
        let mut d = self.degree(note.clamp(lo, hi));
        if self.note(d) > hi + 1e-9 {
            d -= 1;
        }
        if self.note(d) < lo - 1e-9 {
            d += 1;
        }
        let n = self.note(d);
        if n < lo - 1e-9 || n > hi + 1e-9 {
            note.clamp(lo, hi)
        } else {
            n
        }
    }

    pub fn contains(&self, note: f64) -> bool {
        (self.snap(note) - note).abs() < 1e-6
    }
}

/// Consonant intervals in semitones, up and down: unison, 3rds, 4th, 5th, 6ths, octave, and their
/// octave-up forms. In key they are equal-tempered; Free uses simple ratios (just intonation).
pub fn consonances(key: Option<&Key>) -> Vec<f64> {
    let tempered = [0.0, 3.0, 4.0, 5.0, 7.0, 8.0, 9.0, 12.0];
    let just: Vec<f64> = [
        1.0,
        6.0 / 5.0,
        5.0 / 4.0,
        4.0 / 3.0,
        3.0 / 2.0,
        8.0 / 5.0,
        5.0 / 3.0,
        2.0,
    ]
    .iter()
    .map(|r: &f64| 12.0 * r.log2())
    .collect();
    let base: Vec<f64> = if key.is_some() {
        tempered.to_vec()
    } else {
        just
    };
    let mut out = Vec::new();
    for oct in [0.0, 12.0, 24.0] {
        for &i in &base {
            for v in [i + oct, -(i + oct)] {
                if !out.iter().any(|o: &f64| (o - v).abs() < 1e-9) {
                    out.push(v);
                }
            }
        }
    }
    out.sort_by(f64::total_cmp);
    out
}

/// Is this interval (semitones) consonant, for tests: within 0.1 semitone of a consonance.
pub fn is_consonant(interval: f64, key: Option<&Key>) -> bool {
    consonances(key).iter().any(|c| (c - interval).abs() < 0.1)
}

pub fn note_name(n: f64) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let r = n.round() as i64;
    let cents = ((n - r as f64) * 100.0).round() as i64;
    let base = format!(
        "{}{}",
        NAMES[r.rem_euclid(12) as usize],
        r.div_euclid(12) - 1
    );
    if cents == 0 {
        base
    } else {
        format!("{base}{cents:+}")
    }
}

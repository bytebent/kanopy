//! Batch 03: calls, hearing, answers, chorus, harmonise, mimicry, the cap.

use kanopy_core::calls::{self, Motif};
use kanopy_core::life::{Life, Zone};
use kanopy_core::rng::Rng;
use kanopy_core::scale::{is_consonant, Key, Scale};
use kanopy_core::scene::{Scene, SceneSettings, Source};
use kanopy_core::species::*;
use kanopy_core::world::{Forest, ForestSettings, Rect};
use kanopy_core::Vec2;

const D_MINOR: Key = Key::new(2, Scale::Minor);

fn forest() -> Forest {
    Forest::generate(ForestSettings {
        seed: 7,
        density: 0.5,
        tree_variation: 0.5,
        liquid: false,
    })
}

/// Walkers standing still at (x, 0), all one group.
fn at(name: &str, x: f64, n: usize) -> Species {
    Species::new(
        name,
        Habitat {
            area: Rect::new(x, x, 0.0, 0.0),
            individuals: n,
            group_size: (n as u32, n as u32),
            cohesion: 1.0,
            ..Default::default()
        },
    )
}

fn settings(key: Option<Key>) -> SceneSettings {
    let listener = Vec2::new(100.0, 1.5);
    SceneSettings {
        key,
        listener,
        zone: Zone::Around {
            pos: listener,
            radius: 1000.0,
        },
        ..Default::default()
    }
}

fn scene(species: Vec<Species>, key: Option<Key>) -> Scene {
    let mut s = Scene::new(Life::new(forest(), species), settings(key));
    s.keep_score = true;
    s
}

// ---------- keys

#[test]
fn keys_snap_and_count_degrees() {
    let k = D_MINOR;
    for d in -20..40 {
        assert_eq!(k.degree(k.note(d)), d);
    }
    assert_eq!(k.snap(62.0), 62.0); // D
    assert_eq!(k.snap(66.0), 65.0); // F# → F (tie goes down)
    assert!(k.contains(70.0) && !k.contains(71.0)); // A# yes, B no
    assert_eq!(k.snap_within(73.0, 60.0, 72.0), 72.0);
    assert!(
        is_consonant(7.0, Some(&k))
            && is_consonant(-15.0, Some(&k))
            && !is_consonant(6.0, Some(&k))
    );
    assert!(is_consonant(7.02, None) && is_consonant(3.86, None) && !is_consonant(6.0, None));
}

// ---------- calls

fn chirper() -> Species {
    at("chirper", 50.0, 1).with(
        Voice {
            shape: (0.5, 2.0),
            cutoff: (200.0, 900.0),
            resonance: (0.1, 0.6),
            pitch: (70.0, 86.0),
            contour: (-3.0, 3.0),
            attack: (0.005, 0.02),
            release: (0.02, 0.06),
            level: (60.0, 72.0),
        },
        Calls {
            units: (3, 9),
            unit_length: (0.05, 0.2),
            gap: (0.02, 0.1),
            rest: (3.0, 6.0),
            evolve: Evolve::Arc,
            evolve_amount: (0.1, 0.5),
            signature: 0.0,
            variation: 0.0,
        },
        Behaviour {
            talkative: 1.0,
            response: 0.0,
            ..Default::default()
        },
    )
}

#[test]
fn fresh_calls_stay_in_their_ranges() {
    let sp = chirper();
    let mut rng = Rng::new(1);
    for key in [Some(D_MINOR), None] {
        let mut whole = 0;
        let mut total = 0;
        for _ in 0..500 {
            let m = calls::fresh(&sp, 0, key.as_ref(), &mut rng);
            assert!((3..=9).contains(&m.units.len()));
            for (k, u) in m.units.iter().enumerate() {
                assert!(u.dur >= 0.05 - 1e-9 && u.dur <= 0.2 + 1e-9);
                assert!(u.note >= 70.0 - 1e-9 && u.note <= 86.0 + 1e-9);
                assert!(
                    u.shape >= 0.5
                        && u.shape <= 2.0
                        && u.cutoff >= 200.0 - 1e-6
                        && u.cutoff <= 900.0 + 1e-6
                );
                assert!(
                    u.level >= 60.0 && u.level <= 72.0 && u.contour >= -3.0 && u.contour <= 3.0
                );
                if k > 0 {
                    let prev = &m.units[k - 1];
                    let gap = u.at - prev.at - prev.dur;
                    assert!(gap >= 0.02 - 1e-9 && gap <= 0.1 + 1e-9, "gap {gap}");
                }
                if let Some(k) = key {
                    assert!(k.contains(u.note), "{} not in D minor", u.note);
                }
                whole += (u.note.fract() == 0.0) as usize;
                total += 1;
            }
        }
        if key.is_none() {
            assert!(whole < total / 10, "free notes aren't quantised");
        }
    }
}

#[test]
fn evolve_shapes_the_call() {
    let mut rise = chirper();
    rise.calls.evolve = Evolve::Rise;
    rise.calls.evolve_amount = (0.5, 0.5);
    rise.calls.units = (6, 6);
    let mut rng = Rng::new(2);
    for _ in 0..100 {
        let m = calls::fresh(&rise, 0, None, &mut rng);
        assert!(m.units[5].note - m.units[0].note > 7.0);
    }
}

fn same_notes(a: &Motif, b: &Motif) -> bool {
    a.units.len() == b.units.len()
        && a.units
            .iter()
            .zip(&b.units)
            .all(|(x, y)| (x.note - y.note).abs() < 1e-9 && (x.dur - y.dur).abs() < 1e-9)
}

#[test]
fn signatures_repeat_and_vary() {
    for (variation, want_same) in [(0.0, true), (1.0, false)] {
        let mut sp = chirper();
        sp.habitat.individuals = 4;
        sp.habitat.group_size = (4, 4);
        sp.calls.signature = 1.0;
        sp.calls.variation = variation;
        let mut s = scene(vec![sp], Some(D_MINOR));
        s.run(120.0);
        let sig = s.signature(0).clone();
        let same = s
            .score
            .iter()
            .filter(|c| same_notes(&c.motif, &sig))
            .count();
        assert!(s.score.len() > 40);
        assert!(s.score.iter().all(|c| c.source == Source::Signature));
        if want_same {
            assert_eq!(
                same,
                s.score.len(),
                "Variation 0: every call is the signature"
            );
        } else {
            assert!(
                same * 5 < s.score.len(),
                "Variation 1: {same} of {} unchanged",
                s.score.len()
            );
        }
    }
    // different groups, different signatures
    let mut sp = chirper();
    sp.habitat.area = Rect::new(0.0, 200.0, 0.0, 0.0);
    sp.habitat.individuals = 10;
    sp.habitat.group_size = (1, 1);
    let s = scene(vec![sp], Some(D_MINOR));
    let distinct = (0..10)
        .filter(|&g| {
            (0..10)
                .filter(|&h| h != g)
                .all(|h| !same_notes(s.signature(g), s.signature(h)))
        })
        .count();
    assert!(distinct >= 9);
}

// ---------- copies

fn source() -> Species {
    chirper()
}

fn mimic(chance: f64, accuracy: f64, timbre: f64, pitch: f64) -> Species {
    let mut m = chirper();
    m.name = "mimic".into();
    m.voice = Voice {
        shape: (0.0, 3.0),
        cutoff: (100.0, 2000.0),
        resonance: (0.0, 1.0),
        pitch: (60.0, 96.0),
        ..m.voice
    };
    m.behaviour = Behaviour {
        talkative: 1.0,
        mimicry_chance: chance,
        mimicry_accuracy: accuracy,
        timbre_match: timbre,
        pitch_match: pitch,
        ..Default::default()
    };
    m
}

#[test]
fn a_perfect_mimic_sounds_like_the_caller() {
    let (a, b) = (source(), mimic(1.0, 1.0, 1.0, 1.0));
    let mut rng = Rng::new(3);
    for key in [Some(D_MINOR), None] {
        for _ in 0..100 {
            let m = calls::fresh(&a, 0, key.as_ref(), &mut rng);
            let c = calls::copy(&m, &a, &b, 1, key.as_ref(), &mut rng);
            assert_eq!(c.species, 1);
            assert_eq!(c.units.len(), m.units.len());
            for (x, y) in m.units.iter().zip(&c.units) {
                assert!(
                    (x.note - y.note).abs() < 1e-9,
                    "pitch {} vs {}",
                    x.note,
                    y.note
                );
                assert!(
                    (x.shape - y.shape).abs() < 1e-9
                        && (x.cutoff - y.cutoff).abs() < 1e-6
                        && (x.resonance - y.resonance).abs() < 1e-9
                );
                assert!(
                    (x.dur - y.dur).abs() < 1e-9,
                    "same timing ranges, same timing"
                );
            }
        }
    }
}

#[test]
fn pitch_match_zero_keeps_steps_in_own_range() {
    let a = source(); // 70–86
    let mut b = mimic(1.0, 1.0, 0.0, 0.0);
    b.voice.pitch = (40.0, 56.0);
    let mut rng = Rng::new(4);
    let k = D_MINOR;
    for _ in 0..100 {
        let m = calls::fresh(&a, 0, Some(&k), &mut rng);
        let c = calls::copy(&m, &a, &b, 1, Some(&k), &mut rng);
        let steps = |x: &Motif| {
            x.units
                .iter()
                .map(|u| k.degree(u.note) - k.degree(x.units[0].note))
                .collect::<Vec<_>>()
        };
        assert!(c.units.iter().all(|u| u.note >= 40.0 && u.note <= 56.0));
        // the shape of the melody survives where the range allows
        let (sa, sc) = (steps(&m), steps(&c));
        let span = sa.iter().max().unwrap() - sa.iter().min().unwrap();
        if span <= 7 {
            assert_eq!(sa, sc);
        }
    }
}

#[test]
fn timbre_match_zero_keeps_own_sound() {
    let a = source(); // shape 0.5–2
    let mut b = mimic(1.0, 1.0, 0.0, 1.0);
    b.voice.shape = (2.5, 3.0);
    let mut rng = Rng::new(5);
    for _ in 0..50 {
        let m = calls::fresh(&a, 0, None, &mut rng);
        let c = calls::copy(&m, &a, &b, 1, None, &mut rng);
        assert!(c.units.iter().all(|u| u.shape >= 2.5));
        // gesture: a unit high in the caller's range is high in the copier's
        for (x, y) in m.units.iter().zip(&c.units) {
            let px = (x.shape - 0.5) / 1.5;
            let py = (y.shape - 2.5) / 0.5;
            assert!((px - py).abs() < 1e-9);
        }
    }
}

#[test]
fn low_accuracy_copies_drift() {
    let a = source();
    let mut rng = Rng::new(6);
    let differs = |acc: f64, rng: &mut Rng| {
        let b = mimic(1.0, acc, 1.0, 1.0);
        (0..200)
            .filter(|_| {
                let m = calls::fresh(&a, 0, None, rng);
                !same_notes(&m, &calls::copy(&m, &a, &b, 1, None, rng))
            })
            .count()
    };
    assert_eq!(differs(1.0, &mut rng), 0);
    let low = differs(0.0, &mut rng);
    assert!(low > 150, "{low} of 200 changed at accuracy 0");
}

#[test]
fn mimics_copy_what_they_hear() {
    let mut a = source();
    a.calls.signature = 1.0;
    a.calls.variation = 0.0;
    let b = {
        let mut b = mimic(1.0, 1.0, 1.0, 1.0);
        b.habitat.area = Rect::new(60.0, 60.0, 0.0, 0.0);
        b
    };
    let mut s = scene(vec![a, b], None);
    s.run(120.0);
    let sig = s.signature(0).clone();
    let copies: Vec<_> = s.score.iter().filter(|c| c.species == 1).collect();
    assert!(!copies.is_empty());
    let first_a = s.score.iter().find(|c| c.species == 0).unwrap().start;
    for c in &copies {
        if c.start > first_a + 1.0 {
            assert_eq!(c.source, Source::Mimic { of: 0 });
            assert!(
                same_notes(&c.motif, &sig),
                "a perfect copy of the only thing it heard"
            );
        }
    }
    // chance 0: never
    let b0 = mimic(0.0, 1.0, 1.0, 1.0);
    let mut s = scene(vec![source(), b0], None);
    s.run(60.0);
    assert_eq!(s.stats.mimics, 0);
}

// ---------- hearing

#[test]
fn only_what_they_can_hear() {
    // a mimic copies only calls that reached it above its Hearing level
    let copies = |d: f64| {
        let mut a = at("a", 20.0, 1);
        a.voice.level = (70.0, 70.0);
        a.voice.pitch = (60.0, 60.0);
        a.behaviour.talkative = 1.0;
        a.calls.rest = (1.0, 2.0);
        let mut b = at("b", 20.0 + d, 1);
        b.behaviour = Behaviour {
            talkative: 1.0,
            mimicry_chance: 1.0,
            hearing: 30.0,
            ..Default::default()
        };
        let mut s = scene(vec![a, b], None);
        s.run(60.0);
        s.stats.mimics
    };
    // 70 dB at 1 m reaches 30 dB at 100 m (air absorption at middle C is tiny)
    assert!(copies(95.0) > 5);
    assert_eq!(copies(105.0), 0);
    let r = kanopy_core::scene::reach(70.0, 30.0, 0.0);
    assert!((r - 100.0).abs() < 1e-6, "{r}");
    assert!(kanopy_core::scene::reach(70.0, 30.0, 0.05) < 80.0);
}

#[test]
fn answers_come_after_the_call_arrives() {
    let mut a = at("a", 10.0, 1);
    a.behaviour = Behaviour {
        talkative: 1.0,
        response: 1.0,
        ..Default::default()
    };
    a.calls.rest = (2.0, 3.0);
    let mut b = at("a2", 180.0, 1);
    b.behaviour = Behaviour {
        talkative: 0.0,
        response: 1.0,
        hearing: 0.0,
        ..Default::default()
    };
    // same species: two individuals 170 m apart
    let mut sp = a.clone();
    sp.habitat.area = Rect::new(0.0, 200.0, 0.0, 0.0);
    sp.habitat.individuals = 2;
    sp.habitat.group_size = (1, 1);
    sp.habitat.spread = Spread::Even;
    sp.behaviour.hearing = 0.0;
    sp.behaviour.talkative = 0.2;
    let _ = b;
    let mut s = scene(vec![sp], None);
    s.run(300.0);
    let travel = s.life.animals[0].pos.dist(s.life.animals[1].pos) / 343.0;
    assert!(travel > 0.25, "{travel}");
    let mut answered = 0;
    for (i, c) in s.score.iter().enumerate() {
        if !c.answer {
            continue;
        }
        let (who, start) = c.reply_to.expect("an answer says what it answers");
        let prev = s.score[..i]
            .iter()
            .rev()
            .find(|p| p.who == who && p.start == start)
            .expect("the call it answers");
        assert_ne!(who, c.who);
        let gap = c.start - (prev.start + prev.motif.duration());
        assert!(
            gap >= travel + 0.2 - 1e-6,
            "answered {gap:.3} s after the call ended; it takes {travel:.3} s to arrive"
        );
        answered += 1;
    }
    assert!(answered > 5);
}

#[test]
fn only_their_own_species_answer() {
    let mut a = at("a", 50.0, 6);
    a.behaviour = Behaviour {
        talkative: 1.0,
        response: 1.0,
        ..Default::default()
    };
    let mut b = at("b", 52.0, 6);
    b.behaviour = Behaviour {
        talkative: 0.0,
        response: 1.0,
        ..Default::default()
    };
    let mut s = scene(vec![a, b], None);
    s.run(120.0);
    assert!(s.stats.answers > 10);
    assert!(
        s.score.iter().all(|c| c.species == 0),
        "b only answers b, and b never starts"
    );
    // at most one answer per call: sparse callers in one spot, so answers are easy to pair up
    let mut few = at("few", 50.0, 6);
    few.calls = Calls {
        units: (1, 1),
        unit_length: (0.2, 0.2),
        rest: (20.0, 40.0),
        signature: 0.0,
        ..Default::default()
    };
    few.behaviour = Behaviour {
        talkative: 1.0,
        response: 1.0,
        ..Default::default()
    };
    let mut s = scene(vec![few], None);
    s.run(600.0);
    let calls = &s.score;
    let mut most = 0;
    for c in calls {
        let n = calls
            .iter()
            .filter(|a| a.reply_to == Some((c.who, c.start)))
            .count();
        most = most.max(n);
    }
    assert!(s.stats.answers > 20);
    assert_eq!(most, 1, "a call got {most} answers");
    let mut quiet = at("q", 50.0, 6);
    quiet.behaviour = Behaviour {
        talkative: 1.0,
        response: 0.0,
        ..Default::default()
    };
    let mut s = scene(vec![quiet], None);
    s.run(60.0);
    assert_eq!(s.stats.answers, 0);
}

// ---------- chorus

fn chorus_scene(chorus: f64) -> Scene {
    let mut sp = at("frog", 80.0, 8);
    sp.calls = Calls {
        units: (1, 1),
        unit_length: (0.25, 0.25),
        gap: (0.1, 0.1),
        rest: (3.0, 9.0),
        signature: 0.0,
        ..Default::default()
    };
    sp.behaviour = Behaviour {
        talkative: 1.0,
        response: 0.0,
        chorus,
        ..Default::default()
    };
    sp.habitat.group_size = (1, 1); // eight neighbours, not one family
    let mut s = scene(vec![sp], None);
    s.run(900.0);
    s
}

/// Share of calls that start while another one is sounding, and share starting within 0.3 s
/// after another's start.
fn overlap_and_bunching(s: &Scene) -> (f64, f64) {
    let calls = &s.score;
    let (mut over, mut bunch) = (0, 0);
    for (i, c) in calls.iter().enumerate() {
        let earlier = calls[..i]
            .iter()
            .rev()
            .take_while(|p| c.start - p.start < 5.0)
            .filter(|p| p.who != c.who);
        let (mut o, mut b) = (false, false);
        for p in earlier {
            o |= c.start < p.start + p.motif.duration();
            b |= c.start - p.start < 0.3;
        }
        over += o as usize;
        bunch += b as usize;
    }
    (
        over as f64 / calls.len() as f64,
        bunch as f64 / calls.len() as f64,
    )
}

#[test]
fn chorus_synchronises_or_alternates() {
    let (free_o, free_b) = overlap_and_bunching(&chorus_scene(0.0));
    let (sync_o, sync_b) = overlap_and_bunching(&chorus_scene(1.0));
    let (alt_o, _) = overlap_and_bunching(&chorus_scene(-1.0));
    assert!(
        sync_b > 2.0 * free_b,
        "bunching: synchronise {sync_b:.2}, free {free_b:.2}"
    );
    assert!(sync_o > free_o);
    assert!(
        alt_o < 0.5 * free_o,
        "overlap: alternate {alt_o:.3}, free {free_o:.3}"
    );
}

// ---------- harmonise

#[test]
fn harmonised_calls_are_consonant_with_mates() {
    for key in [Some(D_MINOR), None] {
        let mut sp = at("choir", 70.0, 6);
        sp.calls = Calls {
            units: (1, 2),
            unit_length: (0.6, 1.2),
            gap: (0.1, 0.2),
            rest: (2.0, 4.0),
            signature: 0.0,
            ..Default::default()
        };
        sp.voice.pitch = (60.0, 84.0);
        sp.behaviour = Behaviour {
            talkative: 1.0,
            response: 0.0,
            harmonise: 1.0,
            ..Default::default()
        };
        let mut s = scene(vec![sp], key);
        s.run(300.0);
        let calls = &s.score;
        let mut checked = 0;
        for (i, c) in calls.iter().enumerate().filter(|(_, c)| c.harmonised) {
            // the last group mate it heard sounding (same spot, so no travel time)
            let p = calls[..i]
                .iter()
                .rev()
                .find(|p| p.who != c.who && p.start + p.motif.duration() > c.start - 1e-9);
            if let Some(p) = p {
                let iv = c.motif.first_note() - p.motif.first_note();
                assert!(is_consonant(iv, key.as_ref()), "{iv:.2} semitones");
                checked += 1;
            }
        }
        assert!(checked > 30, "{checked}");
        assert!(s.stats.harmonised * 2 > s.stats.calls / 3);
    }
    let mut solo = at("solo", 70.0, 6);
    solo.behaviour = Behaviour {
        talkative: 1.0,
        harmonise: 0.0,
        ..Default::default()
    };
    let mut s = scene(vec![solo], None);
    s.run(60.0);
    assert_eq!(s.stats.harmonised, 0);
}

// ---------- zone and cap

#[test]
fn only_the_active_zone_calls() {
    let mut s = Scene::new(
        Life::new(forest(), demo(0.5)),
        SceneSettings {
            zone: Zone::Around {
                pos: Vec2::new(100.0, 1.5),
                radius: 30.0,
            },
            ..Default::default()
        },
    );
    s.keep_score = true;
    s.run(60.0);
    assert!(s.stats.calls > 50 && s.stats.skipped_inactive > 0);
    for c in &s.score {
        assert!(
            c.pos.dist(Vec2::new(100.0, 1.5)) <= 30.0 + 0.5,
            "called from {:.1} m",
            c.pos.dist(Vec2::new(100.0, 1.5))
        );
    }
}

#[test]
fn the_cap_holds_and_favours_the_near() {
    let listener = Vec2::new(100.0, 1.5);
    let radius = 150.0;
    let mut s = Scene::new(
        Life::new(forest(), demo(1.0)),
        SceneSettings {
            cap: 20,
            zone: Zone::Around {
                pos: listener,
                radius,
            },
            ..Default::default()
        },
    );
    s.keep_score = true;
    s.run(60.0);
    assert!(s.stats.max_calling <= 20);
    assert!(s.stats.skipped_cap > 100);
    let mean_call =
        s.score.iter().map(|c| c.pos.dist(listener)).sum::<f64>() / s.score.len() as f64;
    let mean_all = s
        .life
        .animals
        .iter()
        .map(|a| a.pos.dist(listener))
        .sum::<f64>()
        / s.life.animals.len() as f64;
    assert!(
        mean_call < 0.8 * mean_all,
        "callers {mean_call:.1} m, everyone {mean_all:.1} m"
    );
}

// ---------- repeatability

#[test]
fn same_seed_same_calls_and_sound() {
    let run = |seed: u64| {
        let f = Forest::generate(ForestSettings {
            seed,
            ..Default::default()
        });
        let mut s =
            Scene::new(Life::new(f, demo(0.3)), SceneSettings::default()).with_sound(48_000.0);
        s.keep_score = true;
        let n = s.step_samples();
        let (mut l, mut r) = (vec![0f32; n * 200], vec![0f32; n * 200]);
        for k in 0..200 {
            s.step_audio(&mut l[k * n..(k + 1) * n], &mut r[k * n..(k + 1) * n]);
        }
        let calls: Vec<(u32, u64, u64)> = s
            .score
            .iter()
            .map(|c| (c.who, c.start.to_bits(), c.motif.first_note().to_bits()))
            .collect();
        let bits: Vec<u32> = l.iter().chain(&r).map(|x| x.to_bits()).collect();
        (calls, bits)
    };
    let (a, b) = (run(5), run(5));
    assert!(!a.0.is_empty());
    assert_eq!(a, b);
    assert_ne!(a.0, run(6).0);
}

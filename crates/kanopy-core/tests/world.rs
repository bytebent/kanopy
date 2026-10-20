//! Batch 02: the forest, placement and movement.

use kanopy_core::life::*;
use kanopy_core::species::*;
use kanopy_core::world::*;
use kanopy_core::Vec2;

fn forest(seed: u64, density: f64, variation: f64) -> Forest {
    Forest::generate(ForestSettings {
        seed,
        density,
        tree_variation: variation,
        liquid: false,
    })
}

fn one(h: Habitat) -> Life {
    Life::new(forest(3, 0.6, 0.5), vec![Species::new("test", h)])
}

fn check_all(life: &Life) {
    for i in 0..life.animals.len() {
        if let Err(e) = life.check(i) {
            panic!("t = {:.1} s: {e}", life.time);
        }
    }
}

#[test]
fn layers() {
    assert_eq!(Layer::at(-2.5), Layer::Soil);
    assert_eq!(Layer::at(2.5), Layer::Shrubs);
    assert_eq!(Layer::at(0.2), Layer::Floor);
    assert_eq!(Layer::at(1.0), Layer::Shrubs);
    assert_eq!(Layer::at(3.0), Layer::Understorey);
    assert_eq!(Layer::at(10.0), Layer::Subcanopy);
    assert_eq!(Layer::at(20.0), Layer::Canopy);
    assert_eq!(Layer::at(40.0), Layer::Emergent);
    assert_eq!(Layer::at(50.0), Layer::Emergent);
}

#[test]
fn trees_follow_density_and_variation() {
    for seed in 1..20 {
        for (d, v) in [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)] {
            let f = forest(seed, d, v);
            assert_eq!(f.trees.len(), tree_count(d));
            assert_eq!(
                f.trees[f.tallest].height, TOP,
                "the tallest tree reaches the top"
            );
            for t in &f.trees {
                assert!(t.x > 0.0 && t.x < WIDTH);
                assert!(t.height <= TOP && t.crown_bottom < t.height && t.crown_bottom > 0.0);
                assert!(t.crown_half_width >= 1.5);
            }
        }
        let even = forest(seed, 0.5, 0.0);
        for (i, t) in even.trees.iter().enumerate() {
            if i != even.tallest {
                assert_eq!(t.height, STAND_HEIGHT, "variation 0: an even stand");
            }
        }
    }
    assert!(tree_count(0.0) < tree_count(0.5) && tree_count(0.5) < tree_count(1.0));
    assert!(forest(1, 1.0, 0.5).shrub_fullness > forest(1, 0.0, 0.5).shrub_fullness);
    // more variation, more spread in heights
    let spread = |v: f64| {
        let f = forest(5, 1.0, v);
        let hs: Vec<f64> = f
            .trees
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != f.tallest)
            .map(|(_, t)| t.height)
            .collect();
        let m = hs.iter().sum::<f64>() / hs.len() as f64;
        (hs.iter().map(|h| (h - m).powi(2)).sum::<f64>() / hs.len() as f64).sqrt()
    };
    assert!(spread(1.0) > spread(0.3) && spread(0.3) > 0.0);
}

#[test]
fn crowns() {
    let t = Tree {
        x: 50.0,
        height: 30.0,
        crown_bottom: 14.0,
        crown_half_width: 6.0,
    };
    assert!(t.in_crown(Vec2::new(50.0, 22.0), 0.0));
    assert!(!t.in_crown(Vec2::new(56.5, 22.0), 0.0));
    assert!(t.crown_meets(&Rect::new(0.0, 200.0, 25.0, 40.0)));
    assert!(!t.crown_meets(&Rect::new(0.0, 200.0, 31.0, 40.0)));
    let mut rng = kanopy_core::rng::Rng::new(1);
    let r = Rect::new(0.0, 200.0, 25.0, 40.0);
    for _ in 0..200 {
        let p = t.crown_point(&r, &mut rng);
        assert!(t.in_crown(p, 1e-9) && r.contains(p, 1e-9));
    }
    let near = Tree { x: 60.0, ..t };
    let far = Tree { x: 80.0, ..t };
    assert!(t.touches(&near) && !t.touches(&far));
}

#[test]
fn same_seed_same_forest() {
    let run = |seed: u64| {
        let mut l = Life::new(forest(seed, 0.6, 0.6), demo(1.0));
        l.run(120.0);
        l.animals
            .iter()
            .flat_map(|a| [a.pos.x.to_bits(), a.pos.y.to_bits()])
            .collect::<Vec<_>>()
    };
    assert_eq!(run(9), run(9));
    assert_ne!(run(9), run(10));
}

#[test]
fn counts_and_group_sizes() {
    let l = Life::new(forest(2, 0.5, 0.5), demo(1.0));
    for (si, sp) in l.species.iter().enumerate() {
        let n = l
            .animals
            .iter()
            .filter(|a| a.species as usize == si)
            .count();
        assert_eq!(n, sp.habitat.individuals, "{}", sp.name);
        let groups: Vec<&Group> = l
            .groups
            .iter()
            .filter(|g| g.species as usize == si)
            .collect();
        let (lo, hi) = sp.habitat.group_size;
        for (k, g) in groups.iter().enumerate() {
            assert!(g.len as u32 <= hi);
            if k + 1 < groups.len() {
                assert!(g.len as u32 >= lo, "only the last group may be short");
            }
        }
    }
}

#[test]
fn everyone_stays_where_they_live() {
    for (seed, d, v) in [(1, 0.0, 0.2), (2, 0.5, 0.5), (3, 1.0, 1.0)] {
        let mut l = Life::new(forest(seed, d, v), demo(1.0));
        check_all(&l);
        for _ in 0..12 {
            l.run(50.0);
            check_all(&l);
        }
    }
}

#[test]
fn area_limits_x_too() {
    let mut l = one(Habitat {
        area: Rect::new(60.0, 90.0, 0.0, 3.0),
        individuals: 60,
        cohesion: 0.0,
        ..Default::default()
    });
    for _ in 0..20 {
        l.run(30.0);
        assert!(l.animals.iter().all(|a| a.pos.x >= 60.0 && a.pos.x <= 90.0));
    }
}

#[test]
fn walkers_and_burrowers_heights() {
    let mut l = Life::new(forest(4, 0.5, 0.5), demo(1.0));
    l.run(300.0);
    for a in &l.animals {
        match l.species[a.species as usize].habitat.movement {
            Movement::Walker => assert!(a.pos.y >= 0.0 && a.pos.y <= 3.0),
            Movement::Burrower => assert!(a.pos.y >= -3.0 && a.pos.y <= 0.0),
            _ => {}
        }
    }
}

fn anchors_x(l: &Life) -> Vec<f64> {
    let mut xs: Vec<f64> = l.groups.iter().map(|g| g.anchor.x).collect();
    xs.sort_by(f64::total_cmp);
    xs
}

fn gap_cv(xs: &[f64]) -> f64 {
    let gaps: Vec<f64> = xs.windows(2).map(|w| w[1] - w[0]).collect();
    let m = gaps.iter().sum::<f64>() / gaps.len() as f64;
    (gaps.iter().map(|g| (g - m).powi(2)).sum::<f64>() / gaps.len() as f64).sqrt() / m
}

#[test]
fn spread_even_random_clumped() {
    let base = Habitat {
        individuals: 40,
        group_size: (1, 1),
        spacing: 3.0,
        ..Default::default()
    };
    let even = one(Habitat {
        spread: Spread::Even,
        ..base
    });
    let random = one(Habitat {
        spread: Spread::Random,
        ..base
    });
    let clumped = one(Habitat {
        spread: Spread::Clumped,
        ..base
    });
    let (e, r, c) = (
        gap_cv(&anchors_x(&even)),
        gap_cv(&anchors_x(&random)),
        gap_cv(&anchors_x(&clumped)),
    );
    assert!(e < 0.25, "even {e}");
    assert!(
        c > 2.0 * e && c > r * 0.9,
        "clumped {c}, random {r}, even {e}"
    );
    // Random keeps Spacing when there's room for it
    let xs = anchors_x(&one(Habitat {
        spread: Spread::Random,
        individuals: 20,
        spacing: 5.0,
        ..base
    }));
    assert!(xs.windows(2).all(|w| w[1] - w[0] >= 5.0 - 1e-9), "{xs:?}");
}

#[test]
fn spread_stays_in_the_area() {
    for spread in [Spread::Even, Spread::Random, Spread::Clumped] {
        let l = one(Habitat {
            area: Rect::new(120.0, 180.0, 0.0, 3.0),
            spread,
            individuals: 30,
            ..Default::default()
        });
        assert!(l
            .groups
            .iter()
            .all(|g| g.anchor.x >= 120.0 && g.anchor.x <= 180.0));
    }
}

#[test]
fn cohesion_keeps_groups_together() {
    let spread = |c: f64| {
        let mut l = one(Habitat {
            individuals: 200,
            group_size: (10, 10),
            cohesion: c,
            spacing: 0.0,
            ..Default::default()
        });
        l.run(300.0);
        let d: f64 = l
            .animals
            .iter()
            .map(|a| (a.pos.x - l.groups[a.group as usize].anchor.x).abs())
            .sum();
        d / l.animals.len() as f64
    };
    let (tight, loose) = (spread(0.95), spread(0.05));
    assert!(tight < 2.0 && loose > 6.0, "tight {tight}, loose {loose}");
}

fn flyer(share: f64, perch: Perch, flock: bool, group: u32) -> Habitat {
    Habitat {
        movement: Movement::Flyer,
        area: Rect::new(0.0, WIDTH, 0.0, TOP),
        individuals: 200,
        group_size: (group, group),
        flock,
        flight: Flight {
            band: (10.0, 40.0),
            perch,
            share,
        },
        ..Default::default()
    }
}

#[test]
fn flight_share_matches() {
    for share in [0.2, 0.5, 0.8] {
        let mut l = one(flyer(share, Perch::At(Layer::Canopy), false, 1));
        l.run(120.0);
        let mut sum = 0.0;
        let samples = 180;
        for _ in 0..samples {
            l.run(10.0);
            sum += flying_share(&l, 0);
        }
        let got = sum / samples as f64;
        assert!(
            (got - share).abs() < 0.08,
            "share {share}: measured {got:.3}"
        );
    }
    let mut never = one(flyer(0.3, Perch::Never, false, 1));
    for _ in 0..30 {
        never.run(20.0);
        assert_eq!(flying_share(&never, 0), 1.0);
    }
}

#[test]
fn perches_are_in_their_layer() {
    for (layer, lo, hi) in [
        (Layer::Canopy, 15.0, 30.0),
        (Layer::Shrubs, 0.5, 3.0),
        (Layer::Floor, 0.0, 0.0),
    ] {
        let mut l = one(flyer(0.4, Perch::At(layer), false, 1));
        for _ in 0..10 {
            l.run(30.0);
            for a in &l.animals {
                if let Mode::Perched { .. } = a.mode {
                    assert!(
                        a.pos.y >= lo - 1e-9 && a.pos.y <= hi + 1e-9,
                        "{layer:?}: perched at {}",
                        a.pos.y
                    );
                }
                if let Mode::Flying { .. } = a.mode {
                    assert!(a.pos.y >= 10.0 - 1e-9 && a.pos.y <= 40.0 + 1e-9);
                }
            }
        }
        check_all(&l);
    }
}

fn alignment(l: &Life) -> f64 {
    let mut total = 0.0;
    for g in &l.groups {
        let (mut sx, mut sy) = (0.0, 0.0);
        for a in &l.animals[g.first..g.first + g.len] {
            let s = a.vel.x.hypot(a.vel.y).max(1e-9);
            sx += a.vel.x / s;
            sy += a.vel.y / s;
        }
        total += sx.hypot(sy) / g.len as f64;
    }
    total / l.groups.len() as f64
}

#[test]
fn flocks_fly_together() {
    let mut flock = one(flyer(1.0, Perch::Never, true, 20));
    let mut loose = one(flyer(1.0, Perch::Never, false, 20));
    flock.run(60.0);
    loose.run(60.0);
    let (f, l) = (alignment(&flock), alignment(&loose));
    assert!(f > 0.8 && l < 0.6, "flock {f:.2}, loose {l:.2}");
}

#[test]
fn groups_migrate() {
    let h = Habitat {
        movement: Movement::Climber,
        area: Rect::new(0.0, WIDTH, 0.0, TOP),
        individuals: 60,
        group_size: (4, 6),
        migration_per_hour: 30.0,
        migration_distance: 60.0,
        ..Default::default()
    };
    let mut l = one(h);
    let start: Vec<Option<usize>> = l.groups.iter().map(|g| g.tree).collect();
    l.run(1800.0);
    let moved = l
        .groups
        .iter()
        .zip(&start)
        .filter(|(g, s)| g.tree != **s)
        .count();
    let migrations: u32 = l.groups.iter().map(|g| g.migrations).sum();
    assert!(
        migrations as usize >= l.groups.len() * 8,
        "migrations {migrations}"
    );
    assert!(
        moved * 10 >= l.groups.len() * 7,
        "{moved} of {} groups in another tree",
        l.groups.len()
    );
    check_all(&l);

    let mut still = one(Habitat {
        migration_per_hour: 0.0,
        ..h
    });
    let before: Vec<Option<usize>> = still.groups.iter().map(|g| g.tree).collect();
    still.run(1800.0);
    assert!(still
        .groups
        .iter()
        .zip(&before)
        .all(|(g, b)| g.tree == *b && g.migrations == 0));
}

#[test]
fn climbers_reach_a_far_tree_by_the_floor() {
    // two trees far apart: a migrating climber must come down to the floor on the way
    let mut f = forest(1, 0.0, 0.0);
    f.trees = vec![
        Tree {
            x: 30.0,
            height: 30.0,
            crown_bottom: 15.0,
            crown_half_width: 5.0,
        },
        Tree {
            x: 90.0,
            height: 30.0,
            crown_bottom: 15.0,
            crown_half_width: 5.0,
        },
    ];
    f.tallest = 0;
    let h = Habitat {
        movement: Movement::Climber,
        area: Rect::new(0.0, WIDTH, 0.0, TOP),
        individuals: 1,
        migration_per_hour: 20.0,
        migration_distance: 100.0,
        ..Default::default()
    };
    let mut l = Life::new(f, vec![Species::new("c", h)]);
    let mut lowest = f64::INFINITY;
    while l.groups[0].migrations < 3 {
        l.step();
        lowest = lowest.min(l.animals[0].pos.y);
        assert!(l.time < 4.0 * 3600.0, "no migration");
    }
    assert!(lowest.abs() < 1e-9, "lowest {lowest}");
    check_all(&l);
}

#[test]
fn active_zone() {
    let mut l = Life::new(forest(6, 0.5, 0.5), demo(1.0));
    let pos = Vec2::new(100.0, 1.5);
    let n = l.mark_active(&Zone::Around { pos, radius: 30.0 });
    assert!(n > 0 && n < l.animals.len());
    assert!(l
        .animals
        .iter()
        .all(|a| a.active == (a.pos.dist(pos) <= 30.0)));
    let rect = Rect::new(50.0, 80.0, 10.0, 30.0);
    l.mark_active(&Zone::Box { rect, margin: 5.0 });
    for a in &l.animals {
        let inside = a.pos.x >= 45.0 && a.pos.x <= 85.0 && a.pos.y >= 5.0 && a.pos.y <= 35.0;
        assert_eq!(a.active, inside);
    }
    let all = l.mark_active(&Zone::Box {
        rect: Rect::world(),
        margin: 0.0,
    });
    assert_eq!(all, l.animals.len());
}

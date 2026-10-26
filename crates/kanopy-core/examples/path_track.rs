//! Print a Wander track: t x y speed, one line per step. Usage: path_track SPEED UNDERGROUND MINUTES
use kanopy_core::life::{Life, DT};
use kanopy_core::path::{PathKind, Walker};
use kanopy_core::species::demo;
use kanopy_core::world::{Forest, ForestSettings};
use kanopy_core::Vec2;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let speed: f64 = a.first().map_or(2.0, |s| s.parse().unwrap());
    let under = a.get(1).is_some_and(|s| s == "1");
    let minutes: f64 = a.get(2).map_or(10.0, |s| s.parse().unwrap());
    let mut l = Life::new(Forest::generate(ForestSettings::default()), demo(0.05));
    let mut w = Walker::new(
        PathKind::Wander,
        speed,
        None,
        under,
        Vec2::new(100.0, 1.5),
        1,
    );
    for k in 0..(minutes * 60.0 / DT) as usize {
        let (p, v) = w.step(&l);
        println!(
            "{:.2} {:.3} {:.3} {:.3}",
            k as f64 * DT,
            p.x,
            p.y,
            (v.x * v.x + v.y * v.y).sqrt()
        );
        l.step();
    }
}

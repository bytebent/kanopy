//! Times each part of the sound path on its own: cargo run --release --example parts
use kanopy_core::filter::Svf;
use kanopy_core::mixer::{Listener, Mixer};
use kanopy_core::propagation::{emission_time, Medium};
use kanopy_core::voice::{UnitSpec, Voice};
use kanopy_core::wavetable::Wavetables;
use kanopy_core::{Vec2, BLOCK};
use std::hint::black_box;
use std::time::Instant;

fn per_sample(label: &str, samples: usize, f: impl FnOnce()) {
    let t = Instant::now();
    f();
    println!(
        "{label:<34} {:>6.2} ns/sample",
        t.elapsed().as_nanos() as f64 / samples as f64
    );
}

fn main() {
    let sr = 48_000.0;
    let wt = Wavetables::new(sr);
    let n = 20_000_000;
    let mut buf = [0f32; BLOCK];
    let long = UnitSpec {
        dur: 1e6,
        shape_start: 0.0,
        shape_end: 3.0,
        cutoff_start: 200.0,
        cutoff_end: 2000.0,
        resonance: 0.5,
        ..Default::default()
    };
    per_sample("voice render (osc+body+env)", n, || {
        let mut v = Voice::new(long, sr);
        for _ in 0..n / BLOCK {
            v.render(&wt, &mut buf, 1.0);
            black_box(&buf);
        }
    });
    per_sample("oscillator alone (blend)", n, || {
        let mut v = Voice::new(long, sr);
        for _ in 0..n / BLOCK {
            let p = v.plan(&wt, 1.0, BLOCK);
            v.osc(&wt, &p, &mut buf);
            black_box(&buf);
        }
    });
    per_sample("plan alone (amortised)", n, || {
        let mut v = Voice::new(long, sr);
        for _ in 0..n / BLOCK {
            black_box(v.plan(&wt, 1.0, BLOCK));
        }
    });
    let mut svf = Svf::new(500.0, 0.5, sr);
    per_sample("body filter alone", n, || {
        for _ in 0..n / BLOCK {
            svf.highpass_block(&mut buf);
            black_box(&buf);
        }
    });
    per_sample("filter set() per block (amortised)", n, || {
        for i in 0..n / BLOCK {
            svf.set(200.0 + (i % 1000) as f32, 0.5, sr);
            black_box(&svf);
        }
    });
    let m = Medium::earth();
    per_sample("emission_time x2 per block (amort.)", n, || {
        for i in 0..n / BLOCK {
            let t = i as f64 * 0.001;
            black_box(emission_time(
                t,
                Vec2::new(0.0, 1.5),
                Vec2::new(30.0, 10.0),
                Vec2::new(3.0, 0.0),
                &m,
            ));
            black_box(emission_time(
                t + 0.001,
                Vec2::new(0.0, 1.5),
                Vec2::new(30.0, 10.0),
                Vec2::new(3.0, 0.0),
                &m,
            ));
        }
    });
    // whole mixer with one long unit
    let wt2 = std::sync::Arc::new(Wavetables::new(sr));
    let mut mx = Mixer::new(wt2, m, Listener::still(Vec2::new(0.0, 1.5)));
    mx.add(long, -1.0, Vec2::new(20.0, 10.0), Vec2::default());
    let mut l = vec![0f32; 512];
    let mut r = vec![0f32; 512];
    per_sample("mixer, one unit, everything", n, || {
        for _ in 0..n / 512 {
            mx.render(&mut l, &mut r);
            black_box((&l, &r));
        }
    });
}

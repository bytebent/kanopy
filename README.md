# kanopy

A renderer for living forests: species you design, placed in a forest on any world, heard from
wherever you stand. Each animal only reacts to what actually reaches its ears.

Status: **design agreed; batches 01–03 built**: the sound path, the forest and how its animals move, and how they call, hear, answer, chorus, harmonise and copy each other.

- `design.md`: what kanopy is and how it works
- `plan.md`: build batches
- `experiments.md`: every test and screenshot, briefly

## Build and run

Needs Rust (install from https://rustup.rs). No other dependencies.

```sh
cargo test --release                              # all tests
cargo run --release -p kanopy-cli -- bench        # how many sounding units one core renders live
cargo run --release -p kanopy-cli -- demo         # short WAVs in out/demo
cargo run --release -p kanopy-core --example parts   # time each part of the sound path
cargo run --release -p kanopy-cli -- world --density 0.5 --minutes 10   # forest snapshot → out/world.json
python3 tools/plot_world.py out/world.json forest.png                   # draw it (needs matplotlib)
cargo run --release -p kanopy-cli -- world-bench  # how fast 20,000 individuals live
cargo run --release -p kanopy-cli -- score --minutes 1 --scale 0.15   # every call as text
cargo run --release -p kanopy-cli -- listen --minutes 1 --scale 0.15 --out out/forest.wav
cargo run --release -p kanopy-cli -- scene-bench  # 20,000 individuals calling, hearing, sounding
```

Listen options: `--seed`, `--density`, `--minutes`, `--scale` (share of the demo forest's
animals), `--x`/`--y` (listener), `--radius` (active zone), `--cap`, `--free`.

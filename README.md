# kanopy

A renderer for living forests: species you design, placed in a forest on any world, heard from
wherever you stand. Each animal only reacts to what actually reaches its ears.

Status: **design agreed; batch 01 built** (the voice, propagation, mixer and benchmark).

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
```

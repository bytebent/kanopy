# kanopy

A renderer for living forests: species you design, placed in a forest on any world, heard from
wherever you stand. Each animal only reacts to what actually reaches its ears.

Status: **design agreed; batches 01–06 built**: the sound path, the forest and how its animals move, how they call, hear, answer, chorus, harmonise and copy each other, the worlds, Box listening, the Ear, limiters and reverb, forest files rendered to WAV, and the app with its Forest page, playing live.

- `design.md`: what kanopy is and how it works
- `plan.md`: build batches
- `experiments.md`: every test and screenshot, briefly

## The app

Download **kanopy-mac-intel.zip** from the repo's [Latest Mac build](../../releases/tag/mac-latest)
release, unzip it, move `kanopy.app` to Applications, and the first time right-click it and choose
**Open** (it isn't signed). It opens on the sparse example forest: press **Play** (or Space), drag
the listener around the screen, or switch to **Box** and draw one.

Or run the same page in a browser, from the repo: `cargo run --release -p kanopy-cli -- serve`
and open http://127.0.0.1:8710.

## Render a forest

```sh
cargo run --release -p kanopy-cli -- render examples/forests/sparse-forest.kanopy-forest.json --minutes 10
```

The WAV lands in `renders/`. Forest files are JSON: see `examples/forests/` (and one species
per file in `examples/species/`). A file only needs what differs from the defaults. Options:
`--minutes`, `--seed`, `--fade-in`, `--fade-out`, `--bits 16|24`, `--out FILE`. Every render
carries its forest file inside, so `kanopy render some-render.wav` renders it again exactly.

## Build and run

Needs Rust (install from https://rustup.rs).

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
cargo run --release -p kanopy-cli -- examples     # rewrite the example files
```

`score` and `listen` take `--forest FILE` to use a forest file instead of the demo.

The app: `cargo build --release -p kanopy-app` (needs a webview: on Linux, webkit2gtk), or
`npx @tauri-apps/cli@2 build` in `crates/kanopy-app` for a bundle. Page checks:
`node tools/check-pages.js` against a running `kanopy serve` (needs the `playwright` package).

Listen options: `--seed`, `--density`, `--minutes`, `--scale` (share of the demo forest's
animals), `--x`/`--y` (listener; negative y is underground), `--radius` (active zone), `--cap`,
`--free`, `--medium earth|mars|titan|venus|water`, `--box X0,X1,Y0,Y1`, `--ear fixed|adapting`,
`--reverb AMOUNT` or `--no-reverb`, `--no-bodies`, `--underwater-stereo`, `--compensation 0..1`,
`--normalise`.

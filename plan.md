# kanopy · plan

Follows `design.md`. Each batch ends with its tests passing and its experiments written up in
`experiments.md`.

| # | Batch | Done when | State |
|---|---|---|---|
| 00 | Design passes, plan, experiments log | `design.md` agreed | Done |
| 01 | Rust workspace; one voice (wavetable + resonant high-pass); propagation from a listener (spreading, Earth absorption, delay, Doppler, pan); a mixer; **benchmark** | Tests pass; voice counts measured on the Mac; calling-cap default set | Done (Intel Mac bench: cap 600 live) |
| 02 | World: trees from seed, Density and Tree variation, shrubs, layers; species areas, spread, groups; movement types, flight and perch, migration, flocking; active zone | Seeded worlds repeat exactly; placement tests | Done |
| 03 | Calls and behaviour: call grammar, signatures, hearing between animals, call and response, chorus, Harmonise, mimicry (chance, accuracy, pitch and timbre match), calling cap | Behaviour tests; text score of a forest | Done |
| 04 | Media and loudness: presets and Custom, absorption curves, foliage loss, bodies follow the medium, underground listener, underwater stereo, Box listening, Ear, starting gain, limiters, reverb | Level tests per medium; no clipping in long renders | Done |
| 05 | `kanopy-cli render`: forest file → WAV, streamed; file formats | Same seed, same WAV, bit for bit | Done |
| 06 | App shell and Forest page: Tauri, live audio on one render thread, the scene, dragging, paths, box | Plays live on the Mac | Built; waiting for your first run on the Mac |
| 07 | Species page | | |
| 08 | Designer page | | |
| 09 | Listening round and tuning | Your notes addressed | |

## Batch 01 in detail

- `crates/kanopy-core`: `rng` (seeded, repeatable), `wavetable` (band-limited sine, triangle, saw
  and square tables per octave, morphed by Shape), `filter` (resonant 2-pole high-pass), `voice`
  (one unit: pitch and glide, Shape, body, envelope, level), `propagation` (spreading from 1 m,
  Earth air absorption, delay and Doppler, equal-power pan), `mixer` (renders voices in blocks).
  No dependencies.
- `crates/kanopy-cli`: `bench` (how many voices one thread renders in real time) and `demo`
  (short WAVs to listen to the voice and the distance cues).
- Tests for every module, including bit-for-bit repeatability.
- You run `cargo run --release -p kanopy-cli -- bench` on the Mac; its numbers set the calling-cap
  default.
- Done 20 Oct: all of the above, plus `--example parts` (times each part), units rendered 8 side
  by side, fixed-point oscillator phase. Container: ~12.5 ns per sounding unit-sample.

## Batch 02 in detail

- `world.rs`: layers, trees from the seed (count from Density, heights and crowns from Tree
  variation, tallest always 50 m, crowns as ellipses), shrub fullness.
- `species.rs`: habitat traits (movement type, area, individuals, spread, spacing, group size,
  cohesion, flock, flight band, perch, flight share, migration) and a 7-species demo forest.
- `life.rs`: placing groups and individuals, moving them in 50 ms steps, migration (climbers via
  touching crowns or down, along and up), flocking, perching, the active zone.
- `kanopy world` writes a JSON snapshot; `tools/plot_world.py` draws it. `kanopy world-bench`.
- Done 20 Oct. 20,010 individuals: about 4 % of one core (container).

## Batch 03 in detail

- `scale.rs`: keys, snapping, consonances (tempered in key, just ratios when free).
- `species.rs`: voice, call and behaviour traits as ranges; voices for the 7 demo species.
- `calls.rs`: motifs; fresh calls with Evolve; vary (signature Variation, mimic errors); copy
  (gesture, Pitch match, Timbre match, Accuracy); harmonise.
- `scene.rs`: events at exact times, the active zone, hearing within reach (per-species grid),
  one answer per call, chorus, harmonise, lazy mimic memory, the calling cap with near-first
  priority, units to the mixer with the animal's position and velocity.
- CLI: `score` (calls as text), `listen` (WAV, normalised), `scene-bench`.
- Done 20 Oct. 20,010 individuals all active: 51 % of one core including ~590 voices (container).

## Batch 04 in detail

- `propagation.rs`: media (presets, Custom, loudness offset, body ratio), foliage and soil along
  the path, height cue, stereo width.
- `mixer.rs`: per-species buses, Box listening (no distance, edge fades, pan across the box),
  the medium's loudness and bodies applied per unit.
- `master.rs`: species limiters, the Ear (Fixed, Adapting, starting gain), reverb (8-line FDN),
  final limiter.
- `scene.rs`: settings for all of it, medium compensation, the level estimate, animals hearing
  through the medium and the ground.
- CLI `listen`: `--medium`, `--box`, `--ear`, `--reverb`/`--no-reverb`, `--no-bodies`,
  `--underwater-stereo`, `--compensation`, `--normalise`.
- Done 20 Oct.

## Batch 05 in detail

- `files.rs`: forest and species files (serde JSON), defaults, version and format checks,
  readable errors, ranges on one line.
- `render.rs` and `wav.rs`: streamed WAV with fades, the forest inside (`knpy`), software name
  (`LIST/INFO`); reading the forest back out of a WAV.
- CLI: `render FILE` (forest file or kanopy WAV; `--minutes`, `--seed`, `--fade-in`,
  `--fade-out`, `--bits`, `--out`), `examples`, and `--forest FILE` for `score` and `listen`.
- `examples/`: the 7 demo species, and 6 forests (demo, sparse, Mars woodland, sunken kelp,
  canopy box, under the floor).
- First dependencies: serde and serde_json.
- Done 20 Oct.

## Batch 06 in detail

- `kanopy-core`: live setters on the scene (listener, Listener/Box, key, reverb, Ear), gain
  changes that glide, who's calling now.
- `crates/kanopy-live`: the render thread (125 ms ahead), ring buffer to the sound card (cpal,
  CoreAudio), commands, frames synced to what's heard, listener paths, rebuilds with a fade,
  export on its own thread. Silent real-time mode without a sound card.
- `ui/`: the Forest page (kit.css, kit.js, api.js, forest.js), plain HTML/CSS/JS.
- `crates/kanopy-app`: Tauri 2, the page plus the engine; built-in examples; file dialogs.
- `kanopy serve`: the same page in a browser over HTTP, playing through the computer's sound.
- `tools/check-pages.js`: page checks in headless Chromium.
- CI builds the Mac app and the `kanopy` command and publishes them as a pre-release.

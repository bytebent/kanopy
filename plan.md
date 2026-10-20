# kanopy · plan

Follows `design.md`. Each batch ends with its tests passing and its experiments written up in
`experiments.md`.

| # | Batch | Done when | State |
|---|---|---|---|
| 00 | Design passes, plan, experiments log | `design.md` agreed | Done |
| 01 | Rust workspace; one voice (wavetable + resonant high-pass); propagation from a listener (spreading, Earth absorption, delay, Doppler, pan); a mixer; **benchmark** | Tests pass; voice counts measured on the Mac; calling-cap default set | In progress |
| 02 | World: trees from seed, Density and Tree variation, shrubs, layers; species areas, spread, groups; movement types, flight and perch, migration, flocking; active zone | Seeded worlds repeat exactly; placement tests | |
| 03 | Calls and behaviour: call grammar, signatures, hearing between animals, call and response, chorus, Harmonise, mimicry (chance, accuracy, pitch and timbre match), calling cap | Behaviour tests; text score of a forest | |
| 04 | Media and loudness: presets and Custom, absorption curves, foliage loss, bodies follow the medium, underground listener, underwater stereo, Box listening, Ear, starting gain, limiters, reverb | Level tests per medium; no clipping in long renders | |
| 05 | `kanopy-cli render`: forest file → WAV, streamed; file formats | Same seed, same WAV, bit for bit | |
| 06 | App shell and Forest page: Tauri, live audio on one render thread, the scene, dragging, paths, box | Plays live on the Mac | |
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

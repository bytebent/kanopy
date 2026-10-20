# kanopy · plan

**Draft.** Follows `design.md`; it will change as the design passes settle. Each batch ends with
tests passing and its experiments written up in `experiments.md`.

| # | Batch | Done when |
|---|---|---|
| 00 | Design passes, plan, experiments log | `design.md` agreed |
| 01 | Rust workspace, one voice (wavetable + resonant high-pass), propagation, voice budget, **benchmark** | Voice counts measured on the Mac; limits in §10 confirmed or changed |
| 02 | World: trees from seed and Density, layers, placing individuals by habitat and movement type, migration, flocking | Seeded worlds repeat exactly; placement tests |
| 03 | Hearing and behaviour: who hears whom, calls, call and response, chorus, groups and Harmonise, mimicry and motifs | Behaviour tests; text score dump of a forest |
| 04 | Medium and loudness: presets, absorption, foliage loss, bodies follow the medium, Ear, limiters, reverb | Level tests per medium; no clipping in long renders |
| 05 | `kanopy-cli`: forest file → WAV, streamed | Same seed, same WAV, bit for bit |
| 06 | App shell and Forest page: Tauri, live audio, the scene, dragging and paths | Plays live on the Mac |
| 07 | Species page | |
| 08 | Designer page | |
| 09 | Listening round and tuning | Your notes addressed |

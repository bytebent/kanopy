# kanopy · experiments

Every test, benchmark and listening check, newest last. One line on what was tried, one on what
came out. Screenshots go in `docs/experiments/` as `NN-short-name.png` (NN = batch).

| Date | Batch | Experiment | Result | Screenshot |
|---|---|---|---|---|
| 20 Oct 2026 | 01 | 20 tests: wavetables (equal RMS, nothing past Nyquist, right harmonics, crossfades never cancel), body (−3 dB at cutoff, 12 dB/oct, resonance peak, stable under jumping cutoff), voice (pitch, length, glide, soft start), propagation (−6 dB per doubling, equal-power pan centred above, travel delay, Doppler, culling, same seed same bits), lanes match one-by-one | All pass | |
| 20 Oct 2026 | 01 | First bench, one unit at a time (container, Linux x86, 2.8 GHz Xeon) | ~40 ns per sounding unit-sample, ~275 units at half a core | |
| 20 Oct 2026 | 01 | Timed each part on its own (`--example parts`) | Body filter 7.4 ns and oscillator 12 ns per sample: both waiting on the previous sample, not doing work | |
| 20 Oct 2026 | 01 | 8 units side by side in full blocks (lanes); 32-bit fixed-point oscillator phase | Oscillator 12 → 5.5 ns; whole path ~12.5 ns per unit-sample | |
| 20 Oct 2026 | 01 | Bench, best of 2 runs per row (container) | 400 units 24 % of a core, 800 units 52 %, 1600 units 106 %. About 836 units at half a core. **Not the Mac number:** rerun there | |
| 20 Oct 2026 | 01 | Demo WAVs 01–02: shape sweep sine → square, saw through the body with cutoff 40 Hz → 6 kHz | Harmonics appear in order; resonant peak tracks the cutoff | `docs/experiments/01-voice-spectrograms.png` |
| 20 Oct 2026 | 01 | Demo WAVs 03–05: one call at 2, 8, 32, 128 m; a flyer passing at 25 m/s; 1,500 random units | Farther calls arrive later, quieter and duller; the fly-by bends down through closest approach; crowd is dense and never clips (peak −14 dBFS) | `docs/experiments/01-distance-spectrograms.png` |
| 20 Oct 2026 | 01 | Steps at 64-sample block edges in the long demo units | None: largest jump at block edges is no bigger than elsewhere | |
| 20 Oct 2026 | 02 | 17 world tests: layers; tree count follows Density, tallest tree always 50 m, Variation 0 gives an even stand; crowns; same seed same forest after 2 min, bit for bit; counts and group sizes; everyone inside their area and crown/band/perch over 10 min in three forests; walkers 0–2 m, burrowers in soil; Spread Even/Random/Clumped and Spacing; Cohesion; flight share 0.2/0.5/0.8 measured within ±0.08; perch layers; flocks align (>0.8) and loose groups don't (<0.6); migration; a climber reaches a far tree by the floor; active zone (radius and box + margin) | All pass after two fixes below | |
| 20 Oct 2026 | 02 | First run: flyers just taking off from a perch counted as flying while still below their band | Added a take-off mode (Rising) | |
| 20 Oct 2026 | 02 | Snapshots of the demo forest (7 species, 2,300 individuals) after 10 min: woodland, forest, thicket | Climbers in crowns with trails down trunks when they change trees; perched flyers in their layers; burrowers only in their part of the soil | `docs/experiments/02-woodland.png`, `02-forest.png`, `02-thicket.png` |
| 20 Oct 2026 | 02 | Look at the first snapshots | Flocks streaked across the whole width (pull home too weak); climbers lined up on crown edges their area only clipped. Fixed: stronger pull past the roaming distance; prefer trees that reach well into the area; crown half-width capped at 12 m | |
| 20 Oct 2026 | 02 | World bench: demo forest ×8.7 (20,010 individuals, 4,749 groups), 20 steps a second (container) | 42 ms per second of forest time: about 4 % of one core. Set-up 21 ms | `docs/experiments/02-forest-20k.png` |
| 20 Oct 2026 | 02 | Your note: soil and shrubs too thin | Made them bigger in the world: soil −1 → −3 m deep, shrubs up to 3 m (was 2), understorey now 3–5 m. Walkers live 0–3 m. Images redrawn | `docs/experiments/02-*.png` |
| 20 Oct 2026 | 01 | Bench on GitHub's Apple Silicon Mac | Never ran: no runner free within 15 min (GitHub capacity). Switched to `macos-26-intel`, matching your Intel laptop | |
| 20 Oct 2026 | 01 | Bench on GitHub's Intel Mac (`macos-26-intel`, Core i7-8700B 3.2 GHz, 2018), best of 2 | 400 units 19 % of a core, 800 units 43 %, 1600 units 91 %; ~10–12 ns per unit-sample. About 940 units at half a core | |
| 20 Oct 2026 | 02 | World bench on the same Mac | 20,010 individuals: 33 ms per second of forest time, 3.3 % of one core | |
| 20 Oct 2026 | 01 | Calling cap default set from the Mac bench | **600 live** (about a third of one core, leaving room on a laptop that throttles when hot); export uncapped by speed, default 2,000 | |
| 20 Oct 2026 | 03 | 17 behaviour tests: keys and consonances; fresh calls inside their ranges, in key or free; Evolve; signatures repeat at Variation 0 and drift at 1, groups differ; a perfect mimic matches pitch, sound and timing; Pitch match 0 keeps the steps in its own range; Timbre match 0 keeps its own sound; low Accuracy drifts; mimics copy only what reached them (95 m yes, 105 m no at 70 dB / 30 dB hearing); answers come after the call arrives; only their own species answer, at most one answer per call; chorus synchronise bunches and alternate avoids overlaps; harmonised calls consonant with the mate heard; only the active zone calls; the cap holds and favours the near; same seed same calls and sound | All pass | |
| 20 Oct 2026 | 03 | First score of the demo forest (2,300 animals) | 456 calls/s, more than half answers: every hearer could answer every call. Changed to at most one answer per call | |
| 20 Oct 2026 | 03 | Second score | 395 calls/s: Chorus synchronise shortened rests on every hearing, so they collapsed. Changed to a once-per-rest pull, only when the next call was near | |
| 20 Oct 2026 | 03 | Third score | 195 calls/s, about 40 % answers, mimics and harmonised calls present | `docs/experiments/03-score-sparse.txt` (sparse forest) |
| 20 Oct 2026 | 03 | Scene bench, 20,010 individuals all active, cap 600 (container) | 88 % of one core: 4.2 M hearings/s. Reach now worked out once per call and species; mimic memory worked out only when copying. Now **51 %** (about two thirds of it the ~590 voices sounding) | |
| 20 Oct 2026 | 03 | Five listening minutes: full forest from the floor; full forest with only animals within 40 m calling; sparse (15 %) from the floor, from the canopy, and with free pitch. Normalised to −1 dBFS (no Ear yet) | Full forest is a dense wall; sparse ones have air between calls | `docs/experiments/03-listen-spectrograms.png` |
| 20 Oct 2026 | 03 | Scene bench on GitHub's Intel Mac (i7-8700B), 20,010 individuals all active, cap 600, ~585 voices | **30 % of one core** for life, calls, hearing and sound together. All tests pass there | |

## Notes

- **01 · repeatability.** Same seed, same machine, same build: identical bits. Across machines
  (Mac vs Linux) floating-point library functions can differ in the last bit, so WAVs may differ
  by tiny amounts. To check in batch 05.
- **01 · levels are provisional.** A sine of amplitude 1.0 stands for 100 dB SPL until the Ear and
  starting gain arrive in batch 04.
- **02 · tuning constants.** Not controls yet: top speeds (walker 1.2, climber 0.8, flyer 9,
  burrower 0.05 m/s; each individual 60–100 % of that), mean perch time 40 s, pauses 1–15 s,
  roaming distance 25 m (Cohesion 0) to 1 m (Cohesion 1), 50 ms steps.
- **02 · liquid worlds** use the same geometry; only the names change (kelp, seabed, swimming).
  Nothing extra to simulate until the medium arrives in batch 04.
- **CI.** `.github/workflows/mac.yml` runs all tests and both benches on GitHub's Intel Mac on
  every push and posts the numbers as a run annotation (this session can't download logs).
- **03 · not yet audible:** media other than Earth, foliage loss, the Ear, reverb and Box
  listening (no distance) are batch 04. The listen files are normalised instead.

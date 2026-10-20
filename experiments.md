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

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

## Notes

- **01 · repeatability.** Same seed, same machine, same build: identical bits. Across machines
  (Mac vs Linux) floating-point library functions can differ in the last bit, so WAVs may differ
  by tiny amounts. To check in batch 05.
- **01 · levels are provisional.** A sine of amplitude 1.0 stands for 100 dB SPL until the Ear and
  starting gain arrive in batch 04.

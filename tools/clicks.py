#!/usr/bin/env python3
"""kanopy · find clicks in a WAV.

A click isn't necessarily clipping: it's a moment the sound couldn't have predicted from what came
just before. Each 20 ms frame gets a linear predictor (order 24) fitted to the frame before it;
the prediction error is smooth for tones, noise and their envelopes, and spikes at a break. A
sample is a click when its error is more than K times the error's typical size around it.

usage: clicks.py FILE.wav [--k 12] [--top 20] [--json OUT]
prints: clicks per minute, and the strongest with their times and channel.
"""
import json
import sys
import wave

import numpy as np
from scipy.signal import lfilter


def read(path):
    with wave.open(path, "rb") as w:
        sr, ch, bw, n = w.getframerate(), w.getnchannels(), w.getsampwidth(), w.getnframes()
        raw = w.readframes(n)
    if bw == 2:
        x = np.frombuffer(raw, "<i2").astype(np.float64) / 32768
    elif bw == 3:
        b = np.frombuffer(raw, np.uint8).reshape(-1, 3).astype(np.int32)
        v = b[:, 0] | (b[:, 1] << 8) | (b[:, 2] << 16)
        v = np.where(v >= 1 << 23, v - (1 << 24), v)
        x = v.astype(np.float64) / (1 << 23)
    else:
        raise SystemExit(f"{bw * 8}-bit WAV not handled")
    return sr, x.reshape(-1, ch).T


def lpc(frame, order):
    r = np.correlate(frame, frame, "full")[len(frame) - 1 : len(frame) + order]
    if r[0] <= 1e-12:
        return None
    r[0] *= 1.0 + 1e-6
    a = np.zeros(order + 1)
    a[0] = 1.0
    e = r[0]
    for i in range(1, order + 1):
        k = -(r[i] + a[1:i] @ r[i - 1 : 0 : -1]) / e
        a[1 : i + 1] = a[1 : i + 1] + k * np.concatenate([a[i - 1 : 0 : -1], [1.0]])
        e *= 1 - k * k
        if e <= 0:
            return None
    return a


def residual(x, sr, order=24, frame_s=0.02):
    """Prediction error, each frame predicted with coefficients fitted on the frame before."""
    n = int(frame_s * sr)
    out = np.zeros_like(x)
    a_prev = None
    for s in range(0, len(x) - n, n):
        fr = x[s : s + n]
        if a_prev is not None:
            seg = x[max(0, s - order) : s + n]
            e = lfilter(a_prev, [1.0], seg)[-n:]
            out[s : s + n] = e
        a = lpc(fr * np.hanning(n), order)
        a_prev = a if a is not None else a_prev
    return out


def find(x, sr, k=12.0):
    e = np.abs(residual(x, sr))
    # typical error size around each sample: median over 50 ms
    w = int(0.05 * sr)
    m = len(e) // w
    med = np.median(e[: m * w].reshape(m, w), axis=1)
    typical = np.repeat(np.maximum(med, 1e-6), w)
    typical = np.concatenate([typical, np.full(len(e) - len(typical), typical[-1] if len(typical) else 1e-6)])
    ratio = e / typical
    hits = np.flatnonzero((ratio > k) & (e > 1e-4))
    # one click per 5 ms
    clicks, last = [], -10**9
    for i in hits:
        if i - last > 0.005 * sr:
            j = i + int(np.argmax(ratio[i : i + int(0.005 * sr)]))
            clicks.append((j, float(ratio[j]), float(e[j])))
            last = i
    return clicks


def main():
    args = sys.argv[1:]
    path = args[0]
    k = float(args[args.index("--k") + 1]) if "--k" in args else 12.0
    top = int(args[args.index("--top") + 1]) if "--top" in args else 20
    sr, x = read(path)
    found = []
    for c, ch in enumerate(x):
        for i, ratio, size in find(ch, sr, k):
            found.append({"t": i / sr, "ch": "LR"[c] if len(x) == 2 else str(c), "ratio": ratio, "size": size})
    secs = x.shape[1] / sr
    print(f"{path}: {len(found)} clicks in {secs:.0f} s ({len(found) / secs * 60:.0f} per minute), K={k}")
    for f in sorted(found, key=lambda f: -f["ratio"])[:top]:
        print(f"  {f['t']:9.4f} s  {f['ch']}  ×{f['ratio']:6.1f}  error {20 * np.log10(f['size']):6.1f} dBFS")
    if "--json" in args:
        json.dump(found, open(args[args.index("--json") + 1], "w"))


if __name__ == "__main__":
    main()

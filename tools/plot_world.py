"""Draw a kanopy world snapshot (from `kanopy world`) as a PNG.

    python3 tools/plot_world.py out/world.json docs/experiments/02-forest.png
"""
import json, sys
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Ellipse, Rectangle

COLOURS = ["#61bb46", "#fdb827", "#f5821f", "#e03a3e", "#963d97", "#009ddc", "#8f8f8f", "#c0e070"]
LAYERS = [("Soil", -3, 0), ("Floor", 0, 0.5), ("Shrubs", 0.5, 3), ("Understorey", 3, 5),
          ("Subcanopy", 5, 15), ("Canopy", 15, 30), ("Emergent", 30, 50)]
MARK = {"walker": "s", "burrower": "s", "climber": "o", "flyer": "^"}

def main(src, dst):
    w = json.load(open(src))
    fig, ax = plt.subplots(figsize=(14, 4.8), facecolor="#0b0b0e")
    ax.set_facecolor("#060608")
    ax.add_patch(Rectangle((0, -3), 200, 3, color="#5a3a1a", alpha=0.7, lw=0))
    ax.add_patch(Rectangle((0, 0.5), 200, 2.5, color="#2f6b2a", alpha=0.15 + 0.45 * w["shrubs"], lw=0))
    for name, lo, hi in LAYERS:
        ax.axhline(lo, color="#8f8", lw=0.4, alpha=0.25, ls="--")
        ax.text(201, (lo + hi) / 2, name, color="#8f8", fontsize=7, va="center", family="monospace", alpha=0.8)
    for x, h, cb, hw in w["trees"]:
        ax.plot([x, x], [0, cb + (h - cb) * 0.3], color="#6b4a2a", lw=1.6, solid_capstyle="butt", zorder=1)
        ax.add_patch(Ellipse((x, (cb + h) / 2), 2 * hw, h - cb, color="#2e5e2a", alpha=0.55, lw=0, zorder=1))
    for a in w["animals"]:
        t = a["t"]
        if len(t) >= 4:
            ax.plot(t[0::2], t[1::2], color=COLOURS[a["s"] % 8], lw=0.4, alpha=0.35, zorder=2)
    for si, s in enumerate(w["species"]):
        pts = [a for a in w["animals"] if a["s"] == si]
        for mode, alpha in (("perched", 0.95), ("still", 0.9), ("travel", 0.9), ("flying", 0.9)):
            sel = [a for a in pts if a["m"] == mode]
            if not sel:
                continue
            m = "v" if mode == "perched" else MARK[s["movement"]]
            ax.scatter([a["x"] for a in sel], [a["y"] for a in sel], s=7, marker=m, color=COLOURS[si % 8],
                       alpha=alpha, lw=0, zorder=3, label=s["name"] if mode == "flying" or s["movement"] != "flyer" and mode == "still" else None)
    ax.set_xlim(0, 214); ax.set_ylim(-3.2, 51)
    ax.tick_params(colors="#8f8", labelsize=7)
    for sp in ax.spines.values():
        sp.set_color("#333")
    ax.set_title(f"seed {w['seed']} · density {w['density']} · variation {w['variation']} · {len(w['trees'])} trees · "
                 f"{len(w['animals'])} individuals · after {w['minutes']:g} min (trails: last 20 s)",
                 color="#8f8", fontsize=9, loc="left", family="monospace")
    leg = ax.legend(loc="upper left", fontsize=7, frameon=False, markerscale=2, ncol=7)
    for t in leg.get_texts():
        t.set_color("#ddd")
    plt.tight_layout()
    plt.savefig(dst, dpi=110, facecolor="#0b0b0e")

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])

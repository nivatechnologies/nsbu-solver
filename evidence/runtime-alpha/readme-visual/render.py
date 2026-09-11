"""Render the preserved alpha diagnostic data.

Requires Python 3 and matplotlib 3.10.8 (pip install matplotlib==3.10.8).
"""
import json
from pathlib import Path

import matplotlib.pyplot as plt


ROOT = Path(__file__).resolve().parent
with (ROOT / "data.json").open() as stream:
    points = json.load(stream)["points"]
t = [row["t_normalized"] for row in points]
energy = [row["energy"] for row in points]
enstrophy = [row["enstrophy"] for row in points]

plt.style.use("dark_background")
fig, axes = plt.subplots(1, 2, figsize=(12, 5.4), facecolor="#10161d")
fig.subplots_adjust(left=0.08, right=0.97, top=0.78, bottom=0.25, wspace=0.24)
fig.suptitle("NSBU Solver | alpha diagnostics", fontsize=19, fontweight="bold", color="#f3f6f8")
fig.text(0.5, 0.865, "Actual N=4/M=4 HO trajectories from rest", ha="center", fontsize=11, color="#a9b7c4")

for axis, values, title, color in zip(
    axes, (energy, enstrophy), ("Energy", "Enstrophy"), ("#28d7e8", "#ff9d4d")
):
    axis.set_facecolor("#151e27")
    axis.plot(t, values, color=color, linewidth=2.5, marker="o", markersize=5.5,
              markerfacecolor="#151e27", markeredgewidth=1.5, markeredgecolor=color)
    axis.set_title(title, loc="left", fontsize=13, color="#f3f6f8", pad=10)
    axis.set_xlabel("Normalized time  t / (1/128)", color="#b6c3ce")
    axis.set_ylabel(title, color="#b6c3ce")
    axis.set_xlim(-0.012, 0.512)
    axis.grid(True, color="#526170", alpha=0.25, linewidth=0.7)
    axis.tick_params(colors="#9baab7")
    for spine in axis.spines.values():
        spine.set_color("#40505e")

axes[0].set_ylim(bottom=0)
axes[1].set_ylim(bottom=0)
fig.text(0.5, 0.08, "Coarse diagnostic only • No spatial/force convergence or blow-up claim",
         ha="center", fontsize=10, color="#a9b7c4")
fig.savefig(ROOT.parents[2] / "docs/images/alpha-diagnostics.png", dpi=180, facecolor=fig.get_facecolor())
print(ROOT.parents[2] / "docs/images/alpha-diagnostics.png")

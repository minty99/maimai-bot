# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "plotly>=5.18",
#   "kaleido>=0.2",
#   "numpy>=1.24",
# ]
# ///
"""
Generate a jitter scatter plot of maimai achievement scores using Plotly.

Y axis: achievement % (the meaningful axis)
X axis: uniform random jitter (no information, purely for visual spread)

Input (stdin, JSON):
  {
    "points": [{"achievement": <float>, "level_tenths": <int>}, ...],
    "x_min": <float>   -- lower bound of the Y (achievement) axis
  }

  Each point represents one song's best score. Points are colored by level_tenths.

Output (stdout): PNG bytes
"""

import json
import os
import sys

import numpy as np
import plotly.colors
import plotly.graph_objects as go
import plotly.io as pio

# kaleido uses Chromium under the hood; when running as root (e.g. in Docker)
# Chromium requires --no-sandbox and --disable-gpu to start correctly.
if os.name != "nt" and os.getuid() == 0:
    pio.kaleido.scope.chromium_args = ("--disable-gpu", "--no-sandbox")

# Rank boundary thresholds and their display labels
RANK_THRESHOLDS: list[tuple[float, str]] = [
    (97.0, "S"),
    (98.0, "S+"),
    (99.0, "SS"),
    (99.5, "SS+"),
    (100.0, "SSS"),
    (100.5, "SSS+"),
]

# Qualitative palette — up to 10 distinct level colors
PALETTE = plotly.colors.qualitative.D3  # 10 well-separated colors


def main() -> None:
    data = json.loads(sys.stdin.buffer.read())
    raw_points: list[dict] = data["points"]
    x_min: float = data["x_min"]

    # Determine distinct levels present and assign colors
    levels: list[int] = sorted(set(p["level_tenths"] for p in raw_points))
    color_map: dict[int, str] = {
        lt: PALETTE[i % len(PALETTE)] for i, lt in enumerate(levels)
    }

    rng = np.random.default_rng(seed=0)

    fig = go.Figure()

    n_levels = len(levels)
    # Map level_tenths → lane index (0 = leftmost / lowest level)
    level_index: dict[int, int] = {lt: i for i, lt in enumerate(levels)}
    # Jitter half-width: leave a small gap between lanes (lane width = 1, jitter ±0.35)
    JITTER = 0.35

    # One trace per level so the legend shows each level with its color
    for level_tenths in levels:
        idx = level_index[level_tenths]
        group = [p for p in raw_points if p["level_tenths"] == level_tenths]
        # X: lane center + jitter
        x_vals = (idx + rng.uniform(-JITTER, JITTER, size=len(group))).tolist()
        # Y: achievement %
        y_vals = [p["achievement"] for p in group]

        fig.add_trace(
            go.Scatter(
                x=x_vals,
                y=y_vals,
                mode="markers",
                name=f"Lv {level_tenths / 10:.1f}",
                marker=dict(
                    size=11,
                    color=color_map[level_tenths],
                    opacity=0.82,
                    line=dict(width=0.8, color="white"),
                ),
            )
        )

    # Lane separator lines between adjacent levels
    for i in range(1, n_levels):
        fig.add_vline(
            x=i - 0.5,
            line=dict(dash="dash", color="rgba(180,180,180,0.55)", width=1),
        )

    # Rank boundary horizontal lines (only those visible within the y range)
    for rank_val, rank_label in RANK_THRESHOLDS:
        if rank_val < x_min or rank_val > 101.001:
            continue

        fig.add_hline(
            y=rank_val,
            line=dict(dash="dot", color="rgba(130,130,130,0.75)", width=1.5),
        )
        # Label anchored to the right edge of the plot area
        fig.add_annotation(
            x=1.02,
            y=rank_val,
            text=f"<b>{rank_label}</b>",
            showarrow=False,
            xref="paper",
            yref="y",
            xanchor="left",
            yanchor="middle",
            font=dict(size=10, color="rgba(90,90,90,0.95)"),
            bgcolor="rgba(255,255,255,0.75)",
            borderpad=2,
        )

    # Chart title
    n = len(raw_points)
    if n_levels == 1:
        level_label = f"Lv {levels[0] / 10:.1f}"
    else:
        level_label = f"Lv {levels[0] / 10:.1f}–{levels[-1] / 10:.1f}"
    title = f"{level_label}  —  {n} song{'s' if n != 1 else ''} (last 3 months, ≥90%)"

    # Scale width with number of levels (each lane ~110px), capped at 1200
    fig_width = min(1200, max(450, 110 * n_levels + 220))

    fig.update_layout(
        title=dict(text=title, font=dict(size=14), x=0.0, xanchor="left"),
        xaxis=dict(
            range=[-0.5, n_levels - 0.5],
            tickvals=list(range(n_levels)),
            ticktext=[f"{lt / 10:.1f}" for lt in levels],
            showgrid=False,
            zeroline=False,
            title="Internal Level",
        ),
        yaxis=dict(
            range=[x_min, 101.0],
            title="Achievement %",
            tickformat=".2f",
            showgrid=True,
            gridcolor="rgba(200,200,200,0.5)",
            zeroline=False,
        ),
        plot_bgcolor="white",
        paper_bgcolor="white",
        showlegend=False,  # X axis tick labels already identify each lane
        margin=dict(l=70, r=110, t=55, b=50),
    )

    img_bytes = fig.to_image(format="png", width=fig_width, height=650, scale=2)
    sys.stdout.buffer.write(img_bytes)


if __name__ == "__main__":
    main()

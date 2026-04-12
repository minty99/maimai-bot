# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "plotly~=6.6",
#   "kaleido~=1.2",
#   "numpy~=2.4",
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

import base64
import json
import os
import sys
from pathlib import Path

import numpy as np
import plotly.graph_objects as go
import plotly.io as pio

# kaleido uses Chromium under the hood; when running as root (e.g. in Docker)
# Chromium refuses to start without --no-sandbox.  kaleido >= 1.0 reads
# KALEIDO_CHROMIUM_ARGS before launching its Chromium subprocess (which
# happens lazily at the first to_image call), so setting the env var here
# is sufficient.
if os.name != "nt" and os.getuid() == 0:
    os.environ["KALEIDO_CHROMIUM_ARGS"] = "--disable-gpu --no-sandbox"

# -- Dark theme colors --------------------------------------------------------

BG_COLOR = "#0f0f17"
PLOT_BG_COLOR = "#16161f"
TEXT_COLOR = "#c8c8d0"
TEXT_MUTED = "#8888a0"
GRID_COLOR = "rgba(255, 255, 255, 0.06)"
LANE_SEP_COLOR = "rgba(255, 255, 255, 0.10)"
RANK_LINE_COLOR = "rgba(255, 255, 255, 0.18)"
RANK_LABEL_COLOR = "#b0b0c0"
RANK_LABEL_BG = "rgba(22, 22, 31, 0.85)"
TITLE_COLOR = "#e0e0e8"

# Rank boundary thresholds: (achievement %, label, icon filename stem)
RANK_THRESHOLDS: list[tuple[float, str, str]] = [
    (97.0, "S", "s"),
    (98.0, "S+", "sp"),
    (99.0, "SS", "ss"),
    (99.5, "SS+", "ssp"),
    (100.0, "SSS", "sss"),
    (100.5, "SSS+", "sssp"),
]

# Project root assumed to be the parent of this script's directory.
RANK_ICON_DIR = (
    Path(__file__).resolve().parent.parent
    / "maistats-discord-bot"
    / "assets"
    / "status-emojis"
)


def load_rank_icon_data_uri(stem: str) -> str | None:
    """Read a rank icon PNG and return it as a base64 data URI, or None if missing."""
    path = RANK_ICON_DIR / f"music_icon_{stem}.png"
    try:
        encoded = base64.b64encode(path.read_bytes()).decode("ascii")
    except OSError:
        return None
    return f"data:image/png;base64,{encoded}"

# Hand-picked palette — vivid but not neon, readable on dark background
PALETTE: list[str] = [
    "#5b9ef5",  # blue
    "#f0a050",  # amber
    "#6dd58c",  # green
    "#e86080",  # rose
    "#a07af0",  # purple
    "#50c8c8",  # teal
    "#f07878",  # coral
    "#88b0e0",  # light-blue
    "#d0a060",  # gold
    "#c888e0",  # lavender
]


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
    # Map level_tenths -> lane index (0 = leftmost / lowest level)
    level_index: dict[int, int] = {lt: i for i, lt in enumerate(levels)}
    # Jitter half-width: leave a small gap between lanes (lane width = 1, jitter +/-0.35)
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
                    opacity=0.85,
                    line=dict(width=0.6, color="rgba(0, 0, 0, 0.35)"),
                ),
            )
        )

    # Lane separator lines between adjacent levels
    for i in range(1, n_levels):
        fig.add_vline(
            x=i - 0.5,
            line=dict(dash="dash", color=LANE_SEP_COLOR, width=1),
        )

    # Rank boundary horizontal lines + icon labels (only those visible within the y range)
    rank_images: list[dict] = []
    for rank_val, rank_label, rank_stem in RANK_THRESHOLDS:
        if rank_val < x_min or rank_val > 101.001:
            continue

        fig.add_hline(
            y=rank_val,
            line=dict(dash="dot", color=RANK_LINE_COLOR, width=1.2),
        )

        icon_uri = load_rank_icon_data_uri(rank_stem)
        if icon_uri is None:
            # Fallback to text annotation if the icon file is missing
            fig.add_annotation(
                x=1.02,
                y=rank_val,
                text=f"<b>{rank_label}</b>",
                showarrow=False,
                xref="paper",
                yref="y",
                xanchor="left",
                yanchor="middle",
                font=dict(size=11, color=RANK_LABEL_COLOR),
                bgcolor=RANK_LABEL_BG,
                borderpad=3,
            )
            continue

        # Image positioned just outside the right edge of the plot area.
        # Both xref and yref are paper-relative so the icon keeps a consistent
        # visible size regardless of the y range.
        paper_y = (rank_val - x_min) / (101.0 - x_min)
        rank_images.append(
            dict(
                source=icon_uri,
                xref="paper",
                yref="paper",
                x=1.01,
                y=paper_y,
                sizex=0.08,
                sizey=0.04,
                xanchor="left",
                yanchor="middle",
                sizing="contain",
                layer="above",
            )
        )

    # Chart title — positioned inside the top margin
    n = len(raw_points)
    if n_levels == 1:
        level_label = f"Lv {levels[0] / 10:.1f}"
    else:
        level_label = f"Lv {levels[0] / 10:.1f}\u2013{levels[-1] / 10:.1f}"
    title = f"{level_label}  \u2014  {n} song{'s' if n != 1 else ''} (last 3 months, \u226590%)"

    # Scale width with number of levels (each lane ~110px), capped at 1200
    fig_width = min(1200, max(450, 110 * n_levels + 220))

    # Font stack: prefer Inter (installed via fonts-inter), fall back to
    # DejaVu Sans (always present via fonts-dejavu-core) and generic sans.
    FONT_FAMILY = "Inter, 'DejaVu Sans', sans-serif"

    fig.update_layout(
        font=dict(family=FONT_FAMILY, color=TEXT_COLOR),
        title=dict(
            text=title,
            font=dict(size=17, color=TITLE_COLOR, family=FONT_FAMILY),
            x=0.5,
            xanchor="center",
            y=0.96,
            yanchor="top",
        ),
        xaxis=dict(
            range=[-0.5, n_levels - 0.5],
            tickvals=list(range(n_levels)),
            ticktext=[f"{lt / 10:.1f}" for lt in levels],
            showgrid=False,
            zeroline=False,
            title=dict(
                text="Internal Level",
                font=dict(size=12, color=TEXT_MUTED, family=FONT_FAMILY),
            ),
            tickfont=dict(size=11, color=TEXT_COLOR, family=FONT_FAMILY),
        ),
        yaxis=dict(
            range=[x_min, 101.0],
            title=dict(
                text="Achievement %",
                font=dict(size=12, color=TEXT_MUTED, family=FONT_FAMILY),
            ),
            tickformat=".2f",
            showgrid=True,
            gridcolor=GRID_COLOR,
            zeroline=False,
            tickfont=dict(size=11, color=TEXT_COLOR, family=FONT_FAMILY),
        ),
        plot_bgcolor=PLOT_BG_COLOR,
        paper_bgcolor=BG_COLOR,
        showlegend=False,
        margin=dict(l=70, r=110, t=65, b=55),
        images=rank_images,
    )

    img_bytes = fig.to_image(format="png", width=fig_width, height=650, scale=2)
    sys.stdout.buffer.write(img_bytes)


if __name__ == "__main__":
    main()

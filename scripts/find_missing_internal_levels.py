#!/usr/bin/env python3

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


LEVEL_RE = re.compile(r"^\s*(\d+(?:\.\d+)?)")
CHART_TYPE_OUTPUT = {
    "std": "STD",
    "dx": "DX",
}
DIFFICULTY_OUTPUT = {
    "basic": "BASIC",
    "advanced": "ADVANCED",
    "expert": "EXPERT",
    "master": "MASTER",
    "remaster": "Re:MASTER",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Print 12+ level sheets that are missing internal levels from "
            "data/song_data/data.json."
        )
    )
    parser.add_argument(
        "--input",
        type=Path,
        default=repo_root() / "data" / "song_data" / "data.json",
        help="Path to the song catalog JSON file.",
    )
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def load_catalog(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as exc:
        raise SystemExit(f"input file not found: {path}") from exc
    except json.JSONDecodeError as exc:
        raise SystemExit(f"failed to parse JSON from {path}: {exc}") from exc


def level_value(level: str) -> float | None:
    match = LEVEL_RE.match(level)
    if match is None:
        return None
    return float(match.group(1))


def has_internal_level(sheet: dict[str, Any]) -> bool:
    value = sheet.get("internalLevel")
    return isinstance(value, str) and value.strip() != ""


def iter_missing_internal_levels(catalog: dict[str, Any]) -> list[tuple[str, ...]]:
    rows: list[tuple[str, ...]] = []

    for song in catalog.get("songs", []):
        title = str(song.get("title", ""))
        genre = str(song.get("genre", ""))

        for sheet in song.get("sheets", []):
            level = str(sheet.get("level", ""))
            numeric_level = level_value(level)
            if numeric_level is None or numeric_level < 12:
                continue
            if has_internal_level(sheet):
                continue

            chart_type = CHART_TYPE_OUTPUT.get(
                str(sheet.get("type", "")), str(sheet.get("type", ""))
            )
            difficulty = DIFFICULTY_OUTPUT.get(
                str(sheet.get("difficulty", "")), str(sheet.get("difficulty", ""))
            )
            version = str(sheet.get("version", ""))

            rows.append((title, genre, chart_type, difficulty, level, version))

    return rows


def main() -> int:
    args = parse_args()
    catalog = load_catalog(args.input)
    rows = iter_missing_internal_levels(catalog)

    print("title\tgenre\tchart_type\tdifficulty\tlevel\tversion")
    for row in rows:
        print("\t".join(row))

    if not rows:
        print("No missing internal levels found for level 12+ sheets.", file=sys.stderr)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

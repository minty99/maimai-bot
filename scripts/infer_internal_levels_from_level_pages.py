#!/usr/bin/env python3

import argparse
import html
import itertools
import json
import re
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ENTRY_RE = re.compile(
    r'<div class="music_(?P<difficulty>basic|advanced|expert|master|remaster)_score_back[^"]*"'
    r'.*?<img src="[^"]*/music_(?P<chart>dx|standard)\.png" class="music_kind_icon[^"]*"'
    r'.*?<div class="music_lv_block[^"]*">(?P<level>[^<]+)</div>\s*'
    r'<div class="music_name_block[^"]*">(?P<title>[^<]*)</div>',
    re.S,
)

CHART_TYPE_OUTPUT = {
    "dx": "DX",
    "std": "STD",
}
DIFFICULTY_OUTPUT = {
    "basic": "BASIC",
    "advanced": "ADVANCED",
    "expert": "EXPERT",
    "master": "MASTER",
    "remaster": "Re:MASTER",
}

MIN_SUPPORTED_BASE_LEVEL = 7
MAX_SUPPORTED_BASE_LEVEL = 15


@dataclass(frozen=True)
class CatalogEntry:
    title: str
    genre: str
    chart_type: str
    difficulty: str
    displayed_level: str
    internal_level: float | None
    version: str


@dataclass
class ParsedPageEntry:
    page_level_param: int
    page_label: str
    index: int
    title: str
    chart_type: str
    difficulty: str
    displayed_level: str
    candidates: list[CatalogEntry]
    resolved: CatalogEntry | None
    ambiguous: bool


@dataclass
class ScoredPageEntry:
    entry: ParsedPageEntry
    bucket_index: int
    inferred_internal_level: float


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Infer internal levels from maimai DX NET level pages using genre-order "
            "bucket boundaries, then compare them to data/song_data/data.json."
        )
    )
    parser.add_argument(
        "--html-dir",
        type=Path,
        default=repo_root() / "data" / "internal_level_experiment" / "html",
        help="Directory containing level17.html through level23.html.",
    )
    parser.add_argument(
        "--input",
        type=Path,
        default=repo_root() / "data" / "song_data" / "data.json",
        help="Path to the song catalog JSON file.",
    )
    parser.add_argument(
        "--show-all",
        action="store_true",
        help="Print all resolved rows instead of mismatches only.",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=200,
        help="Maximum number of detailed rows to print.",
    )
    return parser.parse_args()


def load_catalog(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as exc:
        raise SystemExit(f"input file not found: {path}") from exc
    except json.JSONDecodeError as exc:
        raise SystemExit(f"failed to parse JSON from {path}: {exc}") from exc


def parse_internal_level(value: object) -> float | None:
    if isinstance(value, (int, float)):
        return float(value)
    if not isinstance(value, str):
        return None


def parse_displayed_level(displayed_level: str) -> tuple[int, bool] | None:
    stripped = displayed_level.strip()
    if not stripped:
        return None
    if stripped.endswith("+"):
        base = stripped[:-1]
        is_plus = True
    else:
        base = stripped
        is_plus = False
    try:
        base_level = int(base)
    except ValueError:
        return None
    if not (MIN_SUPPORTED_BASE_LEVEL <= base_level <= MAX_SUPPORTED_BASE_LEVEL):
        return None
    if is_plus and base_level >= MAX_SUPPORTED_BASE_LEVEL:
        return None
    return (base_level, is_plus)


def displayed_level_for_param(level_param: int) -> str:
    if not (MIN_SUPPORTED_BASE_LEVEL <= level_param <= 23):
        raise SystemExit(f"unsupported level param: {level_param}")
    offset = level_param - MIN_SUPPORTED_BASE_LEVEL
    base_level = MIN_SUPPORTED_BASE_LEVEL + offset // 2
    is_plus = offset % 2 == 1
    if base_level > MAX_SUPPORTED_BASE_LEVEL or (
        base_level == MAX_SUPPORTED_BASE_LEVEL and is_plus
    ):
        raise SystemExit(f"unsupported level param: {level_param}")
    return f"{base_level}+" if is_plus else str(base_level)


def bucket_count_for_level(base_level: int, is_plus: bool) -> int:
    if is_plus:
        return 4
    if base_level == MAX_SUPPORTED_BASE_LEVEL:
        return 1
    return 6


def start_internal_for_level(base_level: int, is_plus: bool) -> float:
    if is_plus:
        return float(base_level) + 0.6
    return float(base_level)


def supported_level_params() -> range:
    return range(MIN_SUPPORTED_BASE_LEVEL, 24)
    stripped = value.strip()
    if not stripped:
        return None
    try:
        return float(stripped)
    except ValueError:
        return None


def build_catalog_lookup(catalog: dict[str, Any]) -> dict[tuple[str, str, str], list[CatalogEntry]]:
    lookup: dict[tuple[str, str, str], list[CatalogEntry]] = defaultdict(list)

    for song in catalog.get("songs", []):
        title = str(song.get("title", ""))
        genre = str(song.get("genre", ""))

        for sheet in song.get("sheets", []):
            chart_type = str(sheet.get("type", "")).strip().lower()
            difficulty = str(sheet.get("difficulty", "")).strip().lower()
            displayed_level = str(sheet.get("level", "")).strip()
            version = str(sheet.get("version", ""))

            lookup[(title, chart_type, difficulty)].append(
                CatalogEntry(
                    title=title,
                    genre=genre,
                    chart_type=chart_type,
                    difficulty=difficulty,
                    displayed_level=displayed_level,
                    internal_level=parse_internal_level(sheet.get("internalLevel")),
                    version=version,
                )
            )

    return lookup


def parse_page_entries(
    html_path: Path,
    level_param: int,
    lookup: dict[tuple[str, str, str], list[CatalogEntry]],
) -> list[ParsedPageEntry]:
    try:
        text = html_path.read_text()
    except FileNotFoundError as exc:
        raise SystemExit(f"html file not found: {html_path}") from exc

    entries: list[ParsedPageEntry] = []
    for index, match in enumerate(ENTRY_RE.finditer(text), start=1):
        title = html.unescape(match.group("title")).strip()
        chart_type = "dx" if match.group("chart") == "dx" else "std"
        difficulty = match.group("difficulty")
        displayed_level = html.unescape(match.group("level")).strip()

        key = (title, chart_type, difficulty)
        candidates = [
            candidate
            for candidate in lookup.get(key, [])
            if candidate.displayed_level == displayed_level
        ]
        if not candidates:
            candidates = list(lookup.get(key, []))

        resolved = candidates[0] if candidates else None
        entries.append(
            ParsedPageEntry(
                page_level_param=level_param,
                page_label=displayed_level_for_param(level_param),
                index=index,
                title=title,
                chart_type=chart_type,
                difficulty=difficulty,
                displayed_level=displayed_level,
                candidates=candidates,
                resolved=resolved,
                ambiguous=len(candidates) > 1,
            )
        )

    return entries


def infer_genre_order(pages: list[list[ParsedPageEntry]]) -> list[str]:
    genres = sorted(
        {
            entry.resolved.genre
            for page in pages
            for entry in page
            if entry.resolved is not None
        }
    )
    if not genres:
        raise SystemExit("failed to infer genre order: no catalog matches found")

    best_order: tuple[str, ...] | None = None
    best_wrap_count: int | None = None

    for candidate_order in itertools.permutations(genres):
        order_index = {genre: index for index, genre in enumerate(candidate_order)}
        wrap_count = 0
        for page in pages:
            prev_index: int | None = None
            for entry in page:
                if entry.resolved is None:
                    continue
                current_index = order_index[entry.resolved.genre]
                if prev_index is not None and current_index < prev_index:
                    wrap_count += 1
                prev_index = current_index

        if best_wrap_count is None or wrap_count < best_wrap_count:
            best_wrap_count = wrap_count
            best_order = candidate_order

    if best_order is None:
        raise SystemExit("failed to infer genre order")

    return list(best_order)


def bucket_cost(genre_indexes: list[int], start: int, end: int) -> int:
    drops = 0
    prev = genre_indexes[start]
    for current in genre_indexes[start + 1 : end]:
        if current < prev:
            drops += 1
        prev = current
    return drops


def partition_page(entries: list[ParsedPageEntry], genre_order: list[str]) -> list[ScoredPageEntry]:
    if not entries:
        return []

    level_param = entries[0].page_level_param
    parsed = parse_displayed_level(entries[0].page_label)
    if parsed is None:
        raise SystemExit(f"unsupported displayed level: {entries[0].page_label}")
    base_level, is_plus = parsed
    bucket_count = bucket_count_for_level(base_level, is_plus)
    start_internal = start_internal_for_level(base_level, is_plus)
    order_index = {genre: index for index, genre in enumerate(genre_order)}

    try:
        genre_indexes = [
            order_index[entry.resolved.genre]
            for entry in entries
            if entry.resolved is not None
        ]
    except KeyError as exc:
        raise SystemExit(f"missing genre in inferred order: {exc}") from exc

    if len(genre_indexes) != len(entries):
        raise SystemExit(
            f"page {entries[0].page_label} has unresolved rows; "
            "cannot partition without genre labels"
        )

    entry_count = len(entries)
    segment_cost = [[0] * (entry_count + 1) for _ in range(entry_count)]
    for start in range(entry_count):
        for end in range(start + 1, entry_count + 1):
            segment_cost[start][end] = bucket_cost(genre_indexes, start, end)

    inf = 10**9
    dp = [[inf] * (entry_count + 1) for _ in range(bucket_count + 1)]
    prev_cut: list[list[int | None]] = [
        [None] * (entry_count + 1) for _ in range(bucket_count + 1)
    ]
    dp[0][0] = 0

    for used_bucket_count in range(1, bucket_count + 1):
        for end in range(1, entry_count + 1):
            for start in range(used_bucket_count - 1, end):
                score = dp[used_bucket_count - 1][start] + segment_cost[start][end]
                if score < dp[used_bucket_count][end]:
                    dp[used_bucket_count][end] = score
                    prev_cut[used_bucket_count][end] = start

    cuts: list[tuple[int, int]] = []
    end = entry_count
    for used_bucket_count in range(bucket_count, 0, -1):
        start = prev_cut[used_bucket_count][end]
        if start is None:
            raise SystemExit(
                f"failed to partition page {entries[0].page_label} into {bucket_count} buckets"
            )
        cuts.append((start, end))
        end = start
    cuts.reverse()

    scored_entries: list[ScoredPageEntry] = []
    for bucket_index, (start, end) in enumerate(cuts):
        inferred_internal_level = round(start_internal + bucket_index * 0.1, 1)
        for page_entry in entries[start:end]:
            scored_entries.append(
                ScoredPageEntry(
                    entry=page_entry,
                    bucket_index=bucket_index,
                    inferred_internal_level=inferred_internal_level,
                )
            )

    return scored_entries


def iter_all_pages(
    html_dir: Path,
    lookup: dict[tuple[str, str, str], list[CatalogEntry]],
) -> list[list[ParsedPageEntry]]:
    pages: list[list[ParsedPageEntry]] = []
    for level_param in supported_level_params():
        html_path = html_dir / f"level{level_param}.html"
        pages.append(parse_page_entries(html_path, level_param, lookup))
    return pages


def format_internal_level(value: float | None) -> str:
    if value is None:
        return ""
    return f"{value:.1f}"


def print_summary(
    scored_pages: list[list[ScoredPageEntry]],
    genre_order: list[str],
    show_all: bool,
    limit: int,
) -> None:
    resolved_count = 0
    ambiguous_count = 0
    compared_count = 0
    exact_count = 0
    detail_rows: list[list[str]] = []

    print("Inferred genre order:")
    print("  " + " -> ".join(genre_order))
    print()

    print("Per-page summary:")
    for page in scored_pages:
        if not page:
            continue

        page_label = page[0].entry.page_label
        page_level_param = page[0].entry.page_level_param
        parsed = parse_displayed_level(page_label)
        if parsed is None:
            raise SystemExit(f"unsupported displayed level: {page_label}")
        bucket_count = bucket_count_for_level(*parsed)
        page_exact = 0
        page_compared = 0

        for scored_entry in page:
            if scored_entry.entry.resolved is None:
                continue
            resolved_count += 1
            if scored_entry.entry.ambiguous:
                ambiguous_count += 1

            actual = scored_entry.entry.resolved.internal_level
            if actual is None:
                continue

            page_compared += 1
            compared_count += 1
            matches = abs(actual - scored_entry.inferred_internal_level) < 1e-9
            if matches:
                page_exact += 1
                exact_count += 1

            if show_all or not matches:
                detail_rows.append(
                    [
                        str(scored_entry.entry.page_level_param),
                        page_label,
                        str(scored_entry.entry.index),
                        str(scored_entry.bucket_index),
                        scored_entry.entry.title,
                        CHART_TYPE_OUTPUT.get(
                            scored_entry.entry.chart_type, scored_entry.entry.chart_type.upper()
                        ),
                        DIFFICULTY_OUTPUT.get(
                            scored_entry.entry.difficulty,
                            scored_entry.entry.difficulty.upper(),
                        ),
                        scored_entry.entry.resolved.genre,
                        scored_entry.entry.displayed_level,
                        format_internal_level(actual),
                        format_internal_level(scored_entry.inferred_internal_level),
                        "yes" if matches else "no",
                        "yes" if scored_entry.entry.ambiguous else "",
                    ]
                )

        ratio = 0.0 if page_compared == 0 else 100.0 * page_exact / page_compared
        print(
            f"  level={page_level_param} ({page_label}): "
            f"{len(page)} rows, {bucket_count} buckets, "
            f"{page_exact}/{page_compared} exact ({ratio:.2f}%)"
        )

    overall_ratio = 0.0 if compared_count == 0 else 100.0 * exact_count / compared_count

    print()
    print("Overall:")
    print(f"  resolved rows: {resolved_count}")
    print(f"  ambiguous rows: {ambiguous_count}")
    print(f"  compared rows: {compared_count}")
    print(f"  exact matches: {exact_count}/{compared_count} ({overall_ratio:.2f}%)")
    print()

    print(
        "level_param\tpage_level\tpage_index\tbucket_index\ttitle\tchart_type\t"
        "difficulty\tgenre\tdisplayed_level\tactual_internal\tinferred_internal\t"
        "match\tambiguous"
    )
    for row in detail_rows[:limit]:
        print("\t".join(row))

    remaining = len(detail_rows) - min(len(detail_rows), limit)
    if remaining > 0:
        print(f"... ({remaining} more rows omitted; rerun with --limit or --show-all)")


def main() -> int:
    args = parse_args()
    catalog = load_catalog(args.input)
    lookup = build_catalog_lookup(catalog)
    pages = iter_all_pages(args.html_dir, lookup)
    genre_order = infer_genre_order(pages)
    scored_pages = [partition_page(page, genre_order) for page in pages]
    print_summary(scored_pages, genre_order, show_all=args.show_all, limit=args.limit)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

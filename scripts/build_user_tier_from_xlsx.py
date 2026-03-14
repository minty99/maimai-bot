#!/usr/bin/env python3

import argparse
import json
import re
import unicodedata
import urllib.parse
import zipfile
import xml.etree.ElementTree as ET
from collections import Counter, defaultdict
from pathlib import Path


MAIN_NS = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
REL_NS = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
NS = {"a": MAIN_NS}

TARGET_SHEETS = {f"{level / 10:.1f}" for level in range(130, 146)}
TIER_ORDER = [
    "S",
    "A+",
    "A",
    "A-",
    "B+",
    "B",
    "B-",
    "C+",
    "C",
    "C-",
    "D+",
    "D",
    "D-",
    "E+",
    "E",
    "E-",
    "F",
]
TIER_SORT_INDEX = {tier: index for index, tier in enumerate(TIER_ORDER)}
FORMULA_RE = re.compile(r'^IMAGE\("([^"]+)"\)$')
CELL_REF_RE = re.compile(r"([A-Z]+)(\d+)")
INTERNAL_LEVEL_EPSILON = 1e-6
NORMALIZED_CHARACTER_REPLACEMENTS = {
    "∀": "a",
}
SLUG_IDENTITY_OVERRIDES = {
    "linkmaimai": {
        "title": "Link",
        "genre": "maimai",
    },
    "plusdanshi": {
        "title": "+♂",
        "genre": "niconico＆VOCALOID™",
    },
    "trustgv": {
        "title": "Trust",
        "genre": "GAME＆VARIETY",
    },
}

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
        description="Build user tier JSON from a maimai scoring difficulty XLSX file."
    )
    parser.add_argument("input_xlsx", type=Path, help="Path to the source XLSX file")
    parser.add_argument(
        "output_json", type=Path, help="Path to write the generated JSON"
    )
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def catalog_path() -> Path:
    return repo_root() / "data" / "song_data" / "data.json"


def is_allowed_lookup_character(character: str) -> bool:
    codepoint = ord(character)

    if character.isascii() and character.isalnum():
        return True

    if character == "・":
        return False

    return any(
        (
            0x3040 <= codepoint <= 0x309F,
            0x30A0 <= codepoint <= 0x30FF,
            0x31F0 <= codepoint <= 0x31FF,
            0x3400 <= codepoint <= 0x4DBF,
            0x4E00 <= codepoint <= 0x9FFF,
            0x3005 <= codepoint <= 0x3007,
        )
    )


def normalize_lookup_value(value: str) -> str:
    normalized = (
        unicodedata.normalize("NFKC", urllib.parse.unquote(value)).strip().lower()
    )
    for source, target in NORMALIZED_CHARACTER_REPLACEMENTS.items():
        normalized = normalized.replace(source, target)
    return "".join(
        character for character in normalized if is_allowed_lookup_character(character)
    )


def parse_shared_strings(archive: zipfile.ZipFile) -> list[str]:
    path = "xl/sharedStrings.xml"
    if path not in archive.namelist():
        return []

    root = ET.fromstring(archive.read(path))
    strings = []
    for item in root:
        parts = []
        for text_node in item.iter(f"{{{MAIN_NS}}}t"):
            parts.append(text_node.text or "")
        strings.append("".join(parts))
    return strings


def parse_cell_value(cell: ET.Element, shared_strings: list[str]) -> str:
    cell_type = cell.attrib.get("t")
    value_node = cell.find("a:v", NS)
    inline_node = cell.find("a:is", NS)

    if cell_type == "s" and value_node is not None and value_node.text is not None:
        return shared_strings[int(value_node.text)]
    if cell_type == "inlineStr" and inline_node is not None:
        return "".join(
            text_node.text or "" for text_node in inline_node.iter(f"{{{MAIN_NS}}}t")
        )
    if value_node is not None and value_node.text is not None:
        return value_node.text
    return ""


def workbook_sheet_targets(archive: zipfile.ZipFile) -> dict[str, str]:
    workbook = ET.fromstring(archive.read("xl/workbook.xml"))
    rels = ET.fromstring(archive.read("xl/_rels/workbook.xml.rels"))
    rel_targets = {
        rel.attrib["Id"]: rel.attrib["Target"]
        for rel in rels
        if rel.attrib.get("Type", "").endswith("/worksheet")
    }

    targets = {}
    sheets = workbook.find("a:sheets", NS)
    if sheets is None:
        return targets

    for sheet in sheets:
        name = sheet.attrib.get("name")
        rel_id = sheet.attrib.get(f"{{{REL_NS}}}id")
        if not name or not rel_id:
            continue
        target = rel_targets.get(rel_id)
        if target is None:
            continue
        targets[name] = f"xl/{target}"
    return targets


def extract_image_url(formula: str | None) -> str | None:
    if formula is None:
        return None
    match = FORMULA_RE.match(formula.strip())
    if match is None:
        return None
    return match.group(1)


def excel_column_index(column: str) -> int:
    index = 0
    for character in column:
        index = (index * 26) + (ord(character) - ord("A") + 1)
    return index


def iter_source_rows(input_xlsx: Path) -> list[dict[str, object]]:
    with zipfile.ZipFile(input_xlsx) as archive:
        shared_strings = parse_shared_strings(archive)
        sheet_targets = workbook_sheet_targets(archive)
        missing_sheets = sorted(TARGET_SHEETS.difference(sheet_targets))
        if missing_sheets:
            raise ValueError(f"missing target sheets: {', '.join(missing_sheets)}")

        rows = []
        for sheet_name in sorted(TARGET_SHEETS, key=float):
            sheet_root = ET.fromstring(archive.read(sheet_targets[sheet_name]))
            sheet_data = sheet_root.find("a:sheetData", NS)
            if sheet_data is None:
                continue

            for row in sheet_data:
                tier_value = None
                row_number = int(row.attrib["r"])
                for cell in row.findall("a:c", NS):
                    cell_ref = cell.attrib.get("r", "")
                    match = CELL_REF_RE.fullmatch(cell_ref)
                    if match is None:
                        continue
                    column = match.group(1)
                    if column == "B":
                        tier_value = parse_cell_value(cell, shared_strings).strip()

                if not tier_value:
                    continue

                for cell in row.findall("a:c", NS):
                    cell_ref = cell.attrib.get("r", "")
                    match = CELL_REF_RE.fullmatch(cell_ref)
                    if match is None:
                        continue
                    column = match.group(1)
                    if excel_column_index(column) < excel_column_index("C"):
                        continue

                    image_url = extract_image_url(
                        (
                            cell.find("a:f", NS).text
                            if cell.find("a:f", NS) is not None
                            else None
                        )
                    )
                    if image_url is None:
                        continue

                    parsed = urllib.parse.urlparse(image_url)
                    parts = [part for part in parsed.path.split("/") if part]
                    if len(parts) < 2:
                        continue

                    prefix = parts[-2]
                    slug = urllib.parse.unquote(Path(parts[-1]).stem)
                    rows.append(
                        {
                            "internal_level": float(sheet_name),
                            "user_tier": tier_value,
                            "source_url": image_url,
                            "prefix": prefix,
                            "slug": slug,
                            "sheet_name": sheet_name,
                            "row": row_number,
                            "column": column,
                        }
                    )
        return rows


def load_catalog_songs() -> list[dict[str, object]]:
    with catalog_path().open("r", encoding="utf-8") as handle:
        data = json.load(handle)
    songs = data.get("songs")
    if not isinstance(songs, list):
        raise ValueError("song catalog is missing a top-level 'songs' array")
    return songs


def build_title_index(
    songs: list[dict[str, object]],
) -> dict[str, list[dict[str, object]]]:
    index: dict[str, list[dict[str, object]]] = defaultdict(list)
    for song in songs:
        title = song.get("title")
        if not isinstance(title, str):
            continue
        index[normalize_lookup_value(title)].append(song)

    for normalized_slug, override in SLUG_IDENTITY_OVERRIDES.items():
        override_matches = [
            song
            for song in songs
            if song.get("title") == override.get("title")
            and song.get("genre") == override.get("genre")
            and (
                override.get("artist") is None
                or song.get("artist") == override.get("artist")
            )
        ]
        if override_matches:
            index[normalized_slug].extend(override_matches)
    return index


def song_title_lookup_key(song: dict[str, object]) -> str:
    return normalize_lookup_value(str(song.get("title", "")))


def parse_internal_level(value: object) -> float | None:
    if value is None:
        return None
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        try:
            return float(value.strip())
        except ValueError:
            return None
    return None


def internal_level_matches(left: float, right: float) -> bool:
    return abs(left - right) <= INTERNAL_LEVEL_EPSILON


def chart_candidates(
    song: dict[str, object], internal_level: float
) -> list[dict[str, object]]:
    candidates = []
    for sheet in song.get("sheets", []):
        if not isinstance(sheet, dict):
            continue
        sheet_internal = parse_internal_level(sheet.get("internalLevel"))
        if sheet_internal is None or not internal_level_matches(
            sheet_internal, internal_level
        ):
            continue
        candidates.append(sheet)
    return candidates


def filter_chart_candidates(
    song: dict[str, object], prefix: str, internal_level: float
) -> list[dict[str, object]]:
    candidates = chart_candidates(song, internal_level)
    if prefix == "exp":
        return [chart for chart in candidates if chart.get("difficulty") == "expert"]
    if prefix == "rem":
        return [chart for chart in candidates if chart.get("difficulty") == "remaster"]
    if prefix == "dx":
        return [
            chart
            for chart in candidates
            if chart.get("type") == "dx" and chart.get("difficulty") == "master"
        ]
    if prefix == "st":
        return [
            chart
            for chart in candidates
            if chart.get("type") == "std" and chart.get("difficulty") == "master"
        ]
    if prefix == "original":
        return [chart for chart in candidates if chart.get("difficulty") == "master"]
    return []


def narrow_song_candidates_by_chart(
    songs: list[dict[str, object]], prefix: str, internal_level: float
) -> list[dict[str, object]]:
    return [
        song
        for song in songs
        if len(filter_chart_candidates(song, prefix, internal_level)) == 1
    ]


def fallback_song_candidates(
    songs: list[dict[str, object]], normalized_slug: str
) -> list[dict[str, object]]:
    return [
        song
        for song in songs
        if (title_key := song_title_lookup_key(song))
        and (normalized_slug in title_key or title_key in normalized_slug)
    ]


def song_label(song: dict[str, object]) -> str:
    return " / ".join(
        [
            str(song.get("title", "")),
            str(song.get("genre", "")),
            str(song.get("artist", "")),
        ]
    )


def chart_label(chart: dict[str, object]) -> str:
    return " ".join(
        [
            CHART_TYPE_OUTPUT.get(
                str(chart.get("type", "")), str(chart.get("type", ""))
            ),
            DIFFICULTY_OUTPUT.get(
                str(chart.get("difficulty", "")), str(chart.get("difficulty", ""))
            ),
            f"({chart.get('internalLevel')})",
        ]
    )


def resolve_song_candidates(
    entry: dict[str, object],
    exact_songs: list[dict[str, object]],
    all_songs: list[dict[str, object]],
) -> tuple[list[dict[str, object]], bool]:
    prefix = str(entry["prefix"])
    internal_level = float(entry["internal_level"])

    if len(exact_songs) == 1:
        return exact_songs, True

    if len(exact_songs) > 1:
        narrowed = narrow_song_candidates_by_chart(exact_songs, prefix, internal_level)
        if len(narrowed) == 1:
            return narrowed, True
        return exact_songs, False

    normalized_slug = normalize_lookup_value(str(entry["slug"]))
    fallback_matches = fallback_song_candidates(all_songs, normalized_slug)
    narrowed = narrow_song_candidates_by_chart(
        fallback_matches, prefix, internal_level
    )
    if len(narrowed) == 1:
        return narrowed, True
    return fallback_matches, False


def resolve_entry(
    entry: dict[str, object],
    title_index: dict[str, list[dict[str, object]]],
    all_songs: list[dict[str, object]],
) -> tuple[
    dict[str, object] | None, dict[str, object] | None, dict[str, object] | None
]:
    normalized_slug = normalize_lookup_value(str(entry["slug"]))
    exact_songs = title_index.get(normalized_slug, [])
    songs, resolved = resolve_song_candidates(entry, exact_songs, all_songs)

    if resolved and songs:
        song = songs[0]
        charts = filter_chart_candidates(
            song, str(entry["prefix"]), float(entry["internal_level"])
        )
        if not charts:
            return song, None, {"reason": "chart_not_found"}
        if len(charts) != 1:
            return (
                song,
                None,
                {
                    "reason": "chart_ambiguous",
                    "candidates": [chart_label(chart) for chart in charts],
                },
            )

        return song, charts[0], None

    if not songs:
        return None, None, {"reason": "song_not_found"}
    if len(songs) != 1:
        return (
            None,
            None,
            {
                "reason": "song_ambiguous",
                "candidates": [song_label(song) for song in songs],
            },
        )

    return None, None, {"reason": "song_not_found"}


def build_output_record(
    song: dict[str, object], chart: dict[str, object], user_tier: str
) -> dict[str, object]:
    chart_type = CHART_TYPE_OUTPUT.get(str(chart.get("type")))
    difficulty = DIFFICULTY_OUTPUT.get(str(chart.get("difficulty")))
    internal_level = parse_internal_level(chart.get("internalLevel"))

    if chart_type is None or difficulty is None or internal_level is None:
        raise ValueError("resolved chart is missing output fields")

    return {
        "title": song["title"],
        "genre": song["genre"],
        "artist": song["artist"],
        "chart_type": chart_type,
        "difficulty": difficulty,
        "internal_level": internal_level,
        "user_tier": user_tier,
    }


def exclusion_context(entry: dict[str, object]) -> str:
    return (
        f"sheet={entry['sheet_name']} tier={entry['user_tier']} prefix={entry['prefix']} "
        f"slug={entry['slug']} cell={entry['column']}{entry['row']}"
    )


def print_report(
    total_entries: int,
    emitted_rows: int,
    exclusions: list[dict[str, object]],
) -> None:
    counts = Counter(exclusion["reason"] for exclusion in exclusions)
    exclusions_by_reason: dict[str, list[dict[str, object]]] = defaultdict(list)
    for exclusion in exclusions:
        exclusions_by_reason[str(exclusion["reason"])].append(exclusion)

    print(f"total_source_entries: {total_entries}")
    print(f"emitted_rows: {emitted_rows}")
    print(f"excluded_rows: {len(exclusions)}")
    if counts:
        print("excluded_by_reason:")
        for reason in sorted(counts):
            print(f"  {reason}: {counts[reason]}")

    if exclusions:
        print("excluded_details:")
        for reason in sorted(exclusions_by_reason):
            print(f"  {reason}:")
            for exclusion in exclusions_by_reason[reason]:
                detail = exclusion_context(exclusion["entry"])
                print(f"    - {detail}")
                candidates = exclusion.get("candidates")
                if candidates:
                    print(f"      candidates={'; '.join(candidates)}")


def main() -> int:
    args = parse_args()
    songs = load_catalog_songs()
    title_index = build_title_index(songs)
    source_rows = iter_source_rows(args.input_xlsx)

    exclusions: list[dict[str, object]] = []
    tentative_rows: list[tuple[dict[str, object], dict[str, object]]] = []

    for entry in source_rows:
        song, chart, error = resolve_entry(entry, title_index, songs)
        if error is not None:
            exclusions.append({"entry": entry, **error})
            continue

        assert song is not None
        assert chart is not None
        tentative_rows.append(
            (entry, build_output_record(song, chart, str(entry["user_tier"])))
        )

    records_by_key: dict[
        tuple[str, str, str, str, str],
        list[tuple[dict[str, object], dict[str, object]]],
    ] = defaultdict(list)
    for entry, record in tentative_rows:
        key = (
            str(record["title"]),
            str(record["genre"]),
            str(record["artist"]),
            str(record["chart_type"]),
            str(record["difficulty"]),
        )
        records_by_key[key].append((entry, record))

    output_rows: list[dict[str, object]] = []
    for key, grouped in records_by_key.items():
        if len(grouped) == 1:
            output_rows.append(grouped[0][1])
            continue

        candidates = [
            f"{exclusion_context(entry)} user_tier={record['user_tier']}"
            for entry, record in grouped
        ]
        for entry, _record in grouped:
            exclusions.append(
                {
                    "entry": entry,
                    "reason": "duplicate_chart",
                    "candidates": candidates,
                }
            )

    output_rows.sort(
        key=lambda record: (
            float(record["internal_level"]),
            TIER_SORT_INDEX.get(str(record["user_tier"]), len(TIER_SORT_INDEX)),
            str(record["title"]),
            str(record["chart_type"]),
            str(record["difficulty"]),
        )
    )

    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    with args.output_json.open("w", encoding="utf-8") as handle:
        json.dump(output_rows, handle, ensure_ascii=False, indent=2)
        handle.write("\n")

    print_report(len(source_rows), len(output_rows), exclusions)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

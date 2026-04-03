#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from itertools import zip_longest
from pathlib import Path
from typing import Any

OPENAI_API_KEY = ""

OFFICIAL_JSON_URL = "https://maimai.sega.jp/data/maimai_songs.json"
GCM_KO_ALIAS_URL = "https://raw.githubusercontent.com/lomotos10/GCM-bot/main/data/aliases/ko/maimai.tsv"
OPENAI_RESPONSES_URL = "https://api.openai.com/v1/responses"
DEFAULT_MODEL = "gpt-5.4-nano"
DEFAULT_OUTPUT_PATH = Path("maimai_ko_aliases.tsv")
DEFAULT_JSON_OUTPUT_PATH = Path("maimai_ko_aliases.json")
DEFAULT_COMPARE_OUTPUT_PATH = Path("maimai_ko_aliases_compare.txt")
DEFAULT_PROGRESS_PATH = Path("maimai_ko_aliases.progress.json")

JAPANESE_OR_CJK_PATTERN = re.compile(
    r"[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff]"
)
HANGUL_PATTERN = re.compile(r"[가-힣]")
WHITESPACE_PATTERN = re.compile(r"\s+")
ALIAS_ALLOWED_CHARS_PATTERN = re.compile(r"[^0-9A-Za-z가-힣 ]+")


@dataclass(frozen=True)
class OfficialSongTitle:
    title: str
    title_kana: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Generate Korean maimai song aliases from the official JP song JSON with the OpenAI API."
        )
    )
    parser.add_argument("--official-json-url", default=OFFICIAL_JSON_URL)
    parser.add_argument("--gcm-ko-alias-url", default=GCM_KO_ALIAS_URL)
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--batch-size", type=int, default=50)
    parser.add_argument("--reference-limit", type=int, default=24)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--sleep-seconds", type=float, default=0.5)
    parser.add_argument("--max-retries", type=int, default=3)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT_PATH)
    parser.add_argument("--json-out", type=Path, default=DEFAULT_JSON_OUTPUT_PATH)
    parser.add_argument("--compare-out", type=Path, default=DEFAULT_COMPARE_OUTPUT_PATH)
    parser.add_argument("--progress-out", type=Path, default=DEFAULT_PROGRESS_PATH)
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Download sources and print counts without calling the OpenAI API.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    if args.batch_size <= 0:
        raise SystemExit("--batch-size must be positive")
    if args.reference_limit < 0:
        raise SystemExit("--reference-limit must be zero or positive")
    if args.max_retries <= 0:
        raise SystemExit("--max-retries must be positive")

    official_titles = load_official_song_titles(args.official_json_url)
    gcm_aliases = parse_alias_tsv(fetch_text(args.gcm_ko_alias_url))
    target_titles = [
        song for song in official_titles if is_japanese_or_cjk_title(song.title)
    ]
    if args.limit is not None:
        target_titles = target_titles[: args.limit]

    existing_rows = load_alias_rows(args.out) if args.out.exists() else {}
    target_title_set = {song.title for song in target_titles}
    existing_rows = {
        title: aliases
        for title, aliases in existing_rows.items()
        if title in target_title_set
    }
    pending_titles = [song for song in target_titles if song.title not in existing_rows]
    references = select_reference_examples(
        target_titles, gcm_aliases, args.reference_limit
    )

    print(f"official unique titles: {len(official_titles)}")
    print(f"jp/cjk titles: {len(target_titles)}")
    print(f"existing output rows: {len(existing_rows)}")
    print(f"pending titles: {len(pending_titles)}")
    print(f"reference examples: {len(references)}")

    if args.dry_run:
        preview_titles = ", ".join(song.title for song in target_titles[:10])
        print(f"preview: {preview_titles}")
        return 0

    api_key = resolve_api_key()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.json_out.parent.mkdir(parents=True, exist_ok=True)
    args.compare_out.parent.mkdir(parents=True, exist_ok=True)
    args.progress_out.parent.mkdir(parents=True, exist_ok=True)

    generated_rows = dict(existing_rows)
    total_batches = (len(pending_titles) + args.batch_size - 1) // args.batch_size
    for batch_index, start in enumerate(
        range(0, len(pending_titles), args.batch_size), start=1
    ):
        batch = pending_titles[start : start + args.batch_size]
        print(
            f"[batch {batch_index}/{total_batches}] generating aliases for "
            f"{len(batch)} titles...",
            file=sys.stderr,
        )
        batch_rows = generate_alias_batch(
            api_key=api_key,
            model=args.model,
            batch=batch,
            references=references,
            max_retries=args.max_retries,
        )
        generated_rows.update(batch_rows)
        write_alias_tsv(args.out, target_titles, generated_rows)
        completed_count = sum(
            1 for song in target_titles if song.title in generated_rows
        )
        write_progress_json(
            args.progress_out,
            model=args.model,
            official_json_url=args.official_json_url,
            gcm_ko_alias_url=args.gcm_ko_alias_url,
            total_titles=len(target_titles),
            completed_titles=completed_count,
            pending_titles=max(0, len(target_titles) - completed_count),
            last_batch_titles=[song.title for song in batch],
        )
        if start + args.batch_size < len(pending_titles) and args.sleep_seconds > 0:
            time.sleep(args.sleep_seconds)

    write_generated_alias_json(
        args.json_out, target_titles, generated_rows, gcm_aliases
    )
    write_comparison_text(args.compare_out, target_titles, generated_rows, gcm_aliases)

    print(f"wrote {len(generated_rows)} rows to {args.out}")
    print(f"wrote JSON report to {args.json_out}")
    print(f"wrote comparison text to {args.compare_out}")
    return 0


def load_official_song_titles(url: str) -> list[OfficialSongTitle]:
    payload = fetch_json(url)
    if not isinstance(payload, list):
        raise RuntimeError("official song JSON did not return a list")

    titles: list[OfficialSongTitle] = []
    seen_titles: set[str] = set()
    for row in payload:
        if not isinstance(row, dict):
            continue
        if is_utage_song(row):
            continue
        title = str(row.get("title", "")).strip()
        if not title or title in seen_titles:
            continue
        seen_titles.add(title)
        titles.append(
            OfficialSongTitle(
                title=title, title_kana=str(row.get("title_kana", "")).strip()
            )
        )
    return titles


def select_reference_examples(
    songs: list[OfficialSongTitle],
    gcm_aliases: dict[str, list[str]],
    limit: int,
) -> list[dict[str, Any]]:
    if limit == 0:
        return []

    single_alias_examples: list[dict[str, Any]] = []
    multi_alias_examples: list[dict[str, Any]] = []
    for song in songs:
        aliases = sanitize_aliases(song.title, gcm_aliases.get(song.title, []))
        if not aliases:
            continue
        entry = {"title": song.title, "aliases": aliases}
        if len(aliases) > 1:
            multi_alias_examples.append(entry)
        else:
            single_alias_examples.append(entry)

    references: list[dict[str, Any]] = []
    for single, multi in zip_longest(single_alias_examples, multi_alias_examples):
        if single is not None:
            references.append(single)
        if len(references) >= limit:
            break
        if multi is not None:
            references.append(multi)
        if len(references) >= limit:
            break
    return references[:limit]


def resolve_api_key() -> str:
    api_key = OPENAI_API_KEY.strip() or os.environ.get("OPENAI_API_KEY", "").strip()
    if not api_key:
        raise RuntimeError(
            "OpenAI API key is empty. Fill OPENAI_API_KEY in the script or export OPENAI_API_KEY."
        )
    return api_key


def generate_alias_batch(
    *,
    api_key: str,
    model: str,
    batch: list[OfficialSongTitle],
    references: list[dict[str, Any]],
    max_retries: int,
) -> dict[str, list[str]]:
    expected_titles = [song.title for song in batch]
    request_payload = build_openai_payload(
        model=model, batch=batch, references=references
    )

    for attempt in range(1, max_retries + 1):
        try:
            response = post_json(
                OPENAI_RESPONSES_URL,
                request_payload,
                headers={
                    "Authorization": f"Bearer {api_key}",
                    "Content-Type": "application/json",
                },
            )
            parsed = extract_structured_response_json(response)
            return validate_generated_aliases(expected_titles, parsed)
        except Exception as exc:
            if attempt >= max_retries:
                raise RuntimeError(
                    f"failed to generate aliases after {max_retries} attempts"
                ) from exc
            print(
                f"  retry {attempt}/{max_retries - 1} after error: {exc}",
                file=sys.stderr,
            )
            time.sleep(1.5 * attempt)

    raise AssertionError("unreachable")


def build_openai_payload(
    *, model: str, batch: list[OfficialSongTitle], references: list[dict[str, Any]]
) -> dict[str, Any]:
    system_prompt = (
        "You generate Korean search aliases for maimai song titles.\n"
        "Return concise aliases that help Korean users find the song.\n"
        "For each title, prefer:\n"
        "1. A Hangul pronunciation or reading of the Japanese title.\n"
        "2. A concise Korean literal translation when that translation sounds natural.\n"
        "You may return both when both are useful.\n"
        "Rules:\n"
        "- Return 1 to 4 aliases when possible.\n"
        "- Use Hangul-centered aliases.\n"
        "- Avoid punctuation and special symbols whenever possible.\n"
        "- Prefer aliases that can be typed with plain Hangul, spaces, and digits.\n"
        "- Do not include the original Japanese title as an alias.\n"
        "- Do not use Latin-letter romanization.\n"
        "- Do not invent fandom nicknames or unrelated abbreviations.\n"
        "- Remove duplicates.\n"
        "- If a natural Korean alias is not possible, return an empty array."
    )
    user_payload = {
        "task": (
            "Generate Korean aliases for the provided maimai song titles. "
            "The title_kana field is from the official JP JSON and can help infer pronunciation."
        ),
        "style_reference_from_current_gcm_bot_ko_aliases": references,
        "songs": [
            {"title": song.title, "title_kana": song.title_kana} for song in batch
        ],
    }

    return {
        "model": model,
        "input": [
            {"role": "system", "content": system_prompt},
            {
                "role": "user",
                "content": json.dumps(user_payload, ensure_ascii=False, indent=2),
            },
        ],
        "text": {
            "format": {
                "type": "json_schema",
                "name": "ko_song_aliases",
                "schema": {
                    "type": "object",
                    "properties": {
                        "items": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "title": {"type": "string"},
                                    "aliases": {
                                        "type": "array",
                                        "items": {"type": "string"},
                                    },
                                },
                                "required": ["title", "aliases"],
                                "additionalProperties": False,
                            },
                        }
                    },
                    "required": ["items"],
                    "additionalProperties": False,
                },
                "strict": True,
            }
        },
    }


def validate_generated_aliases(
    expected_titles: list[str], parsed: dict[str, Any]
) -> dict[str, list[str]]:
    items = parsed.get("items")
    if not isinstance(items, list):
        raise RuntimeError("structured response did not contain an items array")

    rows_by_title: dict[str, list[str]] = {}
    for item in items:
        if not isinstance(item, dict):
            raise RuntimeError("structured response item was not an object")
        title = str(item.get("title", "")).strip()
        aliases = item.get("aliases")
        if not title:
            raise RuntimeError("structured response item had an empty title")
        if not isinstance(aliases, list):
            raise RuntimeError(
                f"structured response aliases for '{title}' was not a list"
            )
        rows_by_title[title] = sanitize_aliases(
            title, [str(alias) for alias in aliases]
        )

    expected_title_set = set(expected_titles)
    missing = [title for title in expected_titles if title not in rows_by_title]
    extras = [title for title in rows_by_title if title not in expected_title_set]
    if missing or extras:
        raise RuntimeError(
            f"response title mismatch; missing={missing[:5]}, extras={extras[:5]}"
        )

    return {title: rows_by_title[title] for title in expected_titles}


def sanitize_aliases(title: str, aliases: list[str]) -> list[str]:
    sanitized: list[str] = []
    seen: set[str] = set()
    for alias in aliases:
        normalized = normalize_alias(alias)
        if not normalized:
            continue
        if normalized == title:
            continue
        if normalized.lower() == title.lower():
            continue
        if not HANGUL_PATTERN.search(normalized):
            continue
        if normalized in seen:
            continue
        seen.add(normalized)
        sanitized.append(normalized)
        if len(sanitized) >= 4:
            break
    return sanitized


def normalize_alias(alias: str) -> str:
    normalized = ALIAS_ALLOWED_CHARS_PATTERN.sub(" ", alias)
    normalized = WHITESPACE_PATTERN.sub(" ", normalized).strip()
    return normalized


def write_alias_tsv(
    output_path: Path,
    ordered_titles: list[OfficialSongTitle],
    rows: dict[str, list[str]],
) -> None:
    lines: list[str] = []
    for song in ordered_titles:
        aliases = rows.get(song.title, [])
        lines.append("\t".join([song.title, *aliases]))
    output_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_progress_json(
    output_path: Path,
    *,
    model: str,
    official_json_url: str,
    gcm_ko_alias_url: str,
    total_titles: int,
    completed_titles: int,
    pending_titles: int,
    last_batch_titles: list[str],
) -> None:
    payload = {
        "model": model,
        "official_json_url": official_json_url,
        "gcm_ko_alias_url": gcm_ko_alias_url,
        "total_titles": total_titles,
        "completed_titles": completed_titles,
        "pending_titles": pending_titles,
        "last_batch_titles": last_batch_titles,
        "updated_at_epoch_seconds": int(time.time()),
    }
    output_path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )


def write_generated_alias_json(
    output_path: Path,
    ordered_titles: list[OfficialSongTitle],
    generated_rows: dict[str, list[str]],
    gcm_aliases: dict[str, list[str]],
) -> None:
    items = []
    for song in ordered_titles:
        items.append(
            {
                "title": song.title,
                "title_kana": song.title_kana,
                "gcm_aliases": gcm_aliases.get(song.title, []),
                "generated_aliases": generated_rows.get(song.title, []),
            }
        )
    output_path.write_text(
        json.dumps({"items": items}, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


def write_comparison_text(
    output_path: Path,
    ordered_titles: list[OfficialSongTitle],
    generated_rows: dict[str, list[str]],
    gcm_aliases: dict[str, list[str]],
) -> None:
    lines: list[str] = []
    for song in ordered_titles:
        gcm = gcm_aliases.get(song.title, [])
        generated = generated_rows.get(song.title, [])
        lines.extend(
            [
                f"TITLE: {song.title}",
                f"KANA: {song.title_kana or '-'}",
                f"GCM: {', '.join(gcm) if gcm else '-'}",
                f"GENERATED: {', '.join(generated) if generated else '-'}",
                "",
            ]
        )
    output_path.write_text("\n".join(lines), encoding="utf-8")


def load_alias_rows(path: Path) -> dict[str, list[str]]:
    rows: dict[str, list[str]] = {}
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        if not raw_line.strip():
            continue
        columns = [column.strip() for column in raw_line.split("\t")]
        title = columns[0]
        rows[title] = sanitize_aliases(title, columns[1:])
    return rows


def parse_alias_tsv(input_text: str) -> dict[str, list[str]]:
    rows: dict[str, list[str]] = {}
    for raw_line in input_text.splitlines():
        if not raw_line.strip():
            continue
        columns = [column.strip() for column in raw_line.split("\t")]
        title = columns[0]
        rows[title] = dedupe_aliases(title, columns[1:])
    return rows


def dedupe_aliases(title: str, aliases: list[str]) -> list[str]:
    deduped: list[str] = []
    seen: set[str] = set()
    for alias in aliases:
        normalized = WHITESPACE_PATTERN.sub(" ", alias).strip()
        if not normalized:
            continue
        if normalized == title:
            continue
        if normalized in seen:
            continue
        seen.add(normalized)
        deduped.append(normalized)
    return deduped


def is_japanese_or_cjk_title(title: str) -> bool:
    return bool(JAPANESE_OR_CJK_PATTERN.search(title))


def is_utage_song(row: dict[str, Any]) -> bool:
    catcode = str(row.get("catcode", "")).strip()
    return catcode == "宴会場" or "lev_utage" in row


def extract_structured_response_json(response: dict[str, Any]) -> dict[str, Any]:
    candidates: list[str] = []

    output_text = response.get("output_text")
    if isinstance(output_text, str) and output_text.strip():
        candidates.append(output_text)

    candidates.extend(iter_text_candidates(response.get("output", [])))

    for candidate in candidates:
        try:
            parsed = json.loads(candidate)
        except json.JSONDecodeError:
            continue
        if isinstance(parsed, dict):
            return parsed

    raise RuntimeError("could not find structured JSON in OpenAI response")


def iter_text_candidates(node: Any) -> list[str]:
    found: list[str] = []
    if isinstance(node, dict):
        text = node.get("text")
        if isinstance(text, str) and text.strip():
            found.append(text)
        for value in node.values():
            found.extend(iter_text_candidates(value))
    elif isinstance(node, list):
        for item in node:
            found.extend(iter_text_candidates(item))
    return found


def fetch_text(url: str) -> str:
    return request(url).decode("utf-8")


def fetch_json(url: str) -> Any:
    return json.loads(fetch_text(url))


def post_json(
    url: str, payload: dict[str, Any], headers: dict[str, str]
) -> dict[str, Any]:
    request_body = json.dumps(payload).encode("utf-8")
    response_body = request(url, data=request_body, headers=headers)
    parsed = json.loads(response_body)
    if isinstance(parsed, dict) and parsed.get("error"):
        raise RuntimeError(f"OpenAI API returned an error: {parsed['error']}")
    if not isinstance(parsed, dict):
        raise RuntimeError("OpenAI API did not return a JSON object")
    return parsed


def request(
    url: str, data: bytes | None = None, headers: dict[str, str] | None = None
) -> bytes:
    req = urllib.request.Request(url=url, data=data, headers=headers or {})
    try:
        with urllib.request.urlopen(req, timeout=120) as response:
            return response.read()
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {exc.code} for {url}: {body}") from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"request failed for {url}: {exc}") from exc


if __name__ == "__main__":
    raise SystemExit(main())

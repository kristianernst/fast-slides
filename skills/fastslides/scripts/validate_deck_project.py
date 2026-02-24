#!/usr/bin/env python3
"""Validate a FastSlides deck project folder for structure, asset links, and slide density."""

from __future__ import annotations

import argparse
import json
import posixpath
import re
import sys
from pathlib import Path
from urllib.parse import unquote

from path_utils import default_projects_dir as detect_default_projects_dir

PROJECT_NAME_RE = re.compile(r"^[A-Za-z0-9._-]+$")
FRONTMATTER_RE = re.compile(r"\A---\s*\n(.*?)\n---\s*(?:\n|$)", re.DOTALL)
FRONTMATTER_LINE_RE = re.compile(r"^\s*([A-Za-z0-9_-]+)\s*:\s*(.*?)\s*$")
SLIDE_START_RE = re.compile(r"<section\s+className=[\"']slide[\"']\s*>", re.IGNORECASE)
MARKDOWN_LINK_RE = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
ATTR_LINK_RE = re.compile(r"(?:src|href|poster)\s*=\s*[\"']([^\"']+)[\"']")
WORD_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9'./-]*")
IMPORT_EXPORT_RE = re.compile(r"^\s*(import|export)\s+", re.MULTILINE)
USE_CLIENT_RE = re.compile(r"^\s*[\"']use client[\"']\s*;?\s*$", re.MULTILINE)

EXTERNAL_PREFIXES = (
    "http://",
    "https://",
    "data:",
    "blob:",
    "mailto:",
    "tel:",
)


def default_projects_dir() -> Path:
    return detect_default_projects_dir(__file__)


def is_inside(parent: Path, target: Path) -> bool:
    try:
        target.relative_to(parent)
        return True
    except ValueError:
        return False


def resolve_project_dir(args: argparse.Namespace) -> Path:
    if args.project_dir:
        return args.project_dir.expanduser().resolve()

    if not args.project:
        raise ValueError("Provide --project or --project-dir.")

    if not PROJECT_NAME_RE.fullmatch(args.project):
        raise ValueError("Invalid project name. Use letters, numbers, dot, underscore, and dash.")

    return (args.projects_dir.expanduser().resolve() / args.project).resolve()


def normalize_frontmatter_value(raw: str) -> str:
    value = raw.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
        quote = value[0]
        value = value[1:-1]
        value = value.replace(f"\\{quote}", quote).replace("\\\\", "\\")
    return value.strip()


def extract_frontmatter(source: str) -> tuple[dict[str, str], str]:
    match = FRONTMATTER_RE.match(source)
    if not match:
        return {}, source

    values: dict[str, str] = {}
    for raw_line in match.group(1).splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        parsed = FRONTMATTER_LINE_RE.match(line)
        if not parsed:
            continue
        key = parsed.group(1).lower()
        values[key] = normalize_frontmatter_value(parsed.group(2))

    return values, source[match.end() :]


def clean_markdown_target(raw: str) -> str:
    value = raw.strip().strip("<>")
    match = re.match(r'^(\S+)(?:\s+"[^"]*")?$', value)
    if match:
        value = match.group(1)
    return value


def local_asset_path(raw: str) -> str | None:
    value = clean_markdown_target(raw)
    lower = value.lower()

    if not value:
        return None
    if value.startswith("#") or value.startswith("/"):
        return None
    if lower.startswith(EXTERNAL_PREFIXES):
        return None

    no_hash = value.split("#", 1)[0]
    no_query = no_hash.split("?", 1)[0]
    if not no_query:
        return None

    decoded = unquote(no_query).replace("\\", "/")
    normalized = posixpath.normpath(decoded)
    return normalized


def find_asset_targets(source: str) -> list[str]:
    refs: list[str] = []
    refs.extend(match.group(1) for match in MARKDOWN_LINK_RE.finditer(source))
    refs.extend(match.group(1) for match in ATTR_LINK_RE.finditer(source))
    return refs


def extract_slides(source: str) -> list[str]:
    starts = list(SLIDE_START_RE.finditer(source))
    if not starts:
        return []

    slides: list[str] = []
    for index, start_match in enumerate(starts):
        content_start = start_match.end()
        content_end = source.find("</section>", content_start)
        if content_end < 0:
            next_start = starts[index + 1].start() if index + 1 < len(starts) else len(source)
            content_end = next_start
        slides.append(source[content_start:content_end])
    return slides


def count_words(text: str) -> int:
    plain = re.sub(r"<[^>]+>", " ", text)
    return len(WORD_RE.findall(plain))


def bullet_count(text: str) -> int:
    return len(re.findall(r"^\s*(?:[-*+]\s+|\d+\.\s+)", text, flags=re.MULTILINE))


def max_paragraph_words(text: str) -> int:
    plain = re.sub(r"<[^>]+>", " ", text)
    paragraphs = [chunk.strip() for chunk in re.split(r"\n\s*\n", plain) if chunk.strip()]
    if not paragraphs:
        return 0
    return max(len(WORD_RE.findall(paragraph)) for paragraph in paragraphs)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", help="Project name under --projects-dir")
    parser.add_argument("--project-dir", type=Path, help="Direct path to deck project folder")
    parser.add_argument("--projects-dir", type=Path, default=default_projects_dir(), help="Projects root (used with --project)")
    parser.add_argument("--max-words", type=int, default=140, help="Warning threshold for words per slide")
    parser.add_argument("--max-bullets", type=int, default=8, help="Warning threshold for bullets per slide")
    parser.add_argument("--max-paragraph-words", type=int, default=55, help="Warning threshold for longest paragraph")
    parser.add_argument("--strict", action="store_true", help="Treat warnings as failures")
    parser.add_argument("--json", action="store_true", dest="json_output", help="Print machine-readable JSON summary")
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    errors: list[str] = []
    warnings: list[str] = []

    try:
        project_dir = resolve_project_dir(args)
    except ValueError as exc:
        print(f"[ERROR] {exc}")
        return 1

    page_path = project_dir / "page.mdx"
    if not project_dir.exists() or not project_dir.is_dir():
        print(f"[ERROR] Project folder not found: {project_dir}")
        return 1

    if not page_path.exists():
        print(f"[ERROR] Missing page.mdx: {page_path}")
        return 1

    source = page_path.read_text(encoding="utf-8")
    frontmatter, body = extract_frontmatter(source)

    if not frontmatter:
        warnings.append(
            "Missing YAML frontmatter in page.mdx. Add `---` metadata block with project/title/subtitle/date."
        )
    else:
        if not frontmatter.get("project"):
            warnings.append("Frontmatter is missing `project`.")
        if not frontmatter.get("title"):
            warnings.append("Frontmatter is missing `title`.")
        declared_project = frontmatter.get("project")
        if declared_project and declared_project != project_dir.name:
            warnings.append(
                f"Frontmatter project `{declared_project}` does not match folder name `{project_dir.name}`."
            )

    if IMPORT_EXPORT_RE.search(body):
        errors.append("Detected import/export statements in page.mdx; runtime decks should be content-only MDX.")
    if USE_CLIENT_RE.search(body):
        warnings.append('Found "use client" directive in page.mdx; this is usually unnecessary in runtime-loaded MDX.')

    slides = extract_slides(body)
    if not slides:
        errors.append('No `<section className="slide">` blocks were found.')

    slide_stats: list[dict[str, int]] = []
    for index, slide in enumerate(slides, start=1):
        words = count_words(slide)
        bullets = bullet_count(slide)
        paragraph_words = max_paragraph_words(slide)
        slide_stats.append({"slide": index, "words": words, "bullets": bullets, "max_paragraph_words": paragraph_words})

        if words > args.max_words:
            warnings.append(f"Slide {index} has {words} words (threshold: {args.max_words}).")
        if bullets > args.max_bullets:
            warnings.append(f"Slide {index} has {bullets} bullets/list items (threshold: {args.max_bullets}).")
        if paragraph_words > args.max_paragraph_words:
            warnings.append(
                f"Slide {index} has a paragraph with {paragraph_words} words (threshold: {args.max_paragraph_words})."
            )

    seen_paths: set[str] = set()
    for raw in find_asset_targets(body):
        normalized = local_asset_path(raw)
        if normalized is None:
            continue

        if normalized in seen_paths:
            continue
        seen_paths.add(normalized)

        if normalized == ".." or normalized.startswith("../"):
            errors.append(f"Invalid traversal asset path: {raw}")
            continue

        resolved = (project_dir / normalized).resolve()
        if not is_inside(project_dir.resolve(), resolved):
            errors.append(f"Asset path escapes project folder: {raw}")
            continue

        if not resolved.exists():
            errors.append(f"Missing asset target: {raw} -> {resolved}")

    summary = {
        "project_dir": str(project_dir),
        "page_path": str(page_path),
        "frontmatter": frontmatter,
        "slide_count": len(slides),
        "assets_checked": len(seen_paths),
        "errors": errors,
        "warnings": warnings,
        "slide_stats": slide_stats,
    }

    if args.json_output:
        print(json.dumps(summary, indent=2))
    else:
        status = "OK" if not errors and (not args.strict or not warnings) else "FAIL"
        print(f"[{status}] Validation report for {project_dir}")
        print(f"Slides: {summary['slide_count']} | Asset references checked: {summary['assets_checked']}")

        if errors:
            print("\nErrors:")
            for item in errors:
                print(f"- {item}")

        if warnings:
            print("\nWarnings:")
            for item in warnings:
                print(f"- {item}")

        if slide_stats:
            print("\nSlide stats:")
            for stat in slide_stats:
                print(
                    f"- Slide {stat['slide']}: words={stat['words']}, "
                    f"bullets={stat['bullets']}, max_paragraph_words={stat['max_paragraph_words']}"
                )

    has_failures = bool(errors) or (args.strict and bool(warnings))
    return 1 if has_failures else 0


if __name__ == "__main__":
    sys.exit(main())

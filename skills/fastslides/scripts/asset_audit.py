#!/usr/bin/env python3
"""Audit deck project assets: referenced, missing, unused, and largest files."""

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
FRONTMATTER_RE = re.compile(r"\A---\s*\n.*?\n---\s*(?:\n|$)", re.DOTALL)
MARKDOWN_LINK_RE = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
ATTR_LINK_RE = re.compile(r"(?:src|href|poster)\s*=\s*[\"']([^\"']+)[\"']")

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


def strip_frontmatter(source: str) -> tuple[bool, str]:
    match = FRONTMATTER_RE.match(source)
    if not match:
        return False, source
    return True, source[match.end() :]


def clean_target(raw: str) -> str:
    value = raw.strip().strip("<>")
    match = re.match(r'^(\S+)(?:\s+"[^"]*")?$', value)
    if match:
        value = match.group(1)
    return value


def local_asset_path(raw: str) -> str | None:
    value = clean_target(raw)
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


def all_asset_files(project_dir: Path) -> list[Path]:
    files: list[Path] = []
    for path in project_dir.rglob("*"):
        if not path.is_file():
            continue
        if path.name == "page.mdx":
            continue
        files.append(path)
    return files


def to_rel_posix(base: Path, target: Path) -> str:
    return target.relative_to(base).as_posix()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", help="Project name under --projects-dir")
    parser.add_argument("--project-dir", type=Path, help="Direct path to deck project folder")
    parser.add_argument("--projects-dir", type=Path, default=default_projects_dir(), help="Projects root (used with --project)")
    parser.add_argument("--top", type=int, default=10, help="Number of largest files to list")
    parser.add_argument("--strict-unused", action="store_true", help="Fail when unused assets are found")
    parser.add_argument("--json", action="store_true", dest="json_output", help="Print machine-readable JSON")
    return parser.parse_args()


def main() -> int:
    args = parse_args()

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
    frontmatter_detected, body = strip_frontmatter(source)

    referenced_paths: set[str] = set()
    resolved_referenced_files: set[str] = set()
    missing_assets: list[str] = []
    traversal_assets: list[str] = []
    directory_targets: list[str] = []

    for raw in find_asset_targets(body):
        normalized = local_asset_path(raw)
        if normalized is None:
            continue

        if normalized in referenced_paths:
            continue
        referenced_paths.add(normalized)

        if normalized == ".." or normalized.startswith("../"):
            traversal_assets.append(raw)
            continue

        resolved = (project_dir / normalized).resolve()
        if not is_inside(project_dir.resolve(), resolved):
            traversal_assets.append(raw)
            continue

        if not resolved.exists():
            missing_assets.append(f"{raw} -> {resolved}")
            continue

        if resolved.is_dir():
            directory_targets.append(f"{raw} -> {resolved}")
            continue

        resolved_referenced_files.add(to_rel_posix(project_dir, resolved))

    files = all_asset_files(project_dir)
    all_file_rel = sorted(to_rel_posix(project_dir, path) for path in files)

    unused_assets = sorted(set(all_file_rel) - resolved_referenced_files)

    largest = sorted(
        (
            {"path": to_rel_posix(project_dir, path), "bytes": path.stat().st_size}
            for path in files
        ),
        key=lambda item: item["bytes"],
        reverse=True,
    )
    if args.top >= 0:
        largest = largest[: args.top]

    summary = {
        "project_dir": str(project_dir),
        "page_path": str(page_path),
        "frontmatter_detected": frontmatter_detected,
        "referenced_assets": sorted(referenced_paths),
        "referenced_file_count": len(resolved_referenced_files),
        "all_asset_file_count": len(all_file_rel),
        "missing_assets": missing_assets,
        "traversal_assets": traversal_assets,
        "directory_targets": directory_targets,
        "unused_assets": unused_assets,
        "largest_files": largest,
    }

    if args.json_output:
        print(json.dumps(summary, indent=2))
    else:
        has_errors = bool(missing_assets or traversal_assets or directory_targets)
        status = "FAIL" if has_errors else "OK"
        if args.strict_unused and unused_assets:
            status = "FAIL"

        print(f"[{status}] Asset audit for {project_dir}")
        print(
            "Referenced files: "
            f"{summary['referenced_file_count']} | "
            f"Project asset files: {summary['all_asset_file_count']}"
        )

        if traversal_assets:
            print("\nTraversal/invalid asset references:")
            for item in traversal_assets:
                print(f"- {item}")

        if missing_assets:
            print("\nMissing asset references:")
            for item in missing_assets:
                print(f"- {item}")

        if directory_targets:
            print("\nDirectory references (expected file paths):")
            for item in directory_targets:
                print(f"- {item}")

        if unused_assets:
            print("\nUnused asset files:")
            for item in unused_assets:
                print(f"- {item}")

        if largest:
            print("\nLargest asset files:")
            for item in largest:
                print(f"- {item['path']}: {item['bytes']} bytes")

    failed = bool(missing_assets or traversal_assets or directory_targets)
    if args.strict_unused and unused_assets:
        failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())

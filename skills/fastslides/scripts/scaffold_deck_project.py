#!/usr/bin/env python3
"""Scaffold a FastSlides deck project folder with a starter page.mdx and asset directories."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

from path_utils import default_projects_dir as detect_default_projects_dir

PROJECT_NAME_RE = re.compile(r"^[A-Za-z0-9._-]+$")


def default_projects_dir() -> Path:
    return detect_default_projects_dir(__file__)


def yaml_quote(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def build_template(project: str, title: str, subtitle: str, date_label: str) -> str:
    frontmatter = "\n".join(
        [
            "---",
            f"project: {yaml_quote(project)}",
            f"title: {yaml_quote(title)}",
            f"subtitle: {yaml_quote(subtitle)}",
            f"date: {yaml_quote(date_label)}",
            "---",
            "",
        ]
    )
    return frontmatter + f'''<main className="deck">

<section className="slide">

# {title}

<div className="flex flex-col h-full justify-center">
  <div className="text-6xl font-extrabold text-neutral-900 mb-6">{subtitle}</div>
  <div className="text-2xl text-neutral-500">{date_label}</div>
</div>

</section>

<section className="slide">

# Problem

<div className="split">
  <div className="col prose-compact">

  - Current process is fragmented across inboxes and handoffs.
  - Ownership is unclear for time-sensitive messages.
  - Manual triage creates delays and rework.

  </div>
  <div className="col prose-compact">

  - Delays reduce responsiveness and predictability.
  - Teams spend time coordinating instead of resolving.
  - Leadership lacks a clear operational signal.

  </div>
</div>

</section>

<section className="slide">

# Proposal

<div className="split">
  <div className="col prose-compact">

  1. Classify incoming messages by intent and urgency.
  2. Route each message to a clear owner.
  3. Track response timing and outcomes.

  </div>
  <div className="col prose-compact">

  ## Expected Outcome

  - Faster first response
  - Lower coordination overhead
  - Better visibility for management decisions

  </div>
</div>

</section>

</main>
'''


def resolve_project_dir(args: argparse.Namespace) -> Path:
    if args.project_dir:
        return args.project_dir.expanduser().resolve()
    return (args.projects_dir.expanduser().resolve() / args.project).resolve()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--project", help="Project folder name under --projects-dir (for example: citycontainer)")
    group.add_argument("--project-dir", type=Path, help="Direct path to project folder (created if missing)")
    parser.add_argument("--projects-dir", type=Path, default=default_projects_dir(), help="Root directory containing deck projects")
    parser.add_argument("--project-key", help="Frontmatter project key (default: folder name)")
    parser.add_argument("--title", default="Presentation", help="Cover slide heading")
    parser.add_argument("--subtitle", default="Project Overview", help="Cover slide subtitle")
    parser.add_argument("--date", default="Month YYYY", dest="date_label", help="Cover slide date label")
    parser.add_argument("--force", action="store_true", help="Overwrite page.mdx if it already exists")
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    if args.project and not PROJECT_NAME_RE.fullmatch(args.project):
        print("[ERROR] Invalid project name. Use letters, numbers, dot, underscore, and dash.")
        return 1

    project_dir = resolve_project_dir(args)
    page_path = project_dir / "page.mdx"
    project_key = args.project_key or args.project or project_dir.name

    if not project_key:
        print("[ERROR] Could not infer project key. Set --project-key explicitly.")
        return 1

    project_dir.mkdir(parents=True, exist_ok=True)
    (project_dir / "images").mkdir(exist_ok=True)
    (project_dir / "media").mkdir(exist_ok=True)
    (project_dir / "data").mkdir(exist_ok=True)

    if page_path.exists() and not args.force:
        print(f"[ERROR] {page_path} already exists. Use --force to overwrite.")
        return 1

    page_path.write_text(
        build_template(project_key, args.title, args.subtitle, args.date_label),
        encoding="utf-8",
    )

    print(f"[OK] Project scaffold ready: {project_dir}")
    print(f"[OK] Wrote: {page_path}")
    print("[OK] Ensured asset dirs: images/, media/, data/")
    return 0


if __name__ == "__main__":
    sys.exit(main())

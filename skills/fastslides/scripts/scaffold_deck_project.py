#!/usr/bin/env python3
"""Scaffold a FastSlides deck project folder with a starter page.mdx and asset directories."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

from path_utils import default_projects_dir as detect_default_projects_dir

PROJECT_NAME_RE = re.compile(r"^[A-Za-z0-9._-]+$")
DEFAULT_SLIDES_CSS = """:root {
  --slide-bg: #f5f1e8;
  --slide-border: rgba(30, 42, 56, 0.1);
  --slide-radius: 14px;
  --slide-padding: 36px;
  --slide-layout-gap: 18px;
  --slide-card-bg: rgba(255, 255, 255, 0.72);
  --slide-card-border: rgba(30, 42, 56, 0.1);
  --slide-card-radius: 12px;
  --slide-card-padding: 16px;
  --slide-font-family: "IBM Plex Sans", "Inter", system-ui, sans-serif;
  --slide-heading-font: "Iowan Old Style", "Georgia", serif;
  --slide-code-font: "Fira Code", monospace;
  --slide-meta-font: var(--slide-code-font);
  --slide-meta-size: 0.72rem;
  --slide-fg: #16212b;
  --slide-h1-color: #13202c;
  --slide-h2-color: #223446;
  --slide-h3-color: #395166;
  --slide-body-color: #425467;
  --slide-meta-color: rgba(66, 84, 103, 0.82);
  --slide-accent: #1f7a78;
  --slide-link-color: var(--slide-accent);
  --slide-code-bg: rgba(23, 32, 43, 0.06);
  --slide-palette-1: #1f7a78;
  --slide-palette-2: #739e82;
  --slide-palette-3: #d7a65d;
  --slide-palette-4: #ba6b6f;
  --slide-palette-5: #5973a9;
}
"""


def default_projects_dir() -> Path:
    return detect_default_projects_dir()


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
  <Canvas cols={{50}} rows={{25}} gap="1px">
    <Area x={{2}} y={{2}} w={{14}} h={{1}}>
      <Kicker>{project}</Kicker>
    </Area>

    <Area x={{2}} y={{4}} w={{46}} h={{4}}>
      <Takeaway>{title}</Takeaway>
    </Area>

    <Area x={{2}} y={{9}} w={{18}} h={{2}}>
      <p>{subtitle}</p>
    </Area>

    <Area x={{2}} y={{12}} w={{22}} h={{2}}>
      <PillRow>
        <Pill>50 x 25 grid</Pill>
        <Pill>spatial pages</Pill>
        <Pill>typed review</Pill>
      </PillRow>
    </Area>

    <Area x={{34}} y={{10}} w={{14}} h={{11}}>
      <Callout title="How to start" tone="accent">
        Use one takeaway, two to four regions, and explicit evidence blocks instead of ad hoc layout HTML.
      </Callout>
    </Area>

    <Area x={{2}} y={{23}} w={{12}} h={{1}}>
      <Caption>{date_label}</Caption>
    </Area>
  </Canvas>
</section>

<section className="slide">
  <Canvas cols={{50}} rows={{25}} gap="1px">
    <Area x={{2}} y={{2}} w={{16}} h={{1}}>
      <Kicker>Situation</Kicker>
    </Area>

    <Area x={{2}} y={{4}} w={{46}} h={{4}}>
      <Takeaway>Move every deck onto a spatial canvas so agents edit structure, not fragile layout markup.</Takeaway>
    </Area>

    <Area x={{2}} y={{9}} w={{14}} h={{12}}>
      <Panel title="Problem" tone="accent">
        <ul>
          <li>Legacy layout markup drifts slide by slide.</li>
          <li>Review catches issues late.</li>
          <li>Local edits often break composition.</li>
        </ul>
      </Panel>
    </Area>

    <Area x={{18}} y={{9}} w={{14}} h={{12}}>
      <Panel title="2.0 model">
        <ul>
          <li>Canvas defines page geometry.</li>
          <li>Area defines regions.</li>
          <li>Panels and callouts define content structure.</li>
        </ul>
      </Panel>
    </Area>

    <Area x={{34}} y={{9}} w={{14}} h={{12}}>
      <Panel title="Outcome">
        <ul>
          <li>Cleaner decks by default.</li>
          <li>More stable review and export.</li>
          <li>A path to a shared scene renderer.</li>
        </ul>
      </Panel>
    </Area>
  </Canvas>
</section>

<section className="slide">
  <Canvas cols={{50}} rows={{25}} gap="1px">
    <Area x={{2}} y={{2}} w={{18}} h={{1}}>
      <Kicker>Starter kit</Kicker>
    </Area>

    <Area x={{2}} y={{4}} w={{46}} h={{4}}>
      <Takeaway>Every new project should start from reusable page patterns, not from a blank DOM tree.</Takeaway>
    </Area>

    <Area x={{2}} y={{9}} w={{18}} h={{12}}>
      <Panel title="Recommended flow" tone="accent">
        <ol>
          <li>Write the takeaway first.</li>
          <li>Place 2 to 4 regions.</li>
          <li>Add one evidence pattern.</li>
          <li>Validate and snapshot before hand-off.</li>
        </ol>
      </Panel>
    </Area>

    <Area x={{22}} y={{9}} w={{10}} h={{5}}>
      <Metric label="Grid" value="50 x 25" hint="Default authoring canvas" />
    </Area>

    <Area x={{22}} y={{15}} w={{10}} h={{5}}>
      <Metric label="Review" value="Sidebar toggle" hint="Off by default" />
    </Area>

    <Area x={{34}} y={{9}} w={{14}} h={{12}}>
      <Callout title="Next step">
        Keep the slide contract stable so preview and export can move onto the same scene graph.
      </Callout>
    </Area>
  </Canvas>
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
    (project_dir / "slides.css").write_text(DEFAULT_SLIDES_CSS, encoding="utf-8")

    print(f"[OK] Project scaffold ready: {project_dir}")
    print(f"[OK] Wrote: {page_path}")
    print(f"[OK] Wrote: {project_dir / 'slides.css'}")
    print("[OK] Ensured asset dirs: images/, media/, data/")
    return 0


if __name__ == "__main__":
    sys.exit(main())

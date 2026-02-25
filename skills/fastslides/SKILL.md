---
name: fastslides
description: Build and QA folder-based MDX decks for FastSlides Desktop. Use when a task involves creating/editing `page.mdx`, validating deck structure/assets, and controlling the running app through its local hook server.
---

# FastSlides

## Overview

Use this skill for a desktop-first workflow:

- FastSlides Desktop loads project folders directly.
- When the app is running, the local hook server is available.
- Agents should primarily interact through that server.

This skill is intentionally minimal: one main script + core validators.

## Prerequisites

- FastSlides Desktop is running when using hook-backed commands.
- `npx` is installed for Playwright-based screenshot capture.
- Playwright wrapper script exists at one of:
  - `$CODEX_HOME/skills/playwright/scripts/playwright_cli.sh`
  - `~/.agents/skills/playwright/scripts/playwright_cli.sh`

## Main Command

Use a single entry point:

```bash
bash scripts/fastslides.sh <command> [options]
```

Commands:

- `desktop [--install]`: run FastSlides Desktop (`tauri:dev`)
- `health`: hook server health
- `state`: hook server app state
- `open-project --path <absolute-project-path>`
- `validate-project --path <absolute-project-path>`
- `preview-url --path <absolute-project-path>`
- `inspect-slide --path <absolute-project-path> [--slide N] [--output-dir DIR] [--headed]`: capture slide screenshot via Playwright
- `init ...`: forward to `init_deck_project.sh`
- `validate-local ...`: run `validate_deck_project.py`
- `asset-audit ...`: run `asset_audit.py`

## Typical Flow

1. Start the app:

```bash
bash scripts/fastslides.sh desktop
```

2. Check server:

```bash
bash scripts/fastslides.sh health
```

3. Open a deck:

```bash
bash scripts/fastslides.sh open-project --path /absolute/path/to/project-folder
```

4. Validate through app and locally:

```bash
bash scripts/fastslides.sh validate-project --path /absolute/path/to/project-folder
bash scripts/fastslides.sh validate-local --project-dir /absolute/path/to/project-folder
bash scripts/fastslides.sh asset-audit --project-dir /absolute/path/to/project-folder --top 10
```

5. Visually inspect slide output (recommended for every agent-generated deck):

```bash
bash scripts/fastslides.sh inspect-slide --path /absolute/path/to/project-folder --slide 1
```

The command prints an absolute PNG path. Agents should open that image and verify layout, clipping, hierarchy, and readability.

## Project Scaffold

Create a new deck folder:

```bash
bash scripts/fastslides.sh init --project-dir /absolute/path/to/project-folder
```

or named mode:

```bash
bash scripts/fastslides.sh init --project my-deck --projects-dir /absolute/path/to/projects
```

## Deck Contract

Each deck folder should contain:

```text
<project>/
  page.mdx
  images/
  media/
  data/
```

Frontmatter in `page.mdx`:

```yaml
---
project: "folder-name"
title: "Presentation"
subtitle: "Project Overview"
date: "Month YYYY"
---
```

Slides should use:

- `<section className="slide">...</section>` blocks
- relative asset paths only

## Quality Rules

- Keep one key message per slide.
- Avoid overflow; split dense content into more slides.
- Keep heading hierarchy clear.
- Keep assets inside the project folder.
- Run both structural validation and visual inspection before hand-off.

## References

- Runtime + hook behavior: `references/deck_mdx_runtime.md`
- Layout guidance: `references/layout_best_practices.md`
- Visual QA workflow: `references/visual_inspection.md`

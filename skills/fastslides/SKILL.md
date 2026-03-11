---
name: fastslides
description: Build and QA folder-based MDX decks for FastSlides Desktop. Use when a task involves creating/editing `page.mdx`, validating deck structure/assets, and controlling the running app through its local hook server.
---

# FastSlides

Use this skill when an agent needs to create, edit, validate, or visually inspect a FastSlides deck.

## Defaults

- FastSlides Desktop is the runtime.
- The desktop hook and embedded MCP are the source of truth while the app is running.
- The supported slide contract is `Canvas` + `Area`.
- Prefer existing recipes and compositions before inventing raw layouts.
- Validate structurally, then inspect visually.

## Entry Point

Run everything through:

```bash
bash scripts/fastslides.sh <command> [options]
```

Core commands:

- `desktop`
- `health`
- `state`
- `analyze-project --path <absolute-project-path>`
- `design-system`
- `component-catalog`
- `component-template --name <name>`
- `composition-template --name <name>`
- `recipe-template --name <name>`
- `open-project --path <absolute-project-path>`
- `validate-project --path <absolute-project-path>`
- `compile-project-scene --path <absolute-project-path>`
- `compile-project-scene-manifest --path <absolute-project-path>`
- `compile-project-scene-slide --path <absolute-project-path> --index <N>`
- `preview-url --path <absolute-project-path>`
- `inspect-slide --path <absolute-project-path> [--slide N]`
- `smoke --path <absolute-project-path> [--slide N]`
- `mcp-smoke`
- `init`
- `validate-local`
- `asset-audit`

## Runtime Contract

- Hook API: `http://127.0.0.1:38473`
- MCP endpoint: `http://127.0.0.1:38474/mcp`
- Default browser preview: `http://127.0.0.1:1420`
- `preview-url`, `inspect-slide`, and `smoke` require the browser preview to be reachable
- `FASTSLIDES_PREVIEW_URL` can override the preview host/base

## Working Loop

1. Start the app if needed:

```bash
bash scripts/fastslides.sh desktop
```

2. Confirm the hook is up:

```bash
bash scripts/fastslides.sh health
```

3. Read the available system before writing slides:

```bash
bash scripts/fastslides.sh design-system
bash scripts/fastslides.sh component-catalog
```

4. Open the deck:

```bash
bash scripts/fastslides.sh open-project --path /absolute/path/to/project-folder
```

5. Validate through the app and local validators:

```bash
bash scripts/fastslides.sh validate-project --path /absolute/path/to/project-folder
bash scripts/fastslides.sh validate-local --project-dir /absolute/path/to/project-folder
bash scripts/fastslides.sh asset-audit --project-dir /absolute/path/to/project-folder --top 10
```

If rendered output disagrees with the MDX, compile the slide scene before rewriting the deck:

```bash
bash scripts/fastslides.sh compile-project-scene-slide --path /absolute/path/to/project-folder --index 0
```

6. Inspect changed slides when layout or styling changed:

```bash
bash scripts/fastslides.sh inspect-slide --path /absolute/path/to/project-folder --slide 1
```

If you need one command for readiness plus capture, use:

```bash
bash scripts/fastslides.sh smoke --path /absolute/path/to/project-folder --slide 1
```

## Authoring Rules

- Start from a recipe or composition when one exists.
- Fall back to raw `Area` placement only when the existing system cannot express the slide well.
- Keep one conclusion per slide.
- Use empty space intentionally; do not fill the grid because it is available.
- Treat overflow, clipping, and dense commentary rails as failures.
- Keep assets inside the project folder and use relative paths.
- Prefer native FastSlides primitives and real screenshots or editorial images over generated SVG diagrams for business decks.
- If validation and scene compile are clean but captured preview assets are wrong, suspect preview/runtime asset resolution before rewriting the MDX.

## Deck Contract

```text
<project>/
  page.mdx
  slides.css
  images/
  media/
  data/
```

`page.mdx` should include frontmatter like:

```yaml
---
project: "folder-name"
title: "Presentation"
subtitle: "Project Overview"
date: "Month YYYY"
---
```

Each slide should use:

- `<section className="slide">`
- one `Canvas`
- `Area` regions for layout

## Agent Stance

- Be expressive in the slide content, not random in the layout system.
- Prefer strong composition over many decorative primitives.
- Prefer semantic recipes like compare, operating model, or quote-plus-evidence before inventing custom diagrams.
- Use the running app and MCP to inspect what exists before adding new structure.
- Hand off only after validation and visual inspection both pass.

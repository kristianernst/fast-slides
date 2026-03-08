# FastSlides

FastSlides is a desktop-first slide authoring tool: a Tauri app, a local hook API, an embedded MCP server, and a lean skill for agents working on folder-based MDX decks.

## What matters

- The desktop app is the runtime.
- A deck is just a folder with `page.mdx`, `slides.css`, and local assets.
- Agents should use the running app plus MCP as the source of truth, not stale docs.
- The supported slide contract is spatial: one `Canvas`, multiple `Area` regions, relative assets only.

## Run

From [`fastslides-desktop/`](/Users/kristianernst/work/dev/tooling/fast-slides/fastslides-desktop):

```bash
npm install
npm run tauri:dev
```

Toolchain notes:

- Rust stable `>= 1.85`
- Type check: `npx tsc --noEmit`
- Rust lint: `cargo clippy --manifest-path src-tauri/Cargo.toml`

## Local agent surfaces

Hook API:

- `GET http://127.0.0.1:38473/health`
- `GET http://127.0.0.1:38473/app-state`
- `POST http://127.0.0.1:38473/open-project`
- `POST http://127.0.0.1:38473/validate-project`
- `GET http://127.0.0.1:38473/preview-url?path=...`

MCP:

- `http://127.0.0.1:38474/mcp`
- localhost-only
- use it for design-system lookup, project analysis, project open/validate, compile output, and capture helpers

## Skill entry point

Use [`skills/fastslides/SKILL.md`](/Users/kristianernst/work/dev/tooling/fast-slides/skills/fastslides/SKILL.md) and its single command surface:

```bash
bash skills/fastslides/scripts/fastslides.sh <command> [options]
```

Useful commands:

- `desktop`
- `health`
- `open-project --path /absolute/path`
- `validate-project --path /absolute/path`
- `inspect-slide --path /absolute/path --slide 1`
- `smoke --path /absolute/path --slide 1`
- `mcp-smoke`

## Deck contract

```text
<project>/
  page.mdx
  slides.css
  images/
  media/
  data/
```

Rules:

- use `<section className="slide">`
- use one `Canvas` plus `Area` regions per slide
- prefer recipe/composition templates before raw placement
- keep one conclusion per slide
- keep assets inside the deck folder
- validate structurally, then inspect visually

## Repo layout

- [`fastslides-desktop/`](/Users/kristianernst/work/dev/tooling/fast-slides/fastslides-desktop): Tauri app
- [`projects/`](/Users/kristianernst/work/dev/tooling/fast-slides/projects): sample decks
- [`skills/fastslides/`](/Users/kristianernst/work/dev/tooling/fast-slides/skills/fastslides): agent skill and helper scripts

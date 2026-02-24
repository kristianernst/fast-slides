# FastSlides Desktop

Tauri 2 desktop app for managing filesystem-based FastSlides projects (one folder per project with `page.mdx`).

## What it does

- Open any project folder directly (single primary action)
- Keep a tracked project list (add via open, remove from list without deleting files)
- Render selected project slides inside the desktop workspace via `nextjs-preview`
- Expose a local agent hook API for coding-agent control and preview URL access
- Create new projects via backend command/API with a starter deck template
- Scaffolded decks include YAML frontmatter (`project`, `title`, `subtitle`, `date`)
- Edit and save `page.mdx`
- Validate slide structure and local asset links
- Open project folders in your OS file manager

## Stack

- Frontend: Next.js App Router + TypeScript (static export)
- Backend: Rust (Tauri commands)
- Storage: local filesystem + config at `~/.fastslides/config.json`

## Commands

Run from `/Users/kristianernst/work/dev/tooling/fast-slides/fastslides-desktop`:

```bash
npm install
npm run tauri:dev
```

For slide rendering, run the deck preview service in another terminal (port `34773` by default):

```bash
bash /Users/kristianernst/.agents/skills/fastslides/scripts/start_deck_service.sh --runtime local
```

## Agent Hook API

When the desktop app is running, it starts a local API server (default: `http://127.0.0.1:38473`) for coding-agent integration.

Endpoints:

- `GET /health`
- `GET /app-state`
- `GET /preview-url?path=<absolute-project-path>`
- `POST /open-project` with JSON body `{ "path": "/absolute/path/to/project" }`
- `POST /validate-project` with JSON body `{ "path": "/absolute/path/to/project" }`

Examples:

```bash
curl http://127.0.0.1:38473/health
curl "http://127.0.0.1:38473/preview-url?path=/absolute/path/to/project"
curl -X POST http://127.0.0.1:38473/open-project -H "content-type: application/json" -d '{"path":"/absolute/path/to/project"}'
```

## Share FastSlides Skill

In macOS app menu (`FastSlides` next to the Apple logo), use:

- `Download FastSlides Skill…`

This opens a save dialog and exports the FastSlides skill folder as a `.zip` archive.

Build app binary (no installer):

```bash
npm run tauri:build -- --debug --no-bundle
```

Build installers/bundles:

```bash
npm run tauri:build
```

Create macOS DMG installer (drag FastSlides into Applications):

```bash
npm run tauri:build:dmg
```

Output file:

```text
src-tauri/target/release/bundle/dmg/FastSlides_<version>_aarch64.dmg
```

Quick verify the drag-and-drop payload:

```bash
hdiutil attach src-tauri/target/release/bundle/dmg/FastSlides_0.1.0_aarch64.dmg -nobrowse
```

The mounted volume should contain:
- `FastSlides.app`
- `Applications` symlink

## Optional environment variables

- `FASTSLIDES_HOME`: override config directory (default: `~/.fastslides`)
- `FASTSLIDES_PROJECTS_DIR`: optional root discovery source (not required for open-project flow)
- `NEXT_PUBLIC_FASTSLIDES_PREVIEW_URL`: preview server base URL (default: `http://127.0.0.1:34773`)
- `FASTSLIDES_PREVIEW_URL`: backend preview base URL used by agent hook (default: `http://127.0.0.1:34773`)
- `FASTSLIDES_AGENT_HOOK_ADDR`: bind address for local agent hook API (default: `127.0.0.1:38473`)
- `FASTSLIDES_SKILL_DIR`: optional explicit FastSlides skill source directory for menu export

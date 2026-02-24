# FastSlides Desktop

## Cursor Cloud specific instructions

### Architecture

Tauri 2 desktop app: Rust backend (`fastslides-desktop/src-tauri/`) + Next.js frontend (`fastslides-desktop/app/`). No databases or external services required.

### Running the app

From `fastslides-desktop/`:
```
npm run tauri:dev
```
This starts both the Next.js dev server (port 1420) and the Tauri Rust backend. First build takes ~60s for Rust compilation; subsequent rebuilds are incremental.

### Agent Hook API

When the app is running, a local HTTP API is available at `http://127.0.0.1:38473`. Key endpoints:
- `GET /health` — health check
- `GET /app-state` — current config and projects
- `POST /open-project` — open a project folder (JSON body: `{"path": "..."}`)
- `POST /validate-project` — validate a project (JSON body: `{"path": "..."}`)
- `GET /preview-url?path=...` — get preview URL for a project

### Key caveats

- **Rust toolchain**: Requires Rust stable >= 1.85 (the `time` crate uses `edition2024`). Run `rustup default stable` to ensure the latest stable is active.
- **Linux system deps**: Tauri 2 on Ubuntu 24.04 needs `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`, `libssl-dev`, `libxdo-dev`. These are system packages, not managed by the update script.
- **Compiler warnings**: On Linux, you'll see warnings about unused `MenuItem`, `PredefinedMenuItem`, `Manager` imports and unreachable code in `export_fastslides_skill` — these are expected (macOS-only features).
- **EGL warning**: `libEGL warning: DRI3 error` is harmless in headless/VM environments.

### Lint and type checking

- TypeScript: No ESLint config is present. Use `npx tsc --noEmit` in `fastslides-desktop/` for type checking.
- Rust: `cargo clippy` in `fastslides-desktop/src-tauri/` for linting.

### Testing

- No automated test suite exists in this repo.
- Validate functionality via the agent hook API endpoints and the local validation scripts in `skills/fastslides/scripts/`.

### Sample project

A sample deck project is at `projects/tauri-is-awesome/` with `page.mdx` and `assets/`.

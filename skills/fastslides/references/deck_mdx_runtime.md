# FastSlides Runtime Reference

## Core model

FastSlides Desktop starts a local hook server when the app launches.

- Default hook URL: `http://127.0.0.1:38473`
- The app can open project folders directly (`page.mdx` required).
- External URL-based renderers are optional and not required for the core workflow.

## Hook endpoints

- `GET /health`
- `GET /app-state`
- `GET /preview-url?path=<absolute-project-path>`
- `POST /open-project` with `{ "path": "..." }`
- `POST /validate-project` with `{ "path": "..." }`

## Recommended agent flow

1. Start desktop app.
2. Wait for `/health` to return `ok: true`.
3. Open/validate projects via hook endpoints.
4. Edit `page.mdx` + assets as needed.
5. Re-run validation.

## Deck folder contract

```text
<project>/
  page.mdx
  images/
  media/
  data/
```

Validation assumptions:

- `page.mdx` exists.
- slide blocks use `<section className="slide">`.
- local asset links stay inside the project folder.

## Skill scripts kept on purpose

- `scripts/fastslides.sh`: single command interface
- `scripts/init_deck_project.sh`: scaffold wrapper
- `scripts/scaffold_deck_project.py`: scaffold implementation
- `scripts/validate_deck_project.py`: structural/content checks
- `scripts/asset_audit.py`: asset usage checks

No docker/container runtime is part of this skill.

# Visual Inspection (Playwright)

## Why this is required

Structural checks alone do not catch visual regressions like clipping, overlap, poor contrast, or broken hierarchy.  
Agents should perform screenshot-based inspection for slides they generate or heavily edit.

## Prerequisites

- FastSlides Desktop app running (hook server reachable)
- Playwright wrapper available
  - Preferred: `$CODEX_HOME/skills/playwright/scripts/playwright_cli.sh`
  - Alternate: `~/.agents/skills/playwright/scripts/playwright_cli.sh`
- `npx` available on PATH (wrapper depends on it)

## Capture a screenshot

```bash
bash scripts/fastslides.sh inspect-slide --path /absolute/path/to/project-folder --slide 1
```

Optional:

- `--slide N`: capture a specific 1-based slide index
- `--output-dir DIR`: custom artifact folder
- `--headed`: run browser non-headless for interactive debugging

The command prints an absolute PNG path.

## Recommended QA loop

1. Run structural validation.
2. Capture one screenshot per changed slide.
3. Inspect screenshots for:
   - overflow/clipping
   - misaligned grids/cards
   - text contrast/readability
   - code/mermaid rendering quality
4. Apply fixes in `page.mdx` or `slides.css`.
5. Re-capture and verify.

## Typical sequence

```bash
bash scripts/fastslides.sh validate-project --path /absolute/path/to/project-folder
bash scripts/fastslides.sh validate-local --project-dir /absolute/path/to/project-folder
bash scripts/fastslides.sh inspect-slide --path /absolute/path/to/project-folder --slide 1
bash scripts/fastslides.sh inspect-slide --path /absolute/path/to/project-folder --slide 2
```

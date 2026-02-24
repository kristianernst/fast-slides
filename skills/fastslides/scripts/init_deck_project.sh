#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<'USAGE'
Usage:
  bash scripts/init_deck_project.sh [--project <name> | --project-dir <path>] [options]

Options:
  --project <name>          Project folder name under --projects-dir
  --project-dir <path>      Absolute path to project folder
  --projects-dir <dir>      Projects root (used with --project)
  --project-key <key>       Frontmatter project key override
  --title <text>            Cover slide heading (default: Presentation)
  --subtitle <text>         Cover slide subtitle (default: Project Overview)
  --date <text>             Cover date label (default: Month YYYY)
  --force                   Overwrite existing page.mdx

Examples:
  bash scripts/init_deck_project.sh --project tauri-cool --projects-dir /abs/projects
  bash scripts/init_deck_project.sh --project-dir /abs/projects/tauri-cool --title "Why Tauri"
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

exec python3 "${SCRIPT_DIR}/scaffold_deck_project.py" "$@"


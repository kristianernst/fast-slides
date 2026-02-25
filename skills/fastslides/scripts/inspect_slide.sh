#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<'USAGE'
Usage:
  bash scripts/inspect_slide.sh --path <absolute-project-path> [--slide N] [--output-dir DIR] [--headed]

Description:
  Uses FastSlides hook preview URL + Playwright CLI to capture a screenshot of a slide.
  This enables visual QA (not just structural checks) for generated decks.

Options:
  --path <absolute-project-path>   Required project path
  --slide <N>                      1-based slide index to capture (default: 1)
  --output-dir <DIR>               Screenshot artifact dir (default: ./output/playwright/fastslides-inspect)
  --headed                         Launch browser headed (default: headless)
  -h, --help                       Show this help

Environment:
  FASTSLIDES_PLAYWRIGHT_CLI        Explicit path to playwright wrapper script
  CODEX_HOME                       Used to discover $CODEX_HOME/skills/playwright/scripts/playwright_cli.sh
USAGE
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

resolve_pwcli() {
  local candidates=()

  if [[ -n "${FASTSLIDES_PLAYWRIGHT_CLI:-}" ]]; then
    candidates+=("${FASTSLIDES_PLAYWRIGHT_CLI}")
  fi

  local codex_home="${CODEX_HOME:-$HOME/.codex}"
  candidates+=("${codex_home}/skills/playwright/scripts/playwright_cli.sh")
  candidates+=("$HOME/.agents/skills/playwright/scripts/playwright_cli.sh")

  local candidate=""
  for candidate in "${candidates[@]}"; do
    if [[ -x "${candidate}" ]]; then
      printf '%s' "${candidate}"
      return 0
    fi
  done

  return 1
}

project_path=""
slide_index="1"
output_dir=""
headed="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --path)
      project_path="${2:-}"
      shift 2
      ;;
    --slide)
      slide_index="${2:-}"
      shift 2
      ;;
    --output-dir)
      output_dir="${2:-}"
      shift 2
      ;;
    --headed)
      headed="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "${project_path}" ]]; then
  echo "Missing required argument: --path <absolute-project-path>" >&2
  exit 1
fi

if [[ ! -d "${project_path}" ]]; then
  echo "Project path not found: ${project_path}" >&2
  exit 1
fi

if [[ ! "${slide_index}" =~ ^[0-9]+$ ]] || [[ "${slide_index}" -lt 1 ]]; then
  echo "--slide must be a positive integer (1-based)." >&2
  exit 1
fi

if [[ -z "${output_dir}" ]]; then
  output_dir="${PWD}/output/playwright/fastslides-inspect"
fi

require_cmd bash
require_cmd python3

if ! command -v npx >/dev/null 2>&1; then
  echo "npx is required for Playwright CLI wrapper." >&2
  exit 1
fi

pwcli="$(resolve_pwcli || true)"
if [[ -z "${pwcli}" ]]; then
  cat >&2 <<'ERROR_MSG'
Playwright wrapper script not found.
Install the playwright skill and ensure one of these paths exists:
  - $CODEX_HOME/skills/playwright/scripts/playwright_cli.sh
  - ~/.agents/skills/playwright/scripts/playwright_cli.sh
Or set FASTSLIDES_PLAYWRIGHT_CLI explicitly.
ERROR_MSG
  exit 1
fi

preview_json="$(bash "${SCRIPT_DIR}/fastslides.sh" preview-url --path "${project_path}")"
preview_url="$(printf '%s' "${preview_json}" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("preview_url",""))')"

if [[ -z "${preview_url}" ]]; then
  echo "Failed to resolve preview URL from hook response." >&2
  echo "Response: ${preview_json}" >&2
  exit 1
fi

mkdir -p "${output_dir}"

session_name="fastslides-$(date +%s)-$RANDOM"
slide_steps=$((slide_index - 1))

pushd "${output_dir}" >/dev/null

before_screens="$(mktemp)"
after_screens="$(mktemp)"
trap 'rm -f "${before_screens}" "${after_screens}"' EXIT

find . -maxdepth 1 -type f -name '*.png' -print | sort >"${before_screens}"

open_args=(open "${preview_url}")
if [[ "${headed}" == "true" ]]; then
  open_args+=(--headed)
fi

"${pwcli}" --session "${session_name}" "${open_args[@]}"
"${pwcli}" --session "${session_name}" resize 1920 1080
"${pwcli}" --session "${session_name}" snapshot >/dev/null

if [[ "${slide_steps}" -gt 0 ]]; then
  for ((i = 0; i < slide_steps; i += 1)); do
    "${pwcli}" --session "${session_name}" press ArrowRight
  done
fi

"${pwcli}" --session "${session_name}" snapshot >/dev/null
"${pwcli}" --session "${session_name}" screenshot >/dev/null
"${pwcli}" --session "${session_name}" close >/dev/null || true

find . -maxdepth 1 -type f -name '*.png' -print | sort >"${after_screens}"

newest_png="$(comm -13 "${before_screens}" "${after_screens}" | tail -n 1)"
if [[ -z "${newest_png}" ]]; then
  newest_png="$(ls -1t ./*.png 2>/dev/null | head -n 1 || true)"
fi

if [[ -z "${newest_png}" ]]; then
  echo "Screenshot command completed but no PNG artifact was found in ${output_dir}." >&2
  exit 1
fi

abs_path="$(cd -- "$(dirname -- "${newest_png}")" && pwd)/$(basename -- "${newest_png}")"
echo "${abs_path}"

popd >/dev/null

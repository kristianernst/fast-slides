#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
BASE_URL="${FASTSLIDES_AGENT_HOOK_URL:-http://127.0.0.1:38473}"

usage() {
  cat <<USAGE
Usage:
  bash scripts/fastslides.sh <command> [options]

Commands:
  desktop [--install]                          Launch FastSlides Desktop (tauri:dev)
  health                                       Hook server health check
  state                                        Read hook app state
  open-project --path <absolute-project-path>  Open project in desktop app
  validate-project --path <absolute-project-path>
                                                Run desktop validation for a project
  preview-url --path <absolute-project-path>   Build preview URL for a project path
  inspect-slide --path <absolute-project-path> [--slide N] [--output-dir DIR] [--headed]
                                                Open preview URL and capture slide screenshot via Playwright
  init [init_deck_project args...]             Scaffold a new deck project
  validate-local [validate_deck_project args...] 
                                                Run local structural validation
  asset-audit [asset_audit args...]            Run local asset audit

Environment:
  FASTSLIDES_AGENT_HOOK_URL      Hook base URL (default: ${BASE_URL})
  FASTSLIDES_DESKTOP_APP_DIR     Override desktop app path
  DECK_PROJECTS_DIR              Default projects root for init/validate scripts
USAGE
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

has_desktop_app_markers() {
  local base="${1:-}"
  [[ -d "${base}" ]] || return 1
  [[ -f "${base}/package.json" ]] || return 1
  [[ -d "${base}/src-tauri" ]] || return 1
  [[ -f "${base}/app/page.tsx" || -f "${base}/app/page.jsx" || -f "${base}/app/page.js" || -f "${base}/app/page.ts" ]] || return 1
  return 0
}

canonical_dir() {
  local raw="${1:-}"
  cd -- "${raw}" 2>/dev/null && pwd
}

resolve_desktop_app_dir() {
  local candidate=""
  local candidate_abs=""
  local walk=""

  if [[ -n "${FASTSLIDES_DESKTOP_APP_DIR:-}" ]]; then
    if candidate_abs="$(canonical_dir "${FASTSLIDES_DESKTOP_APP_DIR}")"; then
      if has_desktop_app_markers "${candidate_abs}"; then
        echo "${candidate_abs}"
        return 0
      fi
    fi
  fi

  for candidate in "${PWD}/fastslides-desktop" "${PWD}"; do
    if candidate_abs="$(canonical_dir "${candidate}")"; then
      if has_desktop_app_markers "${candidate_abs}"; then
        echo "${candidate_abs}"
        return 0
      fi
    fi
  done

  walk="${PWD}"
  while [[ "${walk}" != "/" ]]; do
    candidate="${walk}/fastslides-desktop"
    if candidate_abs="$(canonical_dir "${candidate}")"; then
      if has_desktop_app_markers "${candidate_abs}"; then
        echo "${candidate_abs}"
        return 0
      fi
    fi
    walk="$(dirname "${walk}")"
  done

  walk="$(canonical_dir "${SCRIPT_DIR}")"
  while [[ -n "${walk}" && "${walk}" != "/" ]]; do
    candidate="${walk}/fastslides-desktop"
    if candidate_abs="$(canonical_dir "${candidate}")"; then
      if has_desktop_app_markers "${candidate_abs}"; then
        echo "${candidate_abs}"
        return 0
      fi
    fi
    walk="$(dirname "${walk}")"
  done

  return 1
}

parse_path_arg() {
  local path_value=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --path)
        path_value="${2:-}"
        shift 2
        ;;
      *)
        echo "Unknown argument: $1" >&2
        usage
        exit 1
        ;;
    esac
  done

  if [[ -z "${path_value}" ]]; then
    echo "Missing required argument: --path <absolute-project-path>" >&2
    exit 1
  fi

  printf '%s' "${path_value}"
}

hook_get() {
  local endpoint="$1"
  require_cmd curl
  exec curl -sS "${BASE_URL}${endpoint}"
}

hook_post_path() {
  local endpoint="$1"
  shift
  local project_path
  project_path="$(parse_path_arg "$@")"

  require_cmd curl
  require_cmd python3

  local payload
  payload="$(python3 -c 'import json,sys; print(json.dumps({"path": sys.argv[1]}))' "${project_path}")"
  exec curl -sS -X POST "${BASE_URL}${endpoint}" -H "content-type: application/json" -d "${payload}"
}

hook_preview_url() {
  local project_path
  project_path="$(parse_path_arg "$@")"
  require_cmd curl
  exec curl -sS -G "${BASE_URL}/preview-url" --data-urlencode "path=${project_path}"
}

start_desktop() {
  local install="false"
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --install)
        install="true"
        shift
        ;;
      *)
        echo "Unknown argument for desktop: $1" >&2
        usage
        exit 1
        ;;
    esac
  done

  local app_dir
  app_dir="$(resolve_desktop_app_dir || true)"
  if [[ -z "${app_dir}" || ! -d "${app_dir}" ]]; then
    echo "FastSlides desktop app directory not found." >&2
    echo "Set FASTSLIDES_DESKTOP_APP_DIR or run from a repo containing fastslides-desktop/." >&2
    exit 1
  fi

  require_cmd npm
  cd "${app_dir}"

  if [[ "${install}" == "true" ]] || [[ ! -d "node_modules" ]]; then
    echo "[INFO] Installing npm dependencies in ${app_dir}"
    npm install
  fi

  echo "[INFO] Starting FastSlides Desktop in dev mode from ${app_dir}"
  exec npm run tauri:dev
}

forward_script() {
  local script_name="$1"
  shift
  local script_path="${SCRIPT_DIR}/${script_name}"
  if [[ ! -f "${script_path}" ]]; then
    echo "Missing helper script: ${script_path}" >&2
    exit 1
  fi
  exec bash "${script_path}" "$@"
}

forward_python() {
  local script_name="$1"
  shift
  local script_path="${SCRIPT_DIR}/${script_name}"
  if [[ ! -f "${script_path}" ]]; then
    echo "Missing helper script: ${script_path}" >&2
    exit 1
  fi
  require_cmd python3
  exec python3 "${script_path}" "$@"
}

cmd="${1:-}"
shift || true

case "${cmd}" in
  desktop)
    start_desktop "$@"
    ;;
  health)
    hook_get "/health"
    ;;
  state)
    hook_get "/app-state"
    ;;
  open-project)
    hook_post_path "/open-project" "$@"
    ;;
  validate-project)
    hook_post_path "/validate-project" "$@"
    ;;
  preview-url)
    hook_preview_url "$@"
    ;;
  inspect-slide)
    forward_script "inspect_slide.sh" "$@"
    ;;
  init)
    forward_script "init_deck_project.sh" "$@"
    ;;
  validate-local)
    forward_python "validate_deck_project.py" "$@"
    ;;
  asset-audit)
    forward_python "asset_audit.py" "$@"
    ;;
  -h|--help|"")
    usage
    ;;
  *)
    echo "Unknown command: ${cmd}" >&2
    usage
    exit 1
    ;;
esac

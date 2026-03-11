#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
BASE_URL="${FASTSLIDES_AGENT_HOOK_URL:-http://127.0.0.1:38473}"
MCP_URL="${FASTSLIDES_MCP_URL:-http://127.0.0.1:38474/mcp}"

usage() {
  cat <<USAGE
Usage:
  bash skills/fastslides/scripts/fastslides.sh <command> [options]

Commands:
  desktop [--install]                          Launch FastSlides Desktop (tauri:dev)
  health                                       Hook server health check
  state                                        Read hook app state
  analyze-project --path <absolute-project-path>
                                                Run project density and outline analysis
  design-system                                Read the base FastSlides design-system registry
  component-catalog                            Read the FastSlides component phonebook
  install-codex-mcp                            Install FastSlides MCP into Codex config.toml
  component-template --name <component>        Read canonical MDX for one primitive, pattern, composition, recipe, or saved snippet
  composition-template --name <composition>    Read canonical MDX for one reusable composition
  recipe-template --name <recipe>              Read canonical MDX for one full-slide recipe
  open-project --path <absolute-project-path>  Open project in desktop app
  validate-project --path <absolute-project-path>
                                                Run desktop validation for a project
  compile-project-scene --path <absolute-project-path>
                                                Compile the full typed scene graph
  compile-project-scene-manifest --path <absolute-project-path>
                                                Compile slide metadata and manifest only
  compile-project-scene-slide --path <absolute-project-path> --index <N>
                                                Compile one slide scene node tree by zero-based index
  preview-url --path <absolute-project-path>   Build browser preview URL for a project path
  inspect-slide --path <absolute-project-path> [--slide N] [--output-dir DIR] [--headed]
                                                Ask the backend to capture a slide screenshot and return the PNG path
  smoke --path <absolute-project-path> [--slide N] [--output-dir DIR] [--headed]
                                                Run end-to-end agent smoke test (hook + browser preview + open + validate + screenshot)
  mcp-smoke [mcp_smoke.py options]             Verify embedded MCP server over localhost
  init [init_deck_project args...]             Scaffold a new deck project
  validate-local [validate_deck_project args...] 
                                                Run local structural validation
  asset-audit [asset_audit args...]            Run local asset audit

Environment:
  FASTSLIDES_AGENT_HOOK_URL      Hook base URL (default: ${BASE_URL})
  FASTSLIDES_MCP_URL             Embedded MCP URL (default: ${MCP_URL})
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

parse_name_arg() {
  local name_value=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --name)
        name_value="${2:-}"
        shift 2
        ;;
      *)
        echo "Unknown argument: $1" >&2
        usage
        exit 1
        ;;
    esac
  done

  if [[ -z "${name_value}" ]]; then
    echo "Missing required argument: --name <template-name>" >&2
    exit 1
  fi

  printf '%s' "${name_value}"
}

hook_get() {
  local endpoint="$1"
  require_cmd curl
  exec curl -sS "${BASE_URL}${endpoint}"
}

hook_post_empty() {
  local endpoint="$1"
  require_cmd curl
  exec curl -sS -X POST "${BASE_URL}${endpoint}"
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

hook_get_path() {
  local endpoint="$1"
  shift
  local project_path
  project_path="$(parse_path_arg "$@")"
  require_cmd curl
  exec curl -sS -G "${BASE_URL}${endpoint}" --data-urlencode "path=${project_path}"
}

hook_compile_project_scene_slide() {
  local project_path=""
  local slide_index=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --path)
        project_path="${2:-}"
        shift 2
        ;;
      --index)
        slide_index="${2:-}"
        shift 2
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
  if [[ -z "${slide_index}" ]]; then
    echo "Missing required argument: --index <zero-based-slide-index>" >&2
    exit 1
  fi
  if [[ ! "${slide_index}" =~ ^[0-9]+$ ]]; then
    echo "--index must be a zero-based integer." >&2
    exit 1
  fi

  require_cmd curl
  exec curl -sS -G \
    "${BASE_URL}/compile-project-scene-slide" \
    --data-urlencode "path=${project_path}" \
    --data-urlencode "index=${slide_index}"
}

hook_named_template() {
  local endpoint="$1"
  shift
  local template_name
  template_name="$(parse_name_arg "$@")"
  require_cmd curl
  exec curl -sS -G "${BASE_URL}${endpoint}" --data-urlencode "name=${template_name}"
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

run_smoke() {
  local project_path=""
  local slide_index="1"
  local output_dir=""
  local headed="false"

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
      *)
        echo "Unknown argument for smoke: $1" >&2
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

  require_cmd curl
  require_cmd python3
  require_cmd bash

  local payload
  payload="$(python3 -c 'import json,sys; print(json.dumps({"path": sys.argv[1]}))' "${project_path}")"

  echo "[SMOKE] Checking agent hook health..."
  local health_json
  health_json="$(curl -sS --fail "${BASE_URL}/health")"
  if ! printf '%s' "${health_json}" | python3 -c 'import json,sys; data=json.load(sys.stdin); raise SystemExit(0 if data.get("ok") else 1)'; then
    echo "Hook health response did not report ok=true." >&2
    echo "Response: ${health_json}" >&2
    exit 1
  fi

  echo "[SMOKE] Resolving preview URL..."
  local preview_json
  local preview_url
  preview_json="$(curl -sS --fail -G "${BASE_URL}/preview-url" --data-urlencode "path=${project_path}")"
  preview_url="$(printf '%s' "${preview_json}" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("preview_url",""))')"
  if [[ -z "${preview_url}" ]]; then
    echo "Failed to resolve preview URL from hook response." >&2
    echo "Response: ${preview_json}" >&2
    exit 1
  fi

  echo "[SMOKE] Checking preview reachability..."
  if ! curl -sS --max-time 5 -o /dev/null "${preview_url}" 2>/dev/null; then
    echo "Preview URL is not reachable: ${preview_url}" >&2
    exit 1
  fi

  echo "[SMOKE] Opening project..."
  local open_json
  open_json="$(curl -sS --fail -X POST "${BASE_URL}/open-project" -H "content-type: application/json" -d "${payload}")"
  if ! printf '%s' "${open_json}" | python3 -c 'import json,sys; data=json.load(sys.stdin); raise SystemExit(0 if data.get("path") else 1)'; then
    echo "Open project response missing expected path field." >&2
    echo "Response: ${open_json}" >&2
    exit 1
  fi

  echo "[SMOKE] Validating project..."
  local validate_json
  validate_json="$(curl -sS --fail -X POST "${BASE_URL}/validate-project" -H "content-type: application/json" -d "${payload}")"
  local error_count
  error_count="$(printf '%s' "${validate_json}" | python3 -c 'import json,sys; data=json.load(sys.stdin); print(len(data.get("errors", [])))')"
  if [[ "${error_count}" != "0" ]]; then
    echo "Validation reported ${error_count} error(s)." >&2
    echo "${validate_json}" >&2
    exit 1
  fi

  local inspect_args=("--path" "${project_path}" "--slide" "${slide_index}")
  if [[ -n "${output_dir}" ]]; then
    inspect_args+=("--output-dir" "${output_dir}")
  fi
  if [[ "${headed}" == "true" ]]; then
    inspect_args+=("--headed")
  fi

  echo "[SMOKE] Capturing screenshot..."
  local screenshot_path
  screenshot_path="$(bash "${SCRIPT_DIR}/inspect_slide.sh" "${inspect_args[@]}")"
  echo "[SMOKE] Screenshot: ${screenshot_path}"
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
  analyze-project)
    hook_get_path "/analyze-project" "$@"
    ;;
  design-system)
    hook_get "/design-system"
    ;;
  component-catalog)
    hook_get "/component-catalog"
    ;;
  component-template)
    hook_named_template "/component-template" "$@"
    ;;
  install-codex-mcp)
    hook_post_empty "/install-codex-mcp"
    ;;
  composition-template)
    hook_named_template "/composition-template" "$@"
    ;;
  recipe-template)
    hook_named_template "/recipe-template" "$@"
    ;;
  open-project)
    hook_post_path "/open-project" "$@"
    ;;
  validate-project)
    hook_post_path "/validate-project" "$@"
    ;;
  compile-project-scene)
    hook_get_path "/compile-project-scene" "$@"
    ;;
  compile-project-scene-manifest)
    hook_get_path "/compile-project-scene-manifest" "$@"
    ;;
  compile-project-scene-slide)
    hook_compile_project_scene_slide "$@"
    ;;
  preview-url)
    hook_preview_url "$@"
    ;;
  inspect-slide)
    forward_script "inspect_slide.sh" "$@"
    ;;
  smoke)
    run_smoke "$@"
    ;;
  mcp-smoke)
    forward_python "mcp_smoke.py" "$@"
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

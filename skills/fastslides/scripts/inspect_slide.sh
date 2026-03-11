#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${FASTSLIDES_AGENT_HOOK_URL:-http://127.0.0.1:38473}"

usage() {
  cat <<USAGE
Usage:
  bash skills/fastslides/scripts/inspect_slide.sh --path <absolute-project-path> [--slide N] [--output-dir DIR] [--headed]

Description:
  Calls the FastSlides backend slide-capture endpoint and returns the absolute
  PNG path for the captured slide.

Options:
  --path <absolute-project-path>   Required project path
  --slide <N>                      1-based slide index to capture (default: 1)
  --output-dir <DIR>               Optional artifact dir override
  --headed                         Launch browser headed during capture
  -h, --help                       Show this help

Environment:
  FASTSLIDES_AGENT_HOOK_URL        Hook base URL (default: ${BASE_URL})
USAGE
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
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

require_cmd curl
require_cmd python3

payload="$(python3 - "${project_path}" "${slide_index}" "${output_dir}" "${headed}" <<'PY'
import json
import sys

project_path, slide_index, output_dir, headed = sys.argv[1:5]
payload = {
    "path": project_path,
    "slide": int(slide_index),
    "headed": headed == "true",
}
if output_dir:
    payload["output_dir"] = output_dir
print(json.dumps(payload))
PY
)"

response="$(curl -sS -X POST "${BASE_URL}/capture-slide-image" -H "content-type: application/json" -d "${payload}")"

image_path="$(printf '%s' "${response}" | python3 -c 'import json,sys; data=json.load(sys.stdin); print(data.get("image_path",""))')"
if [[ -n "${image_path}" ]]; then
  echo "${image_path}"
  exit 0
fi

error_message="$(printf '%s' "${response}" | python3 -c 'import json,sys; data=json.load(sys.stdin); print(data.get("error",""))' 2>/dev/null || true)"
if [[ -n "${error_message}" ]]; then
  echo "${error_message}" >&2
else
  echo "Failed to capture slide image." >&2
fi
echo "Response: ${response}" >&2
exit 1

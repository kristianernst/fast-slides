#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This installer only supports macOS." >&2
  exit 1
fi

repo="${FASTSLIDES_REPO:-kristianernst/fast-slides}"
manifest_url="${FASTSLIDES_MANIFEST_URL:-https://github.com/${repo}/releases/latest/download/latest.json}"
install_dir="${FASTSLIDES_INSTALL_DIR:-$HOME/Applications}"

case "$(uname -m)" in
  arm64|aarch64) platform_target="darwin-aarch64" ;;
  x86_64) platform_target="darwin-x86_64" ;;
  *)
    echo "Unsupported macOS architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

tmpdir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT

manifest_path="$tmpdir/latest.json"
archive_path="$tmpdir/FastSlides.app.tar.gz"
extract_dir="$tmpdir/extracted"

echo "Fetching release manifest from ${manifest_url}"
curl -fsSL "$manifest_url" -o "$manifest_path"

version="$(plutil -extract version raw -expect string "$manifest_path")"
download_url="$(plutil -extract "platforms.${platform_target}.url" raw -expect string "$manifest_path")"

echo "Downloading FastSlides ${version} for ${platform_target}"
curl -fL "$download_url" -o "$archive_path"

mkdir -p "$extract_dir"
tar -xzf "$archive_path" -C "$extract_dir"

source_app="$(find "$extract_dir" -type d -name '*.app' -print -quit)"
if [[ -z "$source_app" ]]; then
  echo "Failed to find an app bundle inside the downloaded archive." >&2
  exit 1
fi

app_name="$(basename "$source_app")"
destination_app="${install_dir}/${app_name}"

osascript -e "tell application \"${app_name%.app}\" to quit" >/dev/null 2>&1 || true
sleep 1

mkdir -p "$install_dir"
rm -rf "$destination_app"
ditto "$source_app" "$destination_app"
xattr -dr com.apple.quarantine "$destination_app" >/dev/null 2>&1 || true

echo
echo "Installed ${app_name%.app} ${version} to ${destination_app}"
echo "Launch it with:"
echo "  open \"$destination_app\""
echo
echo "If macOS still blocks the first launch, open:"
echo "  System Settings > Privacy & Security > Open Anyway"

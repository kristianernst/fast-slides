# macOS release

FastSlides now has a free macOS distribution path:

- GitHub Releases for downloads
- ad-hoc code signing for Apple Silicon compatibility
- Tauri updater signatures for in-app updates
- shell installers that pull the latest release from the repo

This avoids Apple Developer enrollment, but it does not avoid Gatekeeper entirely. Tauri documents that ad-hoc signing still may require users to whitelist the app in Privacy & Security, and Apple documents that software outside the App Store normally relies on Developer ID plus notarization for the cleanest trust flow. Sources: [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/), [Apple Gatekeeper behavior](https://support.apple.com/en-afri/102445).

## What the app expects

- Update manifest endpoint:
  - default: `https://github.com/kristianernst/fast-slides/releases/latest/download/latest.json`
  - override at build time with `FASTSLIDES_UPDATE_ENDPOINT`
- Update public key:
  - required at build time as `FASTSLIDES_UPDATE_PUBKEY`
- Ad-hoc macOS signing:
  - configured via `bundle.macOS.signingIdentity = "-"` in [tauri.conf.json](/Users/kristianernst/work/dev/tooling/fast-slides/fastslides-desktop/src-tauri/tauri.conf.json)

## One-time setup

1. Generate the updater signing key:

```bash
cd fastslides-desktop
npx tauri signer generate -w ~/.fastslides/fastslides-updater.key
```

Keep the private key contents for GitHub secret `TAURI_SIGNING_PRIVATE_KEY`.
Keep the password for `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
Copy the printed public key into repository variable `FASTSLIDES_UPDATE_PUBKEY`.

2. Add GitHub Actions secrets and variables.

Required secret:

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

Required variable:

- `FASTSLIDES_UPDATE_PUBKEY`

Optional variable:

- `FASTSLIDES_UPDATE_ENDPOINT`

Only set `FASTSLIDES_UPDATE_ENDPOINT` if releases will not live on `kristianernst/fast-slides`.

## Release flow

1. Bump the version in:

- `fastslides-desktop/package.json`
- `fastslides-desktop/src-tauri/Cargo.toml`
- `fastslides-desktop/src-tauri/tauri.conf.json`

2. Commit the version bump and push a tag:

```bash
git tag v0.1.2
git push origin v0.1.2
```

That triggers [`.github/workflows/release-macos.yml`](../.github/workflows/release-macos.yml), which:

- builds `aarch64-apple-darwin` and `x86_64-apple-darwin`
- ad-hoc signs the app with `APPLE_SIGNING_IDENTITY=-`
- uploads the DMG plus updater artifacts to the GitHub Release
- publishes `latest.json` for in-app updates

## User install and update

First install:

```bash
curl -fsSL https://raw.githubusercontent.com/kristianernst/fast-slides/main/scripts/install-fastslides-macos.sh | bash
```

Manual update:

```bash
curl -fsSL https://raw.githubusercontent.com/kristianernst/fast-slides/main/scripts/update-fastslides-macos.sh | bash
```

Both scripts install into `~/Applications` by default and remove the quarantine attribute when possible. If macOS still blocks first launch, use `System Settings > Privacy & Security > Open Anyway`.

## Local release check

Before pushing a tag, validate locally:

```bash
cd fastslides-desktop
npx tsc --noEmit
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

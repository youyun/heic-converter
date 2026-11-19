# Packaging

Installers are **not** committed to git. They are produced by GitHub Actions and attached to [GitHub Releases](https://github.com/youyun/heic-converter/releases).

| Artifact | Produced by | Notes |
|----------|-------------|--------|
| Windows `.msi` / NSIS `.exe` (+ `.sig`) | `windows-latest` job in `.github/workflows/release.yml` | libheif/HEVC linked into the binary — no PATH decoder |
| macOS `.dmg` / `.app` / `.app.tar.gz` (+ `.sig`) | `macos-latest` jobs (arm64 + x86_64) | Uses system `sips` |
| `latest.json` | `tauri-apps/tauri-action` | Static updater manifest for in-app updates |

## Cut a release

1. Ensure GitHub Actions secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are set (see root README → **In-app updates**).
2. From a clean `main`:

```sh
git tag v0.2.1
git push origin v0.2.1
```

That triggers the Release workflow. When it finishes, open the Releases page — installers, `.sig` files, and `latest.json` should be attached.

You can also run the workflow manually (**Actions → Release → Run workflow**) and pass a tag like `v0.2.1`.

## Upgrading

- **In-app (v0.2.1+):** **Check for updates** in the header → **Install and restart** when a newer signed release exists.
- **Manual:** re-download the latest installer and run it again (Windows MSI/NSIS and macOS `.app` replace).

## Local packaging (optional)

Signing env vars are required when `createUpdaterArtifacts` is enabled:

```sh
export TAURI_SIGNING_PRIVATE_KEY="path-or-contents-of-private-key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="…"

# macOS host
npm run tauri build

# Windows host (MSVC + WebView2 + vcpkg libheif[hevc]:x64-windows-static-md)
npm run tauri build
```

Outputs land under `src-tauri/target/release/bundle/` (gitignored).

# Packaging

Installers are **not** committed to git. They are produced by GitHub Actions and attached to [GitHub Releases](https://github.com/youyun/heic-converter/releases).

| Artifact | Produced by | Notes |
|----------|-------------|--------|
| Windows `.msi` / NSIS `.exe` | `windows-latest` job in `.github/workflows/release.yml` | libheif/HEVC linked into the binary — no PATH decoder |
| macOS `.dmg` / `.app` | `macos-latest` jobs (arm64 + x86_64) | Uses system `sips` |

## Cut a release

From a clean `main`:

```sh
git tag v0.2.0
git push origin v0.2.0
```

That triggers the Release workflow. When it finishes, open the Releases page and download the installers.

You can also run the workflow manually (**Actions → Release → Run workflow**) and pass a tag like `v0.2.0`.

## Upgrading

Users should re-download the latest installer and run it again — it overwrites/upgrades the previous install (Windows MSI/NSIS and macOS `.app` replace). There is no in-app auto-update yet.

## Local packaging (optional)

```sh
# macOS host
npm run tauri build

# Windows host (MSVC + WebView2 + vcpkg libheif[hevc]:x64-windows-static-md)
npm run tauri build
```

Outputs land under `src-tauri/target/release/bundle/` (gitignored).

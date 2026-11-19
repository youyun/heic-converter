# Packaging

Installers are **not** committed to git. They are produced by GitHub Actions and attached to [GitHub Releases](https://github.com/youyun/heic-converter/releases).

| Artifact | Produced by | Notes |
|----------|-------------|--------|
| Windows `.msi` / NSIS `.exe` | `windows-latest` job in `.github/workflows/release.yml` | libheif/HEVC linked into the binary — no PATH decoder |
| macOS `.dmg` / `.app` | `macos-latest` jobs (arm64 + x86_64) | Uses system `sips` |

## Cut a release

From a clean `main`:

```sh
git tag v0.1.1
git push origin v0.1.1
```

That triggers the Release workflow. When it finishes, open:

https://github.com/youyun/heic-converter/releases

and download the Windows installer asset.

You can also run the workflow manually (**Actions → Release → Run workflow**) and pass a tag like `v0.1.1`.

## Local packaging (optional)

```sh
# macOS host
npm run tauri build

# Windows host (MSVC + WebView2 + vcpkg libheif[hevc]:x64-windows-static-md)
npm run tauri build
```

Outputs land under `src-tauri/target/release/bundle/` (gitignored).

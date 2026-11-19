# HEIC Converter

Bulk **HEIC → JPG** desktop app for **macOS** and **Windows 10**, built with [Tauri 2](https://tauri.app/) + Rust.

## Features (MVP)

- Pick a folder, multi-select HEIC/HEIF files (virtualized list; pages load on scroll)
- Convert with live progress; jobs persist in SQLite and resume after crash/quit (skips done, retries failed)
- Optional destination folder (default: write JPG beside each HEIC)
- Optional delete of the original HEIC **only after** a successful JPG write
- Bounded worker pool (up to 4 threads); one image at a time per worker

## Requirements

### macOS

- Xcode CLT / Xcode
- Rust (`rustup`)
- Node.js 18+
- A HEIC decoder on `PATH`:
  - **Preferred:** [libheif](https://github.com/strukturag/libheif) → `brew install libheif` (provides `heif-dec`)
  - **Fallback:** Apple `sips` (built-in)

### Windows 10

- [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) (usually already present)
- Rust + Node.js
- Visual Studio Build Tools (MSVC) for Tauri
- A HEIC decoder on `PATH`:
  - Install libheif (e.g. via [vcpkg](https://vcpkg.io/) / Chocolatey / scoop) so `heif-dec` is available, **or**
  - Install Microsoft’s **HEIF Image Extensions** / **HEVC Video Extensions** if you later switch the converter to WIC

> Windows conversion is structured the same as macOS (`heif-dec` first). Full UI packaging on Win10 was not verified on this Mac.

## Develop

```sh
cd heic-converter
npm install
npm run tauri dev
```

Build a release bundle locally:

```sh
npm run tauri build
```

Outputs go under `src-tauri/target/release/bundle/` (gitignored). Prefer GitHub Releases for installers — do not commit `.msi` / `.exe` / `.dmg` binaries.

### Smoke / automation env (optional)

```sh
HEIC_CONVERTER_AUTO_FOLDER=/path/to/heics \
HEIC_CONVERTER_AUTO_CONVERT=1 \
npm run tauri dev
```

Opens the folder without a dialog; with `AUTO_CONVERT=1` also selects all and starts a job.

## Windows installer (GitHub Releases)

Installers are built by CI (`.github/workflows/release.yml`) on `windows-latest` and attached to a Release — not stored in git.

1. Tag and push: `git tag v0.1.0 && git push origin v0.1.0`
2. Wait for the **Release** workflow on the Actions tab
3. Open https://github.com/youyun/heic-converter/releases
4. Download the Windows `.msi` or NSIS `.exe`

Or: **Actions → Release → Run workflow** and pass tag `v0.1.0`.

See also [`pkg/README.md`](pkg/README.md).

## Smoke test (converter only)

With sample HEICs present:

```sh
cd src-tauri
cargo test --lib convert::tests::sample_heic_converts -- --nocapture
```

## Job store

SQLite lives in the app data directory (Tauri `app_data_dir`), file `jobs.sqlite3`. On startup, any item left in `running` is reset to `pending` so interrupted work can continue.

## License

Private / personal — adjust as needed.

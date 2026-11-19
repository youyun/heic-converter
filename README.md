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

Build a release bundle:

```sh
npm run tauri build
```

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

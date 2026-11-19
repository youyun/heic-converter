# HEIC Converter

Bulk **HEIC → JPG** desktop app for **macOS** and **Windows 10**, built with [Tauri 2](https://tauri.app/) + Rust.

## Features (MVP)

- Pick a folder, or **drag-and-drop** a HEIC onto the window (offers to open the parent folder for multi-select / bulk convert)
- Multi-select HEIC/HEIF files (virtualized list; pages load on scroll)
- Convert with live progress (`x/y` + bar); jobs show a readable name (folder · time), UUID secondary
- Confirm dialog before start: re-check delete-originals and custom destination (set/clear there)
- Jobs persist in SQLite and resume after crash/quit (skips done, retries failed)
- Optional destination folder (default: write JPG beside each HEIC)
- Optional delete of the original HEIC **only after** a successful JPG write
- Bounded worker pool (up to 4 threads); one image at a time per worker

## Decoder (no PATH setup)

Shipped installs convert **without** installing Homebrew, vcpkg, or Microsoft Store HEIF extensions:

| Platform | Built-in decoder |
|----------|------------------|
| **Windows** | In-process **libheif** + **libde265** (HEVC), statically linked at build time via vcpkg |
| **macOS** | Apple **`sips`** (ImageIO) — already on every Mac |

Optional extras (dev machines only):

- `heif-dec` on `PATH` (e.g. `brew install libheif`)
- A portable `heif-dec` dropped into `src-tauri/resources/heif/` (sidecar; see that folder’s README)

## Requirements (development)

### macOS

- Xcode CLT / Xcode
- Rust (`rustup`)
- Node.js 18+

### Windows 10

- [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) (usually already present)
- Rust + Node.js
- Visual Studio Build Tools (MSVC) for Tauri
- [vcpkg](https://vcpkg.io/) with `libheif[hevc]:x64-windows-static-md` for linking the built-in decoder  
  (`VCPKG_ROOT` + `VCPKGRS_TRIPLET=x64-windows-static-md`)

End users who install from GitHub Releases need **only** WebView2 — the decoder is inside the app.

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

Force the macOS `sips` path (ignore PATH `heif-dec`):

```sh
HEIC_CONVERTER_FORCE_SIPS=1 cargo test --lib convert::tests::sample_heic_converts_via_sips_without_heif_dec -- --nocapture
```

## Windows installer (GitHub Releases)

Installers are built by CI (`.github/workflows/release.yml`) on `windows-latest` and attached to a Release — not stored in git.

1. Tag and push: `git tag v0.1.1 && git push origin v0.1.1`
2. Wait for the **Release** workflow on the Actions tab
3. Open https://github.com/youyun/heic-converter/releases
4. Download the Windows `.msi` or NSIS `.exe`
5. Install → open a folder (or drop a HEIC) → Convert — **no** libheif PATH step

Or: **Actions → Release → Run workflow** and pass tag `v0.1.1`.

See also [`pkg/README.md`](pkg/README.md).

## Smoke test (converter only)

With sample HEICs present:

```sh
cd src-tauri
cargo test --lib -- --nocapture
```

## Job store

SQLite lives in the app data directory (Tauri `app_data_dir`), file `jobs.sqlite3`. On startup, any item left in `running` is reset to `pending` so interrupted work can continue.

## License

Private / personal — adjust as needed.

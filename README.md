# HEIC Converter

Bulk **HEIC → JPG** desktop app for **macOS** and **Windows 10+**. Built with [Tauri 2](https://tauri.app/) + Rust — a thin native shell, not an Electron bag of Chromium.

**Privacy:** conversion is **local-only**. Photos never leave your machine; there is no account, no analytics, and no upload.

## Why this instead of a CLI / Electron?

| Approach | Trade-off |
|----------|-----------|
| `magick` / `heif-convert` CLI | Fast if you already have a toolchain; awkward for non-dev users (PATH, Homebrew, vcpkg, Store codecs). |
| Electron apps | Easy UI, heavy runtime, large downloads. |
| **HEIC Converter** | Folder browse + multi-select + job progress; **decoder shipped inside the app** on Windows; **sips** on macOS. No PATH setup for end users. |

## Install (GitHub Releases)

Download the latest build from:

**https://github.com/youyun/heic-converter/releases**

| Platform | Artifact | Notes |
|----------|----------|--------|
| **Windows 10+** | `.msi` or NSIS `.exe` | WebView2 is usually already installed. **No** libheif / Microsoft HEIF Store extension / PATH step. |
| **macOS** | `.dmg` / `.app` (arm64 + Intel) | Uses system `sips`. First open may need *Right-click → Open* if Gatekeeper warns. |

### Upgrading / reinstalling

Re-download the **latest** installer and run it again — it **overwrites / upgrades** the previous install. You do **not** need to uninstall first.

- **Windows `.msi`:** upgrades in place (same app id). Prefer the MSI when you want a clean Windows Installer upgrade path.
- **Windows NSIS `.exe`:** re-run the new installer over the existing install directory; it replaces the previous files.
- **macOS `.dmg`:** open the disk image and drag **HEIC Converter** onto Applications again (replace when asked), or copy over the old `.app`.

There is **no auto-update** in the app yet (may come later). Check Releases when you want a newer build.

## Features

- Choose a folder, or **drag-and-drop** a HEIC (offers to open the parent folder for bulk work)
- Multi-select HEIC/HEIF (virtualized list; pages load on scroll)
- Convert with live **x/y** progress; jobs show a readable name (folder · time)
- Confirm dialog before start: delete-originals + optional custom destination
- Jobs persist in SQLite and resume after crash/quit
- Optional destination folder (default: JPG beside each HEIC)
- Optional delete of the original HEIC **only after** a successful JPG write
- **English** (default) + **简体中文**, with a language switch that persists
- **Light / dark / system** theme preference (persisted)
- Bounded worker pool (up to 4 threads)

## Screenshots

Screenshots (when available) live in the project media folder used during development. Open the app once after install: choose a folder → select → Convert.

## Decoder (no PATH setup)

| Platform | Built-in decoder |
|----------|------------------|
| **Windows** | In-process **libheif** + **libde265** (HEVC), statically linked at build time via vcpkg |
| **macOS** | Apple **`sips`** (ImageIO) — already on every Mac |

Optional extras (dev machines only): `heif-dec` on `PATH`, or a portable binary under `src-tauri/resources/heif/` (see that folder’s README).

## Develop from source

### macOS

- Xcode CLT / Xcode, Rust (`rustup`), Node.js 18+

### Windows 10+

- [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)
- Rust + Node.js + Visual Studio Build Tools (MSVC)
- [vcpkg](https://vcpkg.io/) with `libheif[hevc]:x64-windows-static-md`  
  (`VCPKG_ROOT` + `VCPKGRS_TRIPLET=x64-windows-static-md`)

```sh
npm install
npm run tauri dev
```

Release bundle locally:

```sh
npm run tauri build
```

Outputs go under `src-tauri/target/release/bundle/` (gitignored). Prefer GitHub Releases for installers — do not commit `.msi` / `.exe` / `.dmg` binaries.

### Smoke / automation (optional)

```sh
HEIC_CONVERTER_AUTO_FOLDER=/path/to/heics \
HEIC_CONVERTER_AUTO_CONVERT=1 \
npm run tauri dev
```

Converter unit tests with your own fixtures:

```sh
export HEIC_CONVERTER_SAMPLES=/path/to/heic-files
cd src-tauri && cargo test --lib -- --nocapture
# or: ./scripts/smoke-samples.sh /path/to/heic-files
```

Force the macOS `sips` path:

```sh
HEIC_CONVERTER_FORCE_SIPS=1 cargo test --lib convert::tests::sample_heic_converts_via_sips_without_heif_dec -- --nocapture
```

## Cutting a release

```sh
git tag v0.2.0
git push origin v0.2.0
```

That runs `.github/workflows/release.yml` and attaches installers to the GitHub Release. See [`pkg/README.md`](pkg/README.md).

## Job store

SQLite lives in the app data directory (Tauri `app_data_dir`), file `jobs.sqlite3`. On startup, items left `running` are reset to `pending`.

## Security & privacy

- All conversion runs **on-device**.
- The app requests folder access you choose (and drag-and-drop paths); it does not phone home.
- See [NOTICE](NOTICE) for third-party decoder licenses (LGPL libheif/libde265 on Windows binaries).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Be kind — [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

[MIT](LICENSE) for this project’s source. Windows builds additionally include LGPL **libheif** / **libde265** — details in [NOTICE](NOTICE).

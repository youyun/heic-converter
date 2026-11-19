# Contributing

Thanks for helping. Keep the product **thin**: bulk HEIC→JPG, optional delete-originals, optional destination folder. Avoid unrelated features.

## Setup

1. Install Node 18+, Rust (`rustup`), and platform build tools (see README).
2. `npm install`
3. `npm run tauri dev`

On Windows, set `VCPKG_ROOT` and install `libheif[hevc]:x64-windows-static-md` before a release build.

## Workflow

- Open an issue for non-trivial changes when possible.
- Use a focused branch and a clear PR description (what / why / how tested).
- Match existing style; do not reformat unrelated files.
- Frontend: TypeScript in `src/` (no heavy UI framework by default).
- Backend: Rust under `src-tauri/src/`.
- User-facing strings go through `src/i18n.ts` (English + 简体中文).

## Tests

```sh
export HEIC_CONVERTER_SAMPLES=/path/to/heic-files   # optional; tests skip if unset
cd src-tauri && cargo test --lib -- --nocapture
npm run build
```

## License

By contributing, you agree your changes are licensed under the MIT License (see LICENSE), subject to third-party notices in NOTICE.

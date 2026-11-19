# Optional bundled `heif-dec` (sidecar)

Windows release builds embed **libheif in-process** (see `Cargo.toml` + CI vcpkg), so this folder is usually empty in git.

You may still drop a portable `heif-dec` / `heif-dec.exe` (plus its DLLs on Windows) here; the app prefers a file in this resource directory over PATH.

CI may populate binaries under this directory; those files are gitignored.

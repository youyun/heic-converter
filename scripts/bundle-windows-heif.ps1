# Populate src-tauri/resources/heif with a portable heif-dec + DLLs (optional sidecar).
# Primary Windows decode path is in-process libheif via vcpkg static-md (see release.yml).
# This script is a fallback / local-dev aid when you want a shell-out binary beside the app.
#
# Usage (from repo root, on Windows with vcpkg):
#   powershell -File scripts/bundle-windows-heif.ps1

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Vcpkg = if ($env:VCPKG_ROOT) { $env:VCPKG_ROOT } else { $env:VCPKG_INSTALLATION_ROOT }
if (-not $Vcpkg) {
  throw "Set VCPKG_ROOT or VCPKG_INSTALLATION_ROOT"
}

Write-Host "Installing libheif[hevc]:x64-windows (dynamic tools + DLLs)…"
& "$Vcpkg\vcpkg.exe" install "libheif[hevc]:x64-windows"

$Installed = Join-Path $Vcpkg "installed\x64-windows"
$Dest = Join-Path $Root "src-tauri\resources\heif"
New-Item -ItemType Directory -Force -Path $Dest | Out-Null

$ToolCandidates = @(
  (Join-Path $Installed "tools\libheif\heif-dec.exe"),
  (Join-Path $Installed "tools\heif\heif-dec.exe"),
  (Join-Path $Installed "bin\heif-dec.exe")
)
$Tool = $ToolCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $Tool) {
  throw "heif-dec.exe not found under $Installed"
}
Copy-Item -Force $Tool (Join-Path $Dest "heif-dec.exe")

# DLLs next to heif-dec.exe so Windows resolves them without PATH.
Get-ChildItem (Join-Path $Installed "bin\*.dll") | Copy-Item -Force -Destination $Dest
Write-Host "Bundled heif-dec + DLLs into $Dest"

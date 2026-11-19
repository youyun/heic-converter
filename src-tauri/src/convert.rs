//! HEIC → JPG conversion helpers.
//!
//! Cross-platform strategy (MVP):
//! 1. Prefer `heif-dec` from PATH (libheif; brew on macOS, vcpkg/choco on Windows).
//! 2. On macOS, fall back to Apple `sips`.
//! 3. On Windows, clear error pointing at HEIF Image Extensions / libheif install.
//!
//! Images are converted one file at a time; no whole-folder decode into RAM.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const JPEG_QUALITY: u8 = 90;

#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error("source not found: {0}")]
    MissingSource(String),
    #[error("failed to create destination directory: {0}")]
    DestDir(String),
    #[error("converter failed: {0}")]
    Tool(String),
    #[error("no HEIC converter available on this system")]
    NoConverter,
    #[error("jpg was not written: {0}")]
    MissingOutput(String),
}

pub fn is_heic_path(path: &Path) -> bool {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
    {
        Some(ext) => matches!(ext.as_str(), "heic" | "heif" | "hif"),
        None => false,
    }
}

pub fn default_jpg_path(heic: &Path, dest_dir: Option<&Path>) -> PathBuf {
    let file_stem = heic.file_stem().unwrap_or_default();
    let mut name = PathBuf::from(file_stem);
    name.set_extension("jpg");
    match dest_dir {
        Some(dir) => dir.join(name),
        None => heic
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(name),
    }
}

/// Convert one HEIC/HEIF file to JPG. Writes atomically via a temp sibling when possible.
pub fn convert_heic_to_jpg(src: &Path, dest: &Path) -> Result<(), ConvertError> {
    if !src.is_file() {
        return Err(ConvertError::MissingSource(src.display().to_string()));
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| ConvertError::DestDir(e.to_string()))?;
    }

    // Tools key off the extension — keep a real `.jpg` suffix on the temp file.
    let tmp = {
        let mut t = dest.as_os_str().to_owned();
        t.push(".tmp.jpg");
        PathBuf::from(t)
    };
    let _ = fs::remove_file(&tmp);

    let used = run_converter(src, &tmp)?;
    if !tmp.is_file() {
        return Err(ConvertError::MissingOutput(format!(
            "tool `{used}` reported success but {} is missing",
            tmp.display()
        )));
    }

    // Replace existing JPG if present.
    if dest.exists() {
        fs::remove_file(dest).map_err(|e| ConvertError::Tool(e.to_string()))?;
    }
    fs::rename(&tmp, dest).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        ConvertError::Tool(format!("rename to {}: {e}", dest.display()))
    })?;

    Ok(())
}

fn run_converter(src: &Path, dest: &Path) -> Result<&'static str, ConvertError> {
    if try_heif_dec(src, dest)? {
        return Ok("heif-dec");
    }
    #[cfg(target_os = "macos")]
    {
        if try_sips(src, dest)? {
            return Ok("sips");
        }
    }
    Err(ConvertError::NoConverter)
}

fn try_heif_dec(src: &Path, dest: &Path) -> Result<bool, ConvertError> {
    let output = match Command::new("heif-dec")
        .arg("-q")
        .arg(JPEG_QUALITY.to_string())
        .arg(src)
        .arg(dest)
        .output()
    {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(ConvertError::Tool(format!("heif-dec spawn: {e}"))),
    };
    if output.status.success() {
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(ConvertError::Tool(format!(
            "heif-dec exited {}: {stderr}",
            output.status
        )))
    }
}

#[cfg(target_os = "macos")]
fn try_sips(src: &Path, dest: &Path) -> Result<bool, ConvertError> {
    let output = match Command::new("sips")
        .args(["-s", "format", "jpeg", "-s", "formatOptions"])
        .arg(JPEG_QUALITY.to_string())
        .arg(src)
        .arg("--out")
        .arg(dest)
        .output()
    {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(ConvertError::Tool(format!("sips spawn: {e}"))),
    };
    if output.status.success() {
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(ConvertError::Tool(format!(
            "sips exited {}: {stderr}",
            output.status
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn sample_heic_converts() {
        let src = PathBuf::from("/Users/yuanda/Downloads/heic-samples/2026-11-08 09.15.22.heic");
        if !src.exists() {
            eprintln!("skip: sample HEIC missing");
            return;
        }
        let dir = std::env::temp_dir().join("heic-converter-test");
        let _ = fs::create_dir_all(&dir);
        let dest = dir.join("sample.jpg");
        let _ = fs::remove_file(&dest);
        convert_heic_to_jpg(&src, &dest).expect("convert");
        assert!(dest.is_file());
        assert!(dest.metadata().unwrap().len() > 10_000);
    }
}

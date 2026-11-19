//! HEIC → JPG conversion helpers.
//!
//! Strategy (no PATH setup required for shipped installs):
//! 1. **Windows:** in-process `libheif` (statically linked via vcpkg at build time).
//! 2. Optional bundled `heif-dec` from the app resource dir (sidecar).
//! 3. Optional `heif-dec` on PATH (dev machines with Homebrew / vcpkg tools).
//! 4. **macOS:** Apple `sips` / ImageIO — works out of the box, no Homebrew.
//!
//! Images are converted one file at a time; no whole-folder decode into RAM.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const JPEG_QUALITY: u8 = 90;

static RESOURCE_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

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

/// Record the Tauri resource directory so bundled `heif-dec` can be found.
pub fn init_resource_dir(dir: Option<PathBuf>) {
    let _ = RESOURCE_DIR.set(dir);
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
    // Append ".jpg" as a string — do NOT use Path::set_extension on the stem.
    // Stems like "IMG.2024.01.02" would treat ".02" as an extension and truncate.
    let stem = heic
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".into());
    let name = format!("{stem}.jpg");
    match dest_dir {
        Some(dir) => dir.join(&name),
        None => heic
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&name),
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

fn force_sips() -> bool {
    matches!(
        std::env::var("HEIC_CONVERTER_FORCE_SIPS").as_deref(),
        Ok("1")
    )
}

fn run_converter(src: &Path, dest: &Path) -> Result<&'static str, ConvertError> {
    if !force_sips() {
        #[cfg(windows)]
        {
            if try_libheif(src, dest)? {
                return Ok("libheif");
            }
        }
        if try_heif_dec(src, dest)? {
            return Ok("heif-dec");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if try_sips(src, dest)? {
            return Ok("sips");
        }
    }
    Err(ConvertError::NoConverter)
}

/// Resolve which `heif-dec` binary to run: env override → bundled resource → PATH.
fn resolve_heif_dec() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("HEIC_CONVERTER_HEIF_DEC") {
        let pb = PathBuf::from(p.trim());
        if pb.is_file() {
            return Some(pb);
        }
    }

    let file_name = if cfg!(windows) {
        "heif-dec.exe"
    } else {
        "heif-dec"
    };

    if let Some(Some(res)) = RESOURCE_DIR.get() {
        for candidate in [
            res.join("heif").join(file_name),
            res.join(file_name),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    // Sibling of the running executable (externalBin / manual copy layouts).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for candidate in [
                dir.join("heif").join(file_name),
                dir.join(file_name),
                dir.join("resources").join("heif").join(file_name),
            ] {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    // Bare name → rely on PATH.
    Some(PathBuf::from(file_name))
}

fn try_heif_dec(src: &Path, dest: &Path) -> Result<bool, ConvertError> {
    let Some(bin) = resolve_heif_dec() else {
        return Ok(false);
    };

    let mut cmd = Command::new(&bin);
    // So Windows loads DLLs sitting next to a bundled heif-dec.exe.
    if let Some(dir) = bin.parent() {
        cmd.current_dir(dir);
    }
    let output = match cmd
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
        // Bundled/PATH binary present but failed — surface the error.
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(ConvertError::Tool(format!(
            "heif-dec ({}) exited {}: {stderr}",
            bin.display(),
            output.status
        )))
    }
}

#[cfg(windows)]
fn try_libheif(src: &Path, dest: &Path) -> Result<bool, ConvertError> {
    use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};

    let lib = LibHeif::new();
    let src_str = src
        .to_str()
        .ok_or_else(|| ConvertError::Tool("source path is not valid UTF-8".into()))?;
    let ctx = HeifContext::read_from_file(src_str)
        .map_err(|e| ConvertError::Tool(format!("libheif read: {e}")))?;
    let handle = ctx
        .primary_image_handle()
        .map_err(|e| ConvertError::Tool(format!("libheif primary: {e}")))?;
    let image = lib
        .decode(&handle, ColorSpace::Rgb(RgbChroma::Rgb), None)
        .map_err(|e| ConvertError::Tool(format!("libheif decode: {e}")))?;
    let plane = image
        .planes()
        .interleaved
        .ok_or_else(|| ConvertError::Tool("libheif: missing interleaved RGB plane".into()))?;

    let width = image.width();
    let height = image.height();
    let stride = plane.stride;
    let mut rgb = Vec::with_capacity((width as usize) * (height as usize) * 3);
    for y in 0..(height as usize) {
        let start = y * stride;
        let end = start + (width as usize) * 3;
        if end > plane.data.len() {
            return Err(ConvertError::Tool(format!(
                "libheif: plane underflow at row {y}"
            )));
        }
        rgb.extend_from_slice(&plane.data[start..end]);
    }

    let img = image::RgbImage::from_raw(width, height, rgb)
        .ok_or_else(|| ConvertError::Tool("libheif: invalid RGB buffer".into()))?;

    let mut file = fs::File::create(dest).map_err(|e| ConvertError::Tool(e.to_string()))?;
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, JPEG_QUALITY);
    encoder
        .encode(
            img.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| ConvertError::Tool(format!("jpeg encode: {e}")))?;
    Ok(true)
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

/// Snapshot of which converters the process can use (for the UI status line).
pub fn converter_status() -> serde_json::Value {
    let bundled = resolve_heif_dec()
        .filter(|p| p.is_absolute() || p.parent().is_some_and(|d| d.as_os_str().len() > 0))
        .and_then(|p| {
            // Bare "heif-dec" means PATH lookup — don't call it bundled.
            if p.file_name().and_then(|f| f.to_str()) == Some(if cfg!(windows) {
                "heif-dec.exe"
            } else {
                "heif-dec"
            }) && p.components().count() == 1
            {
                None
            } else if p.is_file() {
                Some(p.display().to_string())
            } else {
                None
            }
        });

    let path_heif_dec = Command::new(if cfg!(windows) {
        "heif-dec.exe"
    } else {
        "heif-dec"
    })
    .arg("-v")
    .output()
    .ok()
    .filter(|o| o.status.success())
    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    .filter(|s| !s.is_empty());

    serde_json::json!({
        "platform": std::env::consts::OS,
        "libheif": cfg!(windows),
        "bundled_heif_dec": bundled,
        "heif_dec": path_heif_dec,
        "sips": cfg!(target_os = "macos"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn multi_dot_stem_keeps_full_name() {
        let heic = PathBuf::from("/tmp/2026-11-08 09.15.22.heic");
        let out = default_jpg_path(&heic, None);
        assert_eq!(
            out.file_name().unwrap().to_string_lossy(),
            "2026-11-08 09.15.22.jpg"
        );
        let out2 = default_jpg_path(&heic, Some(Path::new("/out")));
        assert_eq!(out2, PathBuf::from("/out/2026-11-08 09.15.22.jpg"));
    }

    fn sample_heic(name: &str) -> Option<PathBuf> {
        let root = std::env::var_os("HEIC_CONVERTER_SAMPLES")?;
        let path = PathBuf::from(root).join(name);
        path.exists().then_some(path)
    }

    #[test]
    fn sample_heic_converts() {
        let Some(src) = sample_heic("2026-11-08 09.15.22.heic") else {
            eprintln!("skip: set HEIC_CONVERTER_SAMPLES (sample HEIC missing)");
            return;
        };
        let dir = std::env::temp_dir().join("heic-converter-test");
        let _ = fs::create_dir_all(&dir);
        let dest = default_jpg_path(&src, Some(&dir));
        assert!(dest
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with("09.15.22.jpg"));
        let _ = fs::remove_file(&dest);
        convert_heic_to_jpg(&src, &dest).expect("convert");
        assert!(dest.is_file());
        assert!(dest.metadata().unwrap().len() > 10_000);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sample_heic_converts_via_sips_without_heif_dec() {
        let Some(src) = sample_heic("2026-11-08 09.35.26.heic") else {
            eprintln!("skip: set HEIC_CONVERTER_SAMPLES (sample HEIC missing)");
            return;
        };
        // Prove macOS OOB path: Apple sips, no Homebrew heif-dec required.
        std::env::set_var("HEIC_CONVERTER_FORCE_SIPS", "1");
        let dir = std::env::temp_dir().join("heic-converter-test-sips");
        let _ = fs::create_dir_all(&dir);
        let dest = default_jpg_path(&src, Some(&dir));
        let _ = fs::remove_file(&dest);
        let result = convert_heic_to_jpg(&src, &dest);
        std::env::remove_var("HEIC_CONVERTER_FORCE_SIPS");
        result.expect("sips convert");
        assert!(dest.is_file());
        assert!(dest.metadata().unwrap().len() > 10_000);
    }
}

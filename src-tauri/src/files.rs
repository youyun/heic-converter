//! Folder scanning — paths + light metadata only (no image bytes).

use crate::convert::is_heic_path;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize)]
pub struct HeicEntry {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub modified_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderScan {
    pub root: String,
    pub total: usize,
    pub entries: Vec<HeicEntry>,
}

/// Scan `root` for HEIC/HEIF files. Non-recursive by default for snappy UX;
/// set `recursive` for nested folders. Only metadata is collected.
pub fn scan_heic_files(root: &Path, recursive: bool) -> Result<Vec<HeicEntry>, String> {
    if !root.is_dir() {
        return Err(format!("not a directory: {}", root.display()));
    }
    let mut out = Vec::new();
    if recursive {
        scan_recursive(root, &mut out)?;
    } else {
        scan_one_dir(root, &mut out)?;
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub fn page_entries(all: &[HeicEntry], offset: usize, limit: usize) -> FolderScan {
    let total = all.len();
    let end = (offset + limit).min(total);
    let entries = if offset >= total {
        Vec::new()
    } else {
        all[offset..end].to_vec()
    };
    FolderScan {
        root: String::new(),
        total,
        entries,
    }
}

fn scan_one_dir(dir: &Path, out: &mut Vec<HeicEntry>) -> Result<(), String> {
    let read = fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    for entry in read {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() && is_heic_path(&path) {
            out.push(entry_from_path(&path)?);
        }
    }
    Ok(())
}

fn scan_recursive(dir: &Path, out: &mut Vec<HeicEntry>) -> Result<(), String> {
    // Iterative walk to avoid deep stack growth on huge trees.
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let read = match fs::read_dir(&current) {
            Ok(r) => r,
            Err(_) => continue, // skip unreadable dirs
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() && is_heic_path(&path) {
                if let Ok(meta) = entry_from_path(&path) {
                    out.push(meta);
                }
            }
        }
    }
    Ok(())
}

fn entry_from_path(path: &Path) -> Result<HeicEntry, String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    let modified_ms = meta.modified().ok().and_then(|t| {
        t.duration_since(SystemTime::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_millis() as u64)
    });
    Ok(HeicEntry {
        path: path.to_string_lossy().into_owned(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        size_bytes: meta.len(),
        modified_ms,
    })
}

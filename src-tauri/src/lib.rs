mod convert;
mod files;
mod jobs;
#[cfg(test)]
mod jobs_smoke;

use files::{page_entries, scan_heic_files, FolderScan, HeicEntry};
use jobs::{JobManager, JobSummary};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

struct AppState {
    /// Current folder scan (paths + light metadata only).
    scan: Mutex<Vec<HeicEntry>>,
    scan_root: Mutex<Option<String>>,
    jobs: JobManager,
}

#[tauri::command]
fn pick_folder(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let path = app.dialog().file().blocking_pick_folder();
    Ok(path.map(|p| match p {
        tauri_plugin_dialog::FilePath::Path(pb) => pb.to_string_lossy().into_owned(),
        other => other.to_string(),
    }))
}

#[tauri::command]
fn scan_folder(
    state: State<'_, Arc<AppState>>,
    path: String,
    recursive: bool,
) -> Result<FolderScan, String> {
    let root = PathBuf::from(&path);
    let entries = scan_heic_files(&root, recursive)?;
    let total = entries.len();
    *state.scan.lock() = entries;
    *state.scan_root.lock() = Some(path.clone());
    let page = page_entries(&state.scan.lock(), 0, 200);
    Ok(FolderScan {
        root: path,
        total,
        entries: page.entries,
    })
}

#[tauri::command]
fn list_heic_page(
    state: State<'_, Arc<AppState>>,
    offset: usize,
    limit: usize,
) -> Result<FolderScan, String> {
    let scan = state.scan.lock();
    let root = state.scan_root.lock().clone().unwrap_or_default();
    let mut page = page_entries(&scan, offset, limit.min(500));
    page.root = root;
    Ok(page)
}

#[tauri::command]
fn create_convert_job(
    state: State<'_, Arc<AppState>>,
    paths: Vec<String>,
    dest_dir: Option<String>,
    delete_originals: bool,
) -> Result<JobSummary, String> {
    state.jobs.create_job(paths, dest_dir, delete_originals)
}

#[tauri::command]
fn start_convert_job(app: AppHandle, state: State<'_, Arc<AppState>>, job_id: String) -> Result<(), String> {
    state.jobs.start_job(app, job_id)
}

#[tauri::command]
fn cancel_convert_job(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.jobs.request_cancel();
    Ok(())
}

#[tauri::command]
fn list_jobs(state: State<'_, Arc<AppState>>) -> Result<Vec<JobSummary>, String> {
    state.jobs.list_jobs()
}

#[tauri::command]
fn get_job(state: State<'_, Arc<AppState>>, job_id: String) -> Result<Option<JobSummary>, String> {
    state.jobs.get_job(&job_id)
}

#[tauri::command]
fn job_is_running(state: State<'_, Arc<AppState>>) -> bool {
    state.jobs.is_running()
}

#[tauri::command]
fn converter_info() -> serde_json::Value {
    let heif_dec = std::process::Command::new("heif-dec")
        .arg("-v")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());
    serde_json::json!({
        "heif_dec": heif_dec,
        "sips": cfg!(target_os = "macos"),
        "platform": std::env::consts::OS,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| e.to_string())?;
            let jobs = JobManager::open(&data_dir)?;
            app.manage(Arc::new(AppState {
                scan: Mutex::new(Vec::new()),
                scan_root: Mutex::new(None),
                jobs,
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pick_folder,
            scan_folder,
            list_heic_page,
            create_convert_job,
            start_convert_job,
            cancel_convert_job,
            list_jobs,
            get_job,
            job_is_running,
            converter_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

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

/// Folder picking is done from the frontend via `@tauri-apps/plugin-dialog`.
/// Do **not** call `blocking_pick_*` from a Tauri command — on macOS that
/// deadlocks the main run loop (dialog needs the UI thread that is blocked
/// waiting for the dialog).

#[tauri::command]
async fn scan_folder(
    state: State<'_, Arc<AppState>>,
    path: String,
    recursive: bool,
) -> Result<FolderScan, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let root = PathBuf::from(&path);
        let entries = scan_heic_files(&root, recursive)?;
        let total = entries.len();
        let first_page = page_entries(&entries, 0, 200).entries;
        *state.scan.lock() = entries;
        *state.scan_root.lock() = Some(path.clone());
        Ok(FolderScan {
            root: path,
            total,
            entries: first_page,
        })
    })
    .await
    .map_err(|e| format!("scan task join: {e}"))?
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
fn start_convert_job(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    job_id: String,
) -> Result<(), String> {
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
    convert::converter_status()
}

#[tauri::command]
fn path_is_dir(path: String) -> bool {
    PathBuf::from(path).is_dir()
}

/// Dev/test hook: if `HEIC_CONVERTER_AUTO_FOLDER` is set, the UI opens it
/// without a native dialog (smoke tests / automation).
#[tauri::command]
fn auto_open_folder() -> Option<String> {
    std::env::var("HEIC_CONVERTER_AUTO_FOLDER")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// When `"1"`, UI selects all and starts convert after auto-open.
#[tauri::command]
fn auto_convert_flag() -> String {
    std::env::var("HEIC_CONVERTER_AUTO_CONVERT").unwrap_or_default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            let resource_dir = app.path().resource_dir().ok();
            convert::init_resource_dir(resource_dir);
            let jobs = JobManager::open(&data_dir)?;
            app.manage(Arc::new(AppState {
                scan: Mutex::new(Vec::new()),
                scan_root: Mutex::new(None),
                jobs,
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_folder,
            list_heic_page,
            create_convert_job,
            start_convert_job,
            cancel_convert_job,
            list_jobs,
            get_job,
            job_is_running,
            converter_info,
            path_is_dir,
            auto_open_folder,
            auto_convert_flag,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

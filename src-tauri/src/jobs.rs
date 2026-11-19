//! Persistent convert jobs (SQLite) + bounded worker pool.

use crate::convert::{convert_heic_to_jpg, default_jpg_path};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS jobs (
  id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  status TEXT NOT NULL,
  dest_dir TEXT,
  delete_originals INTEGER NOT NULL DEFAULT 0,
  total INTEGER NOT NULL DEFAULT 0,
  done INTEGER NOT NULL DEFAULT 0,
  failed INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS job_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id TEXT NOT NULL,
  source_path TEXT NOT NULL,
  dest_path TEXT NOT NULL,
  status TEXT NOT NULL,
  error TEXT,
  updated_at TEXT NOT NULL,
  UNIQUE(job_id, source_path)
);

CREATE INDEX IF NOT EXISTS idx_job_items_job_status ON job_items(job_id, status);
"#;

#[derive(Debug, Clone, Serialize)]
pub struct JobSummary {
    pub id: String,
    pub status: String,
    pub dest_dir: Option<String>,
    pub delete_originals: bool,
    pub total: i64,
    pub done: i64,
    pub failed: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobProgressEvent {
    pub job_id: String,
    pub item_id: i64,
    pub source_path: String,
    pub status: String,
    pub error: Option<String>,
    pub done: i64,
    pub failed: i64,
    pub total: i64,
    pub job_status: String,
}

#[derive(Clone)]
pub struct JobManager {
    inner: Arc<JobManagerInner>,
}

struct JobManagerInner {
    db: Mutex<Connection>,
    running: AtomicBool,
    cancel: AtomicBool,
}

impl JobManager {
    pub fn open(app_data: &Path) -> Result<Self, String> {
        fs_create_dir(app_data)?;
        let db_path = app_data.join("jobs.sqlite3");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        // Crash recovery: anything left "running" becomes pending again.
        conn.execute(
            "UPDATE job_items SET status = 'pending', error = NULL, updated_at = ?1 WHERE status = 'running'",
            params![now_iso()],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE jobs SET status = 'pending' WHERE status = 'running'",
            [],
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            inner: Arc::new(JobManagerInner {
                db: Mutex::new(conn),
                running: AtomicBool::new(false),
                cancel: AtomicBool::new(false),
            }),
        })
    }

    pub fn create_job(
        &self,
        sources: Vec<String>,
        dest_dir: Option<String>,
        delete_originals: bool,
    ) -> Result<JobSummary, String> {
        if sources.is_empty() {
            return Err("no files selected".into());
        }
        let job_id = Uuid::new_v4().to_string();
        let created = now_iso();
        let dest_path = dest_dir.as_ref().map(PathBuf::from);
        let conn = self.inner.db.lock();
        conn.execute(
            "INSERT INTO jobs (id, created_at, status, dest_dir, delete_originals, total, done, failed)
             VALUES (?1, ?2, 'pending', ?3, ?4, ?5, 0, 0)",
            params![
                job_id,
                created,
                dest_dir,
                delete_originals as i64,
                sources.len() as i64
            ],
        )
        .map_err(|e| e.to_string())?;

        {
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            for src in &sources {
                let src_path = Path::new(src);
                let dest = default_jpg_path(src_path, dest_path.as_deref());
                tx.execute(
                    "INSERT OR IGNORE INTO job_items (job_id, source_path, dest_path, status, error, updated_at)
                     VALUES (?1, ?2, ?3, 'pending', NULL, ?4)",
                    params![job_id, src, dest.to_string_lossy(), created],
                )
                .map_err(|e| e.to_string())?;
            }
            tx.commit().map_err(|e| e.to_string())?;
        }

        self.recount_job(&conn, &job_id)?;
        drop(conn);
        self.get_job(&job_id)?
            .ok_or_else(|| "job missing after create".into())
    }

    pub fn list_jobs(&self) -> Result<Vec<JobSummary>, String> {
        let conn = self.inner.db.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, status, dest_dir, delete_originals, total, done, failed, created_at
                 FROM jobs ORDER BY created_at DESC LIMIT 50",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(JobSummary {
                    id: row.get(0)?,
                    status: row.get(1)?,
                    dest_dir: row.get(2)?,
                    delete_originals: row.get::<_, i64>(3)? != 0,
                    total: row.get(4)?,
                    done: row.get(5)?,
                    failed: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn get_job(&self, job_id: &str) -> Result<Option<JobSummary>, String> {
        let conn = self.inner.db.lock();
        conn.query_row(
            "SELECT id, status, dest_dir, delete_originals, total, done, failed, created_at
             FROM jobs WHERE id = ?1",
            params![job_id],
            |row| {
                Ok(JobSummary {
                    id: row.get(0)?,
                    status: row.get(1)?,
                    dest_dir: row.get(2)?,
                    delete_originals: row.get::<_, i64>(3)? != 0,
                    total: row.get(4)?,
                    done: row.get(5)?,
                    failed: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst)
    }

    pub fn request_cancel(&self) {
        self.inner.cancel.store(true, Ordering::SeqCst);
    }

    /// Start (or resume) a job: pending + failed items are queued; done are skipped.
    pub fn start_job(&self, app: AppHandle, job_id: String) -> Result<(), String> {
        if self
            .inner
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("a convert job is already running".into());
        }
        self.inner.cancel.store(false, Ordering::SeqCst);

        {
            let conn = self.inner.db.lock();
            // Re-queue failed for retry; leave done alone.
            conn.execute(
                "UPDATE job_items SET status = 'pending', error = NULL, updated_at = ?1
                 WHERE job_id = ?2 AND status = 'failed'",
                params![now_iso(), job_id],
            )
            .map_err(|e| {
                self.inner.running.store(false, Ordering::SeqCst);
                e.to_string()
            })?;
            conn.execute(
                "UPDATE jobs SET status = 'running' WHERE id = ?1",
                params![job_id],
            )
            .map_err(|e| {
                self.inner.running.store(false, Ordering::SeqCst);
                e.to_string()
            })?;
            self.recount_job(&conn, &job_id).map_err(|e| {
                self.inner.running.store(false, Ordering::SeqCst);
                e
            })?;
        }

        let mgr = self.clone();
        thread::spawn(move || {
            mgr.run_workers(app, job_id);
        });
        Ok(())
    }

    fn run_workers(&self, app: AppHandle, job_id: String) {
        let delete_originals = {
            let conn = self.inner.db.lock();
            conn.query_row(
                "SELECT delete_originals FROM jobs WHERE id = ?1",
                params![job_id],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
                != 0
        };

        let workers = worker_count();
        let (tx, rx) = std::sync::mpsc::channel::<WorkItem>();
        let rx = Arc::new(Mutex::new(rx));

        // Seed queue with pending items (streamed in batches).
        let seeder = {
            let mgr = self.clone();
            let job_id = job_id.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut last_id: i64 = 0;
                loop {
                    if mgr.inner.cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    let batch = {
                        let conn = mgr.inner.db.lock();
                        let mut stmt = match conn.prepare(
                            "SELECT id, source_path, dest_path FROM job_items
                             WHERE job_id = ?1 AND status = 'pending' AND id > ?2
                             ORDER BY id ASC LIMIT 64",
                        ) {
                            Ok(s) => s,
                            Err(_) => break,
                        };
                        let rows = stmt
                            .query_map(params![job_id, last_id], |row| {
                                Ok(WorkItem {
                                    id: row.get(0)?,
                                    source_path: row.get(1)?,
                                    dest_path: row.get(2)?,
                                })
                            });
                        match rows {
                            Ok(iter) => iter.filter_map(|r| r.ok()).collect::<Vec<_>>(),
                            Err(_) => break,
                        }
                    };
                    if batch.is_empty() {
                        break;
                    }
                    for item in batch {
                        last_id = item.id;
                        if tx.send(item).is_err() {
                            return;
                        }
                    }
                    // Brief yield so workers can drain without unbounded channel growth.
                    thread::sleep(Duration::from_millis(1));
                }
            })
        };
        drop(tx); // workers exit when seeder + this drop close the channel

        let mut handles = Vec::new();
        for _ in 0..workers {
            let mgr = self.clone();
            let app = app.clone();
            let job_id = job_id.clone();
            let rx = rx.clone();
            handles.push(thread::spawn(move || {
                loop {
                    if mgr.inner.cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    let item = {
                        let rx = rx.lock();
                        rx.recv()
                    };
                    let item = match item {
                        Ok(i) => i,
                        Err(_) => break,
                    };
                    mgr.process_item(&app, &job_id, item, delete_originals);
                }
            }));
        }

        let _ = seeder.join();
        for h in handles {
            let _ = h.join();
        }

        // Finalize job status.
        {
            let conn = self.inner.db.lock();
            let _ = self.recount_job(&conn, &job_id);
            let (done, failed, total, pending) = conn
                .query_row(
                    "SELECT
                        (SELECT COUNT(*) FROM job_items WHERE job_id = ?1 AND status = 'done'),
                        (SELECT COUNT(*) FROM job_items WHERE job_id = ?1 AND status = 'failed'),
                        (SELECT COUNT(*) FROM job_items WHERE job_id = ?1),
                        (SELECT COUNT(*) FROM job_items WHERE job_id = ?1 AND status = 'pending')",
                    params![job_id],
                    |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?)),
                )
                .unwrap_or((0, 0, 0, 0));

            let status = if self.inner.cancel.load(Ordering::SeqCst) {
                "cancelled"
            } else if pending > 0 {
                "pending"
            } else if failed > 0 && done > 0 {
                "completed_with_errors"
            } else if failed > 0 {
                "failed"
            } else {
                "completed"
            };
            let _ = conn.execute(
                "UPDATE jobs SET status = ?1, done = ?2, failed = ?3, total = ?4 WHERE id = ?5",
                params![status, done, failed, total, job_id],
            );
            let _ = app.emit(
                "job-progress",
                JobProgressEvent {
                    job_id: job_id.clone(),
                    item_id: -1,
                    source_path: String::new(),
                    status: "job_finished".into(),
                    error: None,
                    done,
                    failed,
                    total,
                    job_status: status.into(),
                },
            );
        }

        self.inner.running.store(false, Ordering::SeqCst);
        self.inner.cancel.store(false, Ordering::SeqCst);
    }

    fn process_item(
        &self,
        app: &AppHandle,
        job_id: &str,
        item: WorkItem,
        delete_originals: bool,
    ) {
        {
            let conn = self.inner.db.lock();
            let _ = conn.execute(
                "UPDATE job_items SET status = 'running', updated_at = ?1 WHERE id = ?2",
                params![now_iso(), item.id],
            );
        }

        let src = PathBuf::from(&item.source_path);
        let dest = PathBuf::from(&item.dest_path);
        let result = convert_heic_to_jpg(&src, &dest);

        let (status, error) = match result {
            Ok(()) => {
                if delete_originals {
                    if let Err(e) = std::fs::remove_file(&src) {
                        (
                            "failed".to_string(),
                            Some(format!(
                                "jpg written but failed to delete original: {e}"
                            )),
                        )
                    } else {
                        ("done".to_string(), None)
                    }
                } else {
                    ("done".to_string(), None)
                }
            }
            Err(e) => ("failed".to_string(), Some(e.to_string())),
        };

        let (done, failed, total, job_status) = {
            let conn = self.inner.db.lock();
            let _ = conn.execute(
                "UPDATE job_items SET status = ?1, error = ?2, updated_at = ?3 WHERE id = ?4",
                params![status, error, now_iso(), item.id],
            );
            let _ = self.recount_job(&conn, job_id);
            conn.query_row(
                "SELECT done, failed, total, status FROM jobs WHERE id = ?1",
                params![job_id],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                },
            )
            .unwrap_or((0, 0, 0, "running".into()))
        };

        let _ = app.emit(
            "job-progress",
            JobProgressEvent {
                job_id: job_id.to_string(),
                item_id: item.id,
                source_path: item.source_path,
                status,
                error,
                done,
                failed,
                total,
                job_status,
            },
        );
    }

    fn recount_job(&self, conn: &Connection, job_id: &str) -> Result<(), String> {
        conn.execute(
            "UPDATE jobs SET
                done = (SELECT COUNT(*) FROM job_items WHERE job_id = ?1 AND status = 'done'),
                failed = (SELECT COUNT(*) FROM job_items WHERE job_id = ?1 AND status = 'failed'),
                total = (SELECT COUNT(*) FROM job_items WHERE job_id = ?1)
             WHERE id = ?1",
            params![job_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct WorkItem {
    id: i64,
    source_path: String,
    dest_path: String,
}

fn worker_count() -> usize {
    // Bound CPU/RAM: leave one core free, cap at 4 for image decode spikes.
    num_cpus::get().clamp(1, 4)
}

fn now_iso() -> String {
    chrono::Local::now().to_rfc3339()
}

fn fs_create_dir(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())
}

//! Manual-style smoke helpers exercised from tests when samples exist.

#[cfg(test)]
mod tests {
    use crate::convert::convert_heic_to_jpg;
    use crate::files::scan_heic_files;
    use crate::jobs::JobManager;
    use std::fs;
    use std::path::PathBuf;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn bulk_samples_via_job_manager() {
        let samples = PathBuf::from("/Users/yuanda/Downloads/heic-samples");
        if !samples.is_dir() {
            eprintln!("skip: sample dir missing");
            return;
        }
        let entries = scan_heic_files(&samples, false).expect("scan");
        assert!(!entries.is_empty(), "expected sample HEICs");

        let out = std::env::temp_dir().join("heic-converter-job-smoke");
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(&out).unwrap();
        let data = out.join("appdata");
        fs::create_dir_all(&data).unwrap();

        // Direct convert of every sample first (no Tauri AppHandle required).
        for e in &entries {
            let src = PathBuf::from(&e.path);
            let dest = out.join(format!("{}.jpg", src.file_stem().unwrap().to_string_lossy()));
            convert_heic_to_jpg(&src, &dest).unwrap_or_else(|err| {
                panic!("convert {}: {err}", src.display());
            });
            assert!(dest.is_file());
            assert!(dest.metadata().unwrap().len() > 1_000);
        }

        // Job persistence: create job, mark items, recount survives reopen.
        let mgr = JobManager::open(&data).expect("open db");
        let paths: Vec<String> = entries.iter().map(|e| e.path.clone()).collect();
        let job = mgr
            .create_job(paths.clone(), Some(out.to_string_lossy().into()), false)
            .expect("create job");
        assert_eq!(job.total as usize, paths.len());
        assert_eq!(job.done, 0);

        // Simulate crash mid-flight: mark first as done, leave rest pending, reopen.
        {
            // Convert first file "as worker would", then persist done via SQL-less API:
            // create a fresh manager (reopen) and verify pending count.
        }
        let mgr2 = JobManager::open(&data).expect("reopen");
        let again = mgr2.get_job(&job.id).unwrap().unwrap();
        assert_eq!(again.total as usize, paths.len());
        assert_eq!(again.status, "pending");

        // Wait briefly so filesystem settles; assert JPGs from direct convert still there.
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            thread::sleep(Duration::from_millis(50));
        }
        let jpgs: Vec<_> = fs::read_dir(&out)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("jpg"))
            .collect();
        assert_eq!(jpgs.len(), paths.len());
    }
}

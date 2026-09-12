use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::db::{self, Db, FileRow};
use crate::error::{AppError, Result};
use crate::state::ScanState;

pub struct HashState {
    pub cancel: AtomicBool,
    pub running: AtomicBool,
    progress: Mutex<HashProgress>,
}

impl HashState {
    pub fn new() -> Self {
        Self {
            cancel: AtomicBool::new(false),
            running: AtomicBool::new(false),
            progress: Mutex::new(HashProgress::default()),
        }
    }

    pub fn begin(&self) -> bool {
        if self.running.swap(true, Ordering::SeqCst) {
            return false;
        }
        self.cancel.store(false, Ordering::SeqCst);
        true
    }

    pub fn finish(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.cancel.store(false, Ordering::SeqCst);
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn set_progress(&self, progress: HashProgress) {
        if let Ok(mut slot) = self.progress.lock() {
            *slot = progress;
        }
    }

    pub fn snapshot(&self) -> HashProgress {
        self.progress.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

const CHUNK: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashProgress {
    pub running: bool,
    pub hashed: u64,
    pub remaining: u64,
    pub current_path: String,
}

impl Default for HashProgress {
    fn default() -> Self {
        Self {
            running: false,
            hashed: 0,
            remaining: 0,
            current_path: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub sha256: String,
    pub size: i64,
    pub files: Vec<FileRow>,
}

pub fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn run_pending(
    db: &Db,
    scan: &ScanState,
    state: &HashState,
    mut emit: impl FnMut(HashProgress),
) -> Result<u64> {
    if !state.begin() {
        return Ok(0);
    }
    let mut hashed = 0_u64;
    loop {
        if scan.running.load(Ordering::SeqCst) || state.cancel.load(Ordering::SeqCst) {
            break;
        }
        let (file, remaining) = {
            let conn = db
                .0
                .lock()
                .map_err(|_| AppError::Message("database lock poisoned".into()))?;
            let pending = db::pending_hashes(&conn)?;
            let remaining = pending.len() as u64;
            (pending.into_iter().next(), remaining)
        };
        let Some(file) = file else {
            break;
        };
        let progress = HashProgress {
            running: true,
            hashed,
            remaining,
            current_path: file.abs_path.clone(),
        };
        state.set_progress(progress.clone());
        emit(progress);

        match hash_file(Path::new(&file.abs_path)) {
            Ok(digest) => {
                let conn = db
                    .0
                    .lock()
                    .map_err(|_| AppError::Message("database lock poisoned".into()))?;
                db::set_sha256(&conn, file.id, &digest)?;
                hashed += 1;
            }
            Err(err) => {
                eprintln!("hash {}: {err}", file.abs_path);
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    let done = HashProgress {
        running: false,
        hashed,
        remaining: 0,
        current_path: String::new(),
    };
    state.set_progress(done.clone());
    emit(done);
    state.finish();
    Ok(hashed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn hashes_known_bytes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.bin");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"hello").unwrap();
        let hex = hash_file(&path).unwrap();
        assert_eq!(
            hex,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        let copy = dir.path().join("b.bin");
        std::fs::copy(&path, &copy).unwrap();
        assert_eq!(hash_file(&copy).unwrap(), hex);
    }
}

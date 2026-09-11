use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::scan::ScanProgress;

pub struct ScanState {
    pub pause: AtomicBool,
    pub cancel: AtomicBool,
    pub running: AtomicBool,
    progress: Mutex<ScanProgress>,
}

impl ScanState {
    pub fn new() -> Self {
        Self {
            pause: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            running: AtomicBool::new(false),
            progress: Mutex::new(ScanProgress::default()),
        }
    }

    pub fn begin(&self) -> bool {
        if self.running.swap(true, Ordering::SeqCst) {
            return false;
        }
        self.pause.store(false, Ordering::SeqCst);
        self.cancel.store(false, Ordering::SeqCst);
        true
    }

    pub fn finish(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.pause.store(false, Ordering::SeqCst);
        self.cancel.store(false, Ordering::SeqCst);
    }

    pub fn set_progress(&self, progress: ScanProgress) {
        if let Ok(mut slot) = self.progress.lock() {
            *slot = progress;
        }
    }

    pub fn snapshot(&self) -> ScanProgress {
        self.progress.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

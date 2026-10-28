//! Cross-process writer exclusion compatible with the Python store lock.
//!
//! Unix uses std's flock-based exclusive lock; Windows uses its LockFileEx
//! implementation covering Python's first-byte lock. Windows waits indefinitely,
//! as documented. The lock file must never be truncated or replaced.

use std::fs::{File, OpenOptions};
use std::path::Path;

use crate::fail::{IoAt, R};

/// An exclusive lock owned until the guard is dropped, including error returns.
pub(super) struct Lock(File);

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

/// Acquire the shared lock without truncating its file; keep the guard through sync.
pub(super) fn locked(directory: &Path) -> R<Lock> {
    let path = directory.join(".lock");
    // Never truncate: that breaks locks other processes hold on Windows.
    let mut options = OpenOptions::new();

    if cfg!(windows) {
        options.read(true).write(true);
    } else {
        options.append(true);
    }

    let file = options.create(true).open(&path).at(&path)?;
    file.lock().bare()?;
    Ok(Lock(file))
}

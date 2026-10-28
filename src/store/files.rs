//! Physical filesystem operations, repair, and durable append primitives.
//!
//! Repair and append require the caller's store lock. Error attribution retains
//! the reference distinction between path-based and already-open-file failures.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use crate::fail::{IoAt, R};

/// `os.makedirs(p, exist_ok=True)`, reporting the component that failed.
pub fn makedirs(p: &Path) -> R<()> {
    if let (Some(head), Some(_)) = (p.parent(), p.file_name()) {
        if !head.as_os_str().is_empty() && !head.exists() {
            makedirs(head)?;
        }
    }

    match fs::create_dir(p) {
        Err(e) if !p.is_dir() => Err(e).at(p),
        _ => Ok(()),
    }
}

/// Create the file if it is missing, as `open(p, "a").close()` does.
pub fn touch(p: &Path) -> R<()> {
    OpenOptions::new()
        .append(true)
        .create(true)
        .open(p)
        .at(p)
        .map(drop)
}

/// How many whole records `path` holds. A missing file holds none; any
/// other failure is real and surfaces.
pub fn count(path: &Path, rec: u64) -> R<u64> {
    match fs::metadata(path) {
        Ok(m) => Ok(m.len() / rec),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(e).at(path),
    }
}

/// Drop a partial trailing record left by a crash. It was never
/// acknowledged. Callers hold the lock.
pub(super) fn repair(path: &Path, rec: u64) -> R<()> {
    let n = match fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).at(path),
    };

    if n % rec != 0 {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .at(path)?;
        f.set_len(n - n % rec).bare()?;
    }

    Ok(())
}

/// Append a prepared batch and sync it before the caller releases its writer lock.
pub(super) fn append(path: &Path, bytes: &[u8]) -> R<()> {
    let mut f = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .at(path)?;
    f.write_all(bytes).bare()?;
    f.sync_all().bare()
}

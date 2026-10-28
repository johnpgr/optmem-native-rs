//! How a command stops. Python's `die` prints to stderr and exits 1; an
//! `OSError` anywhere is reported as `<path>: <strerror>.`. Both are values
//! here, so locks and files close on the way out.

use std::io;
use std::path::Path;

/// A command failure carried to the CLI after local files and locks are dropped.
pub enum Fail {
    /// Print to stderr, exit 1.
    Die(String),
    /// The message is already on stdout; exit with this status.
    Exit(i32),
    /// A filesystem failure, with the path Python would put in `e.filename`.
    Io(Option<String>, io::Error),
}

/// Shared command/storage result; no helper exits the process while holding a lock.
pub type R<T> = Result<T, Fail>;

/// Construct a validation error without printing it before buffered stdout.
pub fn die<T>(msg: impl Into<String>) -> R<T> {
    Err(Fail::Die(msg.into()))
}

/// Preserve whether Python would attribute an I/O error to a filename or the store.
pub trait IoAt<T> {
    /// An error from an operation that names a file (open, stat, mkdir).
    fn at(self, p: &Path) -> R<T>;

    /// An error from an operation on an open file (read, write, fsync).
    fn bare(self) -> R<T>;
}

impl<T> IoAt<T> for io::Result<T> {
    fn at(self, p: &Path) -> R<T> {
        self.map_err(|e| Fail::Io(Some(p.to_string_lossy().into_owned()), e))
    }

    fn bare(self) -> R<T> {
        self.map_err(|e| Fail::Io(None, e))
    }
}

/// Python's `e.strerror`: the OS message without Rust's " (os error N)".
pub fn strerror(e: &io::Error) -> String {
    let s = e.to_string();
    let s = match (e.raw_os_error(), s.rfind(" (os error ")) {
        (Some(_), Some(i)) => s[..i].to_string(),
        _ => s,
    };
    // Windows messages end in a period; the caller adds its own.

    if cfg!(windows) {
        s.trim_end_matches('.').to_string()
    } else {
        s
    }
}

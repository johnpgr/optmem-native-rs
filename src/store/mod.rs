//! Fixed-record storage shared with the Python CLI.
//!
//! Record position determines identity: entry i begins at i * LOG_REC. Each
//! summary level is a dense prefix of TREE_REC-byte records. Readers ignore
//! incomplete tails; writers lock, repair, append, and sync before success.
//!
//! The facade exposes storage operations while records, locking, physical I/O,
//! log scanning, tree levels, and pending-work accounting stay separate.

mod files;
mod lock;
mod log;
mod pending;
mod records;
mod tree;

pub use files::{count, makedirs, touch};
pub use log::{log_append, log_get, log_len, log_scan, log_slice};
pub use pending::{pending, pending_count};
pub use records::DatedMemory;
pub use tree::{tree_drop, tree_get, tree_put, Summary};

/// Physical log width, including UTF-8 text, padding, and a final LF byte.
pub const LOG_REC: u64 = 320;

/// Physical summary width; unchanged on Windows, unlike console/config text.
pub const TREE_REC: u64 = 288;

/// One file for all raw memories, addressed by record index.
pub fn log_path(directory: &std::path::Path) -> std::path::PathBuf {
    directory.join("LOG.txt")
}

/// A dense prefix of summaries of a single power-of-two block size.
pub fn tree_path(directory: &std::path::Path, size: u64) -> std::path::PathBuf {
    directory.join("TREE").join(size.to_string())
}

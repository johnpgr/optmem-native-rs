//! Shared UTF-8 file reads; newline and validation policy belongs to the caller.

use std::path::Path;

use crate::fail::{IoAt, R};

/// Distinguish invalid UTF-8 from filesystem failure so each command can explain it.
pub fn read_utf8(path: &Path) -> R<Option<String>> {
    let bytes = std::fs::read(path).at(path)?;

    Ok(String::from_utf8(bytes).ok())
}

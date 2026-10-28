//! Summary reads, ordered appends, and invalidation of dependent tree levels.
//!
//! A summary is available only when its full record exists. Writers recheck
//! level length under the store lock; forgetting truncates summaries but never
//! touches LOG.txt. Blank and corrupt records remain distinct outcomes.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use super::files::{append, repair};
use super::lock::locked;
use super::records::pad;
use super::{count, log_len, tree_path, TREE_REC};
use crate::fail::{IoAt, R};
use crate::pystr;

/// Reading a full record distinguishes repairable absence from invalid UTF-8.
pub enum Summary {
    /// Missing, blank, or incomplete; never expose a torn write as a summary.
    Missing,
    /// A complete record exists but cannot be decoded as UTF-8.
    Corrupt,
    /// Complete, unpadded summary text.
    Text(String),
}

/// The summary of block [lo,hi), in one seek. A record cut short by a crash
/// is not a summary yet: the next nap repairs and rebuilds it.
pub fn tree_get(d: &Path, lo: u64, hi: u64) -> R<Summary> {
    let size = hi - lo;
    let p = tree_path(d, size);
    let mut f = match File::open(&p) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Summary::Missing),
        Err(e) => return Err(e).at(&p),
    };

    f.seek(SeekFrom::Start((lo / size) * TREE_REC)).bare()?;
    let mut rec = Vec::with_capacity(TREE_REC as usize);
    f.take(TREE_REC).read_to_end(&mut rec).bare()?;

    if rec.len() < TREE_REC as usize {
        return Ok(Summary::Missing);
    }

    Ok(match std::str::from_utf8(&rec) {
        Err(_) => Summary::Corrupt,
        Ok(s) => match pystr::rstrip(s) {
            "" => Summary::Missing,
            s => Summary::Text(s.into()),
        },
    })
}

/// Write block [lo,hi). Blocks are built in order, so this only ever
/// appends one record to one level file. False if it was not the next one.
pub fn tree_put(d: &Path, lo: u64, hi: u64, text: &str) -> R<bool> {
    let size = hi - lo;
    let _lock = locked(d)?;
    let p = tree_path(d, size);
    repair(&p, TREE_REC)?;

    if count(&p, TREE_REC)? != lo / size {
        return Ok(false);
    }
    append(&p, &pad(text, TREE_REC)?)?;
    Ok(true)
}

/// Forget block [lo,hi) and every block built from it, by truncating each
/// level back to that point. The log is never touched.
pub fn tree_drop(d: &Path, lo: u64, hi: u64) -> R<Vec<(u64, u64)>> {
    let (mut gone, mut size) = (Vec::new(), hi - lo);
    let _lock = locked(d)?;

    while size <= log_len(d)? {
        let (p, k) = (tree_path(d, size), lo / size);
        let n = count(&p, TREE_REC)?;

        if n > k {
            gone.extend((k..n).map(|i| (i * size, (i + 1) * size)));
            let f = OpenOptions::new().read(true).write(true).open(&p).at(&p)?;
            f.set_len(k * TREE_REC).bare()?;
        }

        let Some(next) = size.checked_mul(2) else {
            break;
        };
        size = next;
    }

    Ok(gone)
}

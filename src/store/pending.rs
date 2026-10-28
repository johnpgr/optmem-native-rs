//! Pending compression derived from each summary level's dense-prefix length.
//!
//! Counts are clamped for older snapshots: another writer may already have
//! compressed beyond that invocation's log length. The count path never needs
//! to allocate a list of every missing summary.

use std::path::Path;

use super::{count, tree_path, TREE_REC};
use crate::fail::R;

/// Blocks that can be built and have not been, smallest first.
pub fn pending(d: &Path, t: u64, limit: Option<usize>) -> R<Vec<(u64, u64)>> {
    let (mut todo, mut size) = (Vec::new(), 2u64);

    while size <= t {
        let have = count(&tree_path(d, size), TREE_REC)?;

        for k in have..t / size {
            todo.push((k * size, (k + 1) * size));

            if limit.is_some_and(|l| todo.len() >= l) {
                return Ok(todo);
            }
        }

        let Some(next) = size.checked_mul(2) else {
            break;
        };
        size = next;
    }

    Ok(todo)
}

/// How many blocks `pending` would list, without listing them. A level can
/// hold MORE blocks than T needs, so each level is clamped at zero.
pub fn pending_count(d: &Path, t: u64) -> R<u64> {
    let (mut n, mut size) = (0u64, 2u64);

    while size <= t {
        n += (t / size).saturating_sub(count(&tree_path(d, size), TREE_REC)?);
        let Some(next) = size.checked_mul(2) else {
            break;
        };
        size = next;
    }

    Ok(n)
}

//! Compression planning and prompts built from raw entries or settled child blocks.
//!
//! Small blocks include raw memories through sixteen entries. Larger blocks
//! require both child summaries. No writes occur while constructing a prompt;
//! command handlers and the locked store decide whether a submission still fits.

use crate::app::App;
use crate::fail::{die, R};
use crate::store::{self, Summary};

/// The reference reads raw entries up to this block width, then merges summaries.
const RAW_MAX: u64 = 16;

/// Read a summary, turning corrupt UTF-8 into an actionable forget instruction.
pub fn summary(app: &App, lo: u64, hi: u64) -> R<Option<String>> {
    match store::tree_get(app.path(), lo, hi)? {
        Summary::Missing => Ok(None),
        Summary::Text(text) => Ok(Some(text)),
        Summary::Corrupt => die(format!(
            "The summary of #{}-{} is corrupt. Run: {} forget {}-{}",
            lo,
            hi - 1,
            app.executable,
            lo,
            hi - 1
        )),
    }
}

/// A blank settled record cannot be repaired by another nap until it is forgotten.
pub fn blank<T>(app: &App, lo: u64, hi: u64) -> R<T> {
    die(format!(
        "The summary of #{}-{} is blank. Run: {} forget {}-{}",
        lo,
        hi - 1,
        app.executable,
        lo,
        hi - 1
    ))
}

/// Offer the smallest missing block and count all work without listing it all.
pub fn next_nap(app: &App, snapshot: u64) -> R<Option<String>> {
    let pending = store::pending(app.path(), snapshot, Some(1))?;
    let Some(&(lo, hi)) = pending.first() else {
        return Ok(None);
    };

    let remaining = store::pending_count(app.path(), snapshot)?.saturating_sub(1);

    nap_prompt(app, lo, hi, remaining).map(Some)
}

/// Preserve the prompt's blank lines and inclusive block IDs exactly.
fn nap_prompt(app: &App, lo: u64, hi: u64, remaining: u64) -> R<String> {
    let body = if hi - lo <= RAW_MAX {
        raw_body(app, lo, hi)?
    } else {
        summary_body(app, lo, hi)?
    };

    let tail = match remaining {
        0 => String::new(),
        1 => "\n1 compression remains after this one.".to_owned(),
        count => format!("\n{} compressions remain after this one.", count),
    };

    Ok(format!(
        "Compress memories #{}-{} into one line of at most {} bytes.\n\
         Keep what has lasting effect, drop what does not. Invent nothing.\n\n\
         {}\n{}\nRun: {} nap {}-{} \"<your line>\"",
        lo,
        hi - 1,
        app.config.entry_bytes,
        body,
        tail,
        app.executable,
        lo,
        hi - 1
    ))
}

/// Include raw memories for small blocks, regardless of available child summaries.
fn raw_body(app: &App, lo: u64, hi: u64) -> R<String> {
    let entries = store::log_slice(app.path(), lo, hi)?;
    let lines: Vec<String> = entries
        .iter()
        .map(|entry| format!("  {}", entry.line()))
        .collect();

    Ok(lines.join("\n"))
}

/// Larger blocks require both children; a missing child indicates a blank record.
fn summary_body(app: &App, lo: u64, hi: u64) -> R<String> {
    let midpoint = lo + (hi - lo) / 2;
    let mut lines = Vec::new();

    for (child_lo, child_hi) in [(lo, midpoint), (midpoint, hi)] {
        let Some(text) = summary(app, child_lo, child_hi)? else {
            return blank(app, child_lo, child_hi);
        };

        lines.push(format!("  #{}-{} {}", child_lo, child_hi - 1, text));
    }

    Ok(lines.join("\n"))
}

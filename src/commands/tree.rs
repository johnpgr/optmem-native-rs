//! Tree navigation and summary invalidation; neither operation edits raw memories.

use crate::app::App;
use crate::compression::summary;
use crate::fail::{die, R};
use crate::pystr::plural;
use crate::store;
use crate::validation::block_id;

/// Drop a summary and everything built on top of it; the next nap computes
/// them again. The log is untouched, so nothing is ever actually lost.
pub(super) fn forget(app: &mut App, args: &[String]) -> R<()> {
    if args.len() != 1 {
        return die(format!("usage: {} forget <lo>-<hi>", app.executable));
    }

    let (lo, hi) = block_id(&args[0])?;
    let gone = store::tree_drop(app.path(), lo, hi)?;
    let Some(first) = gone.first() else {
        return die(format!("No summary at {}.", args[0]));
    };

    app.output.line(format_args!(
        "Forgot {}, from {}-{} up. Run: {} nap",
        plural(gone.len() as u64, "summary"),
        first.0,
        first.1 - 1,
        app.executable
    ));
    Ok(())
}

/// One node of the tree, opened: its two halves, each rendered as wake
/// renders it.
pub(super) fn zoom(app: &mut App, args: &[String]) -> R<()> {
    if args.len() != 1 {
        return die(format!(
            "usage: {} zoom <lo>-<hi>   # a block id, as wake prints them",
            app.executable
        ));
    }

    let (lo, hi) = block_id(&args[0])?;
    let t = store::log_len(app.path())?;

    if lo >= t {
        return die(format!(
            "#{} is beyond the memory: it holds {}. Run: {} wake",
            args[0],
            plural(t, "memory"),
            app.executable
        ));
    }

    let mid = lo + (hi - lo) / 2;

    for (a, b) in [(lo, mid), (mid, hi)] {
        if a >= t {
            continue; // the future: no memories there yet
        }

        if b - a == 1 {
            let line = store::log_get(app.path(), a)?.line();
            app.output.line(format_args!("{}", line));
        } else {
            let s = summary(app, a, b)?.unwrap_or_else(|| "not compressed yet".into());
            app.output.line(format_args!("#{}-{} {}", a, b - 1, s));
        }
    }

    Ok(())
}

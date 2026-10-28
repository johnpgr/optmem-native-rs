//! Submit the next compression, tolerating another session settling it first.
//!
//! Planning reads are advisory; tree_put rechecks the dense-prefix position
//! under the writer lock before acknowledging a summary.

use crate::app::App;
use crate::compression::{next_nap, summary};
use crate::cover::Block;
use crate::fail::{die, R};
use crate::validation::block_id;
use crate::{store, validation};

/// Inspect pending work or submit exactly one offered block and print what follows.
pub(super) fn execute(app: &mut App, args: &[String]) -> R<()> {
    let snapshot = store::log_len(app.path())?;

    if args.is_empty() {
        return render_next(app, snapshot, false);
    }

    let [id, text] = args else {
        return die(format!(
            "usage: {} nap <lo>-<hi> \"<one line>\"",
            app.executable
        ));
    };

    let block = block_id(id)?;
    let pending = store::pending(app.path(), snapshot, Some(1))?;
    let Some(&next) = pending.first() else {
        app.output.line("Nothing left to compress.");
        return Ok(());
    };

    submit(app, block, next, id, text)?;

    render_next(app, snapshot, true)
}

/// Ignore stale settled submissions; validate text only when this block is next.
fn submit(app: &mut App, block: Block, next: Block, id: &str, text: &str) -> R<()> {
    let (lo, hi) = block;

    if block != next {
        if summary(app, lo, hi)?.is_some() {
            app.output
                .line(format_args!("{}-{} is already settled.", lo, hi - 1));
            return Ok(());
        }

        return die(format!(
            "Wrong block: {}. Blocks are built in order; the next is {}-{}. Run: {} nap",
            id,
            next.0,
            next.1 - 1,
            app.executable
        ));
    }

    let text = validation::memory_line(text, app.config.entry_bytes)?;
    let saved = store::tree_put(app.path(), lo, hi, &text)?;
    let status = if saved {
        "saved"
    } else {
        "was settled or forgotten meanwhile"
    };

    app.output
        .line(format_args!("{}-{} {}.", lo, hi - 1, status));

    Ok(())
}

/// Separate a submission acknowledgement from its next prompt by one blank line.
fn render_next(app: &mut App, snapshot: u64, after_submission: bool) -> R<()> {
    let Some(prompt) = next_nap(app, snapshot)? else {
        app.output.line("Nothing left to compress.");
        return Ok(());
    };

    if after_submission {
        app.output.blank_line();
    }

    app.output.line(prompt);

    Ok(())
}

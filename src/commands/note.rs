//! Validate and durably append one memory before acknowledging its assigned ID.

use crate::app::App;
use crate::compression::next_nap;
use crate::fail::{die, R};
use crate::store::DatedMemory;
use crate::{store, today, validation};

/// The store assigns the ID inside the lock; the command only supplies date/text.
pub(super) fn execute(app: &mut App, args: &[String]) -> R<()> {
    let [text] = args else {
        return die(format!(
            "usage: {} note \"<one line, at most {} bytes>\"",
            app.executable, app.config.entry_bytes
        ));
    };

    let text = validation::memory_line(text, app.config.entry_bytes)?;
    let memory = DatedMemory {
        date: today::today(),
        text,
    };

    let id = store::log_append(app.path(), &[memory])?;
    app.output.line(format_args!("Saved as #{}.", id));

    if let Some(prompt) = next_nap(app, id + 1)? {
        app.output.line(format_args!("\n{}", prompt));
    }

    Ok(())
}

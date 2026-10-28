//! Validate an entire dated history before appending it as one locked batch.
//!
//! No invalid later row may leave an acknowledged partial import. Input staging
//! and padded-batch allocation retain the existing import performance tradeoff.

use std::path::Path;

use crate::app::App;
use crate::fail::{die, R};
use crate::pystr::plural;
use crate::store::DatedMemory;
use crate::text::read_utf8;
use crate::{paths, pystr, store, today};

/// Read and validate all historical rows before taking the append lock.
pub(super) fn execute(app: &mut App, args: &[String]) -> R<()> {
    let [source] = args else {
        return die(format!(
            "usage: {} import <file>   # lines of 'YYYY-MM-DD <text>'",
            app.executable
        ));
    };

    let Some(contents) = read_utf8(Path::new(source))? else {
        return die(format!(
            "{} is not UTF-8 text. Convert it, then import again.",
            paths::pretty(source)
        ));
    };

    let memories = validate_history(app, &contents)?;

    if memories.is_empty() {
        return die(format!("{} has no memories.", source));
    }

    let first_id = store::log_append(app.path(), &memories)?;
    let count = memories.len() as u64;
    app.output.line(format_args!(
        "Imported {}, #{} to #{}.",
        plural(count, "memory"),
        first_id,
        first_id + count - 1
    ));

    let pending = store::pending_count(app.path(), store::log_len(app.path())?)?;

    if pending > 0 {
        app.output.line(format_args!(
            "{} pending. Run: {} nap",
            plural(pending, "compression"),
            app.executable
        ));
    }

    Ok(())
}

/// Retain row numbers across blank lines and compare each date to its predecessor.
fn validate_history(app: &App, contents: &str) -> R<Vec<DatedMemory>> {
    let count = store::log_len(app.path())?;
    let mut previous_date = if count > 0 {
        store::log_get(app.path(), count - 1)?.date
    } else {
        "0000-00-00".to_owned()
    };

    let mut memories = Vec::new();

    for (index, line) in pystr::lines(contents).iter().enumerate() {
        if pystr::strip(line).is_empty() {
            continue;
        }

        let memory = parse_row(line, index + 1, &previous_date, app.config.entry_bytes)?;
        previous_date.clone_from(&memory.date);
        memories.push(memory);
    }

    Ok(memories)
}

/// Validate shape, calendar date, order, and byte limit in reference error order.
fn parse_row(line: &str, number: usize, previous_date: &str, byte_limit: u64) -> R<DatedMemory> {
    let (date, text) = line.split_once(' ').unwrap_or((line, ""));

    if !today::is_shaped(date) {
        return die(format!(
            "line {}: expected 'YYYY-MM-DD <text>', got: {}",
            number, line
        ));
    }

    if !today::is_real(date) {
        return die(format!("line {}: {} is not a real date.", number, date));
    }

    if date < previous_date {
        return die(format!(
            "line {}: date {} precedes the previous memory ({}).",
            number, date, previous_date
        ));
    }

    let text = pystr::strip(text);

    if text.is_empty() || text.len() as u64 > byte_limit {
        return die(format!(
            "line {}: {} bytes, limit {}.",
            number,
            text.len(),
            byte_limit
        ));
    }

    Ok(DatedMemory {
        date: date.to_owned(),
        text: text.to_owned(),
    })
}

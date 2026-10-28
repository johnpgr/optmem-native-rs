//! Snapshot-based memory rendering and compression refusals.
//!
//! A pagination request retains its original log length so notes written between
//! pages cannot shift boundaries. Refusals keep their stdout compression prompt.

use crate::app::App;
use crate::compression::{blank, next_nap, summary};
use crate::fail::{die, Fail, R};
use crate::pagination::paginate;
use crate::pystr::plural;
use crate::{cover, pystr, store};

/// The requested page and immutable history boundary are separate concepts.
struct WakeRequest {
    part: u64,
    snapshot: u64,
}

impl WakeRequest {
    /// Parse optional pagination without allowing a snapshot beyond the real log.
    fn parse(app: &App, args: &[String], available: u64) -> R<Self> {
        let usage = || Fail::Die(format!("usage: {} wake [part [T]]", app.executable));

        let (part, snapshot) = match args {
            [] => (1, available),
            [part] => (pystr::uint(part).ok_or_else(usage)?, available),
            [part, snapshot] => (
                pystr::uint(part).ok_or_else(usage)?,
                pystr::uint(snapshot).ok_or_else(usage)?,
            ),
            _ => return Err(usage()),
        };

        if snapshot > available {
            return die(format!(
                "T={}, but the log holds {}. Run: {} wake",
                snapshot,
                plural(available, "memory"),
                app.executable
            ));
        }

        Ok(Self { part, snapshot })
    }
}

/// Render one snapshot page, refusing only when its cover needs an unbuilt block.
pub(super) fn execute(app: &mut App, args: &[String]) -> R<()> {
    let available = store::log_len(app.path())?;
    let request = WakeRequest::parse(app, args, available)?;

    if request.snapshot == 0 {
        app.output.line(format_args!(
            "No memories yet. Record the first with: {} note \"<one line>\"",
            app.executable
        ));
        app.output.line("You are awake.");
        return Ok(());
    }

    let lines = render_cover(app, request.snapshot)?;

    render_page(app, request, lines)
}

/// Build all context lines before publishing any page or continuation boundary.
fn render_cover(app: &mut App, snapshot: u64) -> R<Vec<String>> {
    let mut lines = Vec::new();

    for (lo, hi) in cover::cover(snapshot, app.config.wake_lines) {
        if hi - lo == 1 {
            lines.push(store::log_get(app.path(), lo)?.line());
            continue;
        }

        let text = required_summary(app, lo, hi, snapshot)?;
        lines.push(format!("#{}-{} {}", lo, hi - 1, text));
    }

    Ok(lines)
}

/// Distinguish pending work from a blank summary, then retry a concurrent settle.
fn required_summary(app: &mut App, lo: u64, hi: u64, snapshot: u64) -> R<String> {
    if let Some(text) = summary(app, lo, hi)? {
        return Ok(text);
    }

    if let Some(prompt) = next_nap(app, snapshot)? {
        let remaining = store::pending_count(app.path(), snapshot)?;
        app.output.line(format_args!(
            "Cannot wake: the memory context needs #{}-{}, which is not compressed yet.\n\
             Do the {} below, then run {} wake again.\n",
            lo,
            hi - 1,
            plural(remaining, "compression"),
            app.executable
        ));
        app.output.line(prompt);

        return Err(Fail::Exit(1));
    }

    // Another process may have settled the block after the first read.

    match summary(app, lo, hi)? {
        Some(text) => Ok(text),
        None => blank(app, lo, hi),
    }
}

/// Publish the requested page, with either a snapshot continuation or final wake.
fn render_page(app: &mut App, request: WakeRequest, lines: Vec<String>) -> R<()> {
    let mut pages = paginate(lines, app.config.part_lines, app.config.part_bytes);
    let total = pages.len() as u64;

    if !(1..=total).contains(&request.part) {
        return die(format!(
            "No part {}: the memory has {}. Run: {} wake",
            request.part,
            plural(total, "part"),
            app.executable
        ));
    }

    if total > 1 {
        app.output.line(format_args!(
            "Your memory, part {} of {}, oldest first ({}).",
            request.part,
            total,
            plural(request.snapshot, "memory")
        ));
    }

    let page = std::mem::take(&mut pages[(request.part - 1) as usize]);
    app.output.line(page.join("\n"));

    if request.part < total {
        app.output.line(format_args!(
            "Not awake yet. Run: {} wake {} {}",
            app.executable,
            request.part + 1,
            request.snapshot
        ));
        return Ok(());
    }

    app.output.line("You are awake.");

    if let Some(prompt) = next_nap(app, request.snapshot)? {
        app.output.line(format_args!("\n{}", prompt));
    }

    Ok(())
}

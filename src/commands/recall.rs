//! Per-record native regex search with bounded retention of the newest matches.
//!
//! Every match is counted, even after the output budget is exhausted. The
//! scanner owns the reusable input buffer; this module copies only matched
//! lines, never borrowing text beyond the callback that exposes it.

use std::collections::VecDeque;

use crate::app::App;
use crate::fail::{die, R};
use crate::output::Output;
use crate::pystr::plural;
use crate::store;

/// Total includes evicted lines; retained_bytes counts UTF-8 text plus one LF each.
struct RecallMatches {
    total: u64,
    lines: VecDeque<String>,
    retained_bytes: u64,
    byte_limit: u64,
}

impl RecallMatches {
    /// A configured byte budget includes each eventual output newline.
    fn new(byte_limit: u64) -> Self {
        Self {
            total: 0,
            lines: VecDeque::new(),
            retained_bytes: 0,
            byte_limit,
        }
    }

    /// Retain a newest suffix; even an oversized match evicts older results.
    fn include(&mut self, line: &str) {
        self.total += 1;
        self.retained_bytes += line.len() as u64 + 1;
        self.lines.push_back(line.to_owned());

        while self.retained_bytes > self.byte_limit {
            let Some(oldest) = self.lines.pop_front() else {
                break;
            };

            self.retained_bytes -= oldest.len() as u64 + 1;
        }
    }

    /// Preserve the empty output line when matches exist but none fit the cap.
    fn render(&self, output: &mut Output) {
        if self.total == 0 {
            output.line("No match.");
            return;
        }

        let lines: Vec<&str> = self.lines.iter().map(String::as_str).collect();
        output.line(lines.join("\n"));

        if (self.lines.len() as u64) < self.total {
            output.line(format_args!(
                "Newest {} of {}. Narrow the regex.",
                self.lines.len(),
                plural(self.total, "match")
            ));
            return;
        }

        output.line(format_args!("{}.", plural(self.total, "match")));
    }
}

/// Search the whole canonical record, including its ID/date, without tree lookup.
pub(super) fn execute(app: &mut App, args: &[String]) -> R<()> {
    let [expression] = args else {
        return die(format!("usage: {} recall <regex>", app.executable));
    };

    let pattern = regex::RegexBuilder::new(expression)
        .case_insensitive(true)
        .build()
        .map_err(|error| crate::fail::Fail::Die(format!("bad regex: {}", error)))?;

    let mut matches = RecallMatches::new(app.config.part_bytes);

    store::log_scan(app.path(), |line| {
        if pattern.is_match(line) {
            matches.include(line);
        }
    })?;

    matches.render(&mut app.output);

    Ok(())
}

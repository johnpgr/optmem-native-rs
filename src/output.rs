//! Buffered command output and Python-compatible platform newlines.
//!
//! Commands may produce useful stdout before returning a refusal or error.
//! The CLI always emits this buffer before reporting the final status. Keeping
//! writes here preserves that ordering without making commands own stdio.

use std::fmt::{self, Write as _};

/// Stdout accumulated by a single invocation, including its trailing newlines.
#[derive(Default)]
pub struct Output {
    text: String,
}

impl Output {
    /// Append a formatted line without a custom formatting macro at call sites.
    pub fn line(&mut self, value: impl fmt::Display) {
        writeln!(self.text, "{value}").expect("formatting into a String cannot fail");
    }

    /// Separate logical sections without adding spaces to an otherwise blank line.
    pub fn blank_line(&mut self) {
        self.text.push('\n');
    }

    /// Borrow all stdout, even when the command ultimately returns an error.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// Translate console/config text as Python text mode does; store records stay LF.
pub fn text_mode(text: &str) -> String {
    if cfg!(windows) {
        return text.replace('\n', "\r\n");
    }

    text.to_owned()
}

//! The few pieces of Python string behavior the CLI depends on.

/// `str.isspace`: Unicode White_Space plus the four ASCII separators
/// U+001C..U+001F, which Python counts and Rust does not.
pub fn isspace(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// `str.strip()`.
pub fn strip(s: &str) -> &str {
    s.trim_matches(isspace)
}

/// `str.rstrip()`.
pub fn rstrip(s: &str) -> &str {
    s.trim_end_matches(isspace)
}

/// `str.isdigit()` restricted to ASCII. Python also accepts other Unicode
/// digits; see README.
pub fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// `int(s)` for a string `is_digits` accepted. None if it does not fit u64.
pub fn uint(s: &str) -> Option<u64> {
    if !is_digits(s) {
        return None;
    }

    s.parse().ok()
}

/// Inflect the reference's small noun vocabulary, including memory and summary.
pub fn plural(n: u64, word: &str) -> String {
    if n == 1 {
        return format!("1 {}", word);
    }

    let w = if let Some(stem) = word.strip_suffix('y') {
        format!("{}ie", stem)
    } else if word.ends_with(['s', 'h', 'x']) {
        format!("{}e", word)
    } else {
        word.to_string()
    };
    format!("{} {}s", n, w)
}

/// Text-mode reading with universal newlines: `\r\n`, `\r` and `\n` all end
/// a line. Returns the lines without their terminators.
pub fn lines(s: &str) -> Vec<String> {
    let s = s.replace("\r\n", "\n").replace('\r', "\n");
    let mut out: Vec<String> = s.split('\n').map(str::to_string).collect();

    if s.ends_with('\n') || s.is_empty() {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plurals() {
        assert_eq!(plural(1, "memory"), "1 memory");
        assert_eq!(plural(0, "memory"), "0 memories");
        assert_eq!(plural(2, "match"), "2 matches");
        assert_eq!(plural(3, "summary"), "3 summaries");
        assert_eq!(plural(2, "part"), "2 parts");
        assert_eq!(plural(5, "compression"), "5 compressions");
    }

    #[test]
    fn whitespace() {
        assert_eq!(strip("\x1c a \u{3000}\n"), "a");
        assert_eq!(rstrip("a \u{85}"), "a");
        assert_eq!(strip("\u{200b}a"), "\u{200b}a");
    }

    #[test]
    fn universal_newlines() {
        assert_eq!(lines("a\r\nb\rc\nd"), ["a", "b", "c", "d"]);
        assert_eq!(lines("a\n"), ["a"]);
        assert!(lines("").is_empty());
        assert_eq!(lines("\n"), [""]);
    }
}

//! Exact record encoding, borrowed decoding, and canonical-log recognition.
//!
//! Recall trims ASCII padding eight bytes at a time. Well-formed records are
//! borrowed directly; unusual but parseable heads are rendered canonically so
//! the fast path preserves the reference's observable record normalization.

use crate::fail::{die, R};
use crate::pystr;

/// Validated append input; its ID is assigned only after taking the writer lock.
pub struct DatedMemory {
    pub date: String,
    pub text: String,
}

/// One decoded complete record with its persisted ID, date, and memory text.
pub struct Entry {
    pub id: u64,
    pub date: String,
    pub text: String,
}

impl Entry {
    /// Render the canonical, unpadded line used by wake, prompts, and recall.
    pub fn line(&self) -> String {
        format!("#{} {} {}", self.id, self.date, self.text)
    }
}

/// Report the record's physical position when UTF-8 or its numeric head is invalid.
fn corrupt<T>(i: u64) -> R<T> {
    die(format!("Memory #{} in LOG.txt is corrupt.", i))
}

/// Decode one log record, as `bytes.decode().rstrip()`. The space padding
/// is trimmed as bytes first, eight at a time: it is ASCII, so validating
/// only what precedes it decides the same thing as validating it all.
#[inline]
pub(super) fn decode(rec: &[u8], at: u64) -> R<&str> {
    const SPACES: u64 = u64::from_ne_bytes([b' '; 8]);
    let mut end = rec.len();

    if rec.last() == Some(&b'\n') {
        end -= 1;
    }

    while end >= 8 && u64::from_ne_bytes(rec[end - 8..end].try_into().unwrap()) == SPACES {
        end -= 8;
    }

    while end > 0 && rec[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    match std::str::from_utf8(&rec[..end]) {
        Ok(s) => Ok(pystr::rstrip(s)),
        Err(_) => corrupt(at),
    }
}

/// `head, _, rest = line.partition(" ")` and so on, with `int(head[1:])`.
pub(super) fn parse(line: &str, at: u64) -> R<Entry> {
    let (head, rest) = line.split_once(' ').unwrap_or((line, ""));
    let (date, text) = rest.split_once(' ').unwrap_or((rest, ""));
    let mut cs = head.chars();
    cs.next();
    let Some(id) = pystr::uint(cs.as_str()) else {
        return corrupt(at);
    };

    Ok(Entry {
        id,
        date: date.into(),
        text: text.into(),
    })
}

/// The line `"#%d %s %s" % parse(line)` without building it, when the record
/// already reads that way -- which every record this tool wrote does.
#[inline]
pub(super) fn canonical(line: &str) -> bool {
    let b = line.as_bytes();
    let Some(sp) = b.iter().position(|&c| c == b' ') else {
        return false;
    };

    let digits = &b[1.min(sp)..sp];
    b.first() == Some(&b'#')
        && !digits.is_empty()
        && digits.len() <= 19
        && digits.iter().all(u8::is_ascii_digit)
        && (digits[0] != b'0' || digits.len() == 1)
        && b[sp + 1..].contains(&b' ')
}

/// Encode one exact-width record, rejecting overflow before constructing padding.
pub(super) fn pad(text: &str, rec: u64) -> R<Vec<u8>> {
    let b = text.as_bytes();
    let room = (rec - 1) as usize;

    if b.len() > room {
        return die(format!(
            "Too long: {} bytes. The record holds {}.",
            b.len(),
            room
        ));
    }

    let mut out = Vec::with_capacity(rec as usize);
    out.extend_from_slice(b);
    out.resize(room, b' ');
    out.push(b'\n');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_lines() {
        assert!(canonical("#0 2026-10-21 hello"));
        assert!(canonical("#12 2026-10-21 a b"));
        assert!(!canonical("#012 2026-10-21 a"));
        assert!(!canonical("#1 2026-10-21"));
        assert!(!canonical("x1 2026-10-21 a"));
        assert!(!canonical("# 2026-10-21 a"));
    }

    #[test]
    fn decodes_like_python() {
        let mut rec = pad("#3 2026-01-01 a b \u{3000}", 320).ok().unwrap();
        assert_eq!(decode(&rec, 0).ok().unwrap(), "#3 2026-01-01 a b");
        rec[300] = 0xff;
        assert!(decode(&rec, 0).is_err());
        assert_eq!(decode(b"x\x0c\t \n", 0).ok().unwrap(), "x");
        assert_eq!(decode(b"", 0).ok().unwrap(), "");
    }

    #[test]
    fn padding() {
        let r = pad("hé", 8).ok().unwrap();
        assert_eq!(r, b"h\xc3\xa9    \n");
        assert!(pad("12345678", 8).is_err());
    }
}

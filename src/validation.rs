//! Validation shared by memory-writing and tree-navigation commands.
//!
//! Validate inclusive CLI ranges before converting them to half-open blocks.
//! Bounded arithmetic must refuse oversized input rather than wrap to a
//! different on-disk summary. Text limits count UTF-8 bytes, not characters.

use crate::cover::Block;
use crate::fail::{die, R};
use crate::pystr;

/// Parse an aligned power-of-two block, preserving shape/range error wording.
pub fn block_id(input: &str) -> R<Block> {
    let Some((first, last)) = input.split_once('-') else {
        return malformed_block(input);
    };

    if !pystr::is_digits(first) || !pystr::is_digits(last) {
        return malformed_block(input);
    }

    let Some(lo) = pystr::uint(first) else {
        return invalid_block(input);
    };

    let Some(hi) = pystr::uint(last).and_then(|last| last.checked_add(1)) else {
        return invalid_block(input);
    };

    let size = hi as i128 - lo as i128;

    if size < 2 || size & (size - 1) != 0 || lo as i128 % size != 0 {
        return invalid_block(input);
    }

    Ok((lo, hi))
}

/// Distinguish a syntax error from a numeric range that is not a valid block.
fn malformed_block<T>(input: &str) -> R<T> {
    die(format!(
        "'{}' is not a block id. Copy it from the prompt.",
        input
    ))
}

/// Overflow and alignment failures share the reference's block-validation role.
fn invalid_block<T>(input: &str) -> R<T> {
    die(format!(
        "{} is not a block. Copy the id printed by wake, like 16-31.",
        input
    ))
}

/// Strip Python whitespace, then require one nonempty line within the byte limit.
pub fn memory_line(text: &str, byte_limit: u64) -> R<String> {
    let text = pystr::strip(text);

    if text.is_empty() {
        return die("Empty. A memory is one line of text.");
    }

    if text.contains(['\n', '\r']) {
        return die(format!(
            "{} lines. A memory is one line: merge them, or note them separately.",
            text.matches('\n').count() + 1
        ));
    }

    let bytes = text.len() as u64;

    if bytes > byte_limit {
        return die(format!(
            "Too long: {} bytes, limit {}. Accented characters cost 2 bytes. Compress it further.",
            bytes, byte_limit
        ));
    }

    Ok(text.to_owned())
}

//! Byte- and line-limited wake pages; oversized lines remain intact.
//!
//! A line is never split, even if it alone exceeds a configured part budget.
//! The caller supplies snapshot-stable lines and appends the continuation prompt.

/// Split the document into parts that survive any harness's output cap.
pub fn paginate(lines: Vec<String>, max_lines: u64, max_bytes: u64) -> Vec<Vec<String>> {
    let mut pages = Vec::new();
    let mut current_page = Vec::new();
    let mut current_bytes = 0u64;

    for line in lines {
        let line_bytes = line.len() as u64 + 1;

        if !current_page.is_empty()
            && (current_page.len() as u64 >= max_lines || current_bytes + line_bytes > max_bytes)
        {
            pages.push(std::mem::take(&mut current_page));
            current_bytes = 0;
        }

        current_page.push(line);
        current_bytes += line_bytes;
    }

    if !current_page.is_empty() {
        pages.push(current_page);
    }
    pages
}

//! Raw-memory seeks, bounded streaming recall, and locked batch appends.
//!
//! The scanner retains partial reads between iterations to stay record-aligned
//! when a concurrent writer appends. Callbacks borrow only the current record;
//! keeping a result requires copying it before the next read reuses the buffer.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use super::files::{append, repair};
use super::lock::locked;
use super::records::{canonical, decode, pad, parse, Entry};
use super::{count, log_path, DatedMemory, LOG_REC};
use crate::fail::{die, Fail, IoAt, R};

/// Count complete records only; an interrupted trailing append is not a memory.
pub fn log_len(directory: &Path) -> R<u64> {
    count(&log_path(directory), LOG_REC)
}

/// Attribute an open failure to LOG.txt before later handle-level I/O begins.
fn open_log(directory: &Path) -> R<File> {
    let path = log_path(directory);
    File::open(&path).at(&path)
}

/// Memories [lo,hi) in one read.
pub fn log_slice(directory: &Path, lo: u64, hi: u64) -> R<Vec<Entry>> {
    let mut file = open_log(directory)?;
    file.seek(SeekFrom::Start(lo * LOG_REC)).bare()?;
    let mut buffer = Vec::new();
    file.take((hi - lo) * LOG_REC)
        .read_to_end(&mut buffer)
        .bare()?;
    buffer
        .as_chunks::<{ LOG_REC as usize }>()
        .0
        .iter()
        .zip(lo..)
        .map(|(record, i)| parse(decode(record, i)?, i))
        .collect()
}

/// Memory i, in one seek.
pub fn log_get(directory: &Path, i: u64) -> R<Entry> {
    match log_slice(directory, i, i + 1)?.pop() {
        Some(e) => Ok(e),
        None => die(format!("Memory #{} is missing from LOG.txt.", i)),
    }
}

/// Every memory, streamed in record-aligned buffers, as its printed line.
/// A record still being appended at EOF is not a memory yet.
pub fn log_scan(directory: &Path, mut each: impl FnMut(&str)) -> R<()> {
    const REC: usize = LOG_REC as usize;
    let mut file = open_log(directory)?;
    let mut buffer = vec![0u8; REC * 4096];
    let mut buffered_bytes = 0usize;
    let mut record_index = 0u64;
    let mut canonical_line = String::new();

    loop {
        let n = match file.read(&mut buffer[buffered_bytes..]) {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(Fail::Io(None, e)),
        };

        if n == 0 {
            return Ok(());
        }
        buffered_bytes += n;
        let complete_bytes = buffered_bytes / REC * REC;

        for record in buffer[..complete_bytes].as_chunks::<REC>().0 {
            let line = decode(record, record_index)?;

            if canonical(line) {
                each(line);
            } else {
                canonical_line = parse(line, record_index)?.line();
                each(&canonical_line);
            }
            record_index += 1;
        }

        buffer.copy_within(complete_bytes..buffered_bytes, 0);
        buffered_bytes -= complete_bytes;
        canonical_line.clear();
    }
}

/// Append validated dated memories, assigning IDs inside the lock after repair.
/// Sync the full batch before acknowledging success; return its first assigned ID.
pub fn log_append(directory: &Path, items: &[DatedMemory]) -> R<u64> {
    let _lock = locked(directory)?;
    let path = log_path(directory);
    repair(&path, LOG_REC)?;
    let base = log_len(directory)?;
    let mut buffer = Vec::with_capacity(items.len() * LOG_REC as usize);

    for (offset, memory) in items.iter().enumerate() {
        buffer.extend(pad(
            &format!("#{} {} {}", base + offset as u64, memory.date, memory.text),
            LOG_REC,
        )?);
    }
    append(&path, &buffer)?;
    Ok(base)
}

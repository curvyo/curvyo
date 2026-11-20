//! Reads the last lines of an append-only log backwards in 64 KiB blocks, for `GetLog` (A18).

use std::fs::File;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;

/// The block read from the end at a time.
const BLOCK: u64 = 64 * 1024;
/// The most lines one `GetLog` returns (addendum D3).
pub const MAX_LINES: u32 = 1000;

/// `last` clamped to 1 to [`MAX_LINES`].
#[must_use]
pub fn clamp(last: u32) -> usize {
    usize::try_from(last.clamp(1, MAX_LINES)).unwrap_or(1)
}

/// The last `n` lines of the file at `path`, oldest first, without line ends; none when there is
/// no file. Invalid UTF-8 is replaced.
///
/// # Errors
/// The I/O error of an existing file.
pub fn last_lines(path: &Path, n: usize) -> std::io::Result<Vec<String>> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let len = file.metadata()?.len();
    let mut end = len;
    let mut tail: Vec<u8> = Vec::new();
    // One more line end than `n` in what was read means the first wanted line starts after it.
    #[allow(clippy::naive_bytecount)] // a log's tail, not worth a crate
    while end > 0 && tail.iter().filter(|b| **b == b'\n').count() <= n {
        let start = end.saturating_sub(BLOCK);
        file.seek(SeekFrom::Start(start))?;
        let mut block = vec![0; usize::try_from(end - start).unwrap_or(0)];
        file.read_exact(&mut block)?;
        block.extend_from_slice(&tail);
        tail = block;
        end = start;
    }
    let text = String::from_utf8_lossy(&tail);
    let lines: Vec<&str> = text.lines().collect();
    let skip = lines.len().saturating_sub(n);
    Ok(lines[skip..].iter().map(|l| (*l).to_owned()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    #[test]
    fn the_last_lines_across_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jobs.log");
        assert_eq!(last_lines(&path, 5).unwrap(), Vec::<String>::new());
        let mut text = String::new();
        for i in 0..5000 {
            writeln!(text, "line {i:05} {}", "x".repeat(40)).unwrap();
        }
        std::fs::write(&path, &text).unwrap();
        let got = last_lines(&path, 3).unwrap();
        assert_eq!(got.len(), 3);
        assert!(got[0].starts_with("line 04997"), "{got:?}");
        assert!(got[2].starts_with("line 04999"));
        let all = last_lines(&path, 1000).unwrap();
        assert_eq!(all.len(), 1000);
        assert!(all[0].starts_with("line 04000"));
        std::fs::write(&path, "one\ntwo").unwrap();
        assert_eq!(last_lines(&path, 5).unwrap(), vec!["one", "two"]);
        assert_eq!((clamp(0), clamp(20), clamp(5000)), (1, 20, 1000));
    }
}

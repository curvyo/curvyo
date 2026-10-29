//! The local files a pasted or dropped text names (`specs/0087-image-clipboard-and-drop` criteria
//! 6 and 21, lead decision 2026-10-29): `text/uri-list`, GNOME's `x-special/gnome-copied-files`
//! and plain text read as a list of `file://` URIs or absolute paths, never as a link to fetch.

use std::path::PathBuf;

/// The paths `text` names, in order and without repeats. Each line (LF or CRLF) is a `file://`
/// URI (percent-decoded; an empty host or `localhost` only), an absolute path, a URI of another
/// scheme (skipped: remote links are never fetched), a `#` comment of a URI list, or GNOME's
/// leading `copy`/`cut`. A text with any other line is prose and names no file. `windows` reads
/// drive paths (`file:///C:/…`, `C:\…`) instead of Unix ones.
pub(crate) fn local_paths(text: &str, windows: bool) -> Vec<PathBuf> {
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .peekable();
    if lines
        .peek()
        .is_some_and(|first| *first == "copy" || *first == "cut")
    {
        lines.next();
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    for line in lines {
        let path = match Entry::of(line, windows) {
            Entry::Local(path) => path,
            Entry::Other => continue,
            Entry::Prose => return Vec::new(),
        };
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    paths
}

/// What one line of a file list is.
enum Entry {
    /// A file on this machine.
    Local(PathBuf),
    /// A URI that is not a local file (another scheme or another host): skipped, never fetched.
    Other,
    /// Neither a URI nor an absolute path.
    Prose,
}

impl Entry {
    fn of(line: &str, windows: bool) -> Self {
        if let Some(rest) = strip_prefix_ignore_case(line, "file:") {
            return file_uri_path(rest, windows).map_or(Self::Other, Self::Local);
        }
        if is_absolute(line, windows) {
            return Self::Local(PathBuf::from(line));
        }
        if has_scheme(line) {
            return Self::Other;
        }
        Self::Prose
    }
}

/// The path of a `file:` URI after the scheme: `//host/path` with an empty host or `localhost`,
/// or `/path`; percent-decoded. `None` for another host (a network share) or a path that is not
/// absolute once decoded.
fn file_uri_path(rest: &str, windows: bool) -> Option<PathBuf> {
    let rest = rest.split(['?', '#']).next().unwrap_or_default();
    let path = match rest.strip_prefix("//") {
        Some(authority_and_path) => {
            let slash = authority_and_path.find('/')?;
            let (host, path) = authority_and_path.split_at(slash);
            if !(host.is_empty() || host.eq_ignore_ascii_case("localhost")) {
                return None;
            }
            path
        }
        None => rest,
    };
    let decoded = percent_decode(path)?;
    let decoded = if windows {
        // `/C:/Users/…` is the drive path `C:/Users/…`.
        let drive = decoded.strip_prefix('/').filter(|p| is_drive_path(p))?;
        drive.to_owned()
    } else {
        decoded
    };
    is_absolute(&decoded, windows).then(|| PathBuf::from(decoded))
}

/// `%XX` escapes decoded to bytes, read as UTF-8; `None` for a broken escape or bytes that are
/// not UTF-8 or that hold a NUL.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    if out.contains(&0) {
        return None;
    }
    String::from_utf8(out).ok()
}

/// Whether `text` is an absolute local path: `/…` on Unix, `C:\…` or `C:/…` on Windows (a
/// `\\server` share is not local).
fn is_absolute(text: &str, windows: bool) -> bool {
    if windows {
        is_drive_path(text)
    } else {
        text.starts_with('/') && !text.starts_with("//")
    }
}

/// `C:\…` or `C:/…`.
fn is_drive_path(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

/// Whether `text` starts with a URI scheme (`https:`, `mailto:`): letters, digits, `+`, `-`, `.`
/// after a letter, then `:`.
fn has_scheme(text: &str) -> bool {
    let Some((scheme, _)) = text.split_once(':') else {
        return false;
    };
    scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No path.
    const NONE: [String; 0] = [];

    fn unix(text: &str) -> Vec<String> {
        local_paths(text, false)
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    fn windows(text: &str) -> Vec<String> {
        local_paths(text, true)
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn a_file_uri_is_percent_decoded() {
        assert_eq!(
            unix("file:///home/me/My%20Pictures/caf%C3%A9%20%2350.png"),
            ["/home/me/My Pictures/café #50.png"]
        );
        assert_eq!(unix("file://localhost/tmp/a.png"), ["/tmp/a.png"]);
        assert_eq!(unix("FILE:///tmp/a.png"), ["/tmp/a.png"]);
        assert_eq!(unix("file:/tmp/a.png"), ["/tmp/a.png"]);
    }

    #[test]
    fn gnome_copied_files_skip_the_copy_line() {
        assert_eq!(
            unix("copy\nfile:///tmp/a.png\nfile:///tmp/b%20c.jpg"),
            ["/tmp/a.png", "/tmp/b c.jpg"]
        );
        assert_eq!(unix("cut\nfile:///tmp/a.png"), ["/tmp/a.png"]);
    }

    #[test]
    fn a_crlf_uri_list_with_comments_gives_every_file_once() {
        assert_eq!(
            unix("# dragged\r\nfile:///tmp/a.png\r\nfile:///tmp/b.webp\r\nfile:///tmp/a.png\r\n"),
            ["/tmp/a.png", "/tmp/b.webp"]
        );
    }

    #[test]
    fn plain_absolute_paths_are_files() {
        assert_eq!(
            unix("/tmp/a.png\n/tmp/b c.png\n"),
            ["/tmp/a.png", "/tmp/b c.png"]
        );
    }

    #[test]
    fn remote_links_and_other_schemes_are_skipped_never_read() {
        assert_eq!(unix("https://example.com/a.png"), NONE);
        assert_eq!(
            unix("http://example.com/a.png\r\nftp://example.com/b.png"),
            NONE
        );
        assert_eq!(unix("file://server/share/a.png"), NONE, "another host");
        assert_eq!(unix("//server/share/a.png"), NONE);
        assert_eq!(unix("data:image/png;base64,AAAA"), NONE);
        assert_eq!(
            unix("https://example.com/a.png\nfile:///tmp/b.png"),
            ["/tmp/b.png"],
            "the local entry of a mixed list"
        );
    }

    #[test]
    fn prose_names_no_file() {
        assert_eq!(unix("hello world"), NONE);
        assert_eq!(unix("see /tmp/a.png"), NONE);
        assert_eq!(unix("/tmp/a.png\nand some words"), NONE);
        assert_eq!(unix("relative/a.png"), NONE);
        assert_eq!(unix(""), NONE);
        assert_eq!(unix("copy"), NONE);
    }

    #[test]
    fn broken_escapes_and_nul_are_refused() {
        assert_eq!(unix("file:///tmp/a%2.png"), NONE);
        assert_eq!(unix("file:///tmp/a%zz.png"), NONE);
        assert_eq!(unix("file:///tmp/a%00.png"), NONE);
        assert_eq!(unix("file:///tmp/%FF.png"), NONE, "not UTF-8");
    }

    #[test]
    fn windows_drive_uris_and_paths() {
        assert_eq!(
            windows("file:///C:/Users/Me/My%20Pictures/a.png"),
            ["C:/Users/Me/My Pictures/a.png"]
        );
        assert_eq!(windows("file://localhost/D:/b.jpg"), ["D:/b.jpg"]);
        assert_eq!(windows("C:\\Users\\Me\\a.png"), ["C:\\Users\\Me\\a.png"]);
        assert_eq!(windows("file://server/share/a.png"), NONE, "a share");
        assert_eq!(windows("\\\\server\\share\\a.png"), NONE);
        assert_eq!(windows("file:///tmp/a.png"), NONE, "no drive");
        assert_eq!(windows("/tmp/a.png"), NONE);
        assert!(
            unix("file:///C:/a.png") == ["/C:/a.png"],
            "a Unix path on Unix"
        );
    }
}

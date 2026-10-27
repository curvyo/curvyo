//! The bytes of a `data:` URI (RFC 2397; `specs/0024-svg-import-export` criteria 100 and 105),
//! decoded straight from the attribute text, so no second copy of the base64 text is made. The
//! media type is not read: the bytes are identified by their content later.

/// The decoded bytes of `uri`, or `None` when it is not a `data:` URI or its base64 is damaged.
#[must_use]
pub fn decode_data_uri(uri: &str) -> Option<Vec<u8>> {
    let rest = uri.trim_start();
    if !rest.get(..5)?.eq_ignore_ascii_case("data:") {
        return None;
    }
    let rest = &rest[5..];
    let comma = rest.find(',')?;
    let (header, body) = (&rest[..comma], &rest[comma + 1..]);
    let base64 = header
        .split(';')
        .any(|part| part.trim().eq_ignore_ascii_case("base64"));
    if base64 {
        decode_base64(body)
    } else {
        Some(decode_percent(body))
    }
}

/// The value of one base64 digit, `None` for anything else.
const fn digit(byte: u8) -> Option<u32> {
    Some(match byte {
        b'A'..=b'Z' => (byte - b'A') as u32,
        b'a'..=b'z' => (byte - b'a') as u32 + 26,
        b'0'..=b'9' => (byte - b'0') as u32 + 52,
        b'+' | b'-' => 62,
        b'/' | b'_' => 63,
        _ => return None,
    })
}

/// Base64 (standard or URL alphabet; spaces, line breaks and percent-encoded padding skipped,
/// padding optional). `None` for any other byte or a dangling single digit.
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut acc = 0_u32;
    let mut count = 0_u8;
    let mut at = 0;
    let mut ended = false;
    while at < bytes.len() {
        let byte = bytes[at];
        at += 1;
        match byte {
            b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' => continue,
            b'=' => {
                ended = true;
                continue;
            }
            b'%' if bytes.get(at..at + 2) == Some(b"3D")
                || bytes.get(at..at + 2) == Some(b"3d") =>
            {
                at += 2;
                ended = true;
                continue;
            }
            _ => {}
        }
        if ended {
            return None;
        }
        acc = (acc << 6) | digit(byte)?;
        count += 1;
        if count == 4 {
            out.extend_from_slice(&acc.to_be_bytes()[1..]);
            acc = 0;
            count = 0;
        }
    }
    // invariant: two digits hold 12 bits (8 used), three hold 18 (16 used); the casts keep them.
    #[allow(clippy::cast_possible_truncation)]
    match count {
        0 => {}
        2 => out.push((acc >> 4) as u8),
        3 => out.extend_from_slice(&((acc >> 2) as u16).to_be_bytes()),
        _ => return None,
    }
    Some(out)
}

/// Percent-decoding; a `%` not followed by two hex digits stands for itself.
fn decode_percent(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = |b: u8| char::from(b).to_digit(16);
        if bytes[at] == b'%'
            && let (Some(&h), Some(&l)) = (bytes.get(at + 1), bytes.get(at + 2))
            && let (Some(h), Some(l)) = (hex(h), hex(l))
        {
            // invariant: two hex digits make at most 255.
            #[allow(clippy::cast_possible_truncation)]
            out.push((h * 16 + l) as u8);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn ac100_base64_and_percent() {
        assert_eq!(
            decode_data_uri("data:image/png;base64,aGVs\n bG8=").unwrap(),
            b"hello"
        );
        assert_eq!(decode_data_uri("DATA:;base64,aGk").unwrap(), b"hi");
        assert_eq!(decode_data_uri("data:,a%20b%zz").unwrap(), b"a b%zz");
        assert_eq!(decode_data_uri("data:;base64,aGk%3D").unwrap(), b"hi");
        assert_eq!(decode_data_uri("data:;base64,a$b"), None);
        assert_eq!(decode_data_uri("data:;base64,aGk=x"), None);
        assert_eq!(decode_data_uri("data:;base64,a"), None);
        assert_eq!(decode_data_uri("photo.png"), None);
        assert_eq!(decode_data_uri("data:image/png"), None);
    }
}

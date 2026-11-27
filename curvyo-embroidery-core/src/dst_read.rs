// SPDX-License-Identifier: AGPL-3.0-or-later AND MIT
// `decode_dx`, `decode_dy` and the record classes are transliterated from pyembroidery
// `pyembroidery/DstReader.py` (https://github.com/EmbroidePy/pyembroidery, commit c16d1b46,
// MIT, Copyright (c) 2018 the pyembroidery authors); see `NOTICE.md`.

//! The Tajima DST reader (`specs/0160-embroidery` criterion 17, `adrs.md` EM2): the header's
//! label, counts and extents and the records back as a stitch list. In slice 1 only the tests use
//! it; slice 10 turns its stitches into paths (Frage 432 A).

use crate::dst_write::{
    DstError, END_BYTE, Extents, HEADER_BYTES, JUMP_BIT, STOP_BYTE, TRIM_STEPS,
};
use crate::stitch::{Stitch, StitchKind, StitchPoint};

/// A DST file as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DstRead {
    /// The `LA:` label, trimmed.
    pub label: String,
    /// The `ST:` record count.
    pub records: usize,
    /// The `CO:` colour changes.
    pub colour_changes: usize,
    /// The header's extents.
    pub extents: Extents,
    /// The stitch list, a trim sequence read as one trim, ending with the end record.
    pub stitches: Vec<Stitch>,
}

fn bit(byte: u8, pos: u8) -> i32 {
    i32::from((byte >> pos) & 1)
}

/// The step of one record (pyembroidery's `decode_dx`, `decode_dy`; Y up, unflipped).
fn decode(b: [u8; 3]) -> (i32, i32) {
    let digits = |b: [u8; 3], spec: [(usize, u8, u8, i32); 5]| {
        spec.iter()
            .map(|&(byte, plus, minus, w)| (bit(b[byte], plus) - bit(b[byte], minus)) * w)
            .sum::<i32>()
    };
    let x = digits(
        b,
        [
            (2, 2, 3, 81),
            (1, 2, 3, 27),
            (0, 2, 3, 9),
            (1, 0, 1, 3),
            (0, 0, 1, 1),
        ],
    );
    let y = digits(
        b,
        [
            (2, 5, 4, 81),
            (1, 5, 4, 27),
            (0, 5, 4, 9),
            (1, 7, 6, 3),
            (0, 7, 6, 1),
        ],
    );
    (x, y)
}

/// The number after `key:` in the header text, `-` read as negative.
fn field(text: &str, key: &str) -> Option<i64> {
    let start = text.find(&format!("{key}:"))? + key.len() + 1;
    let value: String = text[start..].chars().take_while(|&c| c != '\r').collect();
    let compact: String = value.chars().filter(|c| !c.is_whitespace()).collect();
    compact.trim_start_matches('+').parse().ok()
}

fn header(bytes: &[u8]) -> Result<(String, usize, usize, Extents), DstError> {
    let head = bytes.get(..HEADER_BYTES).ok_or(DstError::Damaged)?;
    let end = head.iter().position(|&b| b == 0x1A).unwrap_or(HEADER_BYTES);
    let text = String::from_utf8_lossy(&head[..end]).into_owned();
    let label = text
        .strip_prefix("LA:")
        .and_then(|rest| rest.split('\r').next())
        .ok_or(DstError::Damaged)?
        .trim_end()
        .to_owned();
    let number = |key| field(&text, key).ok_or(DstError::Damaged);
    let count = |key| usize::try_from(number(key)?).map_err(|_| DstError::Damaged);
    let extent = |key| i32::try_from(number(key)?).map_err(|_| DstError::Damaged);
    Ok((
        label,
        count("ST")?,
        count("CO")?,
        Extents {
            plus_x: extent("+X")?,
            minus_x: extent("-X")?,
            plus_y: extent("+Y")?,
            minus_y: extent("-Y")?,
        },
    ))
}

/// Whether the records of `body` from `at` on are the writer's trim sequence.
fn is_trim(body: &[u8], at: usize) -> bool {
    (0..3).all(|k| {
        body.get((at + k) * 3..(at + k) * 3 + 3).is_some_and(|r| {
            let r = [r[0], r[1], r[2]];
            r[2] & JUMP_BIT != 0 && r[2] & STOP_BYTE != STOP_BYTE && decode(r) == TRIM_STEPS[k]
        })
    })
}

/// The DST file `bytes` as read.
///
/// # Errors
/// [`DstError::Damaged`] for a file shorter than its header, a header without its label or
/// counts, or records that do not end with the end record.
pub fn read_dst(bytes: &[u8]) -> Result<DstRead, DstError> {
    let (label, records, colour_changes, extents) = header(bytes)?;
    let mut stitches = Vec::new();
    let mut at = StitchPoint::ORIGIN;
    let mut ended = false;
    let body = bytes.get(HEADER_BYTES..).ok_or(DstError::Damaged)?;
    let mut index = 0;
    let record = |i: usize| -> Option<[u8; 3]> { body.get(i * 3..i * 3 + 3)?.try_into().ok() };
    while let Some(b) = record(index) {
        index += 1;
        if b[2] & END_BYTE == END_BYTE {
            stitches.push(Stitch {
                at,
                kind: StitchKind::End,
            });
            ended = true;
            break;
        }
        if b[2] & STOP_BYTE == STOP_BYTE {
            stitches.push(Stitch {
                at,
                kind: StitchKind::ColourChange,
            });
            continue;
        }
        let jump = b[2] & (JUMP_BIT | 0b11) == JUMP_BIT | 0b11;
        let trim = jump && is_trim(body, index - 1);
        if trim {
            stitches.push(Stitch {
                at,
                kind: StitchKind::Trim,
            });
            index += 2;
            continue;
        }
        let (dx, dy) = decode(b);
        at = StitchPoint::new(at.x.get() + dx, at.y.get() + dy);
        let kind = if jump {
            StitchKind::Jump
        } else {
            StitchKind::Normal
        };
        stitches.push(Stitch { at, kind });
    }
    if !ended {
        return Err(DstError::Damaged);
    }
    Ok(DstRead {
        label,
        records,
        colour_changes,
        extents,
        stitches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dst_write::{encode_step, write_dst};

    #[test]
    fn every_step_decodes_to_itself() {
        for x in -121..=121 {
            for y in [-121, -40, -1, 0, 1, 13, 121] {
                assert_eq!(decode(encode_step(x, y, false).unwrap()), (x, y));
            }
        }
    }

    #[test]
    fn a_written_list_reads_back_with_its_trim_and_colour_change() {
        let s = |x, y, kind| Stitch {
            at: StitchPoint::new(x, y),
            kind,
        };
        let list = vec![
            s(0, 0, StitchKind::Normal),
            s(30, -20, StitchKind::Normal),
            s(30, -20, StitchKind::Trim),
            s(100, -20, StitchKind::Jump),
            s(100, -20, StitchKind::ColourChange),
            s(100, 10, StitchKind::Normal),
            s(100, 10, StitchKind::End),
        ];
        let read = read_dst(&write_dst("Badge", &list).unwrap()).unwrap();
        assert_eq!(read.label, "Badge");
        assert_eq!(read.stitches, list);
        assert_eq!(read.colour_changes, 1);
        assert_eq!(read.records, 9, "a trim is three records");
        assert_eq!(
            read.extents,
            Extents {
                plus_x: 100,
                minus_x: 0,
                plus_y: 10,
                minus_y: 20
            }
        );
    }
}

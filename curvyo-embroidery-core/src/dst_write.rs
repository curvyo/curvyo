// SPDX-License-Identifier: AGPL-3.0-or-later AND MIT
// `encode_record` and the trim sequence are transliterated from pyembroidery
// `pyembroidery/DstWriter.py` (https://github.com/EmbroidePy/pyembroidery, commit c16d1b46,
// MIT, Copyright (c) 2018 the pyembroidery authors); see `NOTICE.md`.

//! The Tajima DST writer (`specs/0160-embroidery` criteria 14 to 16, 18, `adrs.md` EM4): a
//! 512-byte header and 3-byte balanced-ternary records of the stitch list's steps.

use crate::stitch::{Stitch, StitchKind, StitchPoint};

/// The size of the header.
pub const HEADER_BYTES: usize = 512;
/// The longest step of one record in tenths.
pub const MAX_STEP: i32 = 121;
/// The jumps that mean a trim (pyembroidery's default `trim_at` 3): out, back over, home.
pub const TRIM_STEPS: [(i32, i32); 3] = [(2, 2), (-4, -4), (2, 2)];

/// Why a stitch list cannot be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DstError {
    /// A step is longer than 12.1 mm in X or Y (the plan splits them).
    #[error("a step is longer than 12.1 mm")]
    StepTooLong,
    /// The bytes are no DST file.
    #[error("the file is no DST file")]
    Damaged,
}

/// How far the design reaches from the hoop centre, in tenths (criterion 14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Extents {
    /// Right of the centre.
    pub plus_x: i32,
    /// Left of the centre.
    pub minus_x: i32,
    /// Above the centre.
    pub plus_y: i32,
    /// Below the centre.
    pub minus_y: i32,
}

/// The extents of the needle points of `stitches` measured from the hoop centre, the centre
/// itself included (EM10: the machine starts there).
#[must_use]
pub fn extents(stitches: &[Stitch]) -> Extents {
    stitches
        .iter()
        .filter(|s| s.kind == StitchKind::Normal)
        .fold(Extents::default(), |e, s| Extents {
            plus_x: e.plus_x.max(s.at.x.get()),
            minus_x: e.minus_x.max(-s.at.x.get()),
            plus_y: e.plus_y.max(s.at.y.get()),
            minus_y: e.minus_y.max(-s.at.y.get()),
        })
}

/// The balanced-ternary digits of a step: weight, byte, the bit for plus and for minus, of X
/// then Y (pyembroidery's `encode_record`).
const X_DIGITS: [(i32, usize, u8, u8); 5] = [
    (81, 2, 2, 3),
    (27, 1, 2, 3),
    (9, 0, 2, 3),
    (3, 1, 0, 1),
    (1, 0, 0, 1),
];
const Y_DIGITS: [(i32, usize, u8, u8); 5] = [
    (81, 2, 5, 4),
    (27, 1, 5, 4),
    (9, 0, 5, 4),
    (3, 1, 7, 6),
    (1, 0, 7, 6),
];
/// The colour change (stop) record's third byte.
pub const STOP_BYTE: u8 = 0b1100_0011;
/// The end record's third byte.
pub const END_BYTE: u8 = 0b1111_0011;
/// The jump flag of the third byte.
pub const JUMP_BIT: u8 = 0b1000_0000;

/// One record of the step `x`, `y` in tenths, Y up (pyembroidery flips its Y-down frame here;
/// the stitch list is Y up already, EM10).
///
/// # Errors
/// [`DstError::StepTooLong`] past 121 tenths in X or Y.
pub fn encode_step(x: i32, y: i32, jump: bool) -> Result<[u8; 3], DstError> {
    if x.abs() > MAX_STEP || y.abs() > MAX_STEP {
        return Err(DstError::StepTooLong);
    }
    let mut bytes = [0_u8, 0, 0b0000_0011];
    if jump {
        bytes[2] |= JUMP_BIT;
    }
    for (mut v, digits) in [(x, X_DIGITS), (y, Y_DIGITS)] {
        for (weight, byte, plus, minus) in digits {
            if v > weight / 2 {
                bytes[byte] |= 1 << plus;
                v -= weight;
            }
            if v < -(weight / 2) {
                bytes[byte] |= 1 << minus;
                v += weight;
            }
        }
    }
    Ok(bytes)
}

/// `value` as `+` and five digits, or `-` and five.
fn signed(value: i32) -> String {
    let sign = if value < 0 { '-' } else { '+' };
    format!("{sign}{:5}", value.unsigned_abs())
}

/// The 512-byte header (criterion 14): `LA:` the label cut to 16 characters, the record count,
/// the colour changes, the extents, the last position, `MX`, `MY`, `PD`, the end mark `0x1A`
/// and spaces.
fn header(label: &str, records: usize, stitches: &[Stitch]) -> Vec<u8> {
    let label: String = label
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(16)
        .collect();
    let changes = stitches
        .iter()
        .filter(|s| s.kind == StitchKind::ColourChange)
        .count();
    let e = extents(stitches);
    let last = stitches.last().map_or(StitchPoint::ORIGIN, |s| s.at);
    let text = format!(
        "LA:{label:<16}\rST:{records:7}\rCO:{changes:3}\r+X:{:5}\r-X:{:5}\r+Y:{:5}\r-Y:{:5}\rAX:{}\rAY:{}\rMX:+    0\rMY:+    0\rPD:******\r",
        e.plus_x,
        e.minus_x,
        e.plus_y,
        e.minus_y,
        signed(last.x.get()),
        signed(last.y.get()),
    );
    let mut bytes = text.into_bytes();
    bytes.truncate(HEADER_BYTES - 1);
    bytes.push(0x1A);
    bytes.resize(HEADER_BYTES, b' ');
    bytes
}

/// The DST file of `stitches` labelled `label` (the page name): the header, then one record
/// per step from the hoop centre, a trim as the jumps out, back over and home, a colour change
/// as the stop record, the end record last. Nothing else is written: no colours (criterion 16).
///
/// # Errors
/// [`DstError::StepTooLong`] for a step past 12.1 mm, which the plan never makes.
pub fn write_dst(label: &str, stitches: &[Stitch]) -> Result<Vec<u8>, DstError> {
    let mut records = Vec::new();
    let mut at = StitchPoint::ORIGIN;
    for stitch in stitches {
        let (dx, dy) = (
            stitch.at.x.get() - at.x.get(),
            stitch.at.y.get() - at.y.get(),
        );
        match stitch.kind {
            StitchKind::Normal => records.push(encode_step(dx, dy, false)?),
            StitchKind::Jump => records.push(encode_step(dx, dy, true)?),
            StitchKind::Trim => {
                for (x, y) in TRIM_STEPS {
                    records.push(encode_step(x, y, true)?);
                }
            }
            StitchKind::ColourChange => records.push([0, 0, STOP_BYTE]),
            StitchKind::End => records.push([0, 0, END_BYTE]),
        }
        if !matches!(
            stitch.kind,
            StitchKind::Trim | StitchKind::ColourChange | StitchKind::End
        ) {
            at = stitch.at;
        }
    }
    let mut out = header(label, records.len(), stitches);
    out.extend(records.into_iter().flatten());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_encode_as_pyembroidery_s_balanced_ternary() {
        assert_eq!(encode_step(1, 0, false), Ok([0b0000_0001, 0, 0b0000_0011]));
        assert_eq!(encode_step(0, 1, false), Ok([0b1000_0000, 0, 0b0000_0011]));
        assert_eq!(
            encode_step(121, 121, true),
            Ok([0b1010_0101, 0b1010_0101, 0b1010_0111])
        );
        assert_eq!(
            encode_step(-121, -121, false),
            Ok([0b0101_1010, 0b0101_1010, 0b0001_1011])
        );
        assert_eq!(encode_step(122, 0, false), Err(DstError::StepTooLong));
    }

    #[test]
    fn the_header_is_512_bytes_with_the_label_cut_to_16_characters() {
        let stitches = [
            Stitch {
                at: StitchPoint::new(50, 50),
                kind: StitchKind::Jump,
            },
            Stitch {
                at: StitchPoint::new(50, 50),
                kind: StitchKind::Normal,
            },
            Stitch {
                at: StitchPoint::new(-50, -30),
                kind: StitchKind::Normal,
            },
            Stitch {
                at: StitchPoint::new(-50, -30),
                kind: StitchKind::End,
            },
        ];
        let bytes = write_dst("A very long page name", &stitches).unwrap();
        assert_eq!(bytes.len(), 512 + 4 * 3);
        let head = String::from_utf8_lossy(&bytes[..120]).into_owned();
        assert!(
            head.starts_with("LA:A very long page\rST:      4\rCO:  0\r"),
            "{head}"
        );
        assert!(
            head.contains("+X:   50\r-X:   50\r+Y:   50\r-Y:   30\rAX:-   50\rAY:-   30\r"),
            "{head}"
        );
        assert_eq!(bytes[511], b' ');
    }
}

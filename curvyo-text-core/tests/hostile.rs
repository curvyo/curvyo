//! Criterion 32 of `specs/0074-text-objects` and the limits of ADR 0020 §2: damaged or hostile
//! font bytes never panic or hang; they are refused or draw as nothing.
#![allow(clippy::unwrap_used)]
#![cfg(not(target_arch = "wasm32"))]

mod common;

use common::{SANS, SERIF, font_bytes};
use curvyo_text_core::limits::{MAX_FACE_BYTES, MAX_SHAPE_CHARS};
use curvyo_text_core::{FaceStore, Mm, TextError, layout};
use proptest::prelude::*;

/// Adds the bytes and, if they parse, lays out and outlines a text that touches many glyphs.
fn exercise(bytes: Vec<u8>) {
    let mut store = FaceStore::new();
    if let Ok(key) = store.add(bytes, 0) {
        let face = store.get(&key).unwrap();
        if let Ok(laid) = layout(face, "AVfi Hxg\u{E000}\nÅß0", Mm(5.0)) {
            let _ = laid.glyph_paths(face);
            let _ = laid.line_box();
        }
    }
}

fn read_u32(bytes: &[u8], at: usize) -> usize {
    usize::try_from(u32::from_be_bytes([
        bytes[at],
        bytes[at + 1],
        bytes[at + 2],
        bytes[at + 3],
    ]))
    .unwrap()
}

#[test]
fn ac32_named_limits() {
    let mut store = FaceStore::new();
    assert_eq!(
        store.add(vec![0; MAX_FACE_BYTES + 1], 0),
        Err(TextError::FaceTooLarge)
    );
    let key = store.add(font_bytes(SERIF), 0).unwrap();
    let face = store.get(&key).unwrap();
    assert!(layout(face, &"a".repeat(MAX_SHAPE_CHARS), Mm(5.0)).is_ok());
    assert_eq!(
        layout(face, &"a".repeat(MAX_SHAPE_CHARS + 1), Mm(5.0)),
        Err(TextError::TextTooLong)
    );
}

/// Truncated files, table offsets past the end, and a composite glyph that contains itself.
#[test]
fn ac32_crafted_hostile_files() {
    for source in [SERIF, SANS] {
        let bytes = font_bytes(source);
        for cut in [0, 4, 12, 100, 1000, bytes.len() / 2, bytes.len() - 1] {
            exercise(bytes[..cut].to_vec());
        }
        let mut past_end = bytes.clone();
        let tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
        for record in 0..tables {
            past_end[12 + 16 * record + 8..12 + 16 * record + 12]
                .copy_from_slice(&0xFFFF_FF00_u32.to_be_bytes());
        }
        exercise(past_end);
    }
    exercise(self_referencing_composite(font_bytes(SERIF)));
}

/// Rewrites every glyph of a TrueType font long enough as a composite of itself (a loop).
fn self_referencing_composite(mut bytes: Vec<u8>) -> Vec<u8> {
    let tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    let find = |tag: &[u8; 4], bytes: &[u8]| {
        (0..tables)
            .map(|r| 12 + 16 * r)
            .find(|at| &bytes[*at..*at + 4] == tag)
            .map(|at| (read_u32(bytes, at + 8), read_u32(bytes, at + 12)))
    };
    let (head, _) = find(b"head", &bytes).unwrap();
    let long = bytes[head + 50] == 0 && bytes[head + 51] == 1;
    let (loca, loca_len) = find(b"loca", &bytes).unwrap();
    let (glyf, _) = find(b"glyf", &bytes).unwrap();
    let count = if long { loca_len / 4 } else { loca_len / 2 } - 1;
    for gid in 0..count {
        let offset = |i: usize| {
            if long {
                read_u32(&bytes, loca + 4 * i)
            } else {
                2 * usize::from(u16::from_be_bytes([
                    bytes[loca + 2 * i],
                    bytes[loca + 2 * i + 1],
                ]))
            }
        };
        let (start, end) = (offset(gid), offset(gid + 1));
        if end - start < 16 {
            continue;
        }
        let at = glyf + start;
        let gid16 = u16::try_from(gid).unwrap().to_be_bytes();
        bytes[at..at + 2].copy_from_slice(&(-1_i16).to_be_bytes());
        // flags 0 (byte arguments, last component), glyph index = itself, offsets 0, 0
        bytes[at + 10..at + 16].copy_from_slice(&[0, 0, gid16[0], gid16[1], 0, 0]);
    }
    bytes
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// Random byte mutations of both outline flavours never panic.
    #[test]
    fn ac32_mutated_fonts_never_panic(
        flips in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..64),
        cff in any::<bool>(),
    ) {
        let mut bytes = font_bytes(if cff { SANS } else { SERIF });
        let len = bytes.len();
        for (index, value) in flips {
            bytes[index.index(len)] = value;
        }
        exercise(bytes);
    }
}

/// The time-boxed soak run of ADR 0020 §2 (the PR records one 600 s run): table-targeted random
/// mutations, every Latin glyph drawn, each case under 2 s (no hang). Stable Rust has no
/// `cargo fuzz`, so this harness takes its place (`plan.md`, dated notes). Run with
/// `CURVYO_HOSTILE_SECONDS=600 cargo test --release -p curvyo-text-core --test hostile -- --ignored`.
#[test]
#[ignore = "time-boxed soak run, started by hand"]
fn ac32_soak_mutations() {
    let seconds: u64 = std::env::var("CURVYO_HOSTILE_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600);
    let sources = [font_bytes(SERIF), font_bytes(SANS)];
    let text: String = (0x20_u32..0x250).filter_map(char::from_u32).collect();
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move |below: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        usize::try_from(state % u64::try_from(below.max(1)).unwrap()).unwrap()
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    let mut cases = 0_u64;
    while std::time::Instant::now() < deadline {
        let mut bytes = sources[next(2)].clone();
        let tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
        let record = 12 + 16 * next(tables);
        let (offset, length) = (read_u32(&bytes, record + 8), read_u32(&bytes, record + 12));
        for _ in 0..=next(16) {
            let at = offset + next(length);
            bytes[at] = u8::try_from(next(256)).unwrap();
        }
        let started = std::time::Instant::now();
        let mut store = FaceStore::new();
        if let Ok(key) = store.add(bytes, 0) {
            let face = store.get(&key).unwrap();
            if let Ok(laid) = layout(face, &text, Mm(5.0)) {
                let _ = laid.glyph_paths(face);
            }
        }
        assert!(started.elapsed().as_secs() < 2, "a case took over 2 s");
        cases += 1;
    }
    println!("hostile soak: {cases} cases in {seconds} s, no panic, no hang");
}

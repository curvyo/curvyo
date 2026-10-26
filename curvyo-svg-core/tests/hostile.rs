//! Hostile input (`specs/0024-svg-import-export` criteria 6, 67 to 70): every file gives a
//! refusal or a drawing, quickly, without a panic, and nothing is fetched or expanded.
#![allow(clippy::unwrap_used, clippy::needless_raw_string_hashes)]

use std::fmt::Write;
use std::time::{Duration, Instant};

use curvyo_svg_core::limits::MAX_FILE_BYTES;
use curvyo_svg_core::{SvgError, SvgNode, read_svg};

const NS: &str = r#"xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink""#;

fn leaf_count(nodes: &[SvgNode]) -> usize {
    nodes
        .iter()
        .map(|n| match n {
            SvgNode::Group { children, .. } => leaf_count(children),
            SvgNode::Leaf { .. } => 1,
        })
        .sum()
}

/// Reads within the time limit of criterion 70 (a).
fn timed(bytes: &[u8]) -> Result<curvyo_svg_core::SvgDrawing, SvgError> {
    let start = Instant::now();
    let result = read_svg(bytes);
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "took {:?}",
        start.elapsed()
    );
    result
}

#[test]
fn entity_bombs_and_external_entities_are_refused() {
    let bomb = r#"<?xml version="1.0"?><!DOCTYPE svg [
        <!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;">
        <!ENTITY c "&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;">]><svg>&c;</svg>"#;
    assert_eq!(timed(bomb.as_bytes()).unwrap_err(), SvgError::Entities);
    let external = r#"<!DOCTYPE svg [<!ENTITY x SYSTEM "file:///etc/passwd">]><svg>&x;</svg>"#;
    assert_eq!(timed(external.as_bytes()).unwrap_err(), SvgError::Entities);
    let parameter = r#"<!DOCTYPE svg [<!ENTITY % p SYSTEM "http://example.com/x.dtd"> %p;]><svg/>"#;
    assert_eq!(timed(parameter.as_bytes()).unwrap_err(), SvgError::Entities);
    let fan = format!(
        r#"<!DOCTYPE svg [<!ENTITY e "{}">]><svg {NS}><desc>{}</desc></svg>"#,
        "x".repeat(256),
        "&e;".repeat(200_000)
    );
    assert_eq!(timed(fan.as_bytes()).unwrap_err(), SvgError::Entities);
}

#[test]
fn use_loops_and_fan_outs_stop_at_the_limits() {
    let looped = format!(
        r##"<svg {NS}><g id="a"><use href="#b"/></g><g id="b"><use href="#a"/><path d="M0 0 H1"/></g><use href="#self" id="self"/></svg>"##
    );
    let d = timed(looped.as_bytes()).unwrap();
    assert!(leaf_count(&d.nodes) >= 1);
    let mut fan = format!(
        r#"<svg {NS} width="10mm" height="10mm" viewBox="0 0 10 10"><defs><path id="u0" d="M0 0 H1"/>"#
    );
    for level in 1..=20 {
        let below = level - 1;
        write!(
            fan,
            r##"<g id="u{level}"><use href="#u{below}"/><use href="#u{below}"/></g>"##
        )
        .unwrap();
    }
    fan.push_str(r##"</defs><use href="#u20"/></svg>"##);
    let d = timed(fan.as_bytes()).unwrap();
    assert!(
        leaf_count(&d.nodes) <= 100_000,
        "a million copies are cut at the copy limit"
    );
}

#[test]
fn hostile_numbers_and_lists() {
    let d = timed(
        format!(
            r#"<svg {NS} width="10mm" height="10mm" viewBox="0 0 10 10"><path d="M0 0 L1e308 1e308"/><path d="M NaN 0 L 1 1"/><rect width="1e400" height="1"/>
            <path stroke="red" stroke-dasharray="{}" d="M0 0 H1"/></svg>"#,
            "1 ".repeat(100_000)
        )
        .as_bytes(),
    )
    .unwrap();
    assert_eq!(leaf_count(&d.nodes), 1);
}

#[test]
fn broken_files_are_refused_with_the_right_reason() {
    assert!(matches!(
        timed(b"<svg><g></svg").unwrap_err(),
        SvgError::NotSvg(Some((1, _)))
    ));
    assert!(matches!(
        timed(b"not xml at all").unwrap_err(),
        SvgError::NotSvg(_)
    ));
    let utf16: Vec<u8> = "\u{feff}<svg/>"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    assert_eq!(timed(&utf16).unwrap_err(), SvgError::NotUtf8);
    assert_eq!(
        timed(&[0x1F, 0x8B, 8, 0, 0, 0]).unwrap_err(),
        SvgError::Compressed
    );
    let deep = format!(
        "<svg {NS}>{}{}</svg>",
        "<g>".repeat(1000),
        "</g>".repeat(1000)
    );
    assert_eq!(timed(deep.as_bytes()).unwrap_err(), SvgError::NotSvg(None));
    let huge = vec![b' '; MAX_FILE_BYTES + 1];
    assert_eq!(timed(&huge).unwrap_err(), SvgError::TooLarge);
}

#[test]
fn nothing_outside_the_file_is_followed() {
    let text = format!(
        r#"<?xml-stylesheet href="http://example.com/x.css"?><svg {NS}><style>@import url(http://example.com/y.css);</style>
        <image href="file:///etc/passwd"/><use href="http://example.com/a.svg#x"/><use href="file.svg#p"/>
        <path fill="url(http://example.com/g.svg#g)" d="M0 0 H1"/></svg>"#
    );
    let d = timed(text.as_bytes()).unwrap();
    assert_eq!(leaf_count(&d.nodes), 1);
}

#[cfg(not(target_arch = "wasm32"))]
mod property {
    use proptest::prelude::*;

    const SEED: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><style>.a{fill:red}</style><g transform="rotate(30)"><path class="a" d="M0 0 C 1 2 3 4 5 6 A 3 3 0 1 1 9 9 Z" stroke-dasharray="1 2"/><rect id="r" rx="2" width="5" height="5"/><use href="#r" x="4"/></g></svg>"##;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        /// Criterion 70: arbitrary bytes give a refusal or a drawing, never a panic.
        #[test]
        fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
            let _ = curvyo_svg_core::read_svg(&bytes);
        }

        /// The seed file with bytes changed or cut never panics either.
        #[test]
        fn mutated_files_never_panic(
            changes in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 0..8),
            cut in any::<prop::sample::Index>(),
        ) {
            let mut bytes = SEED.as_bytes().to_vec();
            for (at, value) in changes {
                let i = at.index(bytes.len());
                bytes[i] = value;
            }
            let end = cut.index(bytes.len() + 1);
            let _ = curvyo_svg_core::read_svg(&bytes[..end]);
        }
    }
}

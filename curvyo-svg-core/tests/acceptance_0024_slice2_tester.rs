//! Independent tester tests for `specs/0024-svg-import-export` slice 2 at the reader: the text
//! limits of criterion 98 (characters per file, numbers per attribute, font names) and the data
//! URI exemption of criterion 105, observed on the font-free run list the reader returns. Written
//! from the specification; only the public names of the reader were looked up. Timing of the
//! same hostile files is in `curvyo-editor-wasm/tests/acceptance_0024_slice2_tester.rs` (thread
//! CPU time).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::format_collect,
    missing_docs
)]

use curvyo_svg_core::{FontFamily, SvgDrawing, SvgNode, SvgText, read_svg};

fn svg(body: &str) -> Vec<u8> {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="210mm" height="297mm" viewBox="0 0 210 297">{body}</svg>"#
    )
    .into_bytes()
}

fn all_texts(d: &SvgDrawing) -> Vec<&SvgText> {
    fn walk<'a>(nodes: &'a [SvgNode], out: &mut Vec<&'a SvgText>) {
        for n in nodes {
            match n {
                SvgNode::Group { children, .. } => walk(children, out),
                SvgNode::Text { text, .. } => out.push(text),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&d.nodes, &mut out);
    out
}

fn char_count(d: &SvgDrawing) -> usize {
    all_texts(d).iter().map(|t| t.chars.len()).sum()
}

/// Criterion 98: a file holds at most 1,000,000 text characters; the rest is skipped.
#[test]
fn c98_two_million_characters_are_cut_at_one_million() {
    let body = format!(
        r#"<text x="1" y="1">{}</text><text x="1" y="2">tail</text>"#,
        "ab".repeat(1_000_000)
    );
    let d = read_svg(&svg(&body)).unwrap();
    assert_eq!(char_count(&d), 1_000_000, "at most 1,000,000 characters");
}

/// Criterion 98: 100,000 one-character texts are read (each counts one element and one command
/// toward criterion 69; both are far below its limits).
#[test]
fn c98_hundred_thousand_one_character_texts() {
    let body: String = (0..100_000)
        .map(|i| format!(r#"<text x="{}" y="1">x</text>"#, i % 200))
        .collect();
    let d = read_svg(&svg(&body)).unwrap();
    assert_eq!(all_texts(&d).len(), 100_000);
}

/// Criterion 98: 10,000 different font names are read; a name over 128 characters is cut.
#[test]
fn c98_ten_thousand_font_names_and_long_names() {
    let body: String = (0..10_000)
        .map(|i| format!(r#"<text x="1" y="1" style="font-family:'Family {i}'">x</text>"#))
        .collect();
    let d = read_svg(&svg(&body)).unwrap();
    assert_eq!(all_texts(&d).len(), 10_000);
    let long = format!(
        r#"<text x="1" y="1" style="font-family:'{}'">x</text>"#,
        "N".repeat(300)
    );
    let d = read_svg(&svg(&long)).unwrap();
    let t = all_texts(&d)[0];
    let fam = &t.styles[t.chars[0].style].families[0];
    match fam {
        FontFamily::Named(name) => assert_eq!(name.chars().count(), 128),
        other => panic!("{other:?}"),
    }
}

/// Criterion 98: per-character lists are read up to 10,000 numbers per attribute; the rest of
/// the characters flow.
#[test]
fn c98_position_lists_are_read_up_to_ten_thousand_numbers() {
    let n = 10_005;
    let xs: Vec<String> = (0..n).map(|i| (i % 100).to_string()).collect();
    let body = format!(
        r#"<text x="{}" y="1" rotate="{}">{}</text>"#,
        xs.join(" "),
        xs.join(" "),
        "x".repeat(n)
    );
    let d = read_svg(&svg(&body)).unwrap();
    let t = all_texts(&d)[0];
    assert_eq!(t.chars.len(), n);
    assert!(t.chars[9_999].x.is_some(), "the 10,000th number is read");
    assert!(t.chars[10_000].x.is_none(), "the 10,001st is not");
    assert_eq!(
        t.chars[10_000].rotate,
        Some(99.0),
        "the last rotate value read holds for the rest"
    );
}

/// Criterion 105: an image `data:` href above 16 MiB is read (bounded by the 50 MiB file
/// limit); its bytes are decoded.
#[test]
fn c105_data_href_above_sixteen_mib_is_read() {
    let payload = "A".repeat(20 * 1024 * 1024);
    let body = format!(
        r#"<image width="10" height="10" xlink:href="data:application/octet-stream;base64,{payload}"/>"#
    );
    let d = read_svg(&svg(&body)).unwrap();
    let found = d.nodes.iter().find_map(|n| match n {
        SvgNode::Image { image, .. } => Some(image.bytes.len()),
        _ => None,
    });
    assert_eq!(
        found,
        Some(15 * 1024 * 1024),
        "20 MiB of base64 decodes to 15 MiB"
    );
}

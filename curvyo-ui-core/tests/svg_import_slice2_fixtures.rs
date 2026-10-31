//! The slice-2 fixtures against `specs/0024-svg-import-export` (criteria 82, 91 to 103, 107) and
//! the hostile text inputs of criterion 98.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

mod svg_fonts;

use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use curvyo_document_core::{DrawingNode, MISSING_FACE_HASH, TextWritingMode};
use curvyo_ui_core::{SvgImport, SvgPlacement};
use svg_fonts::Fonts;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/svg")
}

fn open(fonts: &Fonts, name: &str) -> SvgImport {
    fonts.import(
        &std::fs::read(fixtures().join(name)).unwrap(),
        SvgPlacement::File,
    )
}

fn flat(nodes: &[DrawingNode]) -> Vec<&DrawingNode> {
    let mut out = Vec::new();
    for node in nodes {
        if let DrawingNode::Group { children, .. } = node {
            out.extend(flat(children));
        } else {
            out.push(node);
        }
    }
    out
}

fn text_named<'a>(
    nodes: &[&'a DrawingNode],
    string: &str,
) -> &'a curvyo_document_core::DrawingText {
    nodes
        .iter()
        .find_map(|n| match n {
            DrawingNode::Text { text, .. } | DrawingNode::TextOnPath { text, .. }
                if text.string == string =>
            {
                Some(text)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no text {string:?}"))
}

/// Criteria 81, 82, 84, 91, 92 on Inkscape 1.4.4's own file.
#[test]
fn ac107_text_inkscape() {
    let fonts = Fonts::included();
    let import = open(&fonts, "text_inkscape.svg");
    let all = flat(&import.nodes);
    assert_eq!(
        text_named(&all, "Bold Italic face").font.style,
        "Bold Italic"
    );
    assert_eq!(text_named(&all, "Light face").font.style, "Light");
    assert_eq!(
        text_named(&all, "Missing font").font.face_hash,
        MISSING_FACE_HASH
    );
    assert_eq!(
        text_named(&all, "Missing font").font.family,
        "Nonexistent Sans"
    );
    assert_eq!(
        text_named(&all, "VERT").layout.writing_mode,
        TextWritingMode::VerticalRl
    );
    let turned = text_named(&all, "Rotated").placement.to_array();
    assert!((turned[1].atan2(turned[0]).to_degrees() - 30.0).abs() < 1e-6);
    assert!(text_named(&all, "Mirrored").placement.to_array()[0] < 0.0);
    assert_eq!(text_named(&all, "world").font.style, "Bold");
    assert!(!text_named(&all, "KERNED pair").chars.is_empty());
    let mixed = import.nodes.iter().find_map(|n| match n {
        DrawingNode::Group { children, .. } => children.iter().find_map(|c| match c {
            DrawingNode::Group { registers, .. } => registers.name.clone(),
            _ => None,
        }),
        _ => None,
    });
    assert_eq!(
        mixed.as_deref(),
        Some("t_mixed"),
        "the label names the group"
    );
}

/// Criteria 93 to 95: carriers keep the file's role; a moved text gets a new guide path.
#[test]
fn ac107_text_on_path() {
    let fonts = Fonts::included();
    let import = open(&fonts, "text_on_path.svg");
    let all = flat(&import.nodes);
    let carriers: Vec<_> = all
        .iter()
        .filter_map(|n| match n {
            DrawingNode::Path { registers, .. } | DrawingNode::Primitive { registers, .. } => {
                Some(registers.guide)
            }
            _ => None,
        })
        .collect();
    assert_eq!(carriers, [false, false, true, true, false, true]);
    let on_path = all
        .iter()
        .filter(|n| matches!(n, DrawingNode::TextOnPath { .. }))
        .count();
    assert_eq!(on_path, 5);
}

/// Criteria 100 to 104: embedded pictures come in once per file, the linked one does not.
#[test]
fn ac107_images() {
    let fonts = Fonts::included();
    let import = open(&fonts, "images.svg");
    assert_eq!(import.assets.len(), 2);
    let images: Vec<_> = flat(&import.nodes)
        .into_iter()
        .filter_map(|n| match n {
            DrawingNode::Image {
                registers, asset, ..
            } => Some((registers.name.clone(), *asset)),
            _ => None,
        })
        .collect();
    assert_eq!(images.len(), 7, "the linked image is left out");
    assert_eq!(images[0].0.as_deref(), Some("img_png"));
    assert_eq!(images[0].1, images[1].1, "the same PNG is one asset");
    let others = open(&fonts, "images_webp_and_others.svg");
    assert_eq!(others.leaves, 2, "the WebP and the percent-encoded PNG");
}

/// Criteria 79, 96, 97, 99 on the generator, flowRoot and frame files.
#[test]
fn ac107_generators_flow_and_frames() {
    let fonts = Fonts::included();
    let generators = open(&fonts, "text_generators.svg");
    let all = flat(&generators.nodes);
    text_named(&all, "SHOUTING");
    text_named(&all, "collapsed spaces");
    text_named(&all, "   kept    spaces");
    text_named(&all, "first\nsecond");
    text_named(&all, "Behind the switch");
    let flow = open(&fonts, "text_flowroot.svg");
    let all = flat(&flow.nodes);
    let first = text_named(
        &all,
        "Inkscape 0.92 wrote\nflowed text that browsers\nnever draw.",
    );
    assert_eq!(first.layout.align, curvyo_document_core::TextAlign::Centre);
    let frame = open(&fonts, "text_frame.svg");
    assert_eq!(frame.leaves, 2);
}

fn mm(body: &str) -> Vec<u8> {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="200mm" height="200mm" viewBox="0 0 200 200">{body}</svg>"#)
        .into_bytes()
}

/// The calling thread's CPU time (Linux `schedstat`; wall time since the first call elsewhere), so
/// that a loaded machine does not fail a bound (`CLAUDE.md` §5).
fn thread_cpu() -> Duration {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok())
        .map_or_else(
            || {
                static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                START.get_or_init(std::time::Instant::now).elapsed()
            },
            Duration::from_nanos,
        )
}

/// Criterion 98's hostile inputs: a 2,000,000-character text, 100,000 one-character texts and
/// 10,000 font names import in bounded time (thread CPU) without a panic.
#[test]
fn ac98_hostile_texts() {
    let fonts = Fonts::included();
    // 60 s of CPU in a release build (it takes about 1.5 s there); the unoptimised build the gate
    // runs is some 20 times slower, so its bound is five times longer.
    let bound = Duration::from_secs(if cfg!(debug_assertions) { 300 } else { 60 });
    let long = mm(&format!(
        r#"<text y="10">{}</text>"#,
        "word ".repeat(400_000)
    ));
    let started = thread_cpu();
    let import = fonts.import(&long, SvgPlacement::File);
    let took = thread_cpu().saturating_sub(started);
    assert!(took < bound, "{took:?}");
    let pieces = flat(&import.nodes).len();
    assert_eq!(pieces, 100, "1,000,000 characters in texts of 10,000");
    let mut many = String::new();
    for i in 0..100_000 {
        write!(many, r#"<text x="{}" y="{}">x</text>"#, i % 200, i / 1000).unwrap();
    }
    let started = thread_cpu();
    let import = fonts.import(&mm(&many), SvgPlacement::File);
    let took = thread_cpu().saturating_sub(started);
    assert!(took < bound, "{took:?}");
    assert_eq!(import.leaves, 100_000);
    // A position per character far apart: every character its own text, found in one pass.
    let mut xs = String::new();
    for i in 0..10_000 {
        write!(xs, "{} ", i * 50).unwrap();
    }
    let spread = mm(&format!(
        r#"<text x="{xs}" y="10">{}</text>"#,
        "x".repeat(10_000)
    ));
    let started = thread_cpu();
    let import = fonts.import(&spread, SvgPlacement::File);
    let took = thread_cpu().saturating_sub(started);
    assert!(took < bound, "{took:?}");
    assert_eq!(flat(&import.nodes).len(), 10_000);
    let mut names = String::new();
    for i in 0..10_000 {
        write!(
            names,
            r#"<text y="5" style="font-family:'Face {i}'">y</text>"#
        )
        .unwrap();
    }
    let started = thread_cpu();
    let import = fonts.import(&mm(&names), SvgPlacement::File);
    let took = thread_cpu().saturating_sub(started);
    assert!(took < bound, "{took:?}");
    assert_eq!(import.leaves, 10_000);
}

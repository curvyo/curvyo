//! The budgets of slice 2 of the SVG import (`specs/0024-svg-import-export` criterion 108), from
//! the bytes to a finished document with the faces already loaded. Native release builds only:
//! `cargo test --release -p curvyo-ui-core --test svg_text_budget -- --ignored --nocapture`.
#![allow(clippy::unwrap_used, missing_docs)]

mod svg_fonts;

use std::fmt::Write;
use std::time::{Duration, Instant};

use base64_free::encode;
use curvyo_document_core::{AnchorId, Document, DrawingNode};
use curvyo_ui_core::SvgPlacement;
use svg_fonts::Fonts;

/// Open of `bytes` into a new document, timed.
fn open(fonts: &Fonts, bytes: &[u8]) -> (Duration, usize) {
    let started = Instant::now();
    let import = fonts.import(bytes, SvgPlacement::File);
    let ids: Vec<AnchorId> = (0..DrawingNode::anchor_count(&import.nodes) as u64)
        .map(|n| AnchorId::new(9, n))
        .collect();
    let document =
        Document::from_drawing(7, &import.page, &import.nodes, &ids, &import.assets).unwrap();
    let took = started.elapsed();
    (took, document.read_objects().objects.len())
}

/// 1,000 texts of 50 characters in 3 faces, a third with `dx` lists: under 1.5 s.
#[test]
#[ignore = "budget: release build"]
fn ac108_thousand_texts() {
    let fonts = Fonts::included();
    let mut svg = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000mm" height="1000mm" viewBox="0 0 1000 1000">"#,
    );
    let styles = [
        "font-weight:normal",
        "font-weight:bold",
        "font-style:italic",
    ];
    for i in 0..1000 {
        let dx = if i % 3 == 0 {
            let list: Vec<String> = (0..50)
                .map(|k| format!("{:.2}", f64::from(k % 5) * 0.1))
                .collect();
            format!(r#" dx="{}""#, list.join(" "))
        } else {
            String::new()
        };
        write!(
            svg,
            r#"<text x="{}" y="{}" style="font-family:'Source Sans 3';font-size:4px;{}"{dx}>The quick brown fox jumps over the lazy dog twice.</text>"#,
            (i % 10) * 100,
            (i / 10) * 10 + 5,
            styles[i % 3],
        )
        .unwrap();
    }
    svg.push_str("</svg>");
    let (took, objects) = open(&fonts, svg.as_bytes());
    println!("1,000 texts: {took:?}");
    assert_eq!(objects, 1000);
    assert!(took < Duration::from_millis(1500), "{took:?}");
}

/// One embedded 12-megapixel JPEG: under `0058`'s 2.0 s plus 0.3 s for the base64.
#[test]
#[ignore = "budget: release build"]
fn ac108_twelve_megapixel_jpeg() {
    let fonts = Fonts::included();
    let (w, h) = (4000_u32, 3000_u32);
    let pixels: Vec<u8> = (0..w * h)
        .flat_map(|i| {
            let (x, y) = (i % w, i / w);
            let byte = |v: u32| u8::try_from(v % 256).unwrap();
            [byte(x), byte(y), byte(x ^ y)]
        })
        .collect();
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85)
        .encode(&pixels, w, h, image::ExtendedColorType::Rgb8)
        .unwrap();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="400mm" height="300mm" viewBox="0 0 400 300"><image width="400" height="300" href="data:image/jpeg;base64,{}"/></svg>"#,
        encode(&jpeg)
    );
    let (took, objects) = open(&fonts, svg.as_bytes());
    println!("12 Mpx JPEG ({} bytes): {took:?}", jpeg.len());
    assert_eq!(objects, 1);
    assert!(took < Duration::from_millis(2300), "{took:?}");
}

/// The 100 KB Inkscape-like file of criterion 71 with 20 texts: under 150 ms.
#[test]
#[ignore = "budget: release build"]
fn ac108_small_file_with_texts() {
    let fonts = Fonts::included();
    let mut svg = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd" width="210mm" height="297mm" viewBox="0 0 210 297">"#,
    );
    while svg.len() < 95_000 {
        let i = svg.len() % 200;
        write!(svg, r#"<path style="fill:none;stroke:#000;stroke-width:0.26" d="m {i},10 c 5,-5 10,5 15,0 5,-5 10,5 15,0 z"/>"#).unwrap();
    }
    for i in 0..20 {
        write!(
            svg,
            r#"<text xml:space="preserve" style="font-size:7.05556px;line-height:1.25;font-family:'Source Sans 3'" x="20" y="{0}"><tspan sodipodi:role="line" x="20" y="{0}">Label number {i} on the page</tspan></text>"#,
            20 + i * 12
        )
        .unwrap();
    }
    svg.push_str("</svg>");
    let (took, _) = open(&fonts, svg.as_bytes());
    println!("100 KB with 20 texts: {took:?}");
    assert!(took < Duration::from_millis(150), "{took:?}");
}

/// Base64 for the test, so it needs no crate.
mod base64_free {
    const DIGITS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() / 3 * 4 + 4);
        for chunk in bytes.chunks(3) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0_u32, |acc, (k, b)| acc | u32::from(*b) << (16 - 8 * k));
            for k in 0..4 {
                if k <= chunk.len() {
                    out.push(char::from(DIGITS[(n >> (18 - 6 * k) & 63) as usize]));
                } else {
                    out.push('=');
                }
            }
        }
        out
    }
}

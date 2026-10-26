//! Tester budgets of `specs/0024-svg-import-export` criterion 71 at the spec's own numbers
//! (2.0 s, 2.5 s, 100 ms), timed by the thread's CPU time (`CLAUDE.md` §5), from bytes to a
//! finished document through the editor session (Open and Import SVG…). Native release build:
//! `cargo test --release -p curvyo-editor-wasm --test acceptance_0024_budget_tester -- --ignored --nocapture`.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss, missing_docs)]

mod cpu_time;

use std::fmt::Write;
use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{AnchorId, Document, DrawingNode};
use curvyo_editor_wasm::Session;
use curvyo_ui_core::{SvgPlacement, import_svg_bytes};

/// `paths` paths of 20 nodes (one M and 19 cubic segments) in the compact form Inkscape writes
/// (relative `c`, 3 decimals), in groups of `per_group` when given: 400,000 nodes, about 12 MB.
fn generated(paths: usize, per_group: Option<usize>) -> String {
    let mut text = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000mm" height="1000mm" viewBox="0 0 1000 1000">"#,
    );
    for i in 0..paths {
        if per_group.is_some_and(|n| i % n == 0) {
            text.push_str("<g>");
        }
        let (x, y) = ((i % 200) as f64 * 5.0, (i / 200) as f64 * 5.0);
        write!(
            text,
            r#"<path style="fill:none;stroke:#000;stroke-width:0.1" d="m {x:.3},{y:.3} c"#
        )
        .unwrap();
        for k in 1..20 {
            let t = f64::from(k) * 0.013;
            write!(
                text,
                " {:.3},{:.3} {:.3},{:.3} {:.3},{:.3}",
                0.1 + t,
                0.1,
                0.2,
                0.3 - t,
                0.2,
                0.2
            )
            .unwrap();
        }
        text.push_str(r#""/>"#);
        if per_group.is_some_and(|n| i % n == n - 1) {
            text.push_str("</g>");
        }
    }
    text.push_str("</svg>");
    text
}

/// CPU times of Open (bytes to the session's document), and of its parts: read and map, then
/// the write to Loro.
fn measure(text: &str) -> (Duration, Duration, Duration) {
    let bytes = text.as_bytes();
    let (session, open) = cpu_time_of(|| Session::open_file(3, bytes, "big.svg").unwrap());
    drop(session);
    let (import, read_map) = cpu_time_of(|| import_svg_bytes(bytes, SvgPlacement::File).unwrap());
    let ids: Vec<AnchorId> = (0..DrawingNode::anchor_count(&import.nodes) as u64)
        .map(|n| AnchorId::new(7, n))
        .collect();
    let (_, write) =
        cpu_time_of(|| Document::from_drawing(7, &import.page, &import.nodes, &ids).unwrap());
    println!(
        "{:.1} MB, {} objects: open {open:?} = read+map {read_map:?} + write {write:?}",
        text.len() as f64 / 1e6,
        import.leaves
    );
    (open, read_map, write)
}

#[test]
#[ignore = "0024 FAIL: 2.56 s CPU for Open, 2.46 s for Import SVG… of 20,000 paths (spec 2.0 s; Loro write 1.93 s)"]
fn ac71_twenty_thousand_paths_under_two_seconds_cpu() {
    let text = generated(20_000, None);
    let (open, _, _) = measure(&text);
    let mut s = Session::new(4);
    let (_, import) = cpu_time_of(|| s.import_svg(text.as_bytes(), "big.svg").unwrap());
    println!("Import SVG… {import:?}");
    assert!(open < Duration::from_millis(2000), "open {open:?}");
    assert!(import < Duration::from_millis(2000), "import {import:?}");
}

#[test]
#[ignore = "release-build budget (measured 2.50 s CPU on 2026-10-26, at the limit)"]
fn ac71_five_thousand_groups_of_four_under_two_and_a_half_seconds_cpu() {
    let text = generated(20_000, Some(4));
    let (open, _, _) = measure(&text);
    assert!(open < Duration::from_millis(2500), "open {open:?}");
}

/// A real Inkscape 1.4.4 file tiled to about 100 KB (its Cut layer repeated in translated
/// groups, clones, stars and the combined paths included).
#[test]
#[ignore = "release-build budget"]
fn ac71_a_hundred_kilobyte_inkscape_file_under_100_ms_cpu() {
    let real = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-svg-core/tests/fixtures/svg_real/inkscape/inkscape_mm_a4.svg"),
    )
    .unwrap();
    let start = real.find("<g\n     inkscape:groupmode=\"layer\"").unwrap();
    let end = real.rfind("</svg>").unwrap();
    let layers = &real[start..end];
    let mut text = real[..start].to_owned();
    let mut i = 0;
    while text.len() < 100_000 {
        // Ids must stay unique for the clones to resolve: suffix them per tile.
        let tile = layers
            .replace("id=\"", &format!("id=\"t{i}_"))
            .replace("href=\"#", &format!("href=\"#t{i}_"));
        write!(text, r#"<g transform="translate({} 0)">{tile}</g>"#, i * 5).unwrap();
        i += 1;
    }
    text.push_str("</svg>");
    let (open, _, _) = measure(&text);
    assert!(open < Duration::from_millis(100), "open {open:?}");
}

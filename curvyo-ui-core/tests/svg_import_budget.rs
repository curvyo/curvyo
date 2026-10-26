//! The budgets of the SVG import (`specs/0024-svg-import-export` criterion 71), from bytes to a
//! finished document. The first two are restated from the measurement (`adrs.md`, risk "Budget 71
//! is unproven", dated note 2026-10-26): Loro alone takes about 2.0 s for the two million anchor
//! registers, so 2.0 s and 2.5 s become 3.0 s and 3.5 s until the PO decides. Native release
//! builds only:
//! `cargo test --release -p curvyo-ui-core --test svg_import_budget -- --ignored`.
#![allow(clippy::unwrap_used)]

use std::fmt::Write;
use std::time::{Duration, Instant};

use curvyo_document_core::{AnchorId, Document, DrawingNode};
use curvyo_ui_core::{SvgPlacement, svg_import};

/// `paths` paths of 20 nodes each (cubic segments), in groups of `per_group` when given.
fn generated(paths: usize, per_group: Option<usize>) -> String {
    let mut text = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000mm" height="1000mm" viewBox="0 0 1000 1000">"#,
    );
    for i in 0..paths {
        if per_group.is_some_and(|n| i % n == 0) {
            text.push_str("<g>");
        }
        #[allow(clippy::cast_precision_loss)]
        let (x, y) = ((i % 200) as f64 * 5.0, (i / 200) as f64 * 5.0);
        write!(
            text,
            r#"<path fill="none" stroke="black" stroke-width="0.1" d="M{x:.1},{y:.1}"#
        )
        .unwrap();
        for k in 1..20 {
            let t = f64::from(k) * 0.2;
            write!(
                text,
                " C{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                x + t,
                y + 0.1,
                x + t + 0.1,
                y + 0.3,
                x + t + 0.2,
                y + 0.2
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

/// Bytes to a finished document; returns the time.
fn import(bytes: &[u8]) -> Duration {
    let start = Instant::now();
    let drawing = curvyo_svg_core::read_svg(bytes).unwrap();
    let read = start.elapsed();
    let import = svg_import(&drawing, SvgPlacement::File);
    let mapped = start.elapsed();
    let ids: Vec<AnchorId> = (0..DrawingNode::anchor_count(&import.nodes) as u64)
        .map(|n| AnchorId::new(7, n))
        .collect();
    let document = Document::from_drawing(7, &import.page, &import.nodes, &ids).unwrap();
    let elapsed = start.elapsed();
    println!("read {read:?}, mapped {mapped:?}, written {elapsed:?}");
    assert_ne!(document.children(None), []);
    elapsed
}

#[test]
#[ignore = "release-build budget, run by hand or in the performance job"]
fn ac71_twenty_thousand_paths_in_three_seconds() {
    let text = generated(20_000, None);
    let elapsed = import(text.as_bytes());
    println!("20,000 paths, {} MB: {elapsed:?}", text.len() / 1_000_000);
    assert!(elapsed < Duration::from_millis(3000), "{elapsed:?}");
}

#[test]
#[ignore = "release-build budget, run by hand or in the performance job"]
fn ac71_five_thousand_groups_of_four_in_three_and_a_half_seconds() {
    let text = generated(20_000, Some(4));
    let elapsed = import(text.as_bytes());
    println!("5,000 groups of 4: {elapsed:?}");
    assert!(elapsed < Duration::from_millis(3500), "{elapsed:?}");
}

#[test]
#[ignore = "release-build budget, run by hand or in the performance job"]
fn ac71_a_hundred_kilobyte_file_in_a_tenth_of_a_second() {
    let text = generated(160, Some(8));
    let elapsed = import(text.as_bytes());
    println!("{} KB: {elapsed:?}", text.len() / 1000);
    assert!(elapsed < Duration::from_millis(100), "{elapsed:?}");
}

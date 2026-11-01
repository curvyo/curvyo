//! The budget of the SVG export (`specs/0024-svg-import-export` criteria 71 and 154, `adrs.md`
//! export §6): criterion 71's document of 20,000 paths exported by the session, timed by the
//! thread's CPU time (`CLAUDE.md` §5), under 1.5 s and 25 MB. Asserted only in a release build
//! (the performance job runs it); a debug build prints the numbers:
//! `cargo test --release -p curvyo-editor-wasm --test svg_export_budget -- --nocapture`.
#![allow(clippy::unwrap_used, missing_docs)]

mod cpu_time;

use std::fmt::Write;
use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_editor_wasm::{Session, SvgExportResult};
use curvyo_ui_core::SvgExportOptions;

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

/// The CPU time and size of one export of `session` under `options`.
fn export(session: &Session, options: SvgExportOptions) -> (Duration, usize) {
    let (result, took) = cpu_time_of(|| session.export_svg(options));
    let SvgExportResult::Written { text, objects, .. } = result else {
        panic!("written")
    };
    assert!(objects > 0);
    (took, text.len())
}

#[test]
fn ac71_twenty_thousand_paths_export_in_a_second_and_a_half() {
    let text = generated(20_000, None);
    let session = Session::open_file(7, text.as_bytes(), "big.svg").unwrap();
    let (took, bytes) = export(&session, SvgExportOptions::default());
    println!("export of 20,000 paths: {took:?}, {} MB", bytes / 1_000_000);
    if !cfg!(debug_assertions) {
        assert!(took < Duration::from_millis(1500), "{took:?}");
        assert!(bytes < 25_000_000, "{bytes} bytes");
    }
}

#[test]
#[ignore = "reported, not gated (criterion 154): run by hand in a release build"]
fn ac154_twenty_thousand_paths_with_strokes_as_paths() {
    let text = generated(20_000, None);
    let session = Session::open_file(7, text.as_bytes(), "big.svg").unwrap();
    let options = SvgExportOptions {
        strokes_as_paths: true,
        ..SvgExportOptions::default()
    };
    let (took, bytes) = export(&session, options);
    println!(
        "export of 20,000 paths, strokes as paths: {took:?}, {} MB",
        bytes / 1_000_000
    );
}

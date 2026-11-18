//! Tester's own release budgets of `specs/0124-gradient-fill` criterion 24, on different
//! inputs than the implementer's `gradient_budget.rs`: Radial gradients on irregular paths (not
//! squares), a drag of the Radial centre (the whole circle moves), and an elliptical radial of
//! 256 stops. Thread CPU time (`cpu_time`); asserted in release only, a debug build prints.
//! Run: `cargo test --release -p curvyo-editor-wasm --test acceptance_0124_tester_budget --
//! --nocapture --test-threads=1`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    missing_docs
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{
    AnchorId, AxisRatio, BoxPoint, Color, Document, FillPaint, GradientGeometry, GradientStop,
    GradientStops, NewAnchor, NodeId, Point, StopOffset, StyleEdit, pack,
};
use curvyo_editor_wasm::{Session, Tool};

const FRAME: Duration = Duration::from_millis(16);
const EXTRA: Duration = Duration::from_millis(2);

fn check(label: &str, took: Duration, budget: Duration) {
    let margin = budget.as_secs_f64() / took.as_secs_f64().max(1e-9);
    println!("{label}: {took:?} (budget {budget:?}, margin x{margin:.2})");
    if !cfg!(debug_assertions) {
        assert!(took <= budget, "{label}: {took:?} is over {budget:?}");
    }
}

fn stops(n: u32) -> GradientStops {
    GradientStops::new(
        (0..n)
            .map(|i| GradientStop {
                colour: Color {
                    r: (i * 53 % 256) as u8,
                    g: 40,
                    b: (i * 17 % 256) as u8,
                },
                alpha: (128 + i * 7 % 128) as u8,
                offset: StopOffset::new(f64::from(i) / f64::from(n - 1)).unwrap(),
            })
            .collect(),
    )
    .unwrap()
}

/// `count` irregular pentagons, Colour fill on; with a radial gradient of `n` stops if given.
fn pentagons(count: usize, n: Option<u32>, elliptic: bool) -> (Document, Vec<NodeId>) {
    let d = Document::new(1);
    let ids: Vec<NodeId> = (0..count)
        .map(|k| {
            let (x, y) = ((k % 50) as f64 * 20.0, (k / 50) as f64 * 20.0);
            let pts = [
                (0.0, 0.0),
                (12.0, 1.0),
                (15.0, 9.0),
                (7.0, 14.0),
                (1.0, 8.0),
            ];
            let a: Vec<NewAnchor> = pts
                .iter()
                .enumerate()
                .map(|(i, (px, py))| {
                    NewAnchor::corner(
                        AnchorId::new(1, (k * 5 + i) as u64 + 1),
                        Point::new(x + px, y + py),
                    )
                })
                .collect();
            d.create_path(&a, true)
        })
        .collect();
    d.edit_style(&ids, &StyleEdit::FillEnabled(true)).unwrap();
    if let Some(n) = n {
        d.edit_style(&ids, &StyleEdit::FillPaint(FillPaint::Gradient))
            .unwrap();
        let g = GradientGeometry::Radial {
            centre: BoxPoint::new(0.45, 0.5).unwrap(),
            radius: BoxPoint::new(0.95, 0.3).unwrap(),
            ratio: if elliptic {
                AxisRatio::new(0.4).unwrap()
            } else {
                AxisRatio::CIRCLE
            },
        };
        d.edit_style(&ids, &StyleEdit::GradientGeometry(g)).unwrap();
        d.edit_style(&ids, &StyleEdit::GradientStops(stops(n)))
            .unwrap();
    }
    (d, ids)
}

fn session(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "t").unwrap()).unwrap();
    s.resize_viewport(1600.0, 900.0);
    s.set_tool(Tool::Select);
    s
}

fn frame(s: &Session) -> usize {
    let list = s.frame_draw_list();
    list.gradient_attributes().len() + list.stop_texture().texels.len()
}

fn median(s: &Session) -> Duration {
    let _ = frame(s);
    let mut t: Vec<Duration> = (0..21).map(|_| cpu_time_of(|| frame(s)).1).collect();
    t.sort();
    t[10]
}

#[test]
fn t24a_radial_centre_drag_among_a_thousand_radial_paths() {
    let (d, ids) = pentagons(1_000, Some(4), false);
    let mut s = session(&d);
    let p = Point::new(6.0, 0.5);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    assert_eq!(s.selected_object_count(), 1);
    let h = s.gradient_handles();
    assert!(h.shown);
    let c = s.screen_to_document(h.handles[0].x, h.handles[0].y);
    let before = format!("{:?}", d.object(ids[0]).unwrap().style().fill.gradient);
    s.pointer_hover(c, false, false);
    s.pointer_down(c, false);
    let mut worst = Duration::ZERO;
    for k in 1..=30 {
        let at = Point::new(c.x + f64::from(k) * 0.2, c.y + f64::from(k) * 0.1);
        worst = worst.max(
            cpu_time_of(|| {
                s.pointer_hover(at, false, false);
                frame(&s)
            })
            .1,
        );
    }
    s.pointer_up(Point::new(c.x + 6.0, c.y + 3.0), false, false);
    let doc = curvyo_document_core::unpack(9, &s.pack("t").unwrap()).unwrap();
    let after = format!("{:?}", doc.object(ids[0]).unwrap().style().fill.gradient);
    assert_ne!(before, after, "the drag moved the gradient");
    check("tester 24a radial centre drag, worst of 30", worst, FRAME);
}

#[test]
fn t24b_a_thousand_radial_paths_against_colour() {
    let colour = median(&session(&pentagons(1_000, None, false).0));
    let radial = median(&session(&pentagons(1_000, Some(4), false).0));
    println!("tester 24b colour {colour:?} radial {radial:?}");
    check("tester 24b radial frame", radial, colour + EXTRA);
}

#[test]
fn t24e_elliptic_radial_of_256_stops_against_4() {
    let four = median(&session(&pentagons(1, Some(4), true).0));
    let wide = median(&session(&pentagons(1, Some(256), true).0));
    println!("tester 24e 4 {four:?} 256 {wide:?}");
    check("tester 24e 256-stop elliptic frame", wide, four + EXTRA);
}

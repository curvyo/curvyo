//! Independent tester budget for criterion 36 of `specs/0047-transform-polish/specification.md`:
//! a move drag at the 10,000-object scale of `0019` criterion 49 meets that criterion's frame
//! budget (one preview step under 50 ms; the draw is the artwork's own cost) with the pale red box drawn, and the box adds no second
//! pass over the objects (the step is not slower than a scale step, which draws no box, by more
//! than 25 %). Run in release:
//! `cargo test --release -p curvyo-editor-wasm --test acceptance_0047_budget_tester -- --ignored --nocapture`.

#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
// Mechanical lints of the tester files, allowed so the gate stays strict for production code.
#![allow(clippy::manual_midpoint, clippy::many_single_char_names)]

use std::time::{Duration, Instant};

use curvyo_document_core::{AnchorId, Document, Length, NewAnchor, Point, RectBounds, pack};
use curvyo_editor_wasm::{KeyInput, Session, Tool};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

const SIDE: f64 = 6.0;
const PITCH: f64 = 10.0;

fn scene(paths: usize, nodes: usize, rects: usize) -> Document {
    let d = Document::new(1);
    let cell = |i: usize| {
        (
            5.0 + (i % 100) as f64 * PITCH,
            5.0 + (i / 100) as f64 * PITCH,
        )
    };
    for i in 0..paths {
        let (cx, cy) = cell(i);
        let per = nodes / 4;
        let h = SIDE / 2.0;
        let corners = [(-h, -h), (h, -h), (h, h), (-h, h)];
        let mut anchors = Vec::new();
        for side in 0..4 {
            let (ax, ay) = corners[side];
            let (bx, by) = corners[(side + 1) % 4];
            for k in 0..per {
                let t = k as f64 / per as f64;
                anchors.push(NewAnchor::corner(
                    AnchorId::new(1, (i * nodes + anchors.len()) as u64 + 1),
                    pt(cx + ax + (bx - ax) * t, cy + ay + (by - ay) * t),
                ));
            }
        }
        let _ = d.create_path(&anchors, true);
    }
    for j in 0..rects {
        let (cx, cy) = cell(paths + j);
        let _ = d.create_rect(RectBounds {
            origin: pt(cx - SIDE / 2.0, cy - SIDE / 2.0),
            width: Length::from_mm(SIDE),
            height: Length::from_mm(SIDE),
        });
    }
    d
}

fn best<F: FnMut() -> Duration>(mut f: F, reps: usize) -> Duration {
    (0..reps).map(|_| f()).min().unwrap()
}

#[test]
#[ignore = "release-mode benchmark"]
fn the_red_box_keeps_the_ten_thousand_object_move_frame_within_budget() {
    let n = 10_000;
    let d = scene(5000, 20, 5000);
    let mut s = Session::open(2, &pack(&d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    let out = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    println!("select all: {out:?}");
    assert_eq!(s.selected_object_count(), n);
    let (lo, hi) = (pt(2.0, 2.0), pt(998.0, 998.0));
    let centre = pt((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    let k = s.view().scale();
    let step = |s: &mut Session, from: Point, to: Point, with_draw: bool| {
        s.pointer_hover(from, false, false);
        s.pointer_down(from, false);
        let t = Instant::now();
        s.pointer_hover(to, false, false);
        if with_draw {
            std::hint::black_box(s.draw_list());
        }
        let dt = t.elapsed();
        s.escape();
        s.pointer_up(to, false, false);
        dt
    };
    let move_from = centre;
    let move_to = pt(centre.x + 12.0, centre.y + 7.0);
    let scale_from = hi;
    let scale_to = pt(hi.x + 15.0, hi.y + 9.0);
    for _ in 0..2 {
        step(&mut s, move_from, move_to, true);
        step(&mut s, scale_from, scale_to, true);
    }
    let mv = best(|| step(&mut s, move_from, move_to, true), 7);
    let sc = best(|| step(&mut s, scale_from, scale_to, true), 7);
    let mv0 = best(|| step(&mut s, move_from, move_to, false), 7);
    let idle = best(
        || {
            let t = Instant::now();
            std::hint::black_box(s.draw_list());
            t.elapsed()
        },
        7,
    );
    // The box is really there during the drag: look at one frame with the drag in flight.
    s.pointer_hover(move_from, false, false);
    s.pointer_down(move_from, false);
    s.pointer_hover(move_to, false, false);
    let live = s.draw_list();
    let red = live
        .triangles
        .iter()
        .filter(|v| {
            v.color.r.abs_diff(0xE5) <= 3 && v.color.g.abs_diff(0x48) <= 3 && v.color.a < 255
        })
        .count();
    s.escape();
    s.pointer_up(move_to, false, false);
    println!("red box vertices in a live frame: {red}; idle draw {idle:?}");
    assert!(red > 0, "the box is drawn during the move");
    println!(
        "10,000 objects (k = {k:.2}): move step with draw {mv:?}, scale step with draw {sc:?}, move resolve only {mv0:?}"
    );
    // `0019` criterion 49 measures the preview step itself (the hover that resolves the gesture),
    // not the draw: the frame is the artwork's own cost in both cases.
    assert!(
        mv0 < Duration::from_millis(50),
        "move preview step {mv0:?} exceeds 50 ms"
    );
    assert!(
        mv.as_secs_f64() <= sc.as_secs_f64() * 1.1 + 0.002,
        "the red box costs more than a pass: move {mv:?} vs scale {sc:?}"
    );
}

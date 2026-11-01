//! 200 dimensions follow a drag at the frame rate of a 200-object move
//! (`specs/0070-measure-and-dimension` criterion 11): the frame of a Select drag of 200 measured
//! paths with their 200 dimensions, against the same drag without dimensions. In release builds
//! the frame with dimensions stays within one 60 Hz frame (16.7 ms) or 1.5 times the frame
//! without, whichever is larger (Proposal); a debug build prints the numbers. Every cost is the
//! thread CPU time of the work (`cpu_time`).
//! Run: `cargo test --release -p curvyo-editor-wasm --test dimension_frame_budget -- --nocapture`.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;

use curvyo_document_core::{
    AnchorId, DimensionKind, DimensionPlace, DimensionSlots, Document, Length, NewAnchor, NodeId,
    Point, PointSlot, pack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};

const COUNT: u64 = 200;
const FRAMES: usize = 10;

fn document(with_dimensions: bool) -> Document {
    let document = Document::new(1);
    let paths: Vec<NodeId> = (0..COUNT)
        .map(|k| {
            let (x, y) = ((k % 20) as f64 * 15.0 + 20.0, (k / 20) as f64 * 15.0 + 20.0);
            document.create_path(
                &[
                    NewAnchor::corner(AnchorId::new(1, 2 * k), Point::new(x, y)),
                    NewAnchor::corner(AnchorId::new(1, 2 * k + 1), Point::new(x + 10.0, y)),
                ],
                false,
            )
        })
        .collect();
    if with_dimensions {
        for (k, &path) in (0..COUNT).zip(&paths) {
            document
                .add_dimension(
                    DimensionKind::Aligned,
                    DimensionSlots::Points(vec![
                        PointSlot::Anchor {
                            node: path,
                            anchor: AnchorId::new(1, 2 * k),
                        },
                        PointSlot::Anchor {
                            node: path,
                            anchor: AnchorId::new(1, 2 * k + 1),
                        },
                    ]),
                    DimensionPlace::at_offset(Length::from_mm(3.0)),
                )
                .unwrap();
        }
    }
    document
}

/// The thread CPU time of `FRAMES` frames of a drag of every path, after the drag started.
fn drag_frames(document: &Document) -> Duration {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1600.0, 900.0);
    let _ = session.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    let start = Point::new(25.0, 20.0);
    session.pointer_down(start, false);
    session.pointer_hover(Point::new(30.0, 25.0), false, false);
    let _ = session.draw_list();
    let ((), took) = cpu_time_of(|| {
        for i in 0..FRAMES {
            let at = Point::new(30.0 + i as f64, 25.0 + i as f64);
            session.pointer_hover(at, false, false);
            let _ = session.draw_list();
        }
    });
    session.pointer_up(Point::new(40.0, 35.0), false, false);
    took / u32::try_from(FRAMES).unwrap()
}

#[test]
fn two_hundred_dimensions_follow_a_drag_at_the_frame_rate_of_the_move() {
    let without = drag_frames(&document(false));
    let with = drag_frames(&document(true));
    let budget = Duration::from_micros(16_700).max(without.mul_f64(1.5));
    println!(
        "frame of a 200-object drag: {without:?}; with 200 dimensions: {with:?} (budget {budget:?})"
    );
    if !cfg!(debug_assertions) {
        assert!(with <= budget, "{with:?} is over {budget:?}");
    }
}

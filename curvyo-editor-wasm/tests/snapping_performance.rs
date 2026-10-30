//! Incremental indexed snapping CPU cost on a 10,000-object scene.
mod cpu_time;
use curvyo_document_core::{Document, DocumentSnapping, Point, RectBounds};
use curvyo_ui_core::{SnapExclusions, SnapProbe, SnapSettings, SnapState, SnapTargets, resolve};
use std::hint::black_box;

#[test]
fn ten_thousand_objects_add_less_than_two_ms_per_frame() {
    let d = Document::new(3);
    d.batch("scene", |doc| {
        for index in 0..10_000 {
            let x = f64::from(index % 100) * 40.0;
            let y = f64::from(index / 100) * 40.0;
            let _ = doc.create_rect(RectBounds::from_corners(
                Point::new(x, y),
                Point::new(x + 20.0, y + 20.0),
            ));
        }
    });
    let settings = SnapSettings::from_document(
        DocumentSnapping {
            enabled: true,
            ..DocumentSnapping::DEFAULT
        },
        1.0,
    );
    let nodes = d.read_nodes();
    let (targets, build) = cpu_time::cpu_time_of(|| {
        SnapTargets::gather(&nodes, d.size(), &settings, SnapExclusions::default())
    });
    let frames = 1000;
    let ((), baseline) = cpu_time::cpu_time_of(|| {
        for frame in 0..frames {
            black_box(SnapProbe::box_edges(
                Point::new(f64::from(frame) * 3.7, 80.0),
                Point::new(f64::from(frame) * 3.7 + 20.0, 100.0),
            ));
        }
    });
    let ((), snapped) = cpu_time::cpu_time_of(|| {
        let mut state = SnapState::default();
        for frame in 0..frames {
            let probe = SnapProbe::box_edges(
                Point::new(f64::from(frame) * 3.7, 80.0),
                Point::new(f64::from(frame) * 3.7 + 20.0, 100.0),
            );
            black_box(resolve(&settings, &targets, &probe, &mut state));
        }
    });
    let incremental = snapped.saturating_sub(baseline).as_secs_f64() * 1000.0 / f64::from(frames);
    eprintln!(
        "10k objects: index {:.2}ms, incremental {:.4}ms/frame",
        build.as_secs_f64() * 1000.0,
        incremental
    );
    assert!(incremental <= 2.0, "{incremental}ms/frame exceeds2ms");
}

#[test]
fn dense_curved_guides_stay_within_incremental_frame_budget() {
    use curvyo_document_core::Vec2;
    use curvyo_ui_core::{SnapCurve, SnapTarget, TargetKind};
    let entries = (0..10_000)
        .map(|index| {
            let x = f64::from(index % 100) * 8.0;
            let y = f64::from(index / 100) * 8.0;
            let mut target = SnapTarget::point(
                Point::new(x, y),
                (Point::new(x, y - 3.0), Point::new(x + 6.0, y + 3.0)),
                TargetKind::Guide,
            );
            target.curve = Some(SnapCurve {
                start: Point::new(x, y),
                end: Point::new(x + 6.0, y),
                handle_out: Vec2::new(2.0, 3.0),
                handle_in: Vec2::new(-2.0, -3.0),
            });
            target
        })
        .collect();
    let targets = SnapTargets::new(entries);
    let settings = SnapSettings::from_document(
        DocumentSnapping {
            enabled: true,
            ..DocumentSnapping::DEFAULT
        },
        1.0,
    );
    let ((), elapsed) = cpu_time::cpu_time_of(|| {
        let mut state = SnapState::default();
        for frame in 0..1000 {
            let probe = SnapProbe::point(Point::new(f64::from(frame) * 0.7, 47.0));
            black_box(resolve(&settings, &targets, &probe, &mut state));
        }
    });
    let per_frame = elapsed.as_secs_f64(); // 1000 frames: seconds total equals milliseconds/frame.
    eprintln!("10k dense curved guides: {per_frame:.4}ms/frame");
    assert!(per_frame <= 2.0, "{per_frame}ms/frame exceeds 2ms");
}

#[test]
#[allow(clippy::unwrap_used)]
fn drawing_hover_reuses_the_index_across_ten_thousand_objects() {
    use curvyo_editor_wasm::{Session, Tool};
    let d = Document::new(28);
    d.batch("scene", |doc| {
        for index in 0..10_000 {
            let x = f64::from(index % 100) * 40.0;
            let y = f64::from(index / 100) * 40.0;
            let _ = doc.create_rect(RectBounds::from_corners(
                Point::new(x, y),
                Point::new(x + 20.0, y + 20.0),
            ));
        }
    });
    let mut s = Session::open(29, &curvyo_document_core::pack(&d, "perf").unwrap()).unwrap();
    s.set_tool(Tool::Rectangle);
    let measure = |session: &mut Session| {
        // Build the context once before measuring incremental pointer frames.
        session.pointer_hover(Point::new(0.0, 80.0), false, false);
        cpu_time::cpu_time_of(|| {
            for frame in 0..1000 {
                session.pointer_hover(Point::new(f64::from(frame) * 3.7, 80.0), false, false);
            }
        })
        .1
    };
    let baseline = measure(&mut s);
    s.set_snapping(DocumentSnapping {
        enabled: true,
        ..DocumentSnapping::DEFAULT
    });
    let snapped = measure(&mut s);
    let incremental = snapped.saturating_sub(baseline).as_secs_f64();
    eprintln!("10k drawing hover incremental {incremental:.4}ms/frame");
    assert!(incremental <= 2.0, "{incremental}ms/frame exceeds 2ms");
}

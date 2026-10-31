//! Tester cases for `specs/0089-trace-quality-and-colour` slice M at the session (criterion 29):
//! one group with one filled compound path per colour named by its hex, the largest at the
//! bottom, one undo step, a recolour that never refits, Isolate that never changes what is
//! written. Written from the specification; the results come from `trace_colour_job_with` over
//! the pixels the session itself decoded.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]

use curvyo_document_core::{Length, NodeId, ObjectSnapshot, PathSnapshot, unpack};
use curvyo_editor_wasm::{Session, Tool, TraceCreateOutcome};
use curvyo_ui_core::{
    TraceColourParams, TraceMethod, TraceParams, TraceRgb, read_trace_wire, trace_colour_job_with,
    trace_job,
};
use image::ImageEncoder;

const RED: TraceRgb = TraceRgb::new(200, 30, 30);
const GREEN: TraceRgb = TraceRgb::new(30, 160, 60);
const BLUE: TraceRgb = TraceRgb::new(30, 60, 200);

/// 120 x 120: red 60 x 100 left, green 40 x 50 top right, blue 40 x 50 below it, on white. The
/// green and blue areas tie.
fn png() -> Vec<u8> {
    let mut rgba = Vec::new();
    for y in 0..120_u32 {
        for x in 0..120_u32 {
            let c = if !(10..110).contains(&x) || !(10..110).contains(&y) {
                [255, 255, 255]
            } else if x < 70 {
                [RED.r, RED.g, RED.b]
            } else if y < 60 {
                [GREEN.r, GREEN.g, GREEN.b]
            } else {
                [BLUE.r, BLUE.g, BLUE.b]
            };
            rgba.extend([c[0], c[1], c[2], 255]);
        }
    }
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&rgba, 120, 120, image::ExtendedColorType::Rgba8)
        .unwrap();
    out
}

fn traced() -> (Session, NodeId, Vec<f64>) {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session.import_begin(None);
    session.import_add(&png(), "logo.png");
    session.import_finish(false);
    session.set_tool(Tool::Trace);
    let image = session.trace_view().shown.expect("the image is shown");
    let source = session.trace_source(image, false).unwrap();
    let params = TraceColourParams::new(TraceParams {
        selection: TraceMethod::RemoveBackground {
            background: Some(TraceRgb::new(255, 255, 255)),
            tolerance: 40,
        },
        specks: Length::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    });
    let wire = trace_colour_job_with(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        &params,
        &mut |_| {},
    );
    assert!(session.set_trace_preview(image, &wire));
    assert!(session.set_trace_result(image, &wire));
    (session, image, wire)
}

fn doc(session: &mut Session) -> curvyo_document_core::Document {
    unpack(9, &session.pack("t").unwrap()).unwrap()
}

/// The paths of the one group, bottom first, with their names.
fn group_paths(session: &mut Session) -> (Option<String>, Vec<(Option<String>, PathSnapshot)>) {
    let d = doc(session);
    let mut groups: Vec<NodeId> = d
        .object_ids()
        .into_iter()
        .filter_map(|id| d.parent_group(id))
        .collect();
    groups.dedup();
    assert!(groups.len() <= 1, "more than one group: {groups:?}");
    let Some(&group) = groups.first() else {
        return (None, Vec::new());
    };
    let nodes = d.read_nodes();
    let name = |id: NodeId| {
        nodes
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.registers.name.clone())
    };
    let paths = d
        .children(Some(group))
        .into_iter()
        .map(|id| {
            let Some(ObjectSnapshot::Path(p)) = d.object(id) else {
                panic!("a child of the trace group is a path");
            };
            (name(id), p)
        })
        .collect();
    (name(group), paths)
}

fn hex(c: TraceRgb) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

fn fill(p: &PathSnapshot) -> TraceRgb {
    let c = p.style.fill.color;
    TraceRgb::new(c.r, c.g, c.b)
}

/// The anchors of all outlines as numbers: points and handles.
fn geometry(p: &PathSnapshot) -> Vec<f64> {
    let mut out = Vec::new();
    let all = std::iter::once(&p.anchors).chain(p.extra_subpaths.iter().map(|s| &s.anchors));
    for anchors in all {
        out.push(anchors.len() as f64);
        for a in anchors {
            out.extend([
                a.point.x,
                a.point.y,
                a.handle_in.x,
                a.handle_in.y,
                a.handle_out.x,
                a.handle_out.y,
            ]);
        }
    }
    out
}

#[test]
fn c29_one_group_one_filled_path_per_colour_named_by_hex_largest_at_the_bottom() {
    let (mut session, _image, wire) = traced();
    let layers = read_trace_wire(&wire).unwrap().layers;
    assert_eq!(layers.len(), 3);
    let outcome = session.create_traced_paths();
    assert!(
        matches!(
            outcome,
            TraceCreateOutcome::Created {
                images: 1,
                paths: 3,
                colours: 3,
                ..
            }
        ),
        "{outcome:?}"
    );
    let (group, paths) = group_paths(&mut session);
    assert_eq!(group.as_deref(), Some("Trace of logo.png"));
    assert_eq!(paths.len(), 3, "one compound path per colour");
    for (name, p) in &paths {
        assert!(p.style.fill.enabled, "filled");
        assert!(!p.style.stroke.enabled, "stroke off");
        assert_eq!(
            name.as_deref(),
            Some(hex(fill(p)).as_str()),
            "named by its hex"
        );
    }
    let fills: Vec<TraceRgb> = paths.iter().map(|(_, p)| fill(p)).collect();
    assert_eq!(
        fills[0], RED,
        "the largest area is at the bottom: {fills:?}"
    );
    let mut rest = fills[1..].to_vec();
    rest.sort_by_key(|c| (c.r, c.g, c.b));
    assert_eq!(
        fills[1..],
        rest[..],
        "a tie is ordered by colour value: {fills:?}"
    );
    // The write order follows the layers of the result.
    let layer_fills: Vec<TraceRgb> = layers.iter().map(|l| l.colour).collect();
    assert_eq!(fills, layer_fills);
    // Every hole stays a hole of the same compound path: the region counts match the layers.
    for ((_, p), l) in paths.iter().zip(&layers) {
        assert_eq!(1 + p.extra_subpaths.len(), l.outlines);
    }
}

#[test]
fn c29_create_paths_is_one_undo_step_and_redo_brings_it_back() {
    let (mut session, image, _) = traced();
    let before = doc(&mut session).object_ids();
    assert_eq!(before, [image]);
    let steps = session.undo_len();
    assert!(matches!(
        session.create_traced_paths(),
        TraceCreateOutcome::Created { .. }
    ));
    assert_eq!(
        session.undo_len(),
        steps + 1,
        "one step for the group and its paths"
    );
    let (_, written) = group_paths(&mut session);
    session.undo();
    assert_eq!(
        doc(&mut session).object_ids(),
        [image],
        "one Ctrl+Z reverses all of it"
    );
    assert_eq!(group_paths(&mut session).1.len(), 0);
    session.redo();
    let (_, again) = group_paths(&mut session);
    assert_eq!(again.len(), 3);
    assert_eq!(
        again.iter().map(|(_, p)| geometry(p)).collect::<Vec<_>>(),
        written.iter().map(|(_, p)| geometry(p)).collect::<Vec<_>>()
    );
}

#[test]
fn c29_recolour_changes_fills_and_never_the_outlines() {
    let (mut plain, _, _) = traced();
    plain.create_traced_paths();
    let (_, base) = group_paths(&mut plain);

    let (mut session, _, _) = traced();
    let orange = TraceRgb::new(250, 120, 0);
    session.set_trace_palette_view(vec![(RED, orange), (BLUE, GREEN)], None);
    session.create_traced_paths();
    let (_, recoloured) = group_paths(&mut session);
    assert_eq!(recoloured.len(), base.len());
    for ((_, a), (_, b)) in base.iter().zip(&recoloured) {
        for (x, y) in geometry(a).iter().zip(geometry(b)) {
            assert!((x - y).abs() < 1e-9, "outline changed: {x} vs {y}");
        }
        assert_eq!(geometry(a).len(), geometry(b).len());
    }
    let fills: Vec<TraceRgb> = recoloured.iter().map(|(_, p)| fill(p)).collect();
    let want: Vec<TraceRgb> = base
        .iter()
        .map(|(_, p)| match fill(p) {
            c if c == RED => orange,
            c if c == BLUE => GREEN,
            c => c,
        })
        .collect();
    assert_eq!(fills, want);
    // The order and the number of paths do not follow the new colours.
    assert_eq!(fills[0], orange);
}

#[test]
fn c29_resetting_the_recolour_writes_the_original_fills() {
    let (mut session, _, _) = traced();
    session.set_trace_palette_view(vec![(RED, GREEN)], None);
    session.set_trace_palette_view(Vec::new(), None);
    session.create_traced_paths();
    let (_, paths) = group_paths(&mut session);
    assert!(paths.iter().any(|(_, p)| fill(p) == RED));
}

#[test]
fn c29_isolate_is_view_only_create_paths_still_writes_every_colour_opaque() {
    let (mut session, _, _) = traced();
    session.set_trace_palette_view(Vec::new(), Some(BLUE));
    session.create_traced_paths();
    let (_, paths) = group_paths(&mut session);
    assert_eq!(paths.len(), 3, "Enter still creates every colour");
    let d = doc(&mut session);
    for n in d.read_nodes() {
        if paths.iter().any(|(_, p)| p.id == n.id) {
            assert!(
                (n.opacity.get() - 1.0).abs() < 1e-9,
                "dimmed on write: {}",
                n.opacity.get()
            );
        }
    }
    for (_, p) in &paths {
        assert!((p.style.fill.opacity.get() - 1.0).abs() < 1e-9);
    }
}

#[test]
fn c29_a_single_colour_result_still_writes_one_black_path_without_a_colour_group_name() {
    let (mut session, image, _) = traced();
    let source = session.trace_source(image, false).unwrap();
    let wire = trace_job(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        128,
        80,
        0.0,
    );
    assert!(session.set_trace_preview(image, &wire));
    assert!(session.set_trace_result(image, &wire));
    let outcome = session.create_traced_paths();
    let TraceCreateOutcome::Created { colours, .. } = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(colours, 0);
    assert_eq!(session.trace_palette_json(), "");
}

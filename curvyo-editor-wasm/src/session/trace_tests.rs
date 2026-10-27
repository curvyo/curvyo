//! The Trace tool's session side (`specs/0025-raster-trace` criteria 1 to 3, 7, 12, 13 and 46).

use curvyo_document_core::{NodeId, ObjectSnapshot, Point};
use curvyo_ui_core::trace_job;

use super::{
    DoubleClickHint, EscapeStep, KeyInput, KeyOutcome, Session, Tool, TraceCreateOutcome,
    TraceSelection,
};

const PNG: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");

fn import_images(session: &mut Session, count: usize) -> Vec<NodeId> {
    session.import_begin(None);
    for i in 0..count {
        session.import_add(PNG, &format!("logo-{i}.png"));
    }
    session.import_finish(false);
    session
        .objects()
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.id),
            _ => None,
        })
        .collect()
}

fn key(session: &mut Session, k: &str, ctrl: bool) -> KeyOutcome {
    session.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    })
}

/// A full-size result: a black square in the middle of a 16 x 16 px picture over `map`.
fn wire(map: [f64; 6]) -> Vec<f64> {
    let rgba: Vec<u8> = (0..16 * 16)
        .flat_map(|i| {
            let (x, y) = (i % 16, i / 16);
            let v = if (4..12).contains(&x) && (4..12).contains(&y) {
                0
            } else {
                255
            };
            [v, v, v, 255]
        })
        .collect();
    trace_job(&rgba, 16, 16, map, 128, 80, 0.0)
}

fn give_results(session: &mut Session, images: &[NodeId]) {
    for &image in images {
        let source = session.trace_source(image, false).unwrap();
        assert!(session.set_trace_result(image, &wire(source.map)));
    }
}

/// Criterion 1: I chooses the tool, Ctrl+I does not; Escape and S leave it, the selection stays.
#[test]
fn the_tool_is_key_i_and_escape_or_s_leave_it() {
    let mut session = Session::new(1);
    let images = import_images(&mut session, 1);
    assert_eq!(key(&mut session, "i", true), KeyOutcome::Ignored);
    assert_eq!(session.tool(), Tool::Select);
    assert_eq!(key(&mut session, "i", false), KeyOutcome::ToolChanged);
    assert_eq!(session.tool(), Tool::Trace);
    assert_eq!(session.escape(), EscapeStep::LeftTool);
    assert_eq!(
        (session.tool(), session.selection.ids()),
        (Tool::Select, &images[..])
    );
    key(&mut session, "I", false);
    key(&mut session, "s", false);
    assert_eq!(session.tool(), Tool::Select);
}

/// Criteria 1, 3 and 46: the button shows for images only; the bar's three states.
#[test]
fn the_selection_states_and_the_select_bar_button() {
    let mut session = Session::new(1);
    let images = import_images(&mut session, 3);
    assert!(session.trace_available());
    session.set_tool(Tool::Trace);
    assert!(
        !session.trace_available(),
        "the button is in the Select bar only"
    );
    let view = session.trace_view();
    assert_eq!(view.selection, TraceSelection::Images);
    assert_eq!(view.images.len(), 3);
    assert_eq!(view.shown, images.last().copied());
    session.selection.clear();
    assert_eq!(session.trace_view().selection, TraceSelection::Empty);
    let rect = session
        .document
        .create_rect(curvyo_document_core::RectBounds {
            origin: Point::new(500.0, 500.0),
            width: curvyo_document_core::Length::from_mm(5.0),
            height: curvyo_document_core::Length::from_mm(5.0),
        });
    session.selection.set(&[images[0], rect]);
    assert_eq!(session.trace_view().selection, TraceSelection::Other);
    session.set_tool(Tool::Select);
    assert!(!session.trace_available());
}

/// Criterion 3: a press selects the image under it and makes it the shown one.
#[test]
fn a_press_selects_the_image_and_shows_it() {
    let mut session = Session::new(1);
    let images = import_images(&mut session, 1);
    session.set_tool(Tool::Trace);
    session.selection.clear();
    let corners = session
        .objects()
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.corners()),
            _ => None,
        })
        .unwrap();
    let middle = Point::new(
        f64::midpoint(corners[0].x, corners[2].x),
        f64::midpoint(corners[0].y, corners[2].y),
    );
    session.pointer_down(middle, false);
    session.pointer_up(middle, false, false);
    assert_eq!(session.selection.ids(), &images[..]);
    assert_eq!(session.trace_view().shown, Some(images[0]));
    assert_eq!(
        session.double_click_hint(middle, false, false),
        DoubleClickHint::CreateTrace
    );
}

/// Criteria 2 and 10: the pixels and the pixel map for the worker; the reduced copy keeps the map.
#[test]
fn the_worker_gets_the_pixels_and_the_map() {
    let mut session = Session::new(1);
    let images = import_images(&mut session, 1);
    session.set_tool(Tool::Trace);
    let source = session.trace_source(images[0], false).unwrap();
    assert_eq!((source.width, source.height, source.rgba.len()), (3, 1, 12));
    let image = session
        .objects()
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(source.map, image.pixel_map(3, 1).to_array());
    session.selection.clear();
    assert!(
        session.trace_source(images[0], false).is_none(),
        "not selected"
    );
}

/// Criterion 7: the preview is no node; leaving the tool drops it.
#[test]
fn the_preview_is_no_document_node() {
    let mut session = Session::new(1);
    let images = import_images(&mut session, 1);
    let steps = session.document.own_steps().len();
    session.set_tool(Tool::Trace);
    let source = session.trace_source(images[0], true).unwrap();
    assert!(session.set_trace_preview(images[0], &wire(source.map)));
    assert!(!session.set_trace_preview(images[0], &[1.0, 2.0]));
    assert!(session.trace_draw_list(session.view()).triangle_count() > 0);
    assert_eq!(session.document.own_steps().len(), steps);
    assert_eq!(session.objects().len(), 1);
    session.set_tool(Tool::Select);
    session.set_tool(Tool::Trace);
    assert_eq!(session.trace_draw_list(session.view()).triangle_count(), 0);
}

/// Criteria 13 and 46: one step "Trace image", a group above each image, the images stay, the
/// groups selected in the Select tool; one undo removes all.
#[test]
fn create_paths_is_one_step_over_several_images() {
    let mut session = Session::new(1);
    let images = import_images(&mut session, 3);
    session.set_tool(Tool::Trace);
    assert_eq!(
        session.create_traced_paths(),
        TraceCreateOutcome::Ignored,
        "no results yet"
    );
    give_results(&mut session, &images);
    let steps = session.document.own_steps().len();
    let outcome = session.create_traced_paths();
    assert_eq!(
        outcome,
        TraceCreateOutcome::Created {
            images: 3,
            paths: 3,
            nodes: 12
        }
    );
    let own = session.document.own_steps();
    assert_eq!(own.len(), steps + 1);
    assert_eq!(own.last().unwrap().label, "trace_image");
    assert_eq!(session.tool(), Tool::Select);
    let order = session.document.object_ids();
    assert_eq!(order.len(), 6);
    for (i, &image) in images.iter().enumerate() {
        let at = order.iter().position(|&id| id == image).unwrap();
        let path = order[at + 1];
        let group = session.document.parent_group(path).unwrap();
        assert_eq!(
            session
                .document
                .read_nodes()
                .iter()
                .find(|n| n.id == group)
                .unwrap()
                .registers
                .name
                .as_deref(),
            Some(format!("Trace of logo-{i}.png").as_str())
        );
        assert!(session.selection.contains(path));
        let ObjectSnapshot::Path(compound) = session.document.object(path).unwrap() else {
            panic!("a path");
        };
        assert!(compound.style.fill.enabled && !compound.style.stroke.enabled);
    }
    assert_eq!(session.selection.ids().len(), 3);
    let shown = session.document_read().visible.len();
    assert_eq!(shown, 6, "the images stay visible");
    session.undo();
    assert_eq!(session.document.object_ids(), images);
}

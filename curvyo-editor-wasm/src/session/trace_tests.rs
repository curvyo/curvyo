//! The Trace tool's session side (`specs/0025-raster-trace` criteria 1 to 3, 7, 12, 13, 46, 51
//! and 55).

use curvyo_document_core::{NodeId, ObjectSnapshot, Point};
use curvyo_ui_core::trace_job;

use super::{
    DoubleClickHint, EscapeStep, KeyInput, KeyOutcome, Session, Tool, TraceCreateOutcome,
    TraceSelection, TraceViewMode,
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
    assert!(outline_triangles(&session) > 0);
    assert_eq!(session.document.own_steps().len(), steps);
    assert_eq!(session.objects().len(), 1);
    session.set_tool(Tool::Select);
    session.set_tool(Tool::Trace);
    assert_eq!(outline_triangles(&session), 0);
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

/// Criterion 68 (b): the traced image is washed by white at 65 % directly above it, over its
/// frame; nothing is washed outside the tool's selection.
#[test]
fn the_traced_image_gets_a_white_wash() {
    let mut session = Session::new(1);
    let images = import_images(&mut session, 2);
    session.set_tool(Tool::Trace);
    session.selection.set(&images[..1]);
    let mut objects = session.objects().to_vec();
    session.trace_wash(&mut objects);
    assert_eq!(objects.len(), 3);
    let ObjectSnapshot::Image(image) = &objects[0] else {
        panic!("the image first");
    };
    let ObjectSnapshot::Path(wash) = &objects[1] else {
        panic!("the wash directly above the image");
    };
    assert!(wash.closed && !wash.style.stroke.enabled);
    let fill = &wash.style.fill;
    assert!(fill.enabled && (fill.color.r, fill.color.g, fill.color.b) == (255, 255, 255));
    assert!((fill.opacity.get() - 0.65).abs() < 1e-12);
    let corners: Vec<Point> = wash.anchors.iter().map(|a| a.point).collect();
    assert_eq!(corners, image.corners());
    assert!(
        matches!(objects[2], ObjectSnapshot::Image(_)),
        "the other image is not washed"
    );
}

/// Criteria 30 and 68 (a): the counts of each written group, for its Layers row's tooltip.
#[test]
fn each_trace_group_keeps_its_counts_for_the_layers_tooltip() {
    let mut session = Session::new(1);
    let images = import_images(&mut session, 2);
    session.set_tool(Tool::Trace);
    give_results(&mut session, &images);
    session.create_traced_paths();
    let order = session.document.object_ids();
    for &image in &images {
        let at = order.iter().position(|&id| id == image).unwrap();
        let group = session.document.parent_group(order[at + 1]).unwrap();
        assert_eq!(session.traced_counts(group), Some((1, 4)));
        assert_eq!(session.traced_counts(image), None);
    }
    let json: serde_json::Value = serde_json::from_str(&session.layers_view_json(0, 20)).unwrap();
    let traced: Vec<&serde_json::Value> = json["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| !row["traced"].is_null())
        .collect();
    assert_eq!(traced.len(), 2);
    assert_eq!(traced[0]["traced"], serde_json::json!([1, 4]));
}

/// Criterion 51: Overlay first; Shift+`\` cycles Overlay, Trace, Image; the held peek shows Image
/// without changing the choice; leaving the tool drops both.
#[test]
fn the_view_starts_on_overlay_cycles_and_peeks() {
    let mut session = Session::new(1);
    import_images(&mut session, 1);
    session.set_tool(Tool::Trace);
    assert_eq!(session.trace_view_mode(), TraceViewMode::Overlay);
    session.cycle_trace_view_mode();
    assert_eq!(session.trace_view_mode(), TraceViewMode::Trace);
    session.set_trace_peek(true);
    assert_eq!(session.shown_trace_view_mode(), TraceViewMode::Image);
    assert_eq!(session.trace_view_mode(), TraceViewMode::Trace);
    session.set_trace_peek(false);
    assert_eq!(session.shown_trace_view_mode(), TraceViewMode::Trace);
    session.cycle_trace_view_mode();
    session.cycle_trace_view_mode();
    assert_eq!(session.trace_view_mode(), TraceViewMode::Overlay);
    session.set_trace_view_mode(TraceViewMode::Image);
    session.set_trace_peek(true);
    session.set_tool(Tool::Select);
    session.set_tool(Tool::Trace);
    assert_eq!(session.shown_trace_view_mode(), TraceViewMode::Overlay);
    session.set_tool(Tool::Select);
    session.set_trace_peek(true);
    session.set_tool(Tool::Trace);
    assert_eq!(
        session.shown_trace_view_mode(),
        TraceViewMode::Overlay,
        "no peek outside the tool"
    );
}

/// Criterion 51: Overlay washes the image under the outline; Trace shows the result filled
/// black in place of the image (a mesh kept across frames), at half strength when stale, without
/// the outline; Image shows the original alone. None of it touches the document.
#[test]
fn each_view_draws_what_it_names() {
    let mut session = Session::new(1);
    let images = import_images(&mut session, 1);
    session.set_tool(Tool::Trace);
    let source = session.trace_source(images[0], false).unwrap();
    assert!(session.set_trace_preview(images[0], &wire(source.map)));
    let objects = session.objects().to_vec();
    let artwork = |session: &Session| {
        let mut drawn = objects.clone();
        session.trace_artwork(&mut drawn);
        drawn
    };
    let outline = outline_triangles;

    let overlay = artwork(&session);
    assert_eq!(overlay.len(), 2, "the image and its wash");
    assert!(outline(&session) > 0);

    session.set_trace_view_mode(TraceViewMode::Trace);
    let mut traced = objects.clone();
    assert_eq!(
        session.trace_artwork(&mut traced),
        Some(0),
        "the result goes where the image was"
    );
    assert!(traced.is_empty(), "the image and its wash are taken out");
    let fill = session.trace_fill_mesh(session.view());
    assert!(fill.triangle_count() > 0);
    assert!(
        fill.triangles
            .iter()
            .all(|v| (v.color.r, v.color.g, v.color.b, v.color.a) == (0, 0, 0, 255)),
        "black, opaque"
    );
    assert_eq!(outline(&session), 0, "no outline in the Trace view");
    session.set_trace_preview_stale(true);
    let stale = session.trace_fill_mesh(session.view());
    assert!(
        stale.triangles.iter().all(|v| v.color.a == 128),
        "half strength"
    );

    session.set_trace_view_mode(TraceViewMode::Image);
    assert_eq!(artwork(&session), objects);
    assert_eq!(outline(&session), 0);
    session.set_trace_view_mode(TraceViewMode::Overlay);
    session.set_trace_peek(true);
    assert_eq!(artwork(&session), objects, "the peek is the Image view");
    assert_eq!(
        &session.objects()[..],
        &objects[..],
        "the document is untouched"
    );
}

/// The triangles of the Trace tool's hollow outline in a frame.
fn outline_triangles(session: &Session) -> usize {
    let mut list = curvyo_render_core::DrawList::default();
    session.trace_draw_into(session.view(), &mut list);
    list.triangle_count()
}

/// Squares of 4 nodes, `count` of them, as the worker's numbers.
fn many_squares(count: usize) -> Vec<f64> {
    let mut wire = vec![0.0; curvyo_ui_core::TRACE_WIRE_HEADER];
    #[allow(clippy::cast_precision_loss)]
    {
        wire[1] = count as f64;
        wire[2] = (count * 4) as f64;
        wire[curvyo_ui_core::TRACE_WIRE_HEADER - 1] = count as f64;
    }
    for i in 0..count {
        #[allow(clippy::cast_precision_loss)]
        let (x, y) = ((i % 200) as f64 * 0.5, (i / 200) as f64 * 0.5);
        wire.extend([6.0, 0.0, x, y]);
        wire.extend([1.0, x + 0.25, y, 1.0, x + 0.25, y + 0.25, 1.0, x, y + 0.25]);
        wire.extend([1.0, x, y, 4.0]);
    }
    wire
}

/// Criterion 55: a result over 20,000 nodes is one group with one compound path, written in
/// one step, one row in Layers.
#[test]
fn a_result_over_20000_nodes_is_one_group_in_one_step() {
    let mut session = Session::new(1);
    let images = import_images(&mut session, 1);
    session.set_tool(Tool::Trace);
    assert!(session.set_trace_result(images[0], &many_squares(5_001)));
    let steps = session.history_len();
    let outcome = session.create_traced_paths();
    assert_eq!(
        outcome,
        TraceCreateOutcome::Created {
            images: 1,
            paths: 5_001,
            nodes: 20_004
        }
    );
    assert_eq!(session.history_len(), steps + 1);
    let order = session.document.object_ids();
    assert_eq!(order.len(), 2, "the image and one compound path");
    let group = session.document.parent_group(order[1]).unwrap();
    assert_eq!(session.traced_counts(group), Some((5_001, 20_004)));
    session.undo();
    assert_eq!(session.document.object_ids(), images);
}

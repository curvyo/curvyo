//! The Trace tool's multicolour side (`specs/0089-trace-quality-and-colour` slice M, criteria
//! 25 and 29): Create paths writes one filled compound path per colour in one step, a recolour
//! changes fills only, Isolate dims the other colours, the Mask view shows the segmented
//! palette.

use curvyo_document_core::{Length, NodeId, ObjectSnapshot};
use curvyo_ui_core::{
    TraceColourParams, TraceMethod, TraceParams, TraceRgb, read_trace_wire, trace_colour_job_with,
};

use super::{Session, Tool, TraceCreateOutcome, TraceViewMode};

const PNG: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");

const RED: TraceRgb = TraceRgb::new(208, 34, 42);
const BLUE: TraceRgb = TraceRgb::new(35, 78, 184);

fn traced_image() -> (Session, NodeId) {
    let mut session = Session::new(1);
    session.import_begin(None);
    session.import_add(PNG, "logo.png");
    session.import_finish(false);
    let image = session
        .objects()
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.id),
            _ => None,
        })
        .unwrap();
    session.set_tool(Tool::Trace);
    (session, image)
}

/// A multicolour result over `map`: a red block left of a blue one on white, 40 x 20 px.
fn colour_wire(map: [f64; 6]) -> Vec<f64> {
    colour_wire_excluding(map, &[])
}

/// [`colour_wire`] with the colours `excluded` made background.
fn colour_wire_excluding(map: [f64; 6], excluded: &[TraceRgb]) -> Vec<f64> {
    let rgba: Vec<u8> = (0..40 * 20)
        .flat_map(|i| {
            let (x, y) = (i % 40, i / 40);
            if !(4..36).contains(&x) || !(4..16).contains(&y) {
                [255, 255, 255, 255]
            } else if x < 20 {
                [RED.r, RED.g, RED.b, 255]
            } else {
                [BLUE.r, BLUE.g, BLUE.b, 255]
            }
        })
        .collect();
    let mut params = TraceColourParams::new(TraceParams {
        selection: TraceMethod::RemoveBackground {
            background: Some(TraceRgb::new(255, 255, 255)),
            tolerance: 40,
        },
        specks: Length::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    });
    params.excluded = excluded.to_vec();
    trace_colour_job_with(&rgba, 40, 20, map, &params, &mut |_| {})
}

/// The fills and outline counts of the compound paths in the group Create paths wrote.
fn written(session: &Session) -> Vec<(TraceRgb, usize)> {
    session
        .selection
        .ids()
        .iter()
        .map(|&id| {
            let Some(ObjectSnapshot::Path(path)) = session.document.object(id) else {
                panic!("a path");
            };
            assert!(path.style.fill.enabled && !path.style.stroke.enabled);
            let c = path.style.fill.color;
            (TraceRgb::new(c.r, c.g, c.b), 1 + path.extra_subpaths.len())
        })
        .collect()
}

/// Criterion 29: one group with one filled compound path per colour, largest first, in one
/// undo step; a recolour changes the fill only, the outlines stay those of the result.
#[test]
fn create_paths_writes_one_path_per_colour_and_recolours_fills_only() {
    let (mut session, image) = traced_image();
    let source = session.trace_source(image, false).unwrap();
    let wire = colour_wire(source.map);
    let layers = read_trace_wire(&wire).unwrap().layers;
    assert_eq!(layers.len(), 2);
    assert!(session.set_trace_preview(image, &wire));
    assert!(session.set_trace_result(image, &wire));
    let before = session.trace.preview.as_ref().unwrap().outlines.clone();
    session.set_trace_palette_view(vec![(RED, TraceRgb::new(0, 160, 0))], None);
    assert_eq!(
        session.trace.preview.as_ref().unwrap().outlines,
        before,
        "a recolour traces nothing again"
    );
    let steps = session.document.own_steps().len();
    let outcome = session.create_traced_paths();
    assert!(matches!(
        outcome,
        TraceCreateOutcome::Created {
            images: 1,
            paths: 2,
            colours: 2,
            ..
        }
    ));
    assert_eq!(session.document.own_steps().len(), steps + 1);
    let fills = written(&session);
    assert_eq!(fills.len(), 2);
    assert!(fills.contains(&(TraceRgb::new(0, 160, 0), 1)), "{fills:?}");
    assert!(fills.contains(&(BLUE, 1)), "{fills:?}");
    let path = session.selection.ids()[0];
    let group = session.document.parent_group(path).unwrap();
    let names: Vec<Option<String>> = session
        .document
        .read_nodes()
        .iter()
        .filter(|n| session.selection.ids().contains(&n.id) || n.id == group)
        .map(|n| n.registers.name.clone())
        .collect();
    assert!(
        names.contains(&Some("Trace of logo.png".into())),
        "{names:?}"
    );
    assert!(names.contains(&Some("#234EB8".into())), "{names:?}");
    session.undo();
    assert_eq!(session.document.object_ids(), [image]);
}

/// The strip's palette comes from the shown preview; a single-colour preview has none.
#[test]
fn the_palette_of_the_preview_is_json() {
    let (mut session, image) = traced_image();
    let source = session.trace_source(image, false).unwrap();
    assert!(session.set_trace_preview(image, &colour_wire(source.map)));
    let json: serde_json::Value = serde_json::from_str(&session.trace_palette_json()).unwrap();
    let hexes: Vec<&str> = json["colours"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["hex"].as_str().unwrap())
        .collect();
    assert!(
        hexes.contains(&"#D0222A") && hexes.contains(&"#234EB8"),
        "{hexes:?}"
    );
    assert_eq!(json["layers"].as_array().unwrap().len(), 2);
    let single = curvyo_ui_core::trace_job(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        128,
        80,
        0.0,
    );
    assert!(session.set_trace_preview(image, &single));
    assert_eq!(session.trace_palette_json(), "");
}

/// Isolate dims the other colours in the Trace view (35 %); the isolated one stays opaque.
#[test]
fn isolate_dims_the_other_colours() {
    let (mut session, image) = traced_image();
    let source = session.trace_source(image, false).unwrap();
    assert!(session.set_trace_preview(image, &colour_wire(source.map)));
    session.set_trace_palette_view(Vec::new(), Some(BLUE));
    let fills = &session.trace.preview.as_ref().unwrap().fills;
    let opacity = |c: TraceRgb| {
        fills
            .iter()
            .find(|f| (f.style.fill.color.r, f.style.fill.color.b) == (c.r, c.b))
            .unwrap()
            .style
            .fill
            .opacity
            .get()
    };
    assert!((opacity(RED) - 0.35).abs() < 1e-9);
    assert!(opacity(BLUE) > 0.999);
    session.set_trace_view_mode(TraceViewMode::Trace);
    assert!(session.trace_fill_mesh(session.view()).triangle_count() > 0);
}

/// The Mask view of a multicolour trace is the segmented palette on white; Isolate mixes the
/// other colours with white to 35 %.
#[test]
fn the_colour_mask_is_the_segmented_palette() {
    let (mut session, image) = traced_image();
    // 3 x 1 px: background, red, blue.
    let rgba = [
        0, 0, 0, 0, RED.r, RED.g, RED.b, 255, BLUE.r, BLUE.g, BLUE.b, 255,
    ];
    assert!(session.set_trace_mask(image, 3, 1, &rgba));
    session.set_trace_view_mode(TraceViewMode::Mask);
    let mut list = session.draw_list();
    let pictures = session.frame_images(&mut list);
    let pixels = pictures[0].1.pixels.clone();
    assert_eq!(
        pixels,
        [
            255, 255, 255, 255, RED.r, RED.g, RED.b, 255, BLUE.r, BLUE.g, BLUE.b, 255
        ]
    );
    session.set_trace_palette_view(Vec::new(), Some(BLUE));
    let mut list = session.draw_list();
    let pictures = session.frame_images(&mut list);
    let red = &pictures[0].1.pixels[4..8];
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let mixed = |c: u8| (f64::from(c) * 0.35 + 255.0 * 0.65).round() as u8;
    assert_eq!(red, [mixed(RED.r), mixed(RED.g), mixed(RED.b), 255]);
    assert_eq!(&pictures[0].1.pixels[8..11], [BLUE.r, BLUE.g, BLUE.b]);
}

/// Every colour excluded: the preview shows nothing, the strip still lists the palette (so a
/// colour can be taken back), and Create paths writes nothing and adds no undo step.
#[test]
fn all_colours_excluded_shows_the_palette_and_creates_nothing() {
    let (mut session, image) = traced_image();
    let source = session.trace_source(image, false).unwrap();
    let wire = colour_wire_excluding(source.map, &[RED, BLUE]);
    assert_eq!(read_trace_wire(&wire).unwrap().layers.len(), 0);
    session.set_trace_preview(image, &wire);
    session.set_trace_result(image, &wire);
    let json: serde_json::Value = serde_json::from_str(&session.trace_palette_json()).unwrap();
    assert_eq!(json["colours"].as_array().unwrap().len(), 2, "{json}");
    assert!(json["layers"].as_array().unwrap().is_empty(), "{json}");
    let steps = session.document.own_steps().len();
    assert!(matches!(
        session.create_traced_paths(),
        TraceCreateOutcome::Ignored
    ));
    assert_eq!(session.document.own_steps().len(), steps);
    assert_eq!(session.document.object_ids(), [image]);
}

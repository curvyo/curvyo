//! The Trace tool's multicolour side (`specs/0089-trace-quality-and-colour` slice M, criteria
//! 25 and 29): Create paths writes one filled compound path per colour in one step; the
//! palette of the preview and of a deleted colour.

use curvyo_document_core::{NodeId, ObjectSnapshot};
use curvyo_ui_core::{
    TraceColourParams, TraceJob, TraceMethod, TraceMode, TraceParams, TracePixelMap, TraceRaster,
    TraceRgb, read_trace_wire, run_trace_job,
};

use super::{Session, Tool, TraceCreateOutcome};

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
        specks: curvyo_ui_core::TraceMm::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    });
    params.excluded = excluded.to_vec();
    run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 40,
                height: 20,
                rgba: &rgba,
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Colours(&params),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    )
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
/// undo step, each filled with its palette colour.
#[test]
fn create_paths_writes_one_path_per_colour_with_its_palette_fill() {
    let (mut session, image) = traced_image();
    let source = session.trace_source(image, false).unwrap();
    let wire = colour_wire(source.map);
    let layers = read_trace_wire(&wire).unwrap().layers;
    assert_eq!(layers.len(), 2);
    assert!(session.set_trace_preview(image, &wire));
    assert!(session.set_trace_result(image, &wire));
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
    assert!(fills.contains(&(RED, 1)), "{fills:?}");
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
    let single = super::trace_tests::trace_job(
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

/// `ux-notes.md` §26: deleting a palette colour traces the other colours as fixed colours with
/// their count (what the frontend sends as the kept palette): the palette is exactly them, nothing
/// is found anew, and every pixel of the deleted colour goes to the colour nearest to it, so its
/// area splits where its shades lean to different colours (not one merge into one colour).
#[test]
fn a_deleted_colour_is_matched_again_pixel_by_pixel_among_the_kept_colours() {
    const A: TraceRgb = TraceRgb::new(200, 30, 30);
    const B: TraceRgb = TraceRgb::new(30, 30, 200);
    // One colour to the palette (5.7 bytes apart, under the 7.65 merge distance), its left half
    // nearer A, its right half nearer B.
    const X_A: TraceRgb = TraceRgb::new(117, 30, 113);
    const X_B: TraceRgb = TraceRgb::new(113, 30, 117);
    let rgba: Vec<u8> = (0..60 * 20)
        .flat_map(|i| {
            let (x, y) = (i % 60, i / 60);
            let c = if !(4..56).contains(&x) || !(4..16).contains(&y) {
                TraceRgb::new(255, 255, 255)
            } else if x < 20 {
                A
            } else if x < 28 {
                X_A
            } else if x < 36 {
                X_B
            } else {
                B
            };
            [c.r, c.g, c.b, 255]
        })
        .collect();
    let mut params = TraceColourParams::new(TraceParams {
        selection: TraceMethod::RemoveBackground {
            background: Some(TraceRgb::new(255, 255, 255)),
            tolerance: 40,
        },
        specks: curvyo_ui_core::TraceMm::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    });
    let palette_of = |params: &TraceColourParams| {
        let map = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let wire = run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: 60,
                    height: 20,
                    rgba: &rgba,
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Colours(params),
                lines: &[],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        );
        read_trace_wire(&wire).unwrap().palette
    };
    let found = palette_of(&params);
    assert_eq!(found.len(), 3, "A, B and the one middle colour: {found:?}");
    let near = |c: TraceRgb| {
        let d = |p: &TraceRgb| {
            (i32::from(p.r) - i32::from(c.r)).pow(2)
                + (i32::from(p.g) - i32::from(c.g)).pow(2)
                + (i32::from(p.b) - i32::from(c.b)).pow(2)
        };
        found.iter().map(|e| e.colour).min_by_key(d).unwrap()
    };
    let (a, b) = (near(A), near(B));
    // The delete: the kept palette as fixed colours and their count.
    params.fixed = vec![a, b];
    params.colours = Some(2);
    let kept = palette_of(&params);
    let colours: Vec<TraceRgb> = kept.iter().map(|e| e.colour).collect();
    assert_eq!(colours.len(), 2, "nothing found anew: {kept:?}");
    assert!(colours.contains(&a) && colours.contains(&b), "{kept:?}");
    let pixels = |c: TraceRgb| kept.iter().find(|e| e.colour == c).unwrap().pixels;
    // 16 + 8 columns of 12 rows each side.
    assert_eq!((pixels(a), pixels(b)), (24 * 12, 28 * 12), "{kept:?}");
}

/// Customer 2026-11-09: a palette can be deleted down to one colour; the last kept colour is
/// traced alone (nothing found anew), and a kept palette with no colour does not fail the job.
#[test]
fn a_kept_palette_of_one_colour_is_traced_alone_and_none_does_not_fail() {
    let rgba: Vec<u8> = (0..40 * 20)
        .flat_map(|i| {
            let (x, y) = (i % 40, i / 40);
            let c = if !(4..36).contains(&x) || !(4..16).contains(&y) {
                TraceRgb::new(255, 255, 255)
            } else if x < 20 {
                RED
            } else {
                BLUE
            };
            [c.r, c.g, c.b, 255]
        })
        .collect();
    let mut params = TraceColourParams::new(TraceParams {
        selection: TraceMethod::RemoveBackground {
            background: Some(TraceRgb::new(255, 255, 255)),
            tolerance: 40,
        },
        specks: curvyo_ui_core::TraceMm::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    });
    let map = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    params.fixed = vec![RED];
    params.colours = Some(1);
    let wire = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 40,
                height: 20,
                rgba: &rgba,
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Colours(&params),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    );
    let palette: Vec<TraceRgb> = read_trace_wire(&wire)
        .unwrap()
        .palette
        .iter()
        .map(|e| e.colour)
        .collect();
    assert_eq!(palette, [RED], "the one kept colour, nothing found anew");
    params.fixed.clear();
    params.colours = Some(0);
    let none = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 40,
                height: 20,
                rgba: &rgba,
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Colours(&params),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    );
    assert!(
        read_trace_wire(&none).is_some(),
        "an empty kept palette still answers"
    );
}

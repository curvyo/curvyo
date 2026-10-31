//! The SVG import fixtures against `specs/0024-svg-import-export`: what each one is for (criteria
//! 3, 8, 13, 19, 26 to 28, 33, 45, 47, 50, 51, 73) and the hostile sweep (criterion 70 (a)).
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use curvyo_document_core::{DrawingNode, Point, Shape};
use curvyo_geometry_core::{Outline, OutlineTriple, contains_point_in_outlines};
use curvyo_ui_core::{SvgImport, SvgPlacement};

mod svg_fonts;

/// The import of `drawing` with the included faces (slice 2).
fn svg_import(drawing: &curvyo_svg_core::SvgDrawing, placement: SvgPlacement) -> SvgImport {
    thread_local! {
        static FONTS: svg_fonts::Fonts = svg_fonts::Fonts::included();
    }
    FONTS.with(|fonts| fonts.import_drawing(drawing, placement))
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/svg")
}

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixtures().join(name)).unwrap()
}

fn open(name: &str) -> SvgImport {
    let drawing = curvyo_svg_core::read_svg(&bytes(name)).unwrap();
    svg_import(&drawing, SvgPlacement::File)
}

fn leaves(nodes: &[DrawingNode]) -> Vec<&DrawingNode> {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            DrawingNode::Group { children, .. } => out.extend(leaves(children)),
            leaf => out.push(leaf),
        }
    }
    out
}

/// Whether the nonzero fill of a path leaf covers `point`.
fn filled(leaf: &DrawingNode, x: f64, y: f64) -> bool {
    let DrawingNode::Path { outlines, .. } = leaf else {
        panic!("not a path")
    };
    let owned: Vec<Vec<OutlineTriple>> = outlines
        .iter()
        .map(|o| {
            o.anchors
                .iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect()
        })
        .collect();
    let all: Vec<Outline> = owned.iter().map(|t| Outline::new(t, true)).collect();
    contains_point_in_outlines(&all, Point::new(x, y))
}

#[test]
fn ac8_text_and_empty_files_bring_in_nothing() {
    // Slice 2 (criterion 78): live text comes in as one text object.
    assert_eq!(open("text_live.svg").leaves, 1);
    assert_eq!(open("empty.svg").leaves, 0);
    assert_eq!(open("text_converted_to_paths.svg").leaves, 4);
}

#[test]
fn ac19_illustrator_points_and_old_inkscape_pixels() {
    for name in ["page_illustrator.svg", "page_illustrator_entities.svg"] {
        let size = open(name).page.size;
        assert!((size.width.as_mm() - 210.0).abs() < 0.01, "{name}");
        assert!((size.height.as_mm() - 297.0).abs() < 0.01, "{name}");
    }
    let legacy = open("page_legacy_0_48.svg").page.size;
    assert!((legacy.width.as_mm() - 210.0).abs() < 1e-6);
}

#[test]
fn ac26_to_ac28_ac33_primitives_stay_where_exact() {
    let import = open("primitives.svg");
    let all = leaves(&import.nodes);
    let kind = |i: usize| match all[i] {
        DrawingNode::Primitive { shape, .. } => match shape {
            Shape::Rect { .. } => "rect",
            Shape::Ellipse { .. } => "ellipse",
            Shape::Polygon { .. } => "polygon",
            Shape::Star { .. } => "star",
        },
        DrawingNode::Path { .. } => "path",
        _ => "other",
    };
    let kinds: Vec<&str> = (0..all.len()).map(kind).collect();
    assert_eq!(
        kinds,
        [
            "rect", "rect", "path", "rect", "ellipse", "ellipse", "star", "polygon", "path",
            "ellipse", "ellipse", "path", "path"
        ],
        "slice 3 (criterion 112): the arc and the pie are ellipse arcs, the spiral a path"
    );
    let DrawingNode::Path { outlines, .. } = all[2] else {
        panic!()
    };
    assert_eq!(
        outlines[0].anchors.len(),
        12,
        "elliptical corners, no repeated node"
    );
}

#[test]
fn ac33_examples_of_transformed_rectangles() {
    let read = |transform: &str| {
        let text = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100"><rect width="20" height="10" transform="{transform}"/></svg>"#
        );
        let drawing = curvyo_svg_core::read_svg(text.as_bytes()).unwrap();
        svg_import(&drawing, SvgPlacement::File).nodes.remove(0)
    };
    let DrawingNode::Primitive {
        shape: Shape::Rect { bounds, .. },
        rotation,
        ..
    } = read("rotate(30)")
    else {
        panic!()
    };
    assert!((rotation.as_radians() - 30_f64.to_radians()).abs() < 1e-12);
    let center = (bounds.origin.x + 10.0, bounds.origin.y + 5.0);
    assert!(
        (center.0 - 6.160_254_037_844_386).abs() < 1e-9
            && (center.1 - 9.330_127_018_922_193).abs() < 1e-9
    );
    let DrawingNode::Path { outlines, .. } = read("skewX(30)") else {
        panic!()
    };
    let points: Vec<(f64, f64)> = outlines[0]
        .anchors
        .iter()
        .map(|a| {
            (
                (a.point.x * 1e4).round() / 1e4,
                (a.point.y * 1e4).round() / 1e4,
            )
        })
        .collect();
    assert_eq!(
        points,
        [(0.0, 0.0), (20.0, 0.0), (25.7735, 10.0), (5.7735, 10.0)]
    );
    let DrawingNode::Primitive {
        shape: Shape::Rect { bounds, .. },
        ..
    } = read("scale(2 1)")
    else {
        panic!()
    };
    assert_eq!((bounds.width.as_mm(), bounds.height.as_mm()), (40.0, 10.0));
}

#[test]
fn ac45_layers_keep_names_states_and_order() {
    let import = open("layers.svg");
    let layers: Vec<(String, bool, bool, f64)> = import
        .nodes
        .iter()
        .map(|n| match n {
            DrawingNode::Group {
                registers,
                layer: true,
                ..
            } => (
                registers.name.clone().unwrap(),
                registers.visible,
                registers.locked,
                registers.opacity.get(),
            ),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        layers,
        [
            ("Cut".into(), true, false, 1.0),
            ("Engrave".into(), true, true, 0.5),
            ("Reference".into(), false, false, 1.0),
            ("Empty".into(), true, false, 1.0),
        ]
    );
}

#[test]
fn ac47_clones_and_copies_of_the_inkscape_fixture() {
    let import = open("clones.svg");
    let all = leaves(&import.nodes);
    let clone = |i: usize| match all[i] {
        DrawingNode::Clone {
            original, matrix, ..
        } => Some((*original, matrix.to_array())),
        _ => None,
    };
    assert_eq!(
        clone(1),
        Some((0, [1.0, 0.0, 0.0, 1.0, 40.0, 0.0])),
        "moved"
    );
    let (_, rotated) = clone(2).unwrap();
    assert!(
        (rotated[0] - 30_f64.to_radians().cos()).abs() < 1e-12,
        "rotated by 30°"
    );
    let (_, mirrored) = clone(3).unwrap();
    assert!(
        (mirrored[0] * mirrored[3] - mirrored[1] * mirrored[2] + 1.0).abs() < 1e-12,
        "a flip"
    );
    assert_eq!(clone(4).map(|c| c.0), Some(1), "a clone of a clone");
    assert!(clone(5).is_none(), "a scaled clone is a copy");
    assert!(
        matches!(all[7], DrawingNode::Primitive { .. }),
        "a clone of a rectangle is a copy"
    );
    assert!(
        clone(9).is_some(),
        "a clone in a rotated group of an original in another group"
    );
    assert!(
        clone(10).is_some(),
        "a fill on the use where the target sets one changes nothing"
    );
    let DrawingNode::Path { style, .. } = all[12] else {
        panic!("a use that sets a fill is a copy")
    };
    assert_eq!((style.fill.color.r, style.fill.color.b), (255, 255));
    assert!(
        matches!(all[13], DrawingNode::Path { .. })
            && matches!(all[14], DrawingNode::Primitive { .. }),
        "a symbol is a group of copies"
    );
}

#[test]
fn ac50_ac51_even_odd_files_fill_the_same() {
    let import = open("even_odd.svg");
    let all = leaves(&import.nodes);
    assert!(
        filled(all[0], 15.0, 40.0) && !filled(all[0], 40.0, 40.0),
        "the donut keeps its hole"
    );
    assert!(
        filled(all[1], 85.0, 15.0) && !filled(all[1], 95.0, 25.0) && filled(all[1], 110.0, 40.0),
        "three rings"
    );
    assert!(
        filled(all[2], 40.0, 110.0),
        "the pentagram's centre is filled under nonzero"
    );
    assert!(
        filled(all[3], 92.5, 110.0),
        "crossing circles stay as written"
    );
    assert!(filled(all[4], 15.0, 180.0));
}

#[test]
fn ac13_unsupported_content_is_left_out() {
    let import = open("unsupported.svg");
    // Slice 2 (criterion 100): the embedded 1 x 1 PNG comes in as an image, last in the list.
    assert_eq!(import.leaves, 8);
    assert!(matches!(
        leaves(&import.nodes).last(),
        Some(DrawingNode::Image { .. })
    ));
    let all = leaves(&import.nodes);
    let DrawingNode::Primitive { style, .. } = all[0] else {
        panic!()
    };
    assert_eq!(
        style.fill.color.r, 0x80,
        "a gradient becomes its middle colour"
    );
}

#[test]
fn ac3_import_is_centred_and_layers_become_groups() {
    let drawing = curvyo_svg_core::read_svg(&bytes("layers.svg")).unwrap();
    let import = svg_import(&drawing, SvgPlacement::CentredOn(Point::new(0.0, 0.0)));
    assert!(
        import
            .nodes
            .iter()
            .all(|n| matches!(n, DrawingNode::Group { layer: false, .. }))
    );
    let clones = curvyo_svg_core::read_svg(&bytes("clones.svg")).unwrap();
    let placed = svg_import(&clones, SvgPlacement::CentredOn(Point::new(500.0, 500.0)));
    let file = svg_import(&clones, SvgPlacement::File);
    let (DrawingNode::Clone { matrix: a, .. }, DrawingNode::Clone { matrix: b, .. }) =
        (leaves(&placed.nodes)[2], leaves(&file.nodes)[2])
    else {
        panic!()
    };
    assert_eq!(
        a.to_array()[..4],
        b.to_array()[..4],
        "T·M·T⁻¹ keeps the linear part"
    );
}

/// Criterion 73: `0046` stage 2 reads closed outlines through the same reader.
#[test]
fn ac73_a_template_loader_reads_closed_outlines_from_the_reader() {
    for name in ["primitives.svg", "page_a4.svg"] {
        let drawing = curvyo_svg_core::read_svg(&bytes(name)).unwrap();
        let mut closed = 0;
        let mut stack: Vec<&curvyo_svg_core::SvgNode> = drawing.nodes.iter().collect();
        while let Some(node) = stack.pop() {
            match node {
                curvyo_svg_core::SvgNode::Group { children, .. } => stack.extend(children),
                curvyo_svg_core::SvgNode::Leaf { leaf, .. } => {
                    let path = leaf.kind.outline_mm().unwrap();
                    closed += path
                        .elements()
                        .iter()
                        .filter(|e| matches!(e, kurbo::PathEl::ClosePath))
                        .count();
                }
                curvyo_svg_core::SvgNode::Text { .. } | curvyo_svg_core::SvgNode::Image { .. } => {}
            }
        }
        assert!(closed > 0, "{name}");
        assert!(drawing.page.is_some());
    }
}

/// Criterion 70 (a): every fixture cut at 200 lengths and with 200 single bytes changed gives a
/// refusal or a result in under 2 s, without a panic.
#[test]
fn ac70_truncated_and_changed_fixtures_never_panic() {
    let mut names: Vec<_> = std::fs::read_dir(fixtures())
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| {
            Path::new(n)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("svg") || e.eq_ignore_ascii_case("svgz"))
        })
        .collect();
    names.sort();
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for name in names {
        let original = bytes(&name);
        let len = original.len();
        for step in 0..200 {
            let cut = len * step / 200;
            let mut changed = original.clone();
            if len > 0 {
                let at = usize::try_from(next() % len as u64).unwrap();
                changed[at] = u8::try_from(next() % 256).unwrap();
            }
            for candidate in [&original[..cut], &changed[..]] {
                // Thread CPU time, not wall time: a loaded machine must not fail the bound.
                let started = thread_cpu();
                if let Ok(drawing) = curvyo_svg_core::read_svg(candidate) {
                    let _ = svg_import(&drawing, SvgPlacement::File);
                }
                let took = thread_cpu().saturating_sub(started);
                assert!(
                    took < Duration::from_secs(2),
                    "{name} took too long: {took:?}"
                );
            }
        }
    }
}

/// The calling thread's CPU time (Linux `schedstat`; wall time since the first call elsewhere),
/// the same helper as `svg_import_slice2_fixtures.rs` (`CLAUDE.md` §5).
fn thread_cpu() -> Duration {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok())
        .map_or_else(
            || {
                static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                START.get_or_init(std::time::Instant::now).elapsed()
            },
            Duration::from_nanos,
        )
}

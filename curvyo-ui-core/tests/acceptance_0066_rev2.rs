//! Independent acceptance tests for 0066 revision 2 layout rules (AC31, AC32), through the public
//! `curvyo-ui-core` API only. Written from the criteria text.
#![allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]
use curvyo_document_core::{Length, Point, Vec2, ViewTransform};
use curvyo_ui_core::{
    DimensionGeometry, DimensionItem, SymbolAnchor, SymbolItem, SymbolPlacement, TextBox,
    layout_dimensions, layout_symbols,
};

fn view(scale: f64) -> ViewTransform {
    ViewTransform::new(scale, Point::new(0.0, 0.0))
}
fn item(i: usize, keep: bool) -> SymbolItem {
    SymbolItem {
        anchor: SymbolAnchor::Point {
            key: i,
            at: Point::new((i % 20) as f64 * 40.0, (i / 20) as f64 * 40.0),
        },
        keep,
        expanded: false,
    }
}

#[test]
fn ac32_151_annotations_thin_and_keep_selected_hovered_conflicting_visible() {
    // 100 symbols + 51 dimension texts = 151 annotations.
    let items: Vec<_> = (0..100)
        .map(|i| item(i, i == 3 || i == 40 || i == 99))
        .collect();
    let texts: Vec<_> = (0..51)
        .map(|i| TextBox {
            center: Point::new(1000.0 + i as f64 * 90.0, 2000.0),
            half_width_px: 25.0,
            half_height_px: 8.0,
        })
        .collect();
    let layout = layout_symbols(&items, &texts, view(4.0));
    for (i, p) in layout.placements.iter().enumerate() {
        let kept = i == 3 || i == 40 || i == 99;
        if kept {
            assert!(
                matches!(p, SymbolPlacement::Badge(_)),
                "selected/hovered/conflicting symbol {i} must stay a full badge, got {p:?}"
            );
        } else {
            assert!(
                matches!(p, SymbolPlacement::Dot(_)),
                "151 annotations: symbol {i} must thin to a dot, got {p:?}"
            );
        }
    }
}

#[test]
fn ac32_exactly_150_annotations_do_not_thin() {
    let items: Vec<_> = (0..100).map(|i| item(i, false)).collect();
    let texts: Vec<_> = (0..50)
        .map(|i| TextBox {
            center: Point::new(1000.0 + i as f64 * 90.0, 2000.0),
            half_width_px: 25.0,
            half_height_px: 8.0,
        })
        .collect();
    let layout = layout_symbols(&items, &texts, view(4.0));
    assert!(
        layout
            .placements
            .iter()
            .all(|p| matches!(p, SymbolPlacement::Badge(_))),
        "150 annotations are not above the limit"
    );
}

#[test]
fn ac32_symbols_sharing_a_place_do_not_overlap() {
    let shared = Point::new(50.0, 50.0);
    let items: Vec<_> = (0..4)
        .map(|_| SymbolItem {
            anchor: SymbolAnchor::Spot { at: shared },
            keep: false,
            expanded: true,
        })
        .collect();
    let scale = 4.0;
    let layout = layout_symbols(&items, &[], view(scale));
    let centres: Vec<Point> = layout
        .placements
        .iter()
        .filter_map(|p| match p {
            SymbolPlacement::Badge(c) => Some(*c),
            _ => None,
        })
        .collect();
    assert!(
        centres.len() >= 4 || !layout.more.is_empty(),
        "everything is shown or folded: {layout:?}"
    );
    for (i, a) in centres.iter().enumerate() {
        for b in &centres[i + 1..] {
            let (dx, dy) = ((a.x - b.x).abs() * scale, (a.y - b.y).abs() * scale);
            assert!(
                dx >= 16.0 || dy >= 16.0,
                "16 px badges overlap: {a:?} {b:?}"
            );
        }
    }
}

#[test]
fn ac32_symbol_offsets_keep_constant_screen_size_at_every_zoom() {
    let one = [SymbolItem {
        anchor: SymbolAnchor::Entity {
            key: 0,
            mid: Point::new(100.0, 100.0),
            direction: Vec2 { x: 1.0, y: 0.0 },
        },
        keep: false,
        expanded: false,
    }];
    let mut px = Vec::new();
    for scale in [0.5, 1.0, 4.0, 20.0] {
        let l = layout_symbols(&one, &[], view(scale));
        let SymbolPlacement::Badge(c) = l.placements[0] else {
            panic!("badge expected")
        };
        px.push(((c.x - 100.0) * scale).hypot((c.y - 100.0) * scale));
    }
    for v in &px {
        assert!(
            (v - px[0]).abs() < 1e-6,
            "badge distance in screen px is constant: {px:?}"
        );
    }
}

fn dims_px(geometry: DimensionGeometry, scale: f64) -> curvyo_ui_core::DimensionMarks {
    layout_dimensions(
        &[DimensionItem {
            geometry,
            label: None,
            text_chars: 5,
        }],
        view(scale),
    )
    .remove(0)
}

#[test]
fn ac31_linear_dimension_has_extension_lines_dimension_line_arrowheads_and_text() {
    let g = DimensionGeometry::Linear {
        from: Point::new(0.0, 0.0),
        to: Point::new(50.0, 0.0),
    };
    let m = dims_px(g, 6.0);
    assert_eq!(m.arrows.len(), 2, "arrowheads at both ends");
    let extensions = m
        .lines
        .iter()
        .filter(|(a, b)| (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() > 1e-9)
        .count();
    assert!(
        extensions >= 2,
        "two extension lines perpendicular to the measured line: {:?}",
        m.lines
    );
    let along = m
        .lines
        .iter()
        .filter(|(a, b)| (a.y - b.y).abs() < 1e-9 && (a.x - b.x).abs() > 1e-9)
        .count();
    assert!(along >= 1, "a dimension line: {:?}", m.lines);
    // text on or beside the line, in the middle of the measured span
    assert!(
        (m.text.x - 25.0).abs() < 1.0,
        "text centred on the span: {:?}",
        m.text
    );
}

#[test]
fn ac31_radius_diameter_and_angle_marks() {
    let radius = dims_px(
        DimensionGeometry::Radius {
            center: Point::new(0.0, 0.0),
            radius: Length::from_mm(25.0),
            toward: Vec2 { x: 1.0, y: 0.0 },
        },
        6.0,
    );
    assert!(!radius.lines.is_empty(), "radius has a leader");
    let diameter = dims_px(
        DimensionGeometry::Diameter {
            center: Point::new(0.0, 0.0),
            radius: Length::from_mm(25.0),
            toward: Vec2 { x: 1.0, y: 0.0 },
        },
        6.0,
    );
    assert_eq!(
        diameter.arrows.len(),
        2,
        "diameter line with arrowheads at both ends"
    );
    let angle = dims_px(
        DimensionGeometry::Angle {
            vertex: Point::new(0.0, 0.0),
            first: Vec2 { x: 1.0, y: 0.0 },
            second: Vec2 {
                x: std::f64::consts::FRAC_1_SQRT_2,
                y: std::f64::consts::FRAC_1_SQRT_2,
            },
        },
        6.0,
    );
    assert_eq!(
        angle.arrows.len(),
        2,
        "arc between the lines with arrowheads"
    );
    assert!(
        angle.lines.len() >= 2,
        "an arc polyline between the two lines"
    );
}

#[test]
fn ac31_text_offset_keeps_constant_screen_size_at_every_zoom() {
    let g = DimensionGeometry::Linear {
        from: Point::new(0.0, 0.0),
        to: Point::new(200.0, 0.0),
    };
    let mut offsets = Vec::new();
    for scale in [1.0, 2.0, 5.0, 10.0] {
        let m = dims_px(g, scale);
        offsets.push(m.text.y.abs() * scale);
    }
    for o in &offsets {
        assert!(
            (o - offsets[0]).abs() < 1e-6,
            "dimension line distance in screen px is constant: {offsets:?}"
        );
    }
}

fn centre_in_box(c: Point, b: &TextBox, scale: f64, margin_px: f64) -> bool {
    (c.x - b.center.x).abs() * scale < b.half_width_px + margin_px
        && (c.y - b.center.y).abs() * scale < b.half_height_px + margin_px
}

#[test]
fn review2_a_badge_never_lies_on_a_dimension_text_box() {
    let scale = 4.0;
    let mid = Point::new(50.0, 50.0);
    let text = TextBox {
        center: mid,
        half_width_px: 25.0,
        half_height_px: 8.0,
    };
    let items = [SymbolItem {
        anchor: SymbolAnchor::Entity {
            key: 1,
            mid,
            direction: Vec2::new(0.0, 1.0),
        },
        keep: false,
        expanded: false,
    }];
    let layout = layout_symbols(&items, &[text], view(scale));
    match layout.placements[0] {
        SymbolPlacement::Badge(c) => {
            // a badge is at least 14 px across: its centre is clear of the box by half of it
            assert!(
                !centre_in_box(c, &text, scale, 7.0),
                "badge at {c:?} touches the text box"
            );
        }
        other => assert!(matches!(other, SymbolPlacement::Dot(_)), "{other:?}"),
    }
}

#[test]
fn review2_a_point_badge_does_not_cover_its_point() {
    let scale = 4.0;
    let at = Point::new(10.0, 10.0);
    let items = [SymbolItem {
        anchor: SymbolAnchor::Point { key: 1, at },
        keep: true,
        expanded: false,
    }];
    let layout = layout_symbols(&items, &[], view(scale));
    let SymbolPlacement::Badge(c) = layout.placements[0] else {
        panic!("kept symbol is a badge");
    };
    let d = (c.x - at.x).hypot(c.y - at.y) * scale;
    assert!(d >= 8.0, "badge centre only {d:.1} px from its point glyph");
}

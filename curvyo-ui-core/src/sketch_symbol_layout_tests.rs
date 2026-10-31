use super::*;

const TOLERANCE_PX: f64 = 1e-6;

fn view(scale: f64) -> ViewTransform {
    ViewTransform::new(scale, Point::new(0.0, 0.0))
}
fn on_line(key: usize, mid: Point) -> SymbolItem {
    SymbolItem {
        anchor: SymbolAnchor::Entity {
            key,
            mid,
            direction: Vec2::new(1.0, 0.0),
        },
        keep: false,
        expanded: false,
    }
}
fn badge(placement: SymbolPlacement) -> Point {
    match placement {
        SymbolPlacement::Badge(at) => at,
        other => panic!("expected a badge, got {other:?}"),
    }
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOLERANCE_PX
}

#[test]
fn one_symbol_sits_twelve_pixels_beside_its_entity_at_every_zoom() {
    for scale in [0.5, 1.0, 8.0] {
        let layout = layout_symbols(&[on_line(0, Point::new(10.0, 0.0))], &[], view(scale));
        let at = badge(layout.placements[0]);
        assert!(close(at.x, 10.0) && close(at.y * scale, -12.0), "{scale}");
    }
}

#[test]
fn a_row_holds_three_badges_then_a_plus_badge() {
    let items: Vec<_> = (0..5).map(|_| on_line(7, Point::new(0.0, 0.0))).collect();
    let layout = layout_symbols(&items, &[], view(1.0));
    let shown: Vec<_> = layout.placements[..3].iter().map(|p| badge(*p)).collect();
    assert_eq!(&layout.placements[3..], &[SymbolPlacement::Folded; 2]);
    assert_eq!(layout.more.len(), 1);
    assert_eq!(layout.more[0].count, 2);
    // Four slots 18px apart, centred on the midpoint.
    assert!(close(shown[0].x, -27.0) && close(shown[2].x, 9.0));
    assert!(close(layout.more[0].center.x, 27.0));
    // A hovered or selected entity shows its whole row.
    let mut expanded = items.clone();
    expanded[0].expanded = true;
    let layout = layout_symbols(&expanded, &[], view(1.0));
    assert_eq!(layout.more, []);
    assert!(
        layout
            .placements
            .iter()
            .all(|p| matches!(p, SymbolPlacement::Badge(_)))
    );
}

#[test]
fn badges_never_overlap() {
    // Two parallel lines 4mm apart and two spots on one place.
    let items = [
        on_line(0, Point::new(0.0, 0.0)),
        on_line(1, Point::new(0.0, 4.0)),
        SymbolItem {
            anchor: SymbolAnchor::Spot {
                at: Point::new(50.0, 50.0),
            },
            keep: false,
            expanded: false,
        },
        SymbolItem {
            anchor: SymbolAnchor::Spot {
                at: Point::new(50.0, 50.0),
            },
            keep: false,
            expanded: false,
        },
        SymbolItem {
            anchor: SymbolAnchor::Point {
                key: 3,
                at: Point::new(-60.0, 0.0),
            },
            keep: false,
            expanded: false,
        },
    ];
    let layout = layout_symbols(&items, &[], view(1.0));
    let centers: Vec<_> = layout.placements.iter().map(|p| badge(*p)).collect();
    for (i, a) in centers.iter().enumerate() {
        for b in &centers[..i] {
            assert!(
                (a.x - b.x).abs() >= 18.0 - TOLERANCE_PX
                    || (a.y - b.y).abs() >= 18.0 - TOLERANCE_PX,
                "{a:?} overlaps {b:?}"
            );
        }
    }
    // A point's badge sits up and right of it.
    assert!(close(centers[4].x, -50.0) && close(centers[4].y, -10.0));
}

#[test]
fn above_150_annotations_unkept_symbols_thin_to_dots_and_kept_ones_stay() {
    // 141 symbols plus 10 dimensions: 151 annotations.
    let mut items: Vec<_> = (0..141)
        .map(|i| {
            on_line(
                i,
                Point::new(f64::from(u32::try_from(i).unwrap()) * 100.0, 0.0),
            )
        })
        .collect();
    // One selected, one hovered and one conflicting symbol.
    for index in [3, 50, 140] {
        items[index].keep = true;
    }
    let texts = |count: usize| {
        (0..count)
            .map(|i| TextBox {
                center: Point::new(f64::from(u32::try_from(i).unwrap()) * 100.0, 500.0),
                half_width_px: 20.0,
                half_height_px: 8.0,
            })
            .collect::<Vec<_>>()
    };
    let layout = layout_symbols(&items, &texts(10), view(1.0));
    for (index, placement) in layout.placements.iter().enumerate() {
        if [3, 50, 140].contains(&index) {
            assert!(matches!(placement, SymbolPlacement::Badge(_)), "{index}");
        } else {
            assert!(matches!(placement, SymbolPlacement::Dot(_)), "{index}");
        }
    }
    // At exactly 150 annotations nothing thins.
    let layout = layout_symbols(&items, &texts(9), view(1.0));
    assert!(
        layout
            .placements
            .iter()
            .all(|p| matches!(p, SymbolPlacement::Badge(_)))
    );
}

#[test]
fn a_badge_moves_off_a_dimension_text() {
    let text = TextBox {
        center: Point::new(0.0, -12.0),
        half_width_px: 30.0,
        half_height_px: 8.0,
    };
    let layout = layout_symbols(&[on_line(0, Point::new(0.0, 0.0))], &[text], view(1.0));
    let at = badge(layout.placements[0]);
    assert!(
        close(at.x, 0.0) && at.y <= -12.0 - 8.0 - 9.0 + TOLERANCE_PX,
        "{at:?}"
    );
}

#[test]
fn a_thinned_dot_sits_where_its_badge_would_clear_of_a_dimension_text() {
    // 150 far-away dimension texts plus one symbol: 151 annotations, so the symbol thins.
    let mut texts: Vec<_> = (1..150)
        .map(|i| TextBox {
            center: Point::new(f64::from(i) * 1000.0, 0.0),
            half_width_px: 20.0,
            half_height_px: 8.0,
        })
        .collect();
    texts.push(TextBox {
        center: Point::new(0.0, -12.0),
        half_width_px: 30.0,
        half_height_px: 8.0,
    });
    let mut item = on_line(0, Point::new(0.0, 0.0));
    let thinned = layout_symbols(&[item], &texts, view(1.0)).placements[0];
    let SymbolPlacement::Dot(dot) = thinned else {
        panic!("expected a dot, got {thinned:?}");
    };
    // Hovering keeps it: the badge appears where the dot was, so the pointer stays on it.
    item.keep = true;
    let kept = badge(layout_symbols(&[item], &texts, view(1.0)).placements[0]);
    assert!(
        close(dot.x, kept.x) && close(dot.y, kept.y),
        "{dot:?} {kept:?}"
    );
    assert!(dot.y <= -12.0 - 8.0 - 9.0 + TOLERANCE_PX, "{dot:?}");
}

#[test]
fn a_badge_slides_along_its_entity_off_a_dimension_text_before_stepping_out() {
    // The review's plate: the Vertical badge of the left edge and the "25 mm" text beside it.
    let item = SymbolItem {
        anchor: SymbolAnchor::Entity {
            key: 0,
            mid: Point::new(0.0, 0.0),
            direction: Vec2::new(0.0, 1.0),
        },
        keep: false,
        expanded: false,
    };
    let text = TextBox {
        center: Point::new(-24.0, 0.0),
        half_width_px: 21.5,
        half_height_px: 8.0,
    };
    let at = badge(layout_symbols(&[item], &[text], view(1.0)).placements[0]);
    // Still 12px beside the line, moved along it until clear of the text box plus a 2px gap.
    assert!(close(at.x, -12.0), "{at:?}");
    assert!(at.y.abs() >= 8.0 + 2.0 + 9.0 - TOLERANCE_PX, "{at:?}");
    assert!(
        at.y.abs() <= 27.0 + TOLERANCE_PX,
        "slides at most one and a half pitches: {at:?}"
    );
}

#[test]
fn a_badge_keeps_clear_of_point_glyphs() {
    // The review's plate: Fixed on the corner point sat on the point glyph.
    let corner = Point::new(0.0, 0.0);
    let fixed = SymbolItem {
        anchor: SymbolAnchor::Point { key: 0, at: corner },
        keep: false,
        expanded: false,
    };
    let tangent = SymbolItem {
        anchor: SymbolAnchor::Spot { at: corner },
        keep: false,
        expanded: false,
    };
    for scale in [1.0, 4.0] {
        let layout = layout_symbols_around(&[fixed, tangent], &[], &[corner], view(scale));
        for placement in layout.placements {
            let at = badge(placement);
            // A 16px badge (9px with its gap) and the 10px point glyph (5px plus a 2px gap).
            assert!(
                (at.x * scale).abs() >= 16.0 - TOLERANCE_PX
                    || (at.y * scale).abs() >= 16.0 - TOLERANCE_PX,
                "{scale}: {at:?}"
            );
        }
    }
}

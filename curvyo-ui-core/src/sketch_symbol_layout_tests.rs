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

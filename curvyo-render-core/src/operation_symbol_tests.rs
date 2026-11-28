//! Tests of the canvas symbol pills (`operation_symbol`).

use super::*;

const ALL: [Glyph; 6] = [
    Glyph::Cut,
    Glyph::CutOut,
    Glyph::Score,
    Glyph::Engrave,
    Glyph::EngraveOutline,
    Glyph::Image,
];

fn item(x: f64, glyphs: Vec<Glyph>) -> ChipItem {
    ChipItem {
        bounds: (Point::new(x, 10.0), Point::new(x + 20.0, 30.0)),
        glyphs,
        muted: Vec::new(),
        handles: false,
    }
}

/// The box `(min x, min y, max x, max y)` of the vertices of `list` whose colour passes `keep`.
fn extent(list: &DrawList, keep: impl Fn(crate::color::RgbaColor) -> bool) -> (f64, f64, f64, f64) {
    list.triangles.iter().filter(|v| keep(v.color)).fold(
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        |(a, b, c, d), v| {
            (
                a.min(v.position.x),
                b.min(v.position.y),
                c.max(v.position.x),
                d.max(v.position.y),
            )
        },
    )
}

/// The pill's rectangle: its `--lane-line` edge (identity view: document = screen pixels).
fn pill_box(list: &DrawList) -> (f64, f64, f64, f64) {
    extent(list, |c| c == OP_LANE_LINE)
}

const TOLERANCE: f64 = 0.1;

#[test]
fn a_pill_is_six_plus_sixteen_per_symbol_plus_four_between() {
    assert!((pill_width(1) - 22.0).abs() < TOLERANCE);
    assert!((pill_width(2) - 42.0).abs() < TOLERANCE);
    assert!((pill_width(5) - 102.0).abs() < TOLERANCE);
    let view = ViewTransform::identity();
    for n in 1..=5 {
        let list = chips_list(&[item(100.0, ALL[..n].to_vec())], (800.0, 600.0), view);
        let (x0, y0, x1, y1) = pill_box(&list);
        assert!((x1 - x0 - pill_width(n)).abs() < TOLERANCE, "{n} symbols");
        assert!((y1 - y0 - CHIP_PX).abs() < TOLERANCE, "22 px high");
    }
}

#[test]
fn one_item_in_two_kinds_gets_one_pill() {
    let view = ViewTransform::identity();
    let list = chips_list(
        &[item(100.0, vec![Glyph::Cut, Glyph::Score])],
        (800.0, 600.0),
        view,
    );
    let grounds = list
        .triangles
        .iter()
        .filter(|v| v.color == OP_CHIP_GROUND)
        .count();
    let one = chips_list(&[item(100.0, vec![Glyph::Cut])], (800.0, 600.0), view);
    let one_ground = one
        .triangles
        .iter()
        .filter(|v| v.color == OP_CHIP_GROUND)
        .count();
    assert_eq!(
        grounds, one_ground,
        "one ground shape, whatever the number of kinds"
    );
}

#[test]
fn the_pill_sits_centred_six_px_above_the_object() {
    let view = ViewTransform::identity();
    let list = chips_list(
        &[item(100.0, vec![Glyph::Cut, Glyph::Engrave])],
        (800.0, 600.0),
        view,
    );
    let (x0, _, x1, y1) = pill_box(&list);
    assert!(
        (f64::midpoint(x0, x1) - 110.0).abs() < TOLERANCE,
        "centred on the box (100 to 120)"
    );
    assert!(
        (y1 - (10.0 - CHIP_OFFSET_Y_PX)).abs() < TOLERANCE,
        "bottom 6 px above the top edge"
    );
}

#[test]
fn a_centred_pill_stays_inside_the_canvas() {
    let view = ViewTransform::identity();
    let left = chips_list(
        &[item(-15.0, vec![Glyph::Cut, Glyph::Score])],
        (800.0, 600.0),
        view,
    );
    assert!(
        pill_box(&left).0 >= -TOLERANCE,
        "moved right of the canvas edge"
    );
    let right = chips_list(
        &[item(790.0, vec![Glyph::Cut, Glyph::Score])],
        (800.0, 600.0),
        view,
    );
    assert!(
        pill_box(&right).2 <= 800.0 + TOLERANCE,
        "moved left of the canvas edge"
    );
}

#[test]
fn with_handles_the_pill_takes_the_corner_and_mirrors_at_the_edge() {
    let view = ViewTransform::identity();
    let mut selected = item(100.0, vec![Glyph::Cut]);
    selected.handles = true;
    let (x0, _, _, y1) = pill_box(&chips_list(&[selected.clone()], (800.0, 600.0), view));
    assert!(
        (x0 - (120.0 + CHIP_OFFSET_X_PX)).abs() < TOLERANCE,
        "10 px right of the corner"
    );
    assert!((y1 - (10.0 - CHIP_OFFSET_Y_PX)).abs() < TOLERANCE);
    selected.bounds = (Point::new(760.0, 10.0), Point::new(780.0, 30.0));
    let (_, _, x1, _) = pill_box(&chips_list(&[selected], (800.0, 600.0), view));
    assert!(
        (x1 - (760.0 - CHIP_OFFSET_X_PX)).abs() < TOLERANCE,
        "ends 10 px left of the top-left corner"
    );
}

/// The handles of a box `(x0, y0)`..`(x1, y1)` on an identity view where
/// `curvyo-ui-core::transform_handle_layout` puts them at 1 px per mm: resize at the corners and
/// edge midpoints, rotate 32 px out along each corner diagonal, skew 16 px out of each side's
/// midpoint, move at the centre.
fn box_handles(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<TransformHandleGlyph> {
    use crate::select_decoration::TransformGlyphKind as Kind;
    let glyph = |x: f64, y: f64, kind: Kind| TransformHandleGlyph {
        position: Point::new(x, y),
        kind,
        dragging: false,
        hovered: false,
    };
    let (cx, cy) = (f64::midpoint(x0, x1), f64::midpoint(y0, y1));
    let d = 32.0 / std::f64::consts::SQRT_2;
    let along = curvyo_document_core::Vec2::new(1.0, 0.0);
    let mut handles = Vec::new();
    for (x, y) in [
        (x0, y0),
        (cx, y0),
        (x1, y0),
        (x1, cy),
        (x1, y1),
        (cx, y1),
        (x0, y1),
        (x0, cy),
    ] {
        handles.push(glyph(x, y, Kind::Resize));
    }
    for (x, y) in [
        (x0 - d, y0 - d),
        (x1 + d, y0 - d),
        (x1 + d, y1 + d),
        (x0 - d, y1 + d),
    ] {
        handles.push(glyph(x, y, Kind::Rotate));
    }
    for (x, y) in [
        (cx, y0 - 16.0),
        (x1 + 16.0, cy),
        (cx, y1 + 16.0),
        (x0 - 16.0, cy),
    ] {
        handles.push(glyph(x, y, Kind::Skew { direction: along }));
    }
    handles.push(glyph(cx, cy, Kind::Move));
    handles
}

/// Whether the pill box `pill` keeps `HANDLE_GAP_PX` from every glyph of `handles`.
fn clear_of(pill: (f64, f64, f64, f64), handles: &[TransformHandleGlyph]) -> bool {
    handles.iter().all(|handle| {
        let (width, height) = handle.kind.footprint_px();
        let (half_w, half_h) = (
            width / 2.0 + HANDLE_GAP_PX - TOLERANCE,
            height / 2.0 + HANDLE_GAP_PX - TOLERANCE,
        );
        let centre = handle.position;
        pill.2 <= centre.x - half_w
            || pill.0 >= centre.x + half_w
            || pill.3 <= centre.y - half_h
            || pill.1 >= centre.y + half_h
    })
}

#[test]
fn the_pill_of_a_selected_object_clears_every_handle() {
    let view = ViewTransform::identity();
    for (x, glyphs) in [(100.0, 1), (300.0, 3), (680.0, 5)] {
        let mut selected = item(x, ALL[..glyphs].to_vec());
        selected.handles = true;
        selected.bounds = (Point::new(x, 100.0), Point::new(x + 60.0, 160.0));
        let handles = box_handles(x, 100.0, x + 60.0, 160.0);
        let list = chips_list_clear_of(&[selected], (800.0, 600.0), view, &handles);
        assert!(list.triangle_count() > 0, "drawn at {x}");
        let pill = pill_box(&list);
        assert!(
            clear_of(pill, &handles),
            "{pill:?} clears the handles of the box at {x}"
        );
        assert!(
            (pill.3 - (100.0 - CHIP_OFFSET_Y_PX)).abs() < TOLERANCE,
            "still 6 px above the top"
        );
    }
}

#[test]
fn the_pills_of_a_group_selection_clear_the_group_handles() {
    let view = ViewTransform::identity();
    // Three members; the group box spans all of them. The right one shares the group's
    // top-right corner, the middle one sits under the top skew handle.
    let members = [
        (200.0, 200.0, 240.0, 240.0),
        (290.0, 220.0, 330.0, 260.0),
        (380.0, 200.0, 420.0, 300.0),
    ];
    let handles = box_handles(200.0, 200.0, 420.0, 300.0);
    for (x0, y0, x1, y1) in members {
        let mut member = item(x0, vec![Glyph::Cut, Glyph::Score]);
        member.bounds = (Point::new(x0, y0), Point::new(x1, y1));
        let list = chips_list_clear_of(&[member], (800.0, 600.0), view, &handles);
        assert!(list.triangle_count() > 0, "the member at {x0} keeps a pill");
        let pill = pill_box(&list);
        assert!(
            clear_of(pill, &handles),
            "{pill:?} clears the group handles"
        );
    }
}

#[test]
fn a_pill_with_no_place_left_is_not_drawn() {
    let view = ViewTransform::identity();
    let mut selected = item(0.0, ALL.to_vec());
    selected.handles = true;
    selected.bounds = (Point::new(20.0, 100.0), Point::new(60.0, 140.0));
    let handles = box_handles(20.0, 100.0, 60.0, 140.0);
    let list = chips_list_clear_of(&[selected], (120.0, 600.0), view, &handles);
    assert_eq!(
        list.triangle_count(),
        0,
        "neither side has room on a 120 px canvas"
    );
}

#[test]
fn every_symbol_is_centred_in_its_cell() {
    let view = ViewTransform::identity();
    for glyph in ALL {
        let list = chips_list(&[item(100.0, vec![glyph])], (800.0, 600.0), view);
        let (px0, py0, px1, py1) = pill_box(&list);
        let (x0, y0, x1, y1) = extent(&list, |c| c != OP_LANE_LINE && c != OP_CHIP_GROUND);
        let (cx, cy) = (f64::midpoint(px0, px1), f64::midpoint(py0, py1));
        assert!(
            (f64::midpoint(x0, x1) - cx).abs() < TOLERANCE,
            "{glyph:?} x"
        );
        assert!(
            (f64::midpoint(y0, y1) - cy).abs() < TOLERANCE,
            "{glyph:?} y"
        );
        assert!(
            x0 >= px0 + PILL_PADDING_PX - 0.5 && x1 <= px1 - PILL_PADDING_PX + 0.5,
            "{glyph:?} inside its cell"
        );
    }
}

#[test]
fn an_item_off_the_canvas_gets_no_pill() {
    let view = ViewTransform::new(2.0, Point::new(0.0, 0.0));
    let one = chips_list(
        &[item(10.0, vec![Glyph::Cut, Glyph::Score])],
        (800.0, 600.0),
        view,
    );
    let none = chips_list(&[item(10_000.0, vec![Glyph::Cut])], (800.0, 600.0), view);
    assert!(one.triangle_count() > 0);
    assert_eq!(none.triangle_count(), 0);
}

#[test]
fn a_colliding_pill_is_skipped_whole() {
    let view = ViewTransform::new(2.0, Point::new(0.0, 0.0));
    let glyphs = vec![Glyph::Cut, Glyph::Image];
    let twice = chips_list(
        &[item(10.0, glyphs.clone()), item(12.0, glyphs.clone())],
        (800.0, 600.0),
        view,
    );
    let once = chips_list(&[item(10.0, glyphs)], (800.0, 600.0), view);
    assert_eq!(twice.triangle_count(), once.triangle_count());
}

#[test]
fn the_cap_counts_pills() {
    let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
    let items: Vec<ChipItem> = (0..700)
        .map(|i| ChipItem {
            bounds: (
                Point::new(f64::from(i) * 120.0, 10.0),
                Point::new(f64::from(i) * 120.0 + 20.0, 30.0),
            ),
            glyphs: vec![Glyph::Cut, Glyph::Score, Glyph::Engrave],
            muted: Vec::new(),
            handles: false,
        })
        .collect();
    let capped = chips_list(&items, (100_000.0, 100_000.0), view);
    let expected = chips_list(&items[..CHIP_CAP], (100_000.0, 100_000.0), view);
    assert_eq!(capped.triangle_count(), expected.triangle_count());
}

#[test]
fn the_image_glyph_is_ten_ink_dots() {
    let at = |x: f64, y: f64| Point::new(x, y);
    let list = symbol(Glyph::Image, &at, 1.0, false);
    assert_eq!(list.triangle_count(), 10 * 2, "4 + 3 + 2 + 1 squares");
}

#[test]
fn a_muted_symbol_is_all_muted_ink_even_a_red_one() {
    let at = |x: f64, y: f64| Point::new(x, y);
    for glyph in [Glyph::Image, Glyph::Cut, Glyph::Engrave] {
        let list = symbol(glyph, &at, 1.0, true);
        assert!(
            list.triangles.iter().all(|v| v.color == OP_MUTED_INK),
            "{glyph:?}"
        );
    }
    let mut muted = item(10.0, vec![Glyph::Cut, Glyph::Image]);
    muted.muted = vec![Glyph::Image];
    let view = ViewTransform::new(2.0, Point::new(0.0, 0.0));
    let list = chips_list(&[muted], (800.0, 600.0), view);
    assert!(list.triangles.iter().any(|v| v.color == OP_MUTED_INK));
    assert!(
        list.triangles.iter().any(|v| v.color == OP_CUT),
        "the Cut symbol stays red"
    );
}

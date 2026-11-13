//! `specs/0114-image-trace-mask` slice 2 at the session: the Lasso on M1
//! (`curvyo-ui-core/tests/support_0094`) with the customer decisions of 2026-11-13 (criteria 4 to
//! 7, 17, 23; UX notes §6 and §7): corners by clicks, finished by the first corner, a double
//! click, Enter or a right click; Escape and Ctrl+Z discard it; another tool, part or image and
//! leaving the tool close it; Backspace removes a corner; a selected lasso moves, its corner moves,
//! Delete removes it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::type_complexity
)]

#[path = "../../curvyo-ui-core/tests/support_0094/mod.rs"]
mod support;

use curvyo_document_core::{Document, NodeId, ObjectSnapshot, Point, TraceRegionKind, unpack};
use curvyo_editor_wasm::{KeyInput, RowPress, Session, Tool, TraceRegionTool};
use image::ImageEncoder;
use support::{M1, base64, draw};

/// The picture: 300 px over 100 mm (3 px per mm, so L1's corners are whole pixels).
const PX: u32 = 300;
/// L1 of the spec, mm.
const L1: [(f64, f64); 3] = [(0.0, 0.0), (60.0, 0.0), (0.0, 100.0)];

fn png() -> Vec<u8> {
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&draw(M1, PX), PX, PX, image::ExtendedColorType::Rgba8)
        .unwrap();
    png
}

fn svg(second: bool) -> String {
    let pic = base64(&png());
    let other = if second {
        format!(
            r#"<image id="m2" x="150" y="0" width="50" height="50" preserveAspectRatio="none" xlink:href="data:image/png;base64,{pic}"/>"#
        )
    } else {
        String::new()
    };
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="300mm" height="300mm" viewBox="-100 -100 300 300"><image id="m1" x="0" y="0" width="100" height="100" preserveAspectRatio="none" xlink:href="data:image/png;base64,{pic}"/>{other}</svg>"#
    )
}

fn doc(s: &Session) -> Document {
    unpack(9, &s.pack("t").unwrap()).unwrap()
}

fn images_of(d: &Document) -> Vec<NodeId> {
    d.object_ids()
        .into_iter()
        .filter(|&id| matches!(d.object(id), Some(ObjectSnapshot::Image(_))))
        .collect()
}

/// The document point of the first picture's mm `(x, y)`.
fn pt(s: &Session, (x, y): (f64, f64)) -> Point {
    let d = doc(s);
    let Some(ObjectSnapshot::Image(i)) = d.object(images_of(&d)[0]) else {
        unreachable!()
    };
    let c = i.corners();
    Point::new(c[0].x + x, c[0].y + y)
}

fn select(s: &mut Session, id: NodeId) {
    s.press_layer_row(&serde_json::to_string(&id).unwrap(), RowPress::Plain);
}

/// M1 imported (with a second picture when `second`), the first selected, Trace tool, Lasso on.
fn lasso_on(second: bool) -> (Session, NodeId) {
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    s.import_svg(svg(second).as_bytes(), "m1.svg").unwrap();
    let image = images_of(&doc(&s))[0];
    select(&mut s, image);
    s.set_tool(Tool::Trace);
    s.set_trace_region_tool(TraceRegionTool::Lasso);
    (s, image)
}

fn click(s: &mut Session, at: (f64, f64)) {
    let p = pt(s, at);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn key_with(s: &mut Session, key: &str, ctrl: bool) {
    s.key_down(KeyInput {
        key,
        shift: false,
        ctrl,
        alt: false,
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    });
}

fn key(s: &mut Session, k: &str) {
    key_with(s, k, false);
}

/// The stored regions of `image` over all parts: (kind, subtract, corners in picture pixels).
fn stored(s: &Session, image: NodeId) -> Vec<(TraceRegionKind, bool, Vec<(f64, f64)>)> {
    doc(s)
        .trace_mask(image)
        .unwrap()
        .parts
        .iter()
        .flat_map(|part| part.regions.clone())
        .map(|r| {
            let px = r
                .points
                .iter()
                .map(|p| ((p.u * f64::from(PX)).round(), (p.v * f64::from(PX)).round()))
                .collect();
            (r.kind, r.subtract, px)
        })
        .collect()
}

fn labels(s: &Session) -> Vec<String> {
    let mut rows: Vec<String> = doc(s)
        .history_page(0, 1000)
        .iter()
        .map(|row| row.label.clone())
        .collect();
    rows.reverse();
    rows
}

/// L1's three corners clicked.
fn corners_of_l1(s: &mut Session) {
    for c in L1 {
        click(s, c);
    }
}

/// L1c as the reference: closed by a click on its first corner.
fn l1c() -> (Session, NodeId) {
    let (mut s, image) = lasso_on(false);
    corners_of_l1(&mut s);
    click(&mut s, (0.3, 0.2));
    (s, image)
}

const L1_PX: [(f64, f64); 3] = [(0.0, 0.0), (180.0, 0.0), (0.0, 300.0)];

/// Criterion 4: three clicks and a click on the first corner store L1c with three corners in one
/// step "Draw lasso"; a double click after the third corner and Enter give the same shape.
#[test]
fn c4_the_first_corner_a_double_click_and_enter_finish_the_same_lasso() {
    let (s, image) = l1c();
    assert_eq!(
        stored(&s, image),
        [(TraceRegionKind::Lasso, false, L1_PX.to_vec())]
    );
    assert_eq!(labels(&s).last().unwrap(), "draw_trace_lasso");
    let want = s.trace_mask_json();
    assert_ne!(want, "");
    // A double click: the host sends the second press as a double click, no corner of its own.
    let (mut d, image_d) = lasso_on(false);
    click(&mut d, L1[0]);
    click(&mut d, L1[1]);
    let p = pt(&d, L1[2]);
    d.pointer_down(p, false);
    d.pointer_up(p, false, false);
    d.double_click_hint(p, false, false);
    assert_eq!(stored(&d, image_d), stored(&s, image));
    assert_eq!(d.trace_mask_json(), want);
    let (mut e, image_e) = lasso_on(false);
    corners_of_l1(&mut e);
    let steps = e.undo_len();
    key(&mut e, "Enter");
    assert_eq!(e.undo_len(), steps + 1, "one step");
    assert_eq!(stored(&e, image_e), stored(&s, image));
    assert_eq!(
        e.tool(),
        Tool::Trace,
        "Enter finishes the lasso, it creates no paths"
    );
    assert_eq!(
        e.trace_region_tool(),
        TraceRegionTool::Lasso,
        "the tool stays pressed"
    );
}

/// Customer decision 1 of 2026-11-13: a right click finishes the lasso closed; without a lasso
/// in progress it does nothing.
#[test]
fn a_right_click_finishes_the_lasso_and_does_nothing_without_one() {
    let (want, _) = l1c();
    let (mut s, image) = lasso_on(false);
    assert!(!s.trace_mask_secondary_press(), "no lasso: nothing");
    assert_eq!(stored(&s, image).len(), 0);
    corners_of_l1(&mut s);
    assert!(
        s.trace_mask_secondary_press(),
        "the host consumes the press"
    );
    assert_eq!(s.trace_mask_json(), want.trace_mask_json());
    assert_eq!(s.trace_region_tool(), TraceRegionTool::Lasso);
}

/// Criterion 5 with the customer decisions: L1 not closed is closed by a straight side when
/// Rectangle is pressed, a part is picked, Add part runs, the pipette is armed, the tool is left or
/// another image is chosen; the stored geometry and the mask equal L1c's.
#[test]
fn c5_ending_the_tool_closes_l1_as_l1c() {
    let (want, want_image) = l1c();
    let reference = stored(&want, want_image);
    let check = |end: &dyn Fn(&mut Session, NodeId), what: &str, second: bool| {
        let (mut s, image) = lasso_on(second);
        corners_of_l1(&mut s);
        let steps = s.undo_len();
        end(&mut s, image);
        assert_eq!(stored(&s, image), reference, "{what}");
        assert!(
            labels(&s).iter().any(|l| l == "draw_trace_lasso"),
            "{what}: one step \"Draw lasso\""
        );
        assert!(s.undo_len() > steps, "{what}");
        assert!(!s.trace_lasso_open(), "{what}");
    };
    check(
        &|s, _| s.set_trace_region_tool(TraceRegionTool::Rect),
        "Rectangle",
        false,
    );
    check(
        &|s, _| s.set_trace_region_tool(TraceRegionTool::None),
        "Lasso released",
        false,
    );
    check(
        &|s, _| s.set_tool(Tool::Select),
        "leaving the Trace tool",
        false,
    );
    check(
        &|s, _| {
            s.set_trace_pick(Some(curvyo_editor_wasm::TracePickMode::Background));
        },
        "the pipette",
        false,
    );
    check(
        &|s, _| {
            s.add_trace_part();
        },
        "Add part",
        false,
    );
    check(
        &|s, image| {
            let other = images_of(&doc(s))
                .into_iter()
                .find(|&id| id != image)
                .unwrap();
            select(s, other);
        },
        "another image chosen",
        true,
    );
    // Picking a part: two parts first, the lasso drawn in part 2, part 1 picked.
    let (mut s, image) = lasso_on(false);
    s.add_trace_part();
    corners_of_l1(&mut s);
    assert!(s.set_trace_active_part(0));
    let mask = doc(&s).trace_mask(image).unwrap();
    assert_eq!(
        mask.parts[1].regions.len(),
        1,
        "closed into the part it was drawn in"
    );
    assert_eq!(mask.parts[0].regions.len(), 0);
    // And the trace of the auto-closed L1 is L1c's.
    let (mut t, _) = lasso_on(false);
    corners_of_l1(&mut t);
    t.set_trace_region_tool(TraceRegionTool::None);
    assert_eq!(t.trace_mask_json(), want.trace_mask_json());
}

/// Customer decision 2 and criterion 6: Escape discards a lasso of any length: nothing stored, no
/// step; the next Escape releases Lasso. Two corners when the tool ends store nothing; Backspace
/// removes the last corner; Ctrl+Z discards the lasso and undoes nothing else.
#[test]
fn c6_escape_and_ctrl_z_discard_backspace_removes_and_two_corners_store_nothing() {
    let (mut s, image) = lasso_on(false);
    corners_of_l1(&mut s);
    let steps = s.undo_len();
    key(&mut s, "Escape");
    assert_eq!((stored(&s, image).len(), s.undo_len()), (0, steps));
    assert_eq!(
        s.trace_region_tool(),
        TraceRegionTool::Lasso,
        "Lasso stays pressed"
    );
    key(&mut s, "Escape");
    assert_eq!(
        s.trace_region_tool(),
        TraceRegionTool::None,
        "the next Escape releases it"
    );
    assert_eq!(s.tool(), Tool::Trace);

    let (mut s, image) = lasso_on(false);
    click(&mut s, L1[0]);
    click(&mut s, L1[1]);
    let steps = s.undo_len();
    s.set_trace_region_tool(TraceRegionTool::Rect);
    assert_eq!(stored(&s, image).len(), 0, "two corners: dropped");
    assert_eq!(s.undo_len(), steps, "no History step");

    let (mut s, image) = lasso_on(false);
    corners_of_l1(&mut s);
    click(&mut s, (40.0, 60.0));
    key(&mut s, "Backspace");
    key(&mut s, "Enter");
    assert_eq!(
        stored(&s, image)[0].2,
        L1_PX.to_vec(),
        "Backspace took the fourth corner"
    );
    // Backspace on the only corner discards the lasso.
    click(&mut s, (80.0, 80.0));
    key(&mut s, "Backspace");
    assert!(!s.trace_lasso_open());
    assert_eq!(
        stored(&s, image).len(),
        1,
        "the lasso stays, Backspace deleted no region"
    );

    let (mut s, image) = lasso_on(false);
    corners_of_l1(&mut s);
    key(&mut s, "Enter");
    click(&mut s, (70.0, 10.0));
    click(&mut s, (90.0, 10.0));
    let steps = s.undo_len();
    key_with(&mut s, "z", true);
    assert!(!s.trace_lasso_open(), "Ctrl+Z discards the lasso");
    assert_eq!(s.undo_len(), steps, "and undoes nothing else");
    assert_eq!(stored(&s, image).len(), 1);
}

/// Criteria 7 and 17: a selected lasso moves (one step "Move region"), its corner moves (one step
/// "Move lasso corner"), Delete removes it ("Delete region"); the preview's mask follows each.
#[test]
fn c7_c17_a_selected_lasso_moves_its_corner_moves_and_delete_removes_it() {
    let (mut s, image) = l1c();
    s.set_trace_region_tool(TraceRegionTool::None);
    let first = s.trace_mask_json();
    // A drag inside moves it by 10 mm (30 px).
    let (a, b) = (pt(&s, (10.0, 20.0)), pt(&s, (20.0, 20.0)));
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
    assert!(s.trace_selected_region().is_some());
    assert_eq!(
        stored(&s, image)[0].2,
        vec![(30.0, 0.0), (210.0, 0.0), (30.0, 300.0)]
    );
    assert_eq!(labels(&s).last().unwrap(), "move_trace_region");
    let moved = s.trace_mask_json();
    assert_ne!(moved, first);
    // The second corner dragged to (90, 30) mm.
    let (c, d) = (pt(&s, (70.0, 0.0)), pt(&s, (90.0, 30.0)));
    s.pointer_down(c, false);
    s.pointer_hover(d, false, false);
    s.pointer_up(d, false, false);
    assert_eq!(
        stored(&s, image)[0].2,
        vec![(30.0, 0.0), (270.0, 90.0), (30.0, 300.0)]
    );
    assert_eq!(labels(&s).last().unwrap(), "move_trace_lasso_corner");
    assert_ne!(s.trace_mask_json(), moved);
    key(&mut s, "Delete");
    assert_eq!(stored(&s, image).len(), 0);
    assert_eq!(labels(&s).last().unwrap(), "delete_trace_region");
    assert_eq!(s.trace_mask_json(), "");
    s.undo();
    assert_eq!(stored(&s, image).len(), 1, "undo brings it back");
}

/// Criterion 8 with the lasso and criterion 23: a lasso finished while Subtract is pressed is
/// subtracted; a lasso that crosses itself is stored as drawn and masks by non-zero winding.
#[test]
fn c8_c23_a_subtracted_and_a_self_crossing_lasso() {
    let (mut s, image) = lasso_on(false);
    for c in [(0.0, 30.0), (50.0, 70.0), (50.0, 30.0), (0.0, 70.0)] {
        click(&mut s, c);
    }
    key(&mut s, "Enter");
    let region = s.trace_region().unwrap();
    assert!(region.keeps(pt(&s, (10.0, 50.0))) && region.keeps(pt(&s, (40.0, 50.0))));
    assert!(!region.keeps(pt(&s, (25.0, 35.0))));
    s.set_trace_subtract(true);
    for c in [(5.0, 45.0), (15.0, 45.0), (10.0, 55.0)] {
        click(&mut s, c);
    }
    key(&mut s, "Enter");
    assert!(stored(&s, image)[1].1, "stored as subtracted");
    assert!(!s.trace_region().unwrap().keeps(pt(&s, (10.0, 48.0))));
}

/// UX notes §6: a double click with Lasso pressed never creates paths; a lasso of one corner is
/// not stored.
#[test]
fn a_double_click_with_lasso_creates_no_paths() {
    let (mut s, image) = lasso_on(false);
    let p = pt(&s, (50.0, 50.0));
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    let hint = s.double_click_hint(p, false, false);
    assert_eq!(format!("{hint:?}"), "None");
    assert_eq!(stored(&s, image).len(), 0);
    assert!(!labels(&s).iter().any(|l| l == "trace_image"));
}

/// UX notes §5: a rectangle being drawn shows its size in mm by the pointer; Subtract shows the
/// Remove badge while a region is drawn, from the press or the first corner.
#[test]
fn the_size_readout_and_the_remove_badge_while_drawing() {
    let (mut s, _) = lasso_on(false);
    s.set_trace_subtract(true);
    assert!(!s.move_indicators().remove_badge, "nothing drawn yet");
    click(&mut s, (10.0, 10.0));
    assert!(
        s.move_indicators().remove_badge,
        "a subtracted lasso in progress"
    );
    key(&mut s, "Escape");
    assert!(!s.move_indicators().remove_badge);
    s.set_trace_region_tool(TraceRegionTool::Rect);
    let (a, b) = (pt(&s, (10.0, 10.0)), pt(&s, (52.3, 47.0)));
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    assert!(s.move_indicators().remove_badge);
    assert_eq!(s.live_readout().unwrap().text, "42.3 × 37.0 mm");
    s.pointer_up(b, false, false);
    assert!(!s.move_indicators().remove_badge && s.live_readout().is_none());
}

//! The Image operation kind in the session (`specs/0123-raster-engraving` criteria 1, 47, 48,
//! plan R8): the kind button and its operation, an image as its frame rectangle in Engrave, and
//! the Operations view's looks of each kind.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use curvyo_document_core::{Affine, FIRST_PAGE_ID, NewImage, NodeId};
use curvyo_laser_core::job::{LeafGeometry, MoveKind};

use super::raster_tests::{half_black_png, image_group, laser_page, place_image};
use super::{laser_with_square, rows, view};
use crate::session::Session;

fn row(rows: &serde_json::Value, id: NodeId) -> &serde_json::Value {
    rows["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == serde_json::to_value(id).unwrap())
        .unwrap()
}

fn numbers(s: &mut Session, id: NodeId) {
    s.edit_operation_setting(id, "main", "power", 50.0).unwrap();
    s.edit_operation_setting(id, "main", "speed", 6000.0)
        .unwrap();
}

/// Criterion 47's test: select an image, press Image: "Image 1" with Floyd-Steinberg; press
/// Engrave: "Engrave 1" hatches the image as its frame rectangle (criterion 1).
#[test]
fn criterion_47_the_image_button_makes_image_1_and_engrave_hatches_the_frame() {
    let mut s = laser_page();
    let image = place_image(
        &s,
        &half_black_png(40, 20),
        "Photo 1",
        (50.0, 50.0, 20.0, 10.0),
    );
    s.selection.set(&[image]);
    let raster = s.press_operation_kind("image", false).unwrap();
    let list = rows(&s);
    let r = row(&list, raster);
    assert_eq!(
        (r["name"].as_str(), r["kind"].as_str()),
        (Some("Image 1"), Some("image"))
    );
    assert_eq!(r["section"], "marking");
    assert_eq!(image_group(&s, raster)["dither"], "floyd_steinberg");
    let engrave = s.press_operation_kind("engrave", false).unwrap();
    assert_eq!(row(&rows(&s), engrave)["name"], "Engrave 1");
    numbers(&mut s, raster);
    numbers(&mut s, engrave);
    let job = s.laser_job().unwrap();
    let engrave_block = job.input.blocks.iter().find(|b| b.id == engrave).unwrap();
    assert!(
        matches!(&engrave_block.leaves[0].geometry, LeafGeometry::Outlines(o) if o.len() == 1),
        "the frame ring, no pixels: {:?}",
        engrave_block.leaves[0].geometry
    );
    let index = job
        .built
        .job
        .blocks
        .iter()
        .position(|b| b.id == engrave)
        .unwrap();
    let hatch = job
        .built
        .job
        .moves
        .iter()
        .filter(|m| m.block == index && matches!(m.kind, MoveKind::Burn { .. }))
        .count();
    assert!(hatch >= 20, "the 10 mm frame hatched at 0.1 mm: {hatch}");
    assert_eq!(job.built.job.sweeps.len(), 100, "the dots are Image's only");
    assert!(
        view(&s)["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| !c["text"].as_str().unwrap().contains("not a picture")),
        "an image never warns in any kind"
    );
}

/// Criterion 48: a Colour-filled rectangle in Image burns nothing, warns once naming the
/// operation and selecting the item, and takes the muted look.
#[test]
fn criterion_48_a_plain_shape_in_image_warns_and_is_drawn_muted() {
    let (mut s, square) = laser_with_square();
    s.selection.set(&[square]);
    let op = s.press_operation_kind("image", true).unwrap();
    s.selection.set(&[]);
    numbers(&mut s, op);
    let checks = view(&s)["checks"].as_array().unwrap().clone();
    let line = checks
        .iter()
        .find(|c| c["text"] == "1 item in Image 1 is not a picture and does not burn.")
        .unwrap_or_else(|| panic!("{checks:?}"));
    assert_eq!(line["severity"], "warn");
    assert_eq!(line["target"]["kind"], "items");
    assert_eq!(
        line["target"]["unfold"],
        serde_json::to_value(op).unwrap(),
        "the line unfolds the operation's row too (UX review U6)"
    );
    let objects = s.render_objects();
    let (looked, overlay) = s.operation_looked_artwork(&objects, s.view()).unwrap();
    let drawn = looked.iter().find(|o| o.id() == square).unwrap();
    assert!(drawn.style().stroke.enabled, "the muted dashed outline");
    assert!(!drawn.style().fill.enabled);
    let _ = overlay;
    assert!(
        s.laser_job()
            .unwrap()
            .built
            .job
            .moves
            .iter()
            .all(|m| !matches!(m.kind, MoveKind::Burn { .. } | MoveKind::Sweep { .. }))
    );
}

/// Criterion 1: the Operations view hatches an image in Engrave over its frame.
#[test]
fn criterion_1_the_operations_view_hatches_an_image_in_engrave() {
    let mut s = laser_page();
    let image = place_image(
        &s,
        &half_black_png(8, 8),
        "Photo 1",
        (50.0, 50.0, 20.0, 10.0),
    );
    s.selection.set(&[image]);
    let op = s.press_operation_kind("image", true).unwrap();
    numbers(&mut s, op);
    let hatch = |s: &Session| {
        let objects = s.render_objects();
        s.operation_looked_artwork(&objects, s.view())
            .unwrap()
            .1
            .triangle_count()
    };
    let image_only = hatch(&s);
    s.selection.set(&[image]);
    s.press_operation_kind("engrave", true).unwrap();
    s.selection.set(&[]);
    assert!(hatch(&s) > image_only, "the Engrave hatch over the frame");
}

/// Criterion 48: an image on a placed page in Image burns and never warns; the placed page's
/// frame is its container, no vector shape assigned to Image.
#[test]
fn criterion_48_an_image_on_a_placed_page_does_not_warn() {
    let mut s = laser_page();
    let laser = s.active_page().id;
    let first = s
        .document
        .pages()
        .into_iter()
        .find(|p| p.id == FIRST_PAGE_ID)
        .unwrap();
    s.document
        .add_images(&[NewImage {
            bytes: Arc::from(half_black_png(40, 20).as_slice()),
            placement: Affine::new(
                20.0,
                0.0,
                0.0,
                10.0,
                first.origin.x + 20.0,
                first.origin.y + 20.0,
            ),
            name: "Photo 1".to_owned(),
        }])
        .unwrap();
    assert!(s.activate_page(&laser));
    assert!(s.place_page_on_active(FIRST_PAGE_ID));
    let placement = s.selected_placement().unwrap();
    s.selection.set(&[placement]);
    let op = s.press_operation_kind("image", true).unwrap();
    s.selection.set(&[]);
    numbers(&mut s, op);
    assert!(
        !s.laser_job().unwrap().built.job.sweeps.is_empty(),
        "the image burns"
    );
    let checks = view(&s)["checks"].as_array().unwrap().clone();
    assert!(
        checks
            .iter()
            .all(|c| !c["text"].as_str().unwrap().contains("not a picture")),
        "{checks:?}"
    );
}

/// The colours the Operations view's overlay (hatch and chips) draws, on a 2,000 px canvas whose
/// top-left is the laser page's at 4 px per mm.
fn overlay_colours(s: &mut Session) -> Vec<curvyo_render_core::RgbaColor> {
    s.resize_viewport(2000.0, 2000.0);
    let origin = s.active_manufacturing().unwrap().page.unwrap().origin;
    let view = curvyo_document_core::ViewTransform::new(4.0, origin);
    let objects = s.render_objects();
    let (_, overlay) = s.operation_looked_artwork(&objects, view).unwrap();
    overlay.triangles.iter().map(|v| v.color).collect()
}

/// A laser page with an image and its linked clone 30 mm right; the session and the clone.
fn image_and_clone() -> (Session, NodeId) {
    let mut s = laser_page();
    let image = place_image(
        &s,
        &half_black_png(8, 8),
        "Photo 1",
        (50.0, 50.0, 20.0, 10.0),
    );
    s.selection.set(&[image]);
    s.linked_clone();
    let clone = s.selection.ids()[0];
    s.document
        .translate_objects(&[clone], curvyo_document_core::Vec2::new(30.0, 0.0))
        .unwrap();
    s.selection.set(&[]);
    (s, clone)
}

/// Criteria 1 and 46 (b): a linked clone of an image is hatched over its frame in Engrave and
/// carries the Image symbol in Image, as the image itself does.
#[test]
fn criteria_1_46b_a_picture_clone_is_hatched_in_engrave_and_chipped_in_image() {
    use curvyo_render_core::{OP_CHIP_GROUND, OP_ENGRAVE_HATCH, OP_INK};
    let (mut s, clone) = image_and_clone();
    s.selection.set(&[clone]);
    let engrave = s.press_operation_kind("engrave", true).unwrap();
    s.selection.set(&[]);
    numbers(&mut s, engrave);
    let colours = overlay_colours(&mut s);
    assert!(
        colours.contains(&OP_ENGRAVE_HATCH),
        "the hatch over the clone's frame"
    );
    assert!(colours.contains(&OP_CHIP_GROUND), "the Engrave chip");
    let (mut s, clone) = image_and_clone();
    let image = image_op_of(&mut s, clone);
    numbers(&mut s, image);
    let colours = overlay_colours(&mut s);
    assert!(!colours.contains(&OP_ENGRAVE_HATCH), "dots, no hatch");
    assert!(
        colours.contains(&OP_CHIP_GROUND) && colours.contains(&OP_INK),
        "the Image chip in ink"
    );
}

fn image_op_of(s: &mut Session, item: NodeId) -> NodeId {
    s.selection.set(&[item]);
    let op = s.press_operation_kind("image", true).unwrap();
    s.selection.set(&[]);
    op
}

/// Criterion 48 (UX E10): a vector shape in Image keeps its Image chip, in muted ink.
#[test]
fn criterion_48_a_plain_shape_in_image_keeps_a_muted_image_chip() {
    use curvyo_render_core::{OP_INK, OP_MUTED_INK};
    let (mut s, square) = laser_with_square();
    let op = image_op_of(&mut s, square);
    numbers(&mut s, op);
    let colours = overlay_colours(&mut s);
    assert!(colours.contains(&OP_MUTED_INK), "the muted Image chip");
    assert!(!colours.contains(&OP_INK), "nothing in ink");
}

/// `0101` UX notes 13.2: an item of an Output off operation keeps its kind's chip, muted (a Cut
/// symbol is then not red).
#[test]
fn an_output_off_item_keeps_a_muted_chip() {
    use curvyo_render_core::{OP_CUT, OP_MUTED_INK};
    let (mut s, square) = laser_with_square();
    let cut = super::cut(&mut s, &[square]);
    assert!(
        overlay_colours(&mut s).contains(&OP_CUT),
        "the red Cut chip"
    );
    assert!(s.set_operation_switch(cut, curvyo_document_core::OperationSwitch::Output, false));
    let colours = overlay_colours(&mut s);
    assert!(colours.contains(&OP_MUTED_INK));
    assert!(!colours.contains(&OP_CUT), "muted, not red");
}

/// UX review U8: the History row of an assignment of an image counts "1 image", not "1 path".
#[test]
fn the_history_row_of_an_assigned_image_counts_one_image() {
    let mut s = laser_page();
    let image = place_image(
        &s,
        &half_black_png(8, 8),
        "Photo 1",
        (50.0, 50.0, 20.0, 10.0),
    );
    image_op_of(&mut s, image);
    let len = s.history_len();
    let rows: serde_json::Value = serde_json::from_str(&s.history_rows_json(0, len)).unwrap();
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["name"]
                .as_str()
                .is_some_and(|n| n.starts_with("Assign to Image"))
        })
        .unwrap_or_else(|| panic!("{rows}"));
    assert_eq!(row["summary"], "1 image", "{row}");
}

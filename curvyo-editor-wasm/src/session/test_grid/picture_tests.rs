//! The Image grid's Picture pick and Strength axis in the session (`specs/0135` criteria 51,
//! 52; ADR 0073 slice 2 decisions a to e).

use std::sync::Arc;

use curvyo_document_core::{Affine, NewImage, ObjectSnapshot, Point};

use super::super::Session;
use super::super::image_adjust::tests::ramp_png;
use super::tests::{laser_with_fonts, recipe_of, undo_label};

fn bar(s: &Session) -> serde_json::Value {
    serde_json::to_value(s.test_grid_view().unwrap()).unwrap()
}

/// A laser page with a 40 x 20 px picture with the Photo preset, selected, and the Test grid
/// tool on an Image draft.
fn picture_and_tool() -> (Session, curvyo_document_core::NodeId) {
    let mut s = laser_with_fonts();
    let origin = s.active_page().origin;
    let picture = s
        .document
        .add_images(&[NewImage {
            bytes: Arc::from(ramp_png(40, 20).as_slice()),
            placement: Affine::new(8.0, 0.0, 0.0, 4.0, origin.x + 150.0, origin.y + 150.0),
            name: "Photo 1".to_owned(),
        }])
        .unwrap()[0];
    s.selection.set(&[picture]);
    assert!(s.pick_adjust_preset("photo"));
    s.set_tool(super::super::Tool::TestGrid);
    s.set_test_grid_kind("image").unwrap();
    (s, picture)
}

#[test]
fn c51_the_picture_pick_copies_the_picture_into_the_cells_with_the_cells_strength() {
    let (mut s, picture) = picture_and_tool();
    assert_eq!(bar(&s)["picture"]["value"], "ramp");
    assert_eq!(bar(&s)["picture"]["selected_offered"], true);
    assert_eq!(
        bar(&s)["picture"]["strength_offered"],
        false,
        "Ramp: no Strength"
    );
    assert!(!bar(&s)["dither"].is_null(), "the Ramp has its Dither");
    assert_eq!(s.set_test_grid_picture("selected"), Ok(true));
    assert!(
        bar(&s)["dither"].is_null(),
        "P32: no Dither with Selected picture"
    );
    assert_eq!(bar(&s)["picture"]["strength_offered"], true);
    let offered = bar(&s)["columns"]["offered"].clone();
    assert!(offered.as_array().unwrap().contains(&"strength".into()));
    assert_eq!(s.set_test_grid_axis("columns", "strength"), Ok(true));
    let before = s.document.shown_adjustments(picture).unwrap();
    let origin = s.active_page().origin;
    let group = s
        .test_grid_press(Some(Point::new(origin.x + 1.0, origin.y + 1.0)))
        .unwrap();
    assert_eq!(undo_label(&mut s), "Add test grid");
    let recipe = recipe_of(&s, group);
    let read = s.object_read();
    let cells: Vec<_> = s
        .document
        .leaves_of(group)
        .into_iter()
        .filter_map(|leaf| match read.get(leaf) {
            Some(ObjectSnapshot::Image(image)) => Some(image.clone()),
            _ => None,
        })
        .collect();
    let columns = recipe
        .test
        .axis(curvyo_laser_core::grids::Side::Columns)
        .count();
    let rows = recipe
        .test
        .axis(curvyo_laser_core::grids::Side::Rows)
        .count();
    assert_eq!(cells.len(), usize::from(columns) * usize::from(rows));
    let mut strengths: Vec<u8> = cells.iter().map(|c| c.adjustments.strength).collect();
    strengths.sort_unstable();
    strengths.dedup();
    assert_eq!(
        strengths,
        [0, 25, 50, 75, 100],
        "Min 0, Max 100, Count 5 (P25)"
    );
    assert!(
        cells
            .iter()
            .all(|c| c.adjustments.preset.as_deref() == Some("photo")
                && c.adjustments.dither == before.adjustments.dither)
    );
    assert_eq!(
        s.document.shown_adjustments(picture).unwrap(),
        before,
        "the picture itself is not changed"
    );
    // The selected grid's bar keeps the held picture; every cell burns (decision d).
    s.select_grid(group);
    assert_eq!(bar(&s)["picture"]["value"], "selected");
    assert_eq!(bar(&s)["picture"]["selected_offered"], true);
    let job = s.laser_job().unwrap();
    assert_ne!(job.built.job.sweeps.len(), 0, "every cell burns");
    // Back to the Ramp drops the Strength axis.
    assert_eq!(s.set_test_grid_picture("ramp"), Ok(true));
    let group = s.selected_test_grid().unwrap();
    let recipe = recipe_of(&s, group);
    assert_ne!(
        recipe
            .test
            .axis(curvyo_laser_core::grids::Side::Columns)
            .param(),
        curvyo_laser_core::grids::Param::Strength
    );
}

#[test]
fn c51_without_a_picture_the_selected_segment_is_absent_and_refused() {
    let mut s = laser_with_fonts();
    s.set_tool(super::super::Tool::TestGrid);
    s.set_test_grid_kind("image").unwrap();
    assert_eq!(bar(&s)["picture"]["selected_offered"], false);
    assert_eq!(s.set_test_grid_picture("selected"), Ok(false));
    s.set_test_grid_kind("engrave").unwrap();
    assert!(bar(&s)["picture"].is_null(), "Image grids only");
}

/// The cell images of the selected grid.
fn cells(s: &Session) -> Vec<curvyo_document_core::ImageSnapshot> {
    let group = s.selected_test_grid().unwrap();
    let read = s.object_read();
    s.document
        .leaves_of(group)
        .into_iter()
        .filter_map(|leaf| match read.get(leaf) {
            Some(ObjectSnapshot::Image(image)) => Some(image.clone()),
            _ => None,
        })
        .collect()
}

/// Architect review A4: a cropped picture's cells fit its visible part into the square, not
/// the whole picture.
#[test]
fn c51_a_cropped_pictures_cells_fit_its_visible_part() {
    let (mut s, _) = picture_and_tool();
    // The 40 x 20 px picture cropped to its left quarter: 10 x 20 px, upright.
    s.set_tool(super::super::Tool::Select);
    assert!(s.add_adjust_step("crop"));
    assert!(s.set_crop_rect(0, 0, 25, 100));
    s.close_crop_bar();
    s.set_tool(super::super::Tool::TestGrid);
    s.set_test_grid_kind("image").unwrap();
    assert_eq!(s.set_test_grid_picture("selected"), Ok(true));
    let origin = s.active_page().origin;
    s.test_grid_press(Some(Point::new(origin.x + 1.0, origin.y + 1.0)))
        .unwrap();
    let side = recipe_of(&s, s.selected_test_grid().unwrap())
        .test
        .cell()
        .as_mm();
    for cell in cells(&s) {
        let c = cell.corners();
        let (w, h) = (c[1].x - c[0].x, c[3].y - c[0].y);
        assert!(
            (h - side).abs() < 1e-6,
            "the visible part fills the height: {h} of {side}"
        );
        assert!((w - side / 2.0).abs() < 1e-6, "half as wide: {w}");
    }
}

/// Architect review A5 (Proposal): with the Strength axis switched off again the cells burn
/// at Strength 50; a new Picture pick takes the picture's own.
#[test]
fn c51_strength_after_the_axis_is_switched_off_is_50() {
    let (mut s, _) = picture_and_tool();
    assert_eq!(s.set_test_grid_picture("selected"), Ok(true));
    assert_eq!(s.set_test_grid_axis("columns", "strength"), Ok(true));
    let origin = s.active_page().origin;
    let group = s
        .test_grid_press(Some(Point::new(origin.x + 1.0, origin.y + 1.0)))
        .unwrap();
    s.set_tool(super::super::Tool::Select);
    s.select_grid(group);
    assert_eq!(s.set_test_grid_axis("columns", "power"), Ok(true));
    let strengths: Vec<u8> = cells(&s).iter().map(|c| c.adjustments.strength).collect();
    assert_ne!(strengths.len(), 0);
    assert!(strengths.iter().all(|&v| v == 50), "{strengths:?}");
}

//! Raster items in the session (`specs/0123-raster-engraving` Part C): the job input's pixels
//! and gradients, the Image settings group, the checks' names and targets, and the dots the
//! Operations and Toolpath views draw.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::assert_is_empty
)]

use std::sync::Arc;

use curvyo_document_core::{Affine, NewImage, NodeId};
use curvyo_image_core::{WorkingImage, encode_png};
use curvyo_laser_core::job::LeafGeometry;

use super::{laser_with_square, rows, undo_label, view};
use crate::session::Session;

/// A `width` x `height` picture, black on the left half, white on the right.
pub(super) fn half_black_png(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..height {
        for x in 0..width {
            let g = if x < width / 2 { 0 } else { 255 };
            pixels.extend_from_slice(&[g, g, g, 255]);
        }
    }
    encode_png(&WorkingImage {
        width,
        height,
        pixels,
        original_size: (width, height),
        truncated: false,
        animated: false,
    })
    .unwrap()
}

/// Places `bytes` as an image named `name` of `w` x `h` mm at `(x, y)` mm from the page corner.
pub(super) fn place_image(
    s: &Session,
    bytes: &[u8],
    name: &str,
    (x, y, w, h): (f64, f64, f64, f64),
) -> NodeId {
    let origin = s.active_manufacturing().unwrap().page.unwrap().origin;
    s.document
        .add_images(&[NewImage {
            bytes: Arc::from(bytes),
            placement: Affine::new(w, 0.0, 0.0, h, origin.x + x, origin.y + y),
            name: name.to_owned(),
        }])
        .unwrap()[0]
}

/// An empty laser page.
pub(super) fn laser_page() -> Session {
    let mut s = Session::new(290_002);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    s
}

/// An Image operation holding `items` with own numbers 50 %, 6,000 mm/min.
pub(super) fn image_op(s: &mut Session, items: &[NodeId]) -> NodeId {
    s.selection.set(items);
    let id = s.press_operation_kind("image", true).unwrap();
    s.selection.set(&[]);
    s.edit_operation_setting(id, "main", "power", 50.0).unwrap();
    s.edit_operation_setting(id, "main", "speed", 6000.0)
        .unwrap();
    id
}

/// A laser page with a 20 x 10 mm half-black image in an Image operation; the session, the image and the
/// operation.
pub(super) fn engraved_image() -> (Session, NodeId, NodeId) {
    let mut s = laser_page();
    let image = place_image(
        &s,
        &half_black_png(40, 20),
        "Photo 1",
        (50.0, 50.0, 20.0, 10.0),
    );
    let op = image_op(&mut s, &[image]);
    (s, image, op)
}

#[test]
fn criterion_1_an_image_in_image_reaches_the_job_with_its_pixels_and_runs_as_sweeps() {
    let (s, image, _) = engraved_image();
    let job = s.laser_job().unwrap();
    let leaf = job
        .input
        .blocks
        .iter()
        .flat_map(|b| &b.leaves)
        .find(|l| l.node == image);
    let Some(LeafGeometry::Image {
        pixels: Some(pixels),
        ..
    }) = leaf.map(|l| &l.geometry)
    else {
        panic!("the image with its working image: {leaf:?}")
    };
    assert_eq!((pixels.width, pixels.height), (40, 20));
    assert_eq!(s.output.pixels.len(), 1, "the working image is held once");
    assert_eq!(job.built.job.sweeps.len(), 100, "a sweep per row of 0.1 mm");
    let burned: usize = job.built.job.sweeps.iter().map(|s| s.burned_dots()).sum();
    assert_eq!(
        burned,
        100 * 100,
        "the black half burns, the white half not"
    );
}

#[test]
fn criterion_3_a_gradient_filled_rectangle_is_a_gradient_leaf_and_engraves_as_dots() {
    let (mut s, square) = laser_with_square();
    s.selection.set(&[square]);
    assert!(s.set_fill_kind("gradient"));
    image_op(&mut s, &[square]);
    let job = s.laser_job().unwrap();
    let leaf = job
        .input
        .blocks
        .iter()
        .flat_map(|b| &b.leaves)
        .find(|l| l.node == square);
    assert!(
        matches!(
            leaf.map(|l| &l.geometry),
            Some(LeafGeometry::Gradient { .. })
        ),
        "{leaf:?}"
    );
    assert!(!job.built.job.sweeps.is_empty(), "the shade burns");
}

#[test]
fn criterion_36_an_image_that_cannot_be_read_is_skipped_and_warned_once() {
    let mut s = laser_page();
    let image = place_image(&s, b"not a picture", "Photo 1", (50.0, 50.0, 20.0, 10.0));
    image_op(&mut s, &[image]);
    let job = s.laser_job().unwrap();
    assert!(job.built.job.sweeps.is_empty());
    let texts: Vec<String> = view(&s)["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["text"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        texts.contains(&"1 image cannot be read and is skipped.".to_owned()),
        "{texts:?}"
    );
    let _ = rows(&s);
}

/// The unfolded body's Image settings group of operation `op` in the Operations tab's JSON.
pub(super) fn image_group(s: &Session, op: NodeId) -> serde_json::Value {
    let rows = rows(s);
    let row = rows["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == serde_json::to_value(op).unwrap())
        .cloned()
        .unwrap();
    row["settings"]["options"]["image"].clone()
}

/// Superseded by the Dither on the picture (2026-11-20): the group shows its tone fields on every
/// Image row; its own Dither only with a gradient fill, Lowest power only while an item burns
/// Grey (ADR 0072 amendment A4, Frage 284 B).
#[test]
fn criterion_11_the_image_settings_show_on_every_image_row_and_never_on_engrave() {
    let (mut s, square) = laser_with_square();
    let op = image_op(&mut s, &[square]);
    let group = image_group(&s, op);
    assert!(group["dither"].is_null(), "no gradient fill, no Dither");
    assert!(group["lowest_power"].is_null(), "nothing burns Grey");
    assert_eq!(group["fields"][2]["range"], "Gamma is 0.10 to 4.00.");
    let image = place_image(&s, &half_black_png(8, 8), "Photo 1", (50.0, 50.0, 8.0, 8.0));
    s.selection.set(&[image]);
    let engrave = s.press_operation_kind("engrave", true).unwrap();
    assert!(
        image_group(&s, engrave).is_null(),
        "an Engrave never shows it"
    );
    let rows = rows(&s);
    let hatch = |id: NodeId| {
        rows["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == serde_json::to_value(id).unwrap())
            .unwrap()["settings"]["options"]["hatch"]
            .clone()
    };
    assert!(hatch(op)["crosshatch"].is_null(), "no Crosshatch on Image");
    assert_eq!(hatch(engrave)["crosshatch"], false);
}

#[test]
fn criterion_12_a_value_out_of_range_is_refused_and_a_valid_one_is_one_step() {
    let (mut s, image, op) = engraved_image();
    let steps = s.undo_len();
    assert_eq!(
        s.edit_operation_option(op, "gamma", 4.01),
        Err("Gamma is 0.10 to 4.00.".to_owned())
    );
    assert_eq!(s.undo_len(), steps, "nothing written");
    assert_eq!(s.edit_operation_option(op, "gamma", 1.234), Ok(true));
    assert_eq!(
        image_group(&s, op)["fields"][2]["value"],
        1.23,
        "rounded to its step"
    );
    assert_eq!(undo_label(&mut s), "Edit settings");
    assert!(s.set_operation_dither(op, "grey"));
    assert!(
        image_group(&s, op)["lowest_power"].is_null(),
        "superseded by the Dither on the picture (2026-11-20): the operation's Grey does not reach \
         its picture"
    );
    s.selection.set(&[image]);
    assert!(s.set_picture_dither("grey"));
    assert_eq!(
        image_group(&s, op)["lowest_power"]["field"],
        "lowest_power",
        "the picture burns Grey: the operation's Lowest power"
    );
    assert!(
        !s.set_operation_dither(op, "grey"),
        "the same mode writes nothing"
    );
    assert!(!s.set_operation_dither(op, "halftone_x"));
    assert_eq!(undo_label(&mut s), "Edit adjustments");
    s.undo();
    assert_eq!(s.edit_operation_option(op, "invert", 1.0), Ok(true));
    assert_eq!(image_group(&s, op)["invert"], true);
    s.undo();
    assert_eq!(
        image_group(&s, op)["invert"],
        false,
        "undo restores the setting"
    );
}

/// The check lines of the output group.
fn checks(s: &Session) -> Vec<serde_json::Value> {
    view(s)["checks"].as_array().unwrap().clone()
}

#[test]
fn criterion_34_the_dot_limit_line_names_the_item_and_selects_it() {
    let (mut s, image, op) = engraved_image();
    s.edit_operation_option(op, "interval", 0.01).unwrap();
    let place =
        |s: &Session| place_image(s, &half_black_png(8, 8), "Big", (10.0, 10.0, 100.0, 100.0));
    let big = place(&s);
    s.selection.set(&[big]);
    s.press_operation_kind("image", false);
    let line = checks(&s)
        .into_iter()
        .find(|c| c["text"].as_str().unwrap().contains("would need"))
        .unwrap();
    assert_eq!(
        line["text"],
        "Big would need 100,000,000 dots; the limit is 50,000,000."
    );
    assert_eq!(line["severity"], "block");
    assert_eq!(line["button"], "Big");
    assert_eq!(line["target"]["ids"], serde_json::json!([big]));
    let _ = image;
}

/// Superseded by the Dither on the picture (2026-11-20): the picture burns Grey with its own
/// dither; the line stays the operation's and focuses its Lowest power (Frage 284 B).
#[test]
fn criterion_32_lowest_power_above_power_warns_and_focuses_lowest_power() {
    let (mut s, image, op) = engraved_image();
    s.selection.set(&[image]);
    assert!(s.set_picture_dither("grey"));
    s.edit_operation_option(op, "lowest_power", 60.0).unwrap();
    let line = checks(&s)
        .into_iter()
        .find(|c| c["text"] == "Image 1: Lowest power is above Power.")
        .unwrap();
    assert_eq!(line["severity"], "warn");
    assert_eq!(line["target"]["kind"], "operations");
    assert_eq!(line["target"]["focus"], "lowest_power");
}

/// The opacity of each raster dot picture the frame draws.
pub(super) fn dot_pictures(s: &mut Session) -> Vec<f64> {
    let mut list = s.frame_draw_list();
    let images = s.frame_images(&mut list);
    images
        .iter()
        .filter(|(asset, _)| s.raster_picture_image(asset).is_some())
        .map(|(asset, _)| {
            let quad = list.images.iter().find(|q| q.asset == *asset).unwrap();
            quad.opacity.get()
        })
        .collect()
}

#[test]
fn criterion_20_the_operations_view_draws_a_raster_item_as_its_dots_without_the_hatch() {
    let (mut s, _, op) = engraved_image();
    let pictures = dot_pictures(&mut s);
    assert_eq!(pictures.len(), 1, "one dot picture");
    let rendered = s.output.raster.rendered.get();
    let _ = dot_pictures(&mut s);
    assert_eq!(
        s.output.raster.rendered.get(),
        rendered,
        "a frame with no change renders nothing"
    );
    s.edit_operation_option(op, "brightness", 100.0).unwrap();
    let _ = dot_pictures(&mut s);
    assert_eq!(
        dot_pictures(&mut s).len(),
        0,
        "Brightness +100 % burns nothing: no dots to draw"
    );
}

#[test]
fn criterion_20_the_toolpath_view_draws_the_dots_and_no_line_over_the_rows() {
    let (mut s, _, _) = engraved_image();
    let page = s.active_page().id;
    s.toggle_toolpath_view(&page);
    let overlay = s.toolpath_overlay().unwrap();
    let job = s.laser_job().unwrap();
    let (look, _) = s.toolpath_look(&job);
    assert!(
        look.segments
            .iter()
            .all(|seg| seg.stroke != curvyo_render_core::ToolpathStroke::Engrave),
        "the sweeps are no lines"
    );
    assert_eq!(
        look.arrows.len(),
        1,
        "one arrowhead at the item's first row"
    );
    assert_eq!(look.rings.len(), 1);
    assert!(!overlay.triangles.is_empty());
    assert_eq!(dot_pictures(&mut s), vec![1.0]);
    s.set_play_position(0.0);
    assert_eq!(dot_pictures(&mut s)[0], 0.35, "not reached yet: pale");
}

#[test]
fn criterion_37_g_pictures_in_no_image_operation_decode_nothing_for_the_job() {
    let (mut s, square) = laser_with_square();
    let images: Vec<NodeId> = (0..20)
        .map(|i| {
            let x = 10.0 + f64::from(i) * 12.0;
            place_image(&s, &half_black_png(16, 16), "Photo", (x, 50.0, 10.0, 10.0))
        })
        .collect();
    s.selection.set(&images);
    let cut = s.press_operation_kind("cut", true).unwrap();
    s.edit_operation_setting(cut, "main", "power", 60.0)
        .unwrap();
    s.edit_operation_setting(cut, "main", "speed", 600.0)
        .unwrap();
    let _ = (view(&s), rows(&s), s.frame_draw_list());
    assert_eq!(s.output.pixels.len(), 0, "no image is decoded for the job");
    assert!(s.laser_job().unwrap().built.job.sweeps.is_empty());
    assert_eq!(s.output.raster.rendered.get(), 0, "no dot picture");
    let _ = square;
}

/// ADR 0069 §12 (architect review A9): the first read after an edit works a third of a tick's
/// dots, so an item of 160,000 dots waits for the ticks, and one tick of 300,000 works the rest.
#[test]
fn criterion_37d_the_first_read_works_a_third_of_a_tick() {
    let mut s = laser_page();
    let image = place_image(
        &s,
        &half_black_png(40, 40),
        "Photo 1",
        (20.0, 20.0, 40.0, 40.0),
    );
    let op = image_op(&mut s, &[image]);
    assert_eq!(s.edit_operation_option(op, "interval", 0.1), Ok(true));
    let _ = rows(&s);
    assert!(!s.job_pending(), "a first build runs at once");
    assert_eq!(s.edit_operation_option(op, "gamma", 1.5), Ok(true));
    assert!(
        s.job_pending(),
        "160,000 dots are more than the first read works"
    );
    assert!(!s.advance_job(), "one tick works the other 60,000");
    assert!(s.advance_job(), "the next builds the job");
    assert!(!s.job_pending());
}

/// ADR 0069 §12 (criteria 12, 37 d): an Image setting edit of an item over one tick's dots keeps
/// the last job in the views with the stale words, the host's ticks finish it, and the finished
/// job equals one built at once.
#[test]
fn criterion_37d_a_big_edit_shows_the_last_job_as_stale_until_the_ticks_finish_it() {
    let mut s = laser_page();
    let image = place_image(
        &s,
        &half_black_png(80, 60),
        "Photo 1",
        (20.0, 20.0, 80.0, 60.0),
    );
    let op = image_op(&mut s, &[image]);
    assert_eq!(s.edit_operation_option(op, "interval", 0.1), Ok(true));
    let _ = rows(&s);
    assert!(!s.job_pending(), "a first build runs at once");
    assert_eq!(s.edit_operation_option(op, "gamma", 1.5), Ok(true));
    assert!(s.job_pending(), "480,000 dots wait for the ticks");
    let first = rows(&s);
    assert_ne!(
        first["rows"][0]["run"]["time"], "…",
        "the last numbers stay for the first 300 ms (UX notes P35)"
    );
    s.show_stale_words();
    let stale = rows(&s);
    let row = &stale["rows"][0];
    assert_eq!(row["run"]["time"], "…", "{row}");
    let output = view(&s);
    assert_eq!(output["total"]["time"], "About …");
    assert_eq!(output["total"]["lengths"], "Burn … · travel …");
    let mut ticks = 1;
    while !s.advance_job() {
        ticks += 1;
        assert!(ticks < 100);
    }
    assert!(!s.job_pending());
    assert!(!s.advance_job(), "nothing left");
    let done = rows(&s);
    assert_ne!(done["rows"][0]["run"]["time"], "…");
    let built = s.laser_job().unwrap();
    let mut cache = curvyo_laser_core::job::PlanCache::default();
    let at_once = curvyo_laser_core::job::build_job(&built.input, &[], &mut cache);
    assert_eq!(built.built, at_once, "the ticks give the job built at once");
}

/// Criterion 37 (d) with the Toolpath view pressed: the view draws the full job, so the output
/// group reads that job too and never stays on the stale words (architect review S1).
#[test]
fn criterion_37d_with_the_toolpath_view_on_the_output_group_reads_the_full_job() {
    let mut s = laser_page();
    let image = place_image(
        &s,
        &half_black_png(80, 60),
        "Photo 1",
        (20.0, 20.0, 80.0, 60.0),
    );
    let op = image_op(&mut s, &[image]);
    assert_eq!(s.edit_operation_option(op, "interval", 0.1), Ok(true));
    let page = s.active_page().id;
    s.toggle_toolpath_view(&page);
    let _ = rows(&s);
    assert_eq!(s.edit_operation_option(op, "gamma", 1.5), Ok(true));
    s.show_stale_words();
    let output = view(&s);
    assert_ne!(output["total"]["time"], "About …", "{output}");
    let _ = dot_pictures(&mut s);
    assert!(!s.job_pending(), "the Toolpath view builds the job in full");
}

/// An image that cannot be read stays held as unreadable, so a later read of the page input
/// neither hashes nor decodes it again (architect review S4).
#[test]
fn criterion_36_an_unreadable_image_is_remembered_between_reads() {
    let mut s = laser_page();
    let image = place_image(&s, b"not a picture", "Photo 1", (50.0, 50.0, 20.0, 10.0));
    image_op(&mut s, &[image]);
    assert!(s.page_input().is_some());
    assert_eq!(
        s.output.pixels.len(),
        1,
        "the unreadable image is kept as such"
    );
    assert!(s.page_input().is_some());
    assert_eq!(s.output.pixels.len(), 1);
}

/// The canvas draws the working image the job decoded instead of decoding a second copy
/// (architect review S4).
#[test]
fn criterion_7_the_canvas_reuses_the_working_image_the_job_decoded() {
    let (mut s, image, _) = engraved_image();
    let job = s.laser_job().unwrap();
    let Some(LeafGeometry::Image {
        asset,
        pixels: Some(held),
        ..
    }) = job
        .input
        .blocks
        .iter()
        .flat_map(|b| &b.leaves)
        .find(|l| l.node == image)
        .map(|l| l.geometry.clone())
    else {
        panic!("the image leaf");
    };
    if s.operations_view_on() {
        let page = s.active_page().id;
        s.toggle_operations_view(&page);
    }
    let mut list = s.frame_draw_list();
    let images = s.frame_images(&mut list);
    let drawn = images
        .iter()
        .find(|(a, _)| *a == asset)
        .map(|(_, working)| Arc::clone(working));
    let drawn = drawn.expect("the canvas draws the image");
    assert!(Arc::ptr_eq(&drawn, &held), "one copy of the pixels");
}

/// Criterion 37 (d) in ticks: a tick that reads a new pending input works only that share, and
/// the job is built in a tick of its own after the last share (architect review S7, nit).
#[test]
fn criterion_37d_each_tick_works_one_share_or_builds() {
    let mut s = laser_page();
    let image = place_image(
        &s,
        &half_black_png(80, 60),
        "Photo 1",
        (20.0, 20.0, 80.0, 60.0),
    );
    let op = image_op(&mut s, &[image]);
    assert_eq!(s.edit_operation_option(op, "interval", 0.1), Ok(true));
    let _ = rows(&s);
    assert_eq!(s.edit_operation_option(op, "gamma", 1.5), Ok(true));
    let ticks: Vec<bool> = (0..5).map(|_| s.advance_job()).collect();
    assert_eq!(
        ticks,
        [false, false, false, true, false],
        "the first read's 100,000 dots, 300,000, the other 80,000, the build, nothing"
    );
}

/// UX notes 3.1 (UX review U2): an Image operation with no power and speed still shows its raster items'
/// dots in the Operations view, at full opacity.
#[test]
fn criterion_20_an_image_operation_without_numbers_still_draws_its_dots() {
    let mut s = laser_page();
    let image = place_image(
        &s,
        &half_black_png(40, 20),
        "Photo 1",
        (50.0, 50.0, 20.0, 10.0),
    );
    s.selection.set(&[image]);
    let _ = s.press_operation_kind("image", true).unwrap();
    s.selection.set(&[]);
    if !s.operations_view_on() {
        let page = s.active_page().id;
        s.toggle_operations_view(&page);
    }
    assert_eq!(dot_pictures(&mut s), [1.0], "the dots, not dimmed");
}

/// UX notes 3.1 (UX review U2): a Not assigned image in the Operations view shows its picture
/// dimmed under the dashed frame.
#[test]
fn criterion_20_a_not_assigned_image_shows_its_dimmed_picture() {
    let s = laser_page();
    let image = place_image(
        &s,
        &half_black_png(40, 20),
        "Photo 1",
        (50.0, 50.0, 20.0, 10.0),
    );
    let mut s = s;
    if !s.operations_view_on() {
        let page = s.active_page().id;
        s.toggle_operations_view(&page);
    }
    let list = s.frame_draw_list();
    let quad = list
        .images
        .iter()
        .find(|q| s.raster_picture_image(&q.asset).is_none())
        .expect("the picture is drawn");
    assert!(quad.opacity.get() < 0.5, "dimmed: {}", quad.opacity.get());
    let _ = image;
}

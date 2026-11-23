//! Slice 2 of the Adjustments group in the session (`specs/0135` criteria 3, 5 to 7, 10, 14, 30):
//! Crop, Resize, Channel mix and Curves rows, the job reading the cropped picture, and Copy and
//! Paste adjustments.

use curvyo_document_core::{ObjectSnapshot, Vec2};

use super::tests::{dots, engraved, laser_page, place, undo_label};
use super::view::StepBodyView;

fn burned(s: &crate::session::Session) -> usize {
    dots(s).iter().flatten().filter(|&&level| level > 0).count()
}

fn frame_width(s: &crate::session::Session, id: curvyo_document_core::NodeId) -> f64 {
    let ObjectSnapshot::Image(image) = s.document.object(id).unwrap() else {
        panic!("an image");
    };
    let c = image.corners();
    c[1].x - c[0].x
}

/// Criteria 3, 5, 14: Crop and Resize sit in row 2 of the add toolbar and run with a preset; a
/// new Resize starts at the picture's width and states the working image; Crop narrows the
/// drawn frame and the job burns only the kept part; each edit is one step.
#[test]
fn c5_crop_narrows_the_frame_and_the_dots_and_resize_starts_at_the_width() {
    let (mut s, image, _) = engraved();
    let whole = burned(&s);
    let width = frame_width(&s, image);
    assert!(s.add_adjust_step("crop"));
    assert_eq!(undo_label(&mut s), "Edit adjustments");
    assert_eq!(s.edit_adjust_field(0, "width_pct", 50.0), Ok(true));
    assert!((frame_width(&s, image) - width / 2.0).abs() < 1e-9);
    let cropped = burned(&s);
    // The kept left half is the darker half of the ramp: more than a third, fewer than all.
    assert!(
        cropped * 10 < whole * 8 && cropped * 10 > whole * 3,
        "{cropped} of {whole}"
    );
    assert_eq!(
        s.edit_adjust_field(0, "left_pct", 60.0),
        Err("Left is 0 to 50 %.".to_owned()),
        "UX-2: the range as it stands"
    );
    assert!(s.add_adjust_step("resize"));
    let view = s.adjustments_view().unwrap();
    let resize = &view.steps[1];
    assert_eq!(resize.kind, "resize");
    assert_eq!(resize.fields[0].value, 40.0, "the picture's width");
    assert_eq!(
        resize.body,
        Some(StepBodyView::Resize {
            height_px: 20,
            line: "Working image 40 × 20 px".to_owned()
        })
    );
    // UX review: the line follows a Width edit (the size the Resize gives).
    assert_eq!(s.edit_adjust_field(1, "width_px", 20.0), Ok(true));
    assert_eq!(
        s.adjustments_view().unwrap().steps[1].body,
        Some(StepBodyView::Resize {
            height_px: 10,
            line: "Working image 20 × 10 px".to_owned()
        })
    );
    assert_eq!(s.edit_adjust_field(1, "width_px", 40.0), Ok(true));
    assert!(
        !resize.up && !resize.down,
        "the geometry band keeps its order"
    );
    // Criterion 30: with a preset the geometry rows run and are not "Preset in use".
    assert!(s.pick_adjust_preset("photo"));
    let view = s.adjustments_view().unwrap();
    assert!(view.steps.iter().all(|r| !r.preset_in_use));
    // Slice 3: row 2 also holds the three AI steps.
    assert!(view.add_tone.is_empty() && view.add_geometry.len() == 5);
    let derived = view.derived.unwrap().rows;
    assert!(
        derived
            .iter()
            .all(|r| r.kind != "crop" && r.kind != "resize")
    );
    assert!((frame_width(&s, image) - width / 2.0).abs() < 1e-9);
}

/// Criterion 29 with slice 2: Make editable of a cropped preset picture burns the same dots and
/// keeps the Crop row.
#[test]
fn c29_make_editable_keeps_the_crop_and_the_dots() {
    let (mut s, _, _) = engraved();
    s.add_adjust_step("crop");
    s.edit_adjust_field(0, "height_pct", 50.0).unwrap();
    s.edit_adjust_field(0, "top_pct", 25.0).unwrap();
    s.pick_adjust_preset("graphics");
    let before = dots(&s);
    assert!(s.make_adjust_editable());
    let view = s.adjustments_view().unwrap();
    assert_eq!(view.steps[0].kind, "crop");
    assert_eq!(dots(&s), before);
}

/// Criteria 5 to 7: Channel mix and Curves rows show their strip and graph; a Channel preset and
/// a curve point are one step each and change the dots; Denoise before Channel mix is kept.
#[test]
fn c5_channel_mix_and_curves_rows() {
    let (mut s, _, _) = engraved();
    let plain = dots(&s);
    assert!(s.add_adjust_step("channel_mix"));
    let view = s.adjustments_view().unwrap();
    let Some(StepBodyView::ChannelMix { presets }) = &view.steps[0].body else {
        panic!("a Channel preset strip");
    };
    assert!(presets[0].pressed && presets[0].cell == "Luminance");
    assert_eq!(presets[1].name, "Red filter");
    assert!(s.set_adjust_channel_preset(0, 1));
    assert_eq!(undo_label(&mut s), "Edit adjustments");
    let Some(StepBodyView::ChannelMix { presets }) = &s.adjustments_view().unwrap().steps[0].body
    else {
        panic!("a Channel preset strip");
    };
    assert!(presets[1].pressed && !presets[0].pressed);
    // A grey ramp: Red filter changes nothing (criterion 6).
    assert_eq!(dots(&s), plain);
    assert!(s.add_adjust_step("curves"));
    assert!(s.add_adjust_curve_point(1, 50, 20));
    let view = s.adjustments_view().unwrap();
    assert_eq!(
        view.steps[1].body,
        Some(StepBodyView::Curve {
            points: vec![[0, 0], [50, 20], [100, 100]]
        })
    );
    assert_ne!(dots(&s), plain, "the curve darkens the middle");
    assert!(s.move_adjust_curve_point(1, 1, 60, 30));
    assert!(s.remove_adjust_curve_point(1, 1));
    assert!(!s.remove_adjust_curve_point(1, 0), "an end point stays");
    assert!(!s.straighten_adjust_curve(1), "straight already");
    assert!(s.add_adjust_step("denoise"));
    assert!(s.move_adjust_step(1, true), "Denoise before Channel mix");
    let kinds: Vec<_> = s
        .adjustments_view()
        .unwrap()
        .steps
        .iter()
        .map(|r| r.kind)
        .collect();
    assert_eq!(kinds, ["denoise", "channel_mix", "curves"]);
}

/// Criterion 10: Copy adjustments holds a picture's Preset, Strength, stack and Dither; Paste
/// writes them onto another picture in one step "Paste adjustments"; onto a following clone it
/// gives the clone its own and leaves the original as it was.
#[test]
fn c10_copy_and_paste_adjustments() {
    let (mut s, first, _) = engraved();
    assert!(
        !s.adjustments_view().unwrap().paste_enabled,
        "greyed before a Copy"
    );
    s.pick_adjust_preset("newspaper");
    s.set_adjust_strength(80.0).unwrap();
    s.add_adjust_step("crop");
    let copied = s.document.shown_adjustments(first).unwrap().adjustments;
    assert!(s.copy_adjustments());
    assert!(s.adjustments_view().unwrap().paste_enabled);
    let second = place(&s, 8.0, 4.0);
    s.document
        .translate_objects(&[second], Vec2::new(0.0, 20.0))
        .unwrap();
    s.selection.set(&[second]);
    assert!(s.paste_adjustments());
    assert_eq!(undo_label(&mut s), "Paste adjustments");
    assert_eq!(
        s.document.shown_adjustments(second).unwrap().adjustments,
        copied
    );
    assert!(!s.paste_adjustments(), "the same again changes nothing");
    // Onto a following clone of a third, plain picture: the clone owns them, the original not.
    let mut t = laser_page();
    let original = place(&t, 8.0, 4.0);
    t.selection.set(&[original]);
    t.pick_adjust_preset("photo");
    let before = t.document.shown_adjustments(original).unwrap();
    t.selection.set(&[original]);
    t.linked_clone();
    let clone = t.selection.ids()[0];
    t.image_adjust.copied = s.image_adjust.copied.clone();
    t.selection.set(&[clone]);
    assert!(t.paste_adjustments());
    assert_eq!(t.adjustments_view().unwrap().clone_row, Some("own"));
    assert_eq!(t.document.shown_adjustments(original).unwrap(), before);
    assert_eq!(
        t.document
            .shown_adjustments(clone)
            .unwrap()
            .adjustments
            .preset
            .as_deref(),
        Some("newspaper")
    );
}

/// `ux-notes.md` §3.9: adding a Crop opens the Crop bar on its row; a drag's release writes one
/// step; the frame maps the whole picture; Done closes it, a change of selection hides it.
#[test]
fn c5_the_crop_bar_edits_the_rows_rectangle() {
    let (mut s, image, _) = engraved();
    assert!(s.crop_bar_view().is_none());
    assert!(s.add_adjust_step("crop"));
    let bar = s.crop_bar_view().unwrap();
    assert_eq!((bar.row, bar.pct), (0, [0, 0, 100, 100]));
    assert_eq!((bar.width, bar.height), (40, 20));
    assert_eq!(bar.fields.len(), 4);
    assert_eq!(s.adjustments_view().unwrap().crop_bar_row, Some(0));
    // The quad maps the 8 x 4 mm picture: its width is twice its height on screen.
    assert!((bar.quad[0] - 2.0 * bar.quad[3]).abs() < 1e-6);
    assert!(s.set_crop_rect(10, 20, 50, 60));
    assert_eq!(undo_label(&mut s), "Edit adjustments");
    assert_eq!(s.crop_bar_view().unwrap().pct, [10, 20, 50, 60]);
    assert!(!s.set_crop_rect(60, 0, 50, 100), "past the edge: refused");
    assert!(!s.set_crop_rect(10, 20, 50, 60), "unchanged");
    s.selection.set(&[]);
    assert!(s.crop_bar_view().is_none(), "another selection");
    s.selection.set(&[image]);
    s.close_crop_bar();
    assert!(s.crop_bar_view().is_none());
    assert!(s.open_crop_bar(0));
    assert!(!s.open_crop_bar(5), "no such Crop row");
}

/// Criterion 5: until its look is made a cropped picture draws the visible part of its
/// texture into the cropped frame (never the whole picture squeezed), and its look whole.
#[test]
fn c5_a_cropped_picture_draws_only_its_visible_part() {
    let mut s = crate::session::Session::new(290_138);
    let image = super::tests::place_on_page(&s);
    s.selection.set(&[image]);
    assert!(s.add_adjust_step("crop"));
    assert!(s.set_crop_rect(25, 0, 50, 100));
    let list = s.frame_draw_list();
    let quad = list
        .images
        .iter()
        .find(|q| q.asset == s.picture_leaf(image).unwrap().asset);
    let quad = quad.expect("the imported picture before its look");
    assert!((quad.texture.left - 0.25).abs() < 1e-9 && (quad.texture.width - 0.5).abs() < 1e-9);
    while s.ticks_pending() {
        s.advance_tick();
    }
    let list = s.frame_draw_list();
    let mut list = list;
    let _ = s.frame_images(&mut list);
    let look = &list.images[0];
    assert_eq!(
        look.texture,
        curvyo_document_core::UnitRect::WHOLE,
        "the look is cropped already"
    );
}

/// `ux-notes.md` §3.9 (UX review UX-1): while the Crop bar is open the picture's own handles are
/// hidden and a press on it does not move it; a press elsewhere leaves the bar.
#[test]
fn c5_the_crop_bar_hides_the_pictures_handles_and_blocks_its_move() {
    let mut s = crate::session::Session::new(290_139);
    let image = super::tests::place_on_page(&s);
    s.selection.set(&[image]);
    assert_ne!(s.select_transform_decoration_input().handles.len(), 0);
    assert!(s.add_adjust_step("crop"));
    assert!(s.crop_bar_view().is_some());
    assert!(
        s.select_transform_decoration_input().handles.is_empty(),
        "handles hidden"
    );
    let before = frame_width(&s, image);
    let corners = |s: &crate::session::Session| match s.document.object(image).unwrap() {
        ObjectSnapshot::Image(i) => i.corners(),
        _ => unreachable!(),
    };
    let at = corners(&s);
    let inside = curvyo_document_core::Point::new(
        f64::midpoint(at[0].x, at[2].x),
        f64::midpoint(at[0].y, at[2].y),
    );
    s.pointer_down(inside, false);
    s.pointer_hover(
        curvyo_document_core::Point::new(inside.x + 5.0, inside.y + 5.0),
        false,
        false,
    );
    s.pointer_up(
        curvyo_document_core::Point::new(inside.x + 5.0, inside.y + 5.0),
        false,
        false,
    );
    assert_eq!(corners(&s), at, "not moved");
    assert!((frame_width(&s, image) - before).abs() < 1e-9);
    assert!(
        s.crop_bar_view().is_some(),
        "a press on the picture keeps the bar"
    );
    s.pointer_down(
        curvyo_document_core::Point::new(at[2].x + 50.0, at[2].y + 50.0),
        false,
    );
    s.pointer_up(
        curvyo_document_core::Point::new(at[2].x + 50.0, at[2].y + 50.0),
        false,
        false,
    );
    assert!(
        s.crop_bar_view().is_none(),
        "a press elsewhere leaves the bar"
    );
}

//! Raster items reached through a clone (`specs/0123-raster-engraving` criterion 46, plan R1):
//! a linked clone of an image, a clone of that clone, a clone of a group, an image and its clone
//! on a placed page, a clone of a text, and a clone of a group holding an image and its own linked
//! clone, each a leaf with its own geometry.

#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

use std::sync::Arc;

use curvyo_document_core::{Affine, FIRST_PAGE_ID, NewImage, NodeId, Vec2};
use curvyo_laser_core::job::{LeafGeometry, PointUm, Sweep};
use curvyo_ui_core::TextInput;

use super::raster_tests::{
    dot_pictures, half_black_png, image_group, image_op, laser_page, place_image,
};
use super::{cut, laser_with_square};
use crate::session::{Session, Tool};

/// The raster operation of these tests.
fn raster(s: &mut Session, items: &[NodeId]) -> NodeId {
    image_op(s, items)
}

/// A linked clone of `of`, moved by `(dx, dy)` mm; the clone.
fn clone_of(s: &mut Session, of: NodeId, (dx, dy): (f64, f64)) -> NodeId {
    s.selection.set(&[of]);
    s.linked_clone();
    let clone = s.selection.ids()[0];
    assert_ne!(clone, of, "a clone was made");
    s.document
        .translate_objects(&[clone], Vec2::new(dx, dy))
        .unwrap();
    s.selection.set(&[]);
    clone
}

/// A laser page with a 20 x 10 mm half-black image; the session and the image.
fn page_with_image() -> (Session, NodeId) {
    let s = laser_page();
    let image = place_image(
        &s,
        &half_black_png(40, 20),
        "Photo 1",
        (50.0, 50.0, 20.0, 10.0),
    );
    (s, image)
}

fn sweeps(s: &Session) -> Vec<Sweep> {
    let job = s.laser_job().unwrap();
    job.built.job.sweeps.iter().map(|s| (**s).clone()).collect()
}

fn shifted(sweep: &Sweep, dx: i64) -> Sweep {
    let at = |p: PointUm| PointUm {
        x: p.x + dx,
        y: p.y,
    };
    Sweep {
        start: at(sweep.start),
        end: at(sweep.end),
        levels: sweep.levels.clone(),
    }
}

/// The geometries of every leaf the job input holds.
fn geometries(s: &Session) -> Vec<LeafGeometry> {
    let job = s.laser_job().unwrap();
    job.input
        .blocks
        .iter()
        .flat_map(|b| &b.leaves)
        .map(|l| l.geometry.clone())
        .collect()
}

#[test]
fn criterion_46_a_linked_clone_of_an_image_burns_the_same_rows_at_its_place() {
    let (mut s, image) = page_with_image();
    let alone = {
        let mut one = laser_page();
        let only = place_image(
            &one,
            &half_black_png(40, 20),
            "Photo 1",
            (50.0, 50.0, 20.0, 10.0),
        );
        raster(&mut one, &[only]);
        sweeps(&one)
    };
    let clone = clone_of(&mut s, image, (30.0, 0.0));
    let op = raster(&mut s, &[image, clone]);
    let all = sweeps(&s);
    assert_eq!(all.len(), 2 * alone.len(), "two raster items");
    let moved: Vec<Sweep> = alone.iter().map(|w| shifted(w, 30_000)).collect();
    for sweep in &moved {
        assert!(
            all.contains(sweep),
            "the clone's rows are the original's, moved"
        );
    }
    for sweep in &alone {
        assert!(all.contains(sweep), "the original still burns");
    }
    assert_eq!(
        dot_pictures(&mut s).len(),
        2,
        "dots on both in the Operations view"
    );
    assert!(
        !image_group(&s, op).is_null(),
        "the Image settings group shows"
    );
}

#[test]
fn criterion_46_a_clone_alone_and_a_clone_of_a_clone_burn() {
    let (mut s, image) = page_with_image();
    let clone = clone_of(&mut s, image, (30.0, 0.0));
    let deeper = clone_of(&mut s, clone, (0.0, 20.0));
    raster(&mut s, &[deeper]);
    let all = sweeps(&s);
    assert_eq!(all.len(), 100, "a sweep per row of the clone of the clone");
    assert!(
        all.iter()
            .all(|w| w.start.x >= 80_000 && w.start.y >= 70_000),
        "at the clone of the clone's place"
    );
    raster(&mut s, &[clone]);
    assert_eq!(sweeps(&s).len(), 200, "and the clone alone");
}

#[test]
fn criterion_46_a_clone_of_a_group_burns_its_image_and_its_gradient_rectangle() {
    let (mut s, square) = laser_with_square();
    s.selection.set(&[square]);
    assert!(s.set_fill_kind("gradient"));
    let image = place_image(
        &s,
        &half_black_png(40, 20),
        "Photo 1",
        (50.0, 50.0, 20.0, 10.0),
    );
    s.set_tool(Tool::Select);
    s.selection.set(&[square, image]);
    s.group_selection();
    let group = s.selection.ids()[0];
    let clone = clone_of(&mut s, group, (0.0, 60.0));
    raster(&mut s, &[clone]);
    let kinds = geometries(&s);
    assert_eq!(kinds.len(), 2, "both members: {kinds:?}");
    assert!(
        kinds
            .iter()
            .any(|g| matches!(g, LeafGeometry::Image { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|g| matches!(g, LeafGeometry::Gradient { .. }))
    );
    assert!(!sweeps(&s).is_empty());
}

#[test]
fn criterion_46_an_image_and_its_clone_on_a_placed_page_burn_through_the_placement() {
    let mut s = laser_page();
    let laser = s.active_page().id;
    let first = s
        .document
        .pages()
        .into_iter()
        .find(|p| p.id == FIRST_PAGE_ID)
        .unwrap();
    let image = s
        .document
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
        .unwrap()[0];
    let clone = clone_of(&mut s, image, (30.0, 0.0));
    assert!(s.activate_page(&laser));
    assert!(s.place_page_on_active(FIRST_PAGE_ID));
    let placement = s.selected_placement().unwrap();
    raster(&mut s, &[placement]);
    let images = geometries(&s)
        .into_iter()
        .filter(|g| matches!(g, LeafGeometry::Image { .. }))
        .count();
    assert_eq!(images, 2, "the image and its clone");
    let all = sweeps(&s);
    assert_eq!(all.len(), 200, "both burn");
    let _ = clone;
}

#[test]
fn criterion_46_a_clone_of_a_text_cuts() {
    let mut s = crate::session::text_tests::session();
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let origin = s.active_manufacturing().unwrap().page.unwrap().origin;
    let at = curvyo_document_core::Point::new(origin.x + 20.0, origin.y + 40.0);
    s.set_tool(Tool::Text);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    for c in "Cut".chars() {
        s.text_input(TextInput::Insert(c.to_string()));
    }
    s.finish_text_edit();
    crate::session::text_tests::load_wanted(&mut s);
    let text = s.selection.ids()[0];
    s.set_tool(Tool::Select);
    let clone = clone_of(&mut s, text, (0.0, 40.0));
    cut(&mut s, &[text, clone]);
    let outlines = geometries(&s)
        .into_iter()
        .filter(|g| matches!(g, LeafGeometry::Outlines(o) if !o.is_empty()))
        .count();
    assert_eq!(outlines, 2, "the text and its clone cut");
}

/// A laser page with an image and its linked clone 30 mm right, grouped, the clone stacked below
/// the image when `clone_first`, and a clone of the group 60 mm down in an Image operation.
fn group_of_image_and_clone(clone_first: bool) -> Session {
    let (mut s, image) = page_with_image();
    let inner = clone_of(&mut s, image, (30.0, 0.0));
    if clone_first {
        let key = serde_json::to_string(&inner).unwrap();
        s.reorder_layer_rows(&key, curvyo_ui_core::Reorder::Back);
    }
    s.set_tool(Tool::Select);
    s.selection.set(&[image, inner]);
    s.group_selection();
    let group = s.selection.ids()[0];
    let outer = clone_of(&mut s, group, (0.0, 60.0));
    raster(&mut s, &[outer]);
    s
}

#[test]
fn criterion_46_a_clone_of_a_group_holding_an_image_and_its_linked_clone_burns_both_in_place() {
    for clone_first in [false, true] {
        let mut s = group_of_image_and_clone(clone_first);
        let images = geometries(&s)
            .into_iter()
            .filter(|g| matches!(g, LeafGeometry::Image { .. }))
            .count();
        assert_eq!(
            images, 2,
            "the image and the inner clone (clone first: {clone_first})"
        );
        let all = sweeps(&s);
        assert_eq!(all.len(), 200, "both burn (clone first: {clone_first})");
        let at = |x: i64| {
            all.iter()
                .filter(|w| {
                    w.start.y >= 110_000
                        && w.start.x.min(w.end.x) >= x
                        && w.start.x.max(w.end.x) <= x + 20_000
                })
                .count()
        };
        assert_eq!(
            at(50_000),
            100,
            "the image's rows at the group clone's place"
        );
        assert_eq!(
            at(80_000),
            100,
            "the inner clone's rows at the group clone's place"
        );
        assert_eq!(
            dot_pictures(&mut s).len(),
            2,
            "a dot picture per leaf, not one per cloned group (clone first: {clone_first})"
        );
        let page = s.active_page().id;
        s.toggle_toolpath_view(&page);
        assert_eq!(dot_pictures(&mut s).len(), 2, "and in the Toolpath view");
    }
}

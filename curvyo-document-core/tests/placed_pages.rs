//! Placed pages: placing, moving, refusing off the laser page, copying and the delete of their
//! drawing page (`specs/0100-bring-content-in` criteria 13, 17 to 19 and 24; ADR 0045 §7).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use curvyo_document_core::{
    Affine, Angle, Document, DocumentSize, ManufacturingError, NewOutputPage,
    PLACE_PAGE_COMMIT_LABEL, Page, Point, Vec2,
};

fn laser(d: &Document) -> Page {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(400.0, 300.0),
    })
    .unwrap()
}

fn setup() -> (Document, Page, Page) {
    let d = Document::new(1);
    let drawing = d.pages()[0].clone();
    let out = laser(&d);
    (d, drawing, out)
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

#[test]
fn criterion_13_place_page_is_one_step_and_lies_above_earlier_placements() {
    let (d, drawing, out) = setup();
    let first = d
        .place_page(&out.id, &drawing.id, Affine::IDENTITY)
        .unwrap();
    let second = d
        .place_page(
            &out.id,
            &drawing.id,
            Affine::translation(Vec2::new(10.0, 0.0)),
        )
        .unwrap();
    assert_eq!(last_label(&d), PLACE_PAGE_COMMIT_LABEL);
    let ids: Vec<_> = d.placed_pages().iter().map(|p| p.id).collect();
    assert_eq!(ids, vec![first, second]);
    // Only a drawing page on a laser page.
    assert!(matches!(
        d.place_page(&drawing.id, &drawing.id, Affine::IDENTITY),
        Err(ManufacturingError::NotFound)
    ));
    assert!(matches!(
        d.place_page(&out.id, &out.id, Affine::IDENTITY),
        Err(ManufacturingError::NotFound)
    ));
}

#[test]
fn criterion_17_a_turn_and_scale_writes_only_the_link() {
    let (d, drawing, out) = setup();
    let id = d
        .place_page(&out.id, &drawing.id, Affine::IDENTITY)
        .unwrap();
    let pages_before = d.pages();
    let p = d.placed_page(id).unwrap();
    let corners = p.frame_corners().unwrap();
    let centre = Point::new(
        f64::midpoint(corners[0].x, corners[2].x),
        f64::midpoint(corners[0].y, corners[2].y),
    );
    let t = Affine::translation(Vec2::new(centre.x, centre.y))
        .compose(Affine::rotation_about(
            Point::new(0.0, 0.0),
            Angle::from_radians(std::f64::consts::FRAC_PI_2),
        ))
        .compose(Affine::scale(0.5, 0.5))
        .compose(Affine::translation(Vec2::new(-centre.x, -centre.y)));
    d.transform_placements(&[(id, t)], "transform_objects")
        .unwrap();
    assert_eq!(d.pages(), pages_before, "no page is written");
    let after = d.placed_page(id).unwrap();
    let want = t.compose(p.global_map().unwrap());
    assert!(after.global_map().unwrap().approx_eq(want, 1e-9));
}

#[test]
fn criterion_18_a_move_off_the_laser_page_is_refused_whole() {
    let (d, drawing, out) = setup();
    let id = d
        .place_page(&out.id, &drawing.id, Affine::IDENTITY)
        .unwrap();
    let steps = d.own_steps().len();
    let far = Affine::translation(Vec2::new(5000.0, 0.0));
    assert!(matches!(
        d.transform_placements(&[(id, far)], "translate_objects"),
        Err(ManufacturingError::OffPage)
    ));
    assert_eq!(d.own_steps().len(), steps);
    // An edge-only touch is off the page: the frame's left edge on the laser page's right edge.
    let p = d.placed_page(id).unwrap();
    let corners = p.frame_corners().unwrap();
    let dx = out.origin.x + out.size.width.as_mm() - corners[0].x;
    assert!(matches!(
        d.transform_placements(
            &[(id, Affine::translation(Vec2::new(dx, 0.0)))],
            "translate_objects"
        ),
        Err(ManufacturingError::OffPage)
    ));
    assert!(
        d.transform_placements(
            &[(id, Affine::translation(Vec2::new(dx - 1.0, 0.0)))],
            "translate_objects"
        )
        .is_ok()
    );
}

#[test]
fn criterion_19_duplicate_makes_a_second_placement_of_the_same_page() {
    let (d, drawing, out) = setup();
    let id = d
        .place_page(&out.id, &drawing.id, Affine::IDENTITY)
        .unwrap();
    let copies = d
        .duplicate_placements(&[id], Vec2::new(5.0, 0.0), "duplicate_objects")
        .unwrap();
    let copy = d.placed_page(copies[0]).unwrap();
    assert_eq!(copy.source.unwrap().id, drawing.id);
    assert_eq!(copy.matrix, Affine::translation(Vec2::new(5.0, 0.0)));
    assert_eq!(d.placed_pages().len(), 2);
}

#[test]
fn criterion_24_deleting_the_drawing_page_removes_its_placements_in_one_step() {
    let (d, drawing, out) = setup();
    let other = d.add_page();
    d.place_page(&out.id, &drawing.id, Affine::IDENTITY)
        .unwrap();
    d.place_page(&out.id, &other.id, Affine::IDENTITY).unwrap();
    let steps = d.own_steps().len();
    d.delete_page(&drawing.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert_eq!(d.own_steps().len(), steps + 1);
    let left: Vec<_> = d
        .placed_pages()
        .iter()
        .map(|p| p.source.as_ref().unwrap().id.clone())
        .collect();
    assert_eq!(left, vec![other.id.clone()]);
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert_eq!(d.placed_pages().len(), 2);
}

fn frame_object(
    d: &Document,
    id: curvyo_document_core::NodeId,
) -> curvyo_document_core::ObjectSnapshot {
    d.placed_page_objects()
        .into_iter()
        .find(|o| o.id() == id)
        .expect("a frame object")
}

#[test]
fn criterion_17_the_object_writers_move_turn_and_scale_a_placed_page_writing_only_its_link() {
    let (d, drawing, out) = setup();
    let id = d
        .place_page(&out.id, &drawing.id, Affine::IDENTITY)
        .unwrap();
    let pages = d.pages();
    // A move through translate_objects, one step.
    let steps = d.own_steps().len();
    d.translate_objects(&[id], Vec2::new(5.0, 7.0)).unwrap();
    assert_eq!(d.own_steps().len(), steps + 1);
    assert_eq!(
        d.placed_page(id).unwrap().matrix,
        Affine::translation(Vec2::new(5.0, 7.0))
    );
    // A turn by 90 degrees and a scale to 50 % through transform_objects, as a gesture does.
    let before = d.placed_page(id).unwrap().global_map().unwrap();
    let object = frame_object(&d, id);
    let corners = d.placed_page(id).unwrap().frame_corners().unwrap();
    let centre = Point::new(
        f64::midpoint(corners[0].x, corners[2].x),
        f64::midpoint(corners[0].y, corners[2].y),
    );
    let t = Affine::translation(Vec2::new(centre.x, centre.y))
        .compose(Affine::rotation_about(
            Point::new(0.0, 0.0),
            Angle::from_radians(std::f64::consts::FRAC_PI_2),
        ))
        .compose(Affine::scale(0.5, 0.5))
        .compose(Affine::translation(Vec2::new(-centre.x, -centre.y)));
    let curvyo_document_core::ObjectSnapshot::Image(image) = object else {
        panic!("a frame object")
    };
    let turned = curvyo_document_core::ObjectSnapshot::Image(image.mapped_by(t));
    d.transform_objects(&[turned], false).unwrap();
    assert!(
        d.placed_page(id)
            .unwrap()
            .global_map()
            .unwrap()
            .approx_eq(t.compose(before), 1e-9)
    );
    assert_eq!(d.pages(), pages, "no page written");
}

#[test]
fn criterion_18_19_the_object_writers_refuse_off_page_copy_delete_and_never_group() {
    let (d, drawing, out) = setup();
    let id = d
        .place_page(&out.id, &drawing.id, Affine::IDENTITY)
        .unwrap();
    let steps = d.own_steps().len();
    assert_eq!(
        d.translate_objects(&[id], Vec2::new(9000.0, 0.0)),
        Err(curvyo_document_core::ObjectEditError::OffPage)
    );
    assert!(d.take_placement_refusal());
    assert_eq!(d.own_steps().len(), steps, "no step");
    let copies = d
        .duplicate_objects(
            &[curvyo_document_core::CopySource {
                id,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(3.0, 0.0),
        )
        .unwrap();
    assert_eq!(d.placed_pages().len(), 2);
    assert_eq!(
        d.placed_page(copies[0]).unwrap().matrix,
        Affine::translation(Vec2::new(3.0, 0.0))
    );
    assert_eq!(
        d.group_objects(&[id, copies[0]]),
        Err(curvyo_document_core::ObjectEditError::PlacedPage)
    );
    d.delete_objects(&[copies[0]]).unwrap();
    assert_eq!(d.placed_pages().len(), 1);
    assert_eq!(d.pages(), d.pages());
}

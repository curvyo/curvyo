//! Independent acceptance tests for `specs/0106-move-versus-edit` at the core: the context table
//! of criterion 1 read from the real membership, mixed selections (4), the double-click targets
//! (13 to 16) and the classification of every command label (5, 9).
#![allow(
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::needless_lifetimes,
    missing_docs
)]

use curvyo_document_core::{
    Document, DocumentSize, NewOutputPage, NodeId, Page, PageKind, Point, RectBounds, Style,
};
use curvyo_sketch_core as sk;
use curvyo_ui_core::{
    ActivePage, ContextKind, ContextPages, DoubleClickTarget, EDIT_STEPS, FREE_STEPS, Home,
    MOVE_STEPS, OPERATION_NAMES, PageMembership, Scope, StepScope, double_click_target, in_context,
    object_bounds, scope, step_scope,
};

fn rect(d: &Document, x: f64, y: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + 20.0, y + 10.0),
    ))
}

struct World {
    pages: ContextPages,
    membership: PageMembership,
    doc: Document,
    list: Vec<Page>,
    ids: Vec<(&'static str, NodeId)>,
}

fn world() -> World {
    let d = Document::new(1);
    let b = d.add_page_of(&PageKind::Drawing).unwrap();
    let l = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let s = d.add_page_of(&PageKind::Sketch).unwrap();
    let sketch = sk::create_polyline(
        &sk::Sketch::new("origin".into()),
        "edge",
        &[
            sk::Position::mm(-20.0, -15.0),
            sk::Position::mm(20.0, -15.0),
            sk::Position::mm(20.0, 15.0),
            sk::Position::mm(-20.0, 15.0),
        ],
        true,
    )
    .unwrap();
    let sketch_id = d
        .create_page_sketch(&s.id, &sketch, &Style::default())
        .unwrap();
    let list = d.pages();
    let get = |id: &str| list.iter().find(|p| p.id == id).unwrap().clone();
    let (a, b, l, s) = (get("first"), get(&b.id), get(&l.id), get(&s.id));
    let ids = vec![
        ("ra", rect(&d, a.origin.x + 10.0, a.origin.y + 10.0)),
        ("rb", rect(&d, b.origin.x + 10.0, b.origin.y + 10.0)),
        ("rl", rect(&d, l.origin.x + 10.0, l.origin.y + 10.0)),
        ("rp", rect(&d, a.origin.x + 10.0, a.origin.y - 80.0)),
        ("rs", rect(&d, s.origin.x + 5.0, s.origin.y + 5.0)),
        ("rbl", rect(&d, b.origin.x + 220.0, b.origin.y + 100.0)),
        ("sketch", sketch_id),
    ];
    let list = d.pages();
    let membership = PageMembership::of(&d, &list);
    let pages = ContextPages::new(list.clone(), |p| match p.kind {
        PageKind::Sketch => ContextKind::Sketch,
        PageKind::Drawing => ContextKind::Drawing,
        _ => ContextKind::Laser,
    });
    World {
        pages,
        membership,
        doc: d,
        list,
        ids,
    }
}

fn home(w: &World, name: &str) -> Home {
    let id = w.ids.iter().find(|(n, _)| *n == name).unwrap().1;
    let o = w.doc.object(id).unwrap();
    w.pages.home_of(&w.membership, id, object_bounds(&o))
}

fn active(w: &World, i: usize) -> ActivePage<'_> {
    ActivePage {
        id: &w.list[i].id,
        kind: w.pages.kind_of(&w.list[i].id).unwrap(),
    }
}

#[test]
fn ac1_the_table_over_the_real_membership() {
    let w = world();
    assert_eq!(w.list.len(), 4);
    let (a, b, l, s) = (0, 1, 2, 3);
    let inside: [(usize, &[&str]); 4] = [
        (a, &["ra", "rp", "rs"]),
        (b, &["rb", "rp", "rs"]),
        (l, &["rl", "rbl"]),
        (s, &["sketch"]),
    ];
    for (page, ok) in inside {
        for (name, _) in &w.ids {
            let h = home(&w, name);
            assert_eq!(
                in_context(active(&w, page), &h),
                ok.contains(name),
                "{name} ({h:?}) with page {page} active"
            );
        }
    }
}

#[test]
fn ac1_homes_are_as_the_table_names_them() {
    let w = world();
    assert_eq!(home(&w, "rp"), Home::Pasteboard);
    assert_eq!(
        home(&w, "rs"),
        Home::Pasteboard,
        "over a sketch page: pasteboard"
    );
    assert_eq!(
        home(&w, "rbl"),
        Home::Laser(w.list[2].id.clone()),
        "laser wins over drawing"
    );
    assert_eq!(home(&w, "ra"), Home::Drawing(vec!["first".into()]));
    assert_eq!(home(&w, "sketch"), Home::Sketch(w.list[3].id.clone()));
}

#[test]
fn ac4_a_selection_is_move_only_when_any_object_is_out_of_context() {
    let w = world();
    let ra = home(&w, "ra");
    let rb = home(&w, "rb");
    let rp = home(&w, "rp");
    let a = active(&w, 0);
    assert_eq!(scope(a, [&ra, &rp]), Scope::Edit);
    assert_eq!(scope(a, [&ra, &rb]), Scope::MoveOnly);
    assert_eq!(scope(a, [&rb]), Scope::MoveOnly);
    assert_eq!(scope(a, std::iter::empty::<&Home>()), Scope::Edit);
    // Order does not matter.
    assert_eq!(scope(a, [&rb, &ra]), Scope::MoveOnly);
}

#[test]
fn ac13_to_16_double_click_targets() {
    let w = world();
    let (a, b, l, s) = (
        "first",
        w.list[1].id.as_str(),
        w.list[2].id.as_str(),
        w.list[3].id.as_str(),
    );
    let t = |active_i: usize, name: &str, last: Option<&str>| {
        double_click_target(active(&w, active_i), &home(&w, name), last, &w.pages)
    };
    assert_eq!(t(0, "ra", None), DoubleClickTarget::AsBuilt);
    assert_eq!(t(0, "rb", None), DoubleClickTarget::Activate(b.into()));
    assert_eq!(t(0, "rl", None), DoubleClickTarget::Activate(l.into()));
    assert_eq!(t(0, "rbl", None), DoubleClickTarget::Activate(l.into()));
    assert_eq!(
        t(0, "rp", Some(b)),
        DoubleClickTarget::AsBuilt,
        "drawing active: as built"
    );
    assert_eq!(t(2, "rp", Some(b)), DoubleClickTarget::Activate(b.into()));
    assert_eq!(
        t(2, "rp", None),
        DoubleClickTarget::Activate(a.into()),
        "first drawing page"
    );
    assert_eq!(
        t(2, "rp", Some("deleted")),
        DoubleClickTarget::Activate(a.into())
    );
    assert_eq!(
        t(2, "rp", Some(l)),
        DoubleClickTarget::Activate(a.into()),
        "a laser page is never the last drawing page"
    );
    assert_eq!(t(3, "rs", Some(b)), DoubleClickTarget::Activate(b.into()));
    assert_eq!(t(1, "sketch", None), DoubleClickTarget::Activate(s.into()));
    assert_eq!(t(3, "sketch", None), DoubleClickTarget::AsBuilt);
    // No drawing page at all.
    let only_laser = ContextPages::new(vec![w.list[2].clone()], |_| ContextKind::Laser);
    assert_eq!(
        double_click_target(
            ActivePage {
                id: l,
                kind: ContextKind::Laser
            },
            &Home::Pasteboard,
            None,
            &only_laser
        ),
        DoubleClickTarget::NoDrawingPage
    );
}

#[test]
fn laser_off_and_unknown_kind_pages_read_as_drawing_pages() {
    let w = world();
    // The session maps such pages to Drawing; the rule treats them as drawing pages.
    let pages = ContextPages::new(w.list.clone(), |_| ContextKind::Drawing);
    let h = pages.home_of(
        &w.membership,
        w.ids[2].1,
        object_bounds(&w.doc.object(w.ids[2].1).unwrap()),
    );
    // The membership read still holds the laser rectangle; the home is a drawing home or a
    // pasteboard home, never Laser.
    assert!(!matches!(h, Home::Laser(_)), "{h:?}");
}

#[test]
fn ac9_every_history_label_is_classified_exactly_once() {
    for (label, _) in OPERATION_NAMES {
        let n = [MOVE_STEPS, FREE_STEPS, EDIT_STEPS]
            .iter()
            .filter(|l| l.contains(label))
            .count();
        assert_eq!(n, 1, "{label} sits in {n} lists");
    }
}

#[test]
fn ac5_ac9_moves_are_moves_edits_are_edits_unknowns_are_edits() {
    for l in [
        "translate_objects",
        "duplicate_objects",
        "linked_clone",
        "place_on_page",
        "place_page",
        "place_linked_clones",
        "delete_objects",
        "flip_horizontal",
        "flip_vertical",
        "rotate_object",
        "transform_rotate",
        "transform_resize",
        "transform_skew",
        "transform_objects",
        "align_left",
        "distribute_h",
        "arrange_grid",
        "resize_rect",
        "skew_path",
    ] {
        assert_eq!(step_scope(l), StepScope::Move, "{l}");
    }
    for l in [
        "edit_style",
        "own_style",
        "own_placement",
        "follow_placement",
        "unlink_clone",
        "group",
        "ungroup",
        "boolean_union",
        "boolean_difference",
        "offset_path",
        "stroke_to_path",
        "convert_to_paths",
        "combine_paths",
        "move_anchors",
        "insert_anchor",
        "delete_anchors",
        "edit_text",
        "change_font",
        "change_text_size",
        "put_text_on_path",
        "spread_colours",
        "set_guide",
        "move_nodes",
        "set_corner_radius",
        "set_ellipse_arc",
        "trace_image",
        "set_node_opacity",
        "fill_pattern",
        "split_at_anchor",
        "join_endpoints",
        "weld",
    ] {
        assert_eq!(step_scope(l), StepScope::Edit, "{l}");
    }
    assert_eq!(step_scope("a_command_added_next_year"), StepScope::Edit);
    assert_eq!(step_scope(""), StepScope::Edit);
    assert_eq!(step_scope("set_node_visible"), StepScope::Free);
    assert_eq!(step_scope("set_node_locked"), StepScope::Free);
}

#[test]
fn ac9_no_move_label_hides_an_edit_in_its_name() {
    for l in MOVE_STEPS {
        for bad in [
            "style", "fill", "stroke", "boolean", "group", "text", "anchor", "offset", "node",
        ] {
            assert!(!l.contains(bad), "{l} looks like an edit ({bad})");
        }
    }
}

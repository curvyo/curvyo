//! Linked values of sketch instances (`specs/0099-sketch-pages` criteria 32 and 34 to 36,
//! `adrs.md` A13 to A15): link groups on the sketch, the unlinked marks on a clone, the solved
//! graph of a clone and the move preview through the document's solve cache.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{
    Affine, CloneSnapshot, Document, ExposedKey, HeldClones, InstanceError, InstanceValue,
    InstanceWrite, KeySet, NodeId, ObjectSnapshot, SketchDelta, Style, ValueSet,
};
use curvyo_sketch_core as sketch;

#[path = "support/sketch_links_graph.rs"]
mod sketch_links_graph;
use sketch_links_graph::graph;

fn dim(id: &str) -> ExposedKey {
    ExposedKey::dimension(&id.into())
}

/// A sketch exposing both lengths, both angles and `w`, and one clone of it.
fn setup() -> (Document, NodeId, NodeId) {
    let d = Document::new(1);
    let sk = d
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    for (key, name) in [
        ("len_a", "Leg 1"),
        ("len_b", "Leg 2"),
        ("ang_a", "Angle 1"),
        ("ang_b", "Angle 2"),
    ] {
        d.expose_parameter(sk, &dim(key), Some(name)).unwrap();
    }
    d.expose_parameter(sk, &ExposedKey::parameter(&"w".into()), None)
        .unwrap();
    let clone = d.linked_clone(&[sk], &[]).unwrap()[0];
    (d, sk, clone)
}

fn groups(d: &Document, sk: NodeId) -> Vec<(String, Option<usize>)> {
    d.exposed_params(sk)
        .into_iter()
        .map(|param| (param.label, param.group))
        .collect()
}

fn clone_of(d: &Document, id: NodeId) -> CloneSnapshot {
    match d.read_objects().get(id).unwrap() {
        ObjectSnapshot::Clone(clone) => clone.clone(),
        other => panic!("not a clone: {other:?}"),
    }
}

fn write(d: &Document, clone: NodeId, pairs: &[(&str, f64)], unlinked: &[&str], label: &str) {
    let mut own = ValueSet::default();
    for (key, mm) in pairs {
        own.insert(dim(key), InstanceValue::from_mm(*mm));
    }
    d.write_instance_values(
        &[InstanceWrite {
            clone,
            own,
            last_solvable: None,
            unlinked: unlinked.iter().map(|key| dim(key)).collect(),
        }],
        label,
        None,
    )
    .unwrap();
}

/// Criterion 32: a link of two ungrouped values starts a group, a third joins it, values of
/// another kind cannot link, two groups merge, and no default value moves.
#[test]
fn links_start_join_and_merge_groups_of_one_kind() {
    let (d, sk, _clone) = setup();
    let w = ExposedKey::parameter(&"w".into());
    assert_eq!(
        d.link_exposed(sk, &dim("len_a"), &dim("ang_a")),
        Err(InstanceError::NotLinkable),
        "a length and an angle"
    );
    d.link_exposed(sk, &dim("len_a"), &dim("len_b")).unwrap();
    d.link_exposed(sk, &dim("ang_a"), &dim("ang_b")).unwrap();
    assert_eq!(
        groups(&d, sk),
        [
            ("Leg 1".into(), Some(0)),
            ("Leg 2".into(), Some(0)),
            ("Angle 1".into(), Some(1)),
            ("Angle 2".into(), Some(1)),
            ("w".into(), None),
        ]
    );
    assert_eq!(
        d.link_exposed(sk, &dim("len_b"), &dim("len_a")),
        Err(InstanceError::NotLinkable),
        "one group already"
    );
    d.link_exposed(sk, &w, &dim("len_a")).unwrap();
    assert_eq!(groups(&d, sk)[4], ("w".into(), Some(0)), "joins the group");
    let defaults: Vec<f64> = d.exposed_params(sk).iter().map(|p| p.default).collect();
    assert_eq!(defaults, [60.0, 40.0, 0.0, 90.0, 10.0]);
}

/// Criterion 32: linking members of two groups merges them; the marks of the members that moved
/// go in the same step.
#[test]
fn linking_two_groups_merges_them_and_drops_the_moved_marks() {
    let (d, sk, clone) = setup();
    let (w, v) = (
        ExposedKey::parameter(&"w".into()),
        ExposedKey::parameter(&"v".into()),
    );
    d.expose_parameter(sk, &v, None).unwrap();
    d.link_exposed(sk, &dim("len_a"), &dim("len_b")).unwrap();
    d.link_exposed(sk, &w, &v).unwrap();
    let mut own = ValueSet::default();
    own.insert(w.clone(), InstanceValue::from_mm(12.0));
    d.write_instance_values(
        &[InstanceWrite {
            clone,
            own,
            last_solvable: None,
            unlinked: [w.clone()].into_iter().collect(),
        }],
        "change_parameter",
        None,
    )
    .unwrap();
    assert!(clone_of(&d, clone).chain.unlinked.contains(&w));
    d.link_exposed(sk, &dim("len_a"), &w).unwrap();
    let in_first = groups(&d, sk)
        .iter()
        .filter(|(_, group)| *group == Some(0))
        .count();
    assert_eq!(in_first, 4, "{:?}", groups(&d, sk));
    assert!(
        clone_of(&d, clone).chain.unlinked.is_empty(),
        "w left its group"
    );
}

/// Criterion 35: Unlink of a group of two dissolves it and drops the clones' marks of both.
#[test]
fn unlinking_a_group_of_two_dissolves_it_and_drops_the_marks() {
    let (d, sk, clone) = setup();
    d.link_exposed(sk, &dim("len_a"), &dim("len_b")).unwrap();
    write(
        &d,
        clone,
        &[("len_a", 50.0)],
        &["len_a"],
        "change_parameter",
    );
    assert!(!clone_of(&d, clone).chain.unlinked.is_empty());
    d.unlink_exposed(sk, &dim("len_b")).unwrap();
    assert_eq!(groups(&d, sk)[0].1, None);
    assert_eq!(groups(&d, sk)[1].1, None);
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(!json.contains("instance_unlinked"), "{json}");
    assert!(!json.contains("\"link\""), "both links go");
    assert_eq!(
        d.unlink_exposed(sk, &dim("len_b")),
        Err(InstanceError::NotLinkable)
    );
}

/// Criterion 35: Stop exposing a member leaves the group in the same step; a group left with one
/// member dissolves with the marks; Undo restores both.
#[test]
fn stop_exposing_a_member_leaves_the_group_in_the_same_step() {
    let (d, sk, clone) = setup();
    d.link_exposed(sk, &dim("len_a"), &dim("len_b")).unwrap();
    write(
        &d,
        clone,
        &[("len_b", 30.0)],
        &["len_b"],
        "change_parameter",
    );
    d.unexpose_parameter(sk, &dim("len_a")).unwrap();
    assert_eq!(groups(&d, sk)[0], ("Leg 2".into(), None));
    assert!(clone_of(&d, clone).chain.unlinked.is_empty());
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(!json.contains("instance_unlinked") && !json.contains("\"link\""));
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert_eq!(groups(&d, sk)[1], ("Leg 2".into(), Some(0)));
    assert!(clone_of(&d, clone).chain.unlinked.contains(&dim("len_b")));
}

/// Criterion 35: a deleted dimension leaves its group by read: the group of one is none, the
/// marks of its partner are ignored, nothing more is written.
#[test]
fn a_deleted_dimension_leaves_its_group_by_read() {
    let (d, sk, clone) = setup();
    d.link_exposed(sk, &dim("len_a"), &dim("len_b")).unwrap();
    write(
        &d,
        clone,
        &[("len_b", 30.0)],
        &["len_b"],
        "change_parameter",
    );
    let mut delta = SketchDelta::default();
    delta.constraints.insert("len_a".into(), None);
    d.apply_sketch_delta(sk, &delta).unwrap();
    assert_eq!(groups(&d, sk)[0], ("Leg 2".into(), None));
    assert!(clone_of(&d, clone).chain.unlinked.is_empty(), "ignored");
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(json.contains("instance_unlinked"), "kept as stored");
}

/// `adrs.md` A13: a limit or name edit keeps the entry's link.
#[test]
fn a_limit_edit_keeps_the_link() {
    let (d, sk, _clone) = setup();
    d.link_exposed(sk, &dim("len_a"), &dim("len_b")).unwrap();
    d.set_exposed_limits(sk, &dim("len_a"), Some(10.0), Some(90.0), Some("Leg A"))
        .unwrap();
    assert_eq!(groups(&d, sk)[0], ("Leg A".into(), Some(0)));
}

/// Criterion 36: the mark belongs to the clone that set it; a clone of it inherits the value,
/// not the mark.
#[test]
fn a_mark_is_never_inherited_down_a_chain() {
    let (d, sk, clone) = setup();
    d.link_exposed(sk, &dim("len_a"), &dim("len_b")).unwrap();
    write(
        &d,
        clone,
        &[("len_a", 50.0)],
        &["len_a"],
        "change_parameter",
    );
    let below = d.linked_clone(&[clone], &[]).unwrap()[0];
    let read = clone_of(&d, below);
    assert!(read.chain.unlinked.is_empty());
    assert_eq!(
        read.chain.inherited_values.get(&dim("len_a")),
        Some(InstanceValue::from_mm(50.0))
    );
    let marks = KeySet::default();
    assert!(marks.is_empty());
}

/// Criterion 29 and 36: Unlink clone removes the marks with the own values.
#[test]
fn unlink_clone_removes_the_marks() {
    let (d, sk, clone) = setup();
    d.link_exposed(sk, &dim("len_a"), &dim("len_b")).unwrap();
    write(
        &d,
        clone,
        &[("len_a", 50.0)],
        &["len_a"],
        "change_parameter",
    );
    let count = clone_of(&d, clone).resolved.all_anchors().count();
    d.unlink_clones(&[curvyo_document_core::CopySource {
        id: clone,
        anchor_ids: (0..count)
            .map(|i| curvyo_document_core::AnchorId::new(77, 1000 + i as u64))
            .collect(),
    }])
    .unwrap();
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(!json.contains("instance_unlinked"));
}

/// `adrs.md` A12: the solved graph of a clone comes from the same cache entry as its preview, so
/// asking for it after a preview step solves nothing more.
#[test]
fn the_solution_of_a_preview_costs_no_second_solve() {
    let (d, _sk, clone) = setup();
    let mut own = ValueSet::default();
    own.insert(dim("len_a"), InstanceValue::from_mm(70.0));
    assert!(d.instance_preview(clone, &own).is_some());
    let before = d.instance_solve_count();
    let solved = d.instance_solution(clone, Some(&own)).unwrap();
    assert_eq!(d.instance_solve_count(), before);
    let a = solved.point(&"a".into()).unwrap();
    assert!((a.x.0 - 70.0).abs() < 0.01, "{a:?}");
    let committed = d.instance_solution(clone, None).unwrap();
    let a = committed.point(&"a".into()).unwrap();
    assert!((a.x.0 - 60.0).abs() < 0.01, "the sketch's own: {a:?}");
}

/// `adrs.md` A15 (a): the move preview of the clones behind an original keeps a clone's saved
/// fallback, through the document's solve cache.
#[test]
fn the_move_preview_keeps_the_saved_fallback() {
    let d = Document::new(1);
    let mut g = graph();
    // A redundant distance that conflicts when the length changes.
    g.constraints.insert(
        "reach".into(),
        sketch::Constraint::driving(sketch::Relation::Distance {
            a: "origin".into(),
            b: "a".into(),
            value: sketch::Expression::Literal(sketch::Quantity::length(60.0).unwrap()),
        }),
    );
    let sk = d
        .create_sketch(&g, Affine::IDENTITY, &Style::default())
        .unwrap();
    d.expose_parameter(sk, &dim("len_a"), Some("Leg 1"))
        .unwrap();
    d.expose_parameter(sk, &dim("reach"), Some("Reach 1"))
        .unwrap();
    let clone = d.linked_clone(&[sk], &[]).unwrap()[0];
    write(
        &d,
        clone,
        &[("len_a", 70.0), ("reach", 70.0)],
        &[],
        "own_parameters",
    );
    d.write_instance_values(
        &[InstanceWrite {
            clone,
            own: {
                let mut set = ValueSet::default();
                set.insert(dim("len_a"), InstanceValue::from_mm(80.0));
                set.insert(dim("reach"), InstanceValue::from_mm(70.0));
                set
            },
            last_solvable: None,
            unlinked: KeySet::default(),
        }],
        "change_parameter",
        None,
    )
    .unwrap();
    let read = clone_of(&d, clone);
    assert!(read.chain.no_solution);
    assert!(!read.chain.fallback_values.is_empty());
    let follower = d.linked_clone(&[clone], &[]).unwrap()[0];
    let objects = d.read_objects().objects.to_vec();
    let moved = ObjectSnapshot::Clone(
        clone_of(&d, clone).mapped_by(Affine::new(1.0, 0.0, 0.0, 1.0, 5.0, 0.0)),
    );
    let before = d.instance_solve_count();
    let previews = d
        .clone_previews(&objects, &[moved], usize::MAX, &HeldClones::new(), false)
        .unwrap();
    assert_eq!(
        d.instance_solve_count(),
        before,
        "no re-solve on a move step"
    );
    let shown = previews.iter().find(|path| path.id == follower).unwrap();
    let xs: Vec<f64> = shown.anchors.iter().map(|a| a.point.x).collect();
    let max = xs.iter().copied().fold(f64::MIN, f64::max);
    assert!(
        (max - 75.0).abs() < 0.05,
        "the fallback's 70 mm moved by 5: {max}"
    );
}

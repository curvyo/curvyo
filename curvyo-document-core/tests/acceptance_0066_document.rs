//! Independent document acceptance derived before implementation review.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Affine, Document, SketchDelta, Style, pack, unpack};
use curvyo_sketch_core as sketch;
fn graph() -> sketch::Sketch {
    let mut s = sketch::Sketch::new("origin".into());
    s.entities.insert(
        "end".into(),
        sketch::Entity::Point {
            position: sketch::Position::mm(60.0, 0.0),
            fixed: false,
        },
    );
    s.entities.insert(
        "line".into(),
        sketch::Entity::Line {
            start: "origin".into(),
            end: "end".into(),
        },
    );
    for (id, value) in [("width", 80.0), ("margin", 10.0)] {
        s.parameters.insert(
            id.into(),
            sketch::Parameter::literal(id, sketch::Quantity::length(value).unwrap()),
        );
    }
    let formula = sketch::Expression::parse("width - 2 * margin", &s.parameters).unwrap();
    s.parameters.insert(
        "spacing".into(),
        sketch::Parameter::expression("spacing", sketch::Dimension::Length, formula),
    );
    s.constraints.insert(
        "horizontal".into(),
        sketch::Constraint::driving(sketch::Relation::Horizontal("line".into())),
    );
    s.constraints.insert(
        "length".into(),
        sketch::Constraint::driving(sketch::Relation::Length {
            curve: "line".into(),
            value: sketch::Expression::Reference("spacing".into()),
        }),
    );
    s
}
fn edit(id: &str, value: f64) -> SketchDelta {
    let mut delta = SketchDelta::default();
    delta.parameters.insert(
        id.into(),
        Some(sketch::Parameter::literal(
            id,
            sketch::Quantity::length(value).unwrap(),
        )),
    );
    delta
}
fn prime(d: &Document, id: curvyo_document_core::NodeId) {
    let version = d.version();
    let steps = d.history_len();
    let a = d.sketch(id).unwrap();
    let b = d.sketch(id).unwrap();
    assert_eq!(a.graph, b.graph);
    assert_eq!(a.solution, b.solution);
    assert_eq!(a.status, b.status);
    assert_eq!(a.resolved, b.resolved);
    assert_eq!(d.version(), version);
    assert_eq!(d.history_len(), steps);
}
fn width(d: &Document, id: curvyo_document_core::NodeId, expected: f64) {
    let s = d.sketch(id).unwrap();
    assert!(!s.stale, "{:?}", s.diagnosis);
    let p = s.solution.point(&"end".into()).unwrap();
    assert!((p.x.0 - expected).abs() < 1e-6, "{p:?}");
    assert!(p.y.0.abs() < 1e-6);
}
#[test]
fn d01_reopen_formula_edit_changes_actual_geometry_without_read_writes() {
    let d = Document::new(1);
    let id = d
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let restored = unpack(2, &pack(&d, "acceptance").unwrap()).unwrap();
    assert_eq!(restored.sketch(id).unwrap().graph, graph());
    width(&restored, id, 60.0);
    let before = restored.version();
    for _ in 0..3 {
        width(&restored, id, 60.0);
    }
    assert_eq!(restored.version(), before);
    restored
        .apply_sketch_delta(id, &edit("width", 100.0))
        .unwrap();
    width(&restored, id, 80.0);
    let again = unpack(3, &pack(&restored, "acceptance").unwrap()).unwrap();
    width(&again, id, 80.0);
}
#[test]
fn d03_d05_independent_peer_intent_survives_local_undo() {
    let a = Document::new(1);
    let id = a
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let b = unpack(2, &pack(&a, "acceptance").unwrap()).unwrap();
    prime(&a, id);
    a.apply_sketch_delta(id, &edit("width", 100.0)).unwrap();
    let step = a.own_steps().last().unwrap().step;
    b.apply_sketch_delta(id, &edit("margin", 15.0)).unwrap();
    prime(&a, id);
    a.import_updates(&b.export_updates(&a.version()).unwrap())
        .unwrap();
    width(&a, id, 70.0);
    prime(&a, id);
    a.undo_step(step).unwrap();
    width(&a, id, 50.0);
    prime(&a, id);
    a.redo_step(step).unwrap();
    width(&a, id, 70.0);
}
#[test]
fn d04_contradictory_intent_is_inspectable_and_repairable() {
    let d = Document::new(1);
    let id = d
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let initial = d.sketch(id).unwrap();
    let mut delta = SketchDelta::default();
    delta.constraints.insert(
        "conflict".into(),
        Some(sketch::Constraint::driving(sketch::Relation::Length {
            curve: "line".into(),
            value: sketch::Expression::Literal(sketch::Quantity::length(20.0).unwrap()),
        })),
    );
    let count = d.history_len();
    d.apply_sketch_delta(id, &delta).unwrap();
    assert_eq!(d.history_len(), count + 1);
    let current = d.sketch(id).unwrap();
    assert!(current.graph.constraints.contains_key(&"conflict".into()));
    assert!(current.stale);
    assert_eq!(current.status, Some(sketch::SolveStatus::Conflicting));
    assert_eq!(current.resolved, initial.resolved);
    assert!(!current.conflicting.is_empty());
    delta.constraints.insert("conflict".into(), None);
    d.apply_sketch_delta(id, &delta).unwrap();
    width(&d, id, 60.0);
}
#[test]
fn d02_goldens_distinguish_repairable_intent_from_structural_damage() {
    for name in ["plate", "slot", "converted", "dangling"] {
        let bytes = std::fs::read(format!(
            "{}/tests/fixtures/sketch_{name}_v28.curvyo",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let d = unpack(1, &bytes).unwrap();
        let version = d.version();
        let _ = d.read_objects();
        assert_eq!(d.version(), version);
    }
    for name in ["checkpoint_invalid", "record_invalid"] {
        let bytes = std::fs::read(format!(
            "{}/tests/fixtures/sketch_{name}_v28.curvyo",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert!(matches!(
            unpack(1, &bytes),
            Err(curvyo_document_core::OpenError::Damaged)
        ));
    }
}
fn object(d: &Document, id: curvyo_document_core::NodeId) -> curvyo_document_core::ObjectSnapshot {
    d.read_objects()
        .objects
        .iter()
        .find(|o| o.id() == id)
        .unwrap()
        .clone()
}
#[test]
fn d06_same_entity_and_construction_flag_merge_atomically() {
    let a = Document::new(1);
    let mut seed = graph();
    seed.entities.insert(
        "free".into(),
        sketch::Entity::Point {
            position: sketch::Position::mm(1.0, 2.0),
            fixed: false,
        },
    );
    let id = a
        .create_sketch(&seed, Affine::IDENTITY, &Style::default())
        .unwrap();
    let b = unpack(2, &pack(&a, "acceptance").unwrap()).unwrap();
    let record = |x, construction| curvyo_document_core::SketchEntityRecord {
        entity: sketch::Entity::Point {
            position: sketch::Position::mm(x, 2.0),
            fixed: false,
        },
        construction,
    };
    let ra = record(3.0, true);
    let rb = record(4.0, false);
    for (doc, r) in [(&a, ra.clone()), (&b, rb.clone())] {
        let mut delta = SketchDelta::default();
        delta.entities.insert("free".into(), Some(r));
        doc.apply_sketch_delta(id, &delta).unwrap();
    }
    let ua = a.export_updates(&b.version()).unwrap();
    let ub = b.export_updates(&a.version()).unwrap();
    a.import_updates(&ub).unwrap();
    b.import_updates(&ua).unwrap();
    let sa = a.sketch(id).unwrap();
    let sb = b.sketch(id).unwrap();
    assert_eq!(sa.graph, sb.graph);
    let winning = curvyo_document_core::SketchEntityRecord {
        entity: sa.graph.entities[&"free".into()].clone(),
        construction: sa.graph.construction.contains(&"free".into()),
    };
    assert!(winning == ra || winning == rb);
}
#[test]
fn d12_d13_clones_follow_reopened_formula_and_source_removal_undo() {
    let d = Document::new(1);
    let id = d
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let first = d.linked_clone(&[id], &[]).unwrap()[0];
    let second = d.linked_clone(&[first], &[]).unwrap()[0];
    d.apply_sketch_delta(id, &edit("width", 100.0)).unwrap();
    for clone in [first, second] {
        let obj = object(&d, clone);
        assert!(matches!(
            obj,
            curvyo_document_core::ObjectSnapshot::Clone(_)
        ));
        let path = obj.drawn_path().unwrap();
        assert!(path.all_anchors().any(|a| (a.point.x - 80.0).abs() < 1e-6));
    }
    d.delete_objects(&[id]).unwrap();
    let deletion = d.own_steps().last().unwrap().step;
    for clone in [first, second] {
        assert!(object(&d, clone).drawn_path().is_some());
    }
    d.undo_step(deletion).unwrap();
    width(&d, id, 80.0);
    d.apply_sketch_delta(id, &edit("width", 120.0)).unwrap();
    for clone in [first, second] {
        assert!(
            object(&d, clone)
                .drawn_path()
                .unwrap()
                .all_anchors()
                .any(|a| (a.point.x - 100.0).abs() < 1e-6)
        );
    }
}
#[test]
fn d14_d15_rigid_conversion_preserves_geometry_identity_and_undo() {
    let d = Document::new(1);
    let map = Affine::new(0.0, 1.0, -1.0, 0.0, 30.0, -20.0);
    let id = d.create_sketch(&graph(), map, &Style::default()).unwrap();
    let before = d.sketch(id).unwrap();
    assert_eq!(before.graph, graph());
    let points: Vec<_> = before.resolved.all_anchors().map(|a| a.point).collect();
    assert!(
        points
            .iter()
            .any(|p| (p.x - 30.0).abs() < 1e-6 && (p.y - 40.0).abs() < 1e-6)
    );
    let ids: Vec<_> = (0..points.len())
        .map(|n| curvyo_document_core::AnchorId::new(77, n as u64))
        .collect();
    let count = d.history_len();
    d.convert_sketch_to_path(id, &ids).unwrap();
    assert_eq!(d.history_len(), count + 1);
    let step = d.own_steps().last().unwrap().step;
    let converted = object(&d, id);
    assert!(matches!(
        converted,
        curvyo_document_core::ObjectSnapshot::Path(_)
    ));
    assert_eq!(
        converted
            .drawn_path()
            .unwrap()
            .all_anchors()
            .map(|a| a.point)
            .collect::<Vec<_>>(),
        points
    );
    d.undo_step(step).unwrap();
    assert_eq!(d.sketch(id).unwrap().graph, before.graph);
    assert_eq!(d.sketch(id).unwrap().placement, map);
}
fn crafted(d: &Document, edit: impl FnOnce(&loro::LoroMap)) -> Vec<u8> {
    use std::io::{Cursor, Write};
    let raw = loro::LoroDoc::new();
    raw.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = raw.get_tree("paths");
    let meta=tree.nodes().into_iter().map(|id|tree.get_meta(id).unwrap()).find(|m|matches!(m.get("shape"),Some(loro::ValueOrContainer::Value(loro::LoroValue::String(s))) if s.as_str()=="sketch")).unwrap();
    edit(&meta);
    raw.commit();
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (name,bytes) in [("manifest.json",serde_json::to_vec(&serde_json::json!({"format_version":28,"loro_snapshot_version":1,"app_version":"acceptance"})).unwrap()),("document.loro",raw.export(loro::ExportMode::Snapshot).unwrap()),("document.json",b"{}".to_vec())] {zip.start_file(name,options).unwrap();zip.write_all(&bytes).unwrap();}
    zip.finish().unwrap().into_inner()
}
fn collection(meta: &loro::LoroMap, key: &str) -> loro::LoroMap {
    let Some(loro::ValueOrContainer::Container(loro::Container::Map(map))) = meta.get(key) else {
        panic!("missing {key}")
    };
    map
}
#[test]
fn d07_d09_dangling_intent_and_stale_checkpoint_remain_read_only_repairable() {
    let d = Document::new(1);
    let id = d
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let stale = crafted(&d, |m| {
        collection(m, "sketch_parameters")
            .insert(
                "width",
                loro::LoroValue::from(
                    serde_json::to_value(sketch::Parameter::literal(
                        "width",
                        sketch::Quantity::length(100.0).unwrap(),
                    ))
                    .unwrap(),
                ),
            )
            .unwrap();
    });
    let restored = unpack(2, &stale).unwrap();
    width(&restored, id, 80.0);
    let version = restored.version();
    width(&restored, id, 80.0);
    assert_eq!(version, restored.version());
    let dangling = crafted(&d, |m| {
        collection(m, "sketch_entities").delete("end").unwrap();
    });
    let repaired = unpack(3, &dangling).unwrap();
    let s = repaired.sketch(id).unwrap();
    assert!(s.stale);
    assert!(s.diagnosis.is_some());
    assert!(!s.graph.entities.contains_key(&"end".into()));
    assert!(s.graph.constraints.contains_key(&"length".into()));
    assert_eq!(s.resolved, d.sketch(id).unwrap().resolved);
    let mut delta = SketchDelta::default();
    delta.entities.insert(
        "end".into(),
        Some(curvyo_document_core::SketchEntityRecord {
            entity: graph().entities[&"end".into()].clone(),
            construction: false,
        }),
    );
    repaired.apply_sketch_delta(id, &delta).unwrap();
    width(&repaired, id, 60.0);
}
#[test]
fn d10_matching_hash_does_not_authenticate_rank_or_geometry() {
    let d = Document::new(1);
    let mut seed = graph();
    seed.entities.insert(
        "free".into(),
        sketch::Entity::Point {
            position: sketch::Position::mm(2.0, 3.0),
            fixed: false,
        },
    );
    let id = d
        .create_sketch(&seed, Affine::IDENTITY, &Style::default())
        .unwrap();
    let forged = crafted(&d, |m| {
        let Some(loro::ValueOrContainer::Value(v)) = m.get("sketch_checkpoint") else {
            panic!()
        };
        let mut c: curvyo_document_core::SketchCheckpoint =
            serde_json::from_value(v.into()).unwrap();
        c.status = sketch::SolveStatus::FullyConstrained;
        c.fully_constrained = Some(c.candidate.entities.keys().cloned().collect());
        m.insert(
            "sketch_checkpoint",
            loro::LoroValue::from(serde_json::to_value(c).unwrap()),
        )
        .unwrap();
    });
    let restored = unpack(2, &forged).unwrap();
    assert_eq!(
        restored.sketch(id).unwrap().status,
        Some(sketch::SolveStatus::UnderConstrained(2))
    );
    let invalid = crafted(&d, |m| {
        let Some(loro::ValueOrContainer::Value(v)) = m.get("sketch_checkpoint") else {
            panic!()
        };
        let mut c: curvyo_document_core::SketchCheckpoint =
            serde_json::from_value(v.into()).unwrap();
        c.candidate.entities.insert(
            "origin".into(),
            sketch::Entity::Point {
                position: sketch::Position::mm(9.0, 9.0),
                fixed: true,
            },
        );
        m.insert(
            "sketch_checkpoint",
            loro::LoroValue::from(serde_json::to_value(c).unwrap()),
        )
        .unwrap();
    });
    assert!(matches!(
        unpack(3, &invalid),
        Err(curvyo_document_core::OpenError::Damaged)
    ));
}
#[test]
fn d12_d15_magnified_circle_and_ellipse_clones_meet_final_space_tolerance() {
    for ellipse in [false, true] {
        let document = Document::new(1);
        let seed = if ellipse {
            sketch::create_ellipse(
                &sketch::Sketch::new("origin".into()),
                "curve",
                sketch::Position::mm(0.0, 0.0),
                sketch::Position::mm(10.0, 0.0),
                sketch::Length(5.0),
            )
            .unwrap()
        } else {
            sketch::create_circle(
                &sketch::Sketch::new("origin".into()),
                "curve",
                sketch::Position::mm(0.0, 0.0),
                sketch::Length(10.0),
            )
            .unwrap()
        };
        let id = document
            .create_sketch(&seed, Affine::IDENTITY, &Style::default())
            .unwrap();
        prime(&document, id);
        let clone = document.linked_clone(&[id], &[]).unwrap()[0];
        let curvyo_document_core::ObjectSnapshot::Clone(clone_snapshot) = object(&document, clone)
        else {
            panic!()
        };
        document
            .transform_objects(
                &[curvyo_document_core::ObjectSnapshot::Clone(
                    clone_snapshot.mapped_by(Affine::scale(1000.0, 1000.0)),
                )],
                false,
            )
            .unwrap();
        let obj = object(&document, clone);
        let outline = obj.drawn_path().unwrap();
        let path = outline.subpaths().find(|p| p.closed).unwrap_or_else(|| {
            panic!(
                "ellipse={ellipse}, subpaths {:?}",
                outline
                    .subpaths()
                    .map(|p| (p.closed, p.anchors.len()))
                    .collect::<Vec<_>>()
            )
        });
        assert_eq!(outline.subpaths().filter(|p| p.closed).count(), 1);
        let major = 10000.0;
        let minor = if ellipse { 5000.0 } else { 10000.0 };
        let mut worst: f64 = 0.0;
        for index in 0..path.anchors.len() {
            let start = path.anchors[index];
            let end = path.anchors[(index + 1) % path.anchors.len()];
            for sample in 0..=40 {
                let t = f64::from(sample) / 40.0;
                let u = 1.0 - t;
                let x = u.powi(3) * start.point.x
                    + 3.0 * u * u * t * (start.point.x + start.handle_out.x)
                    + 3.0 * u * t * t * (end.point.x + end.handle_in.x)
                    + t.powi(3) * end.point.x;
                let y = u.powi(3) * start.point.y
                    + 3.0 * u * u * t * (start.point.y + start.handle_out.y)
                    + 3.0 * u * t * t * (end.point.y + end.handle_in.y)
                    + t.powi(3) * end.point.y;
                let angle = (y / minor).atan2(x / major);
                worst = worst.max((x - major * angle.cos()).hypot(y - minor * angle.sin()));
            }
        }
        assert!(
            worst <= 0.01,
            "ellipse={ellipse}, final-space deviation {worst} mm"
        );
        assert_eq!(document.sketch(id).unwrap().graph, seed);
    }
}

#[test]
fn d10_checkpoint_identity_construction_and_arc_guards_reject_forgery() {
    for mutation in 0..4 {
        let document = Document::new(900);
        let seed = if mutation == 3 {
            sketch::create_arc_three_points(
                &sketch::Sketch::new("origin".into()),
                "arc",
                [
                    sketch::Position::mm(10.0, 0.0),
                    sketch::Position::mm(0.0, 10.0),
                    sketch::Position::mm(-10.0, 0.0),
                ],
            )
            .unwrap()
        } else {
            graph()
        };
        document
            .create_sketch(&seed, Affine::IDENTITY, &Style::default())
            .unwrap();
        let bytes = crafted(&document, |meta| {
            let Some(loro::ValueOrContainer::Value(value)) = meta.get("sketch_checkpoint") else {
                panic!()
            };
            let mut checkpoint: curvyo_document_core::SketchCheckpoint =
                serde_json::from_value(value.into()).unwrap();
            match mutation {
                0 => {
                    checkpoint.candidate.entities.remove(&"end".into());
                }
                1 => {
                    checkpoint.candidate.entities.insert(
                        "forged".into(),
                        sketch::Entity::Point {
                            position: sketch::Position::mm(0.0, 0.0),
                            fixed: false,
                        },
                    );
                }
                2 => {
                    checkpoint.candidate.construction.insert("line".into());
                }
                _ => {
                    for entity in checkpoint.candidate.entities.values_mut() {
                        if let sketch::Entity::Arc { radius, .. } = entity {
                            radius.0 = 20.0;
                        }
                    }
                }
            }
            meta.insert(
                "sketch_checkpoint",
                loro::LoroValue::from(serde_json::to_value(checkpoint).unwrap()),
            )
            .unwrap();
        });
        assert!(
            matches!(
                unpack(901, &bytes),
                Err(curvyo_document_core::OpenError::Damaged)
            ),
            "mutation {mutation}"
        );
    }
}

#[test]
fn d11_d16_style_placement_preserve_fingerprint_and_path_writers_require_conversion() {
    let document = Document::new(910);
    let id = document
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let original = document.sketch(id).unwrap();
    let fingerprint = original.checkpoint.unwrap().fingerprint;
    prime(&document, id);
    document
        .edit_style(
            &[id],
            &curvyo_document_core::StyleEdit::StrokeWidth(curvyo_document_core::Length::from_mm(
                2.0,
            )),
        )
        .unwrap();
    prime(&document, id);
    document
        .translate_objects(&[id], curvyo_document_core::Vec2::new(30.0, 10.0))
        .unwrap();
    let current = document.sketch(id).unwrap();
    assert_eq!(current.checkpoint.unwrap().fingerprint, fingerprint);
    assert_eq!(current.graph, graph());
    let anchor = current.resolved.anchors[0];
    let version = document.version();
    assert!(
        document
            .move_anchors(&[(id, anchor.id, curvyo_document_core::Point::new(99.0, 99.0))])
            .is_err()
    );
    assert_eq!(document.version(), version);
    let ids: Vec<_> = (0..current.resolved.all_anchors().count())
        .map(|n| curvyo_document_core::AnchorId::new(910, n as u64))
        .collect();
    document.convert_sketch_to_path(id, &ids).unwrap();
    document
        .move_anchors(&[(id, ids[0], curvyo_document_core::Point::new(99.0, 99.0))])
        .unwrap();
    assert!(document.sketch(id).is_none());
}

#[test]
fn d13_hidden_sketch_clone_own_style_and_unlink_preserve_geometry() {
    let document = Document::new(920);
    let id = document
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let clone = document.linked_clone(&[id], &[]).unwrap()[0];
    document.set_own_style(&[clone], true).unwrap();
    document
        .edit_style(
            &[clone],
            &curvyo_document_core::StyleEdit::StrokeWidth(curvyo_document_core::Length::from_mm(
                3.0,
            )),
        )
        .unwrap();
    document.set_node_visible(&[id], false).unwrap();
    document
        .apply_sketch_delta(id, &edit("width", 100.0))
        .unwrap();
    let resolved = object(&document, clone);
    let path = resolved.drawn_path().unwrap();
    assert!(path.all_anchors().any(|a| (a.point.x - 80.0).abs() < 1e-6));
    let style = path.style.clone();
    let points: Vec<_> = path.all_anchors().map(|a| a.point).collect();
    let ids: Vec<_> = (0..points.len())
        .map(|n| curvyo_document_core::AnchorId::new(921, n as u64))
        .collect();
    document
        .unlink_clones(&[curvyo_document_core::CopySource {
            id: clone,
            anchor_ids: ids,
        }])
        .unwrap();
    assert!(matches!(
        object(&document, clone),
        curvyo_document_core::ObjectSnapshot::Path(_)
    ));
    let independent = object(&document, clone);
    assert_eq!(independent.style(), &style);
    assert_eq!(
        independent
            .drawn_path()
            .unwrap()
            .all_anchors()
            .map(|a| a.point)
            .collect::<Vec<_>>(),
        points
    );
    document
        .apply_sketch_delta(id, &edit("width", 120.0))
        .unwrap();
    assert_eq!(
        object(&document, clone)
            .drawn_path()
            .unwrap()
            .all_anchors()
            .map(|a| a.point)
            .collect::<Vec<_>>(),
        points
    );
}

#[test]
fn d15_converted_arc_ellipse_and_slot_keep_analytic_tolerance_and_closure() {
    for kind in ["arc", "ellipse", "slot"] {
        let seed = match kind {
            "arc" => sketch::create_arc_three_points(
                &sketch::Sketch::new("origin".into()),
                "shape",
                [
                    sketch::Position::mm(10.0, 0.0),
                    sketch::Position::mm(0.0, 10.0),
                    sketch::Position::mm(-10.0, 0.0),
                ],
            )
            .unwrap(),
            "ellipse" => sketch::create_ellipse(
                &sketch::Sketch::new("origin".into()),
                "shape",
                sketch::Position::mm(0.0, 0.0),
                sketch::Position::mm(10.0, 0.0),
                sketch::Length(5.0),
            )
            .unwrap(),
            _ => sketch::create_slot(
                &sketch::Sketch::new("origin".into()),
                "shape",
                sketch::Position::mm(0.0, 0.0),
                sketch::Position::mm(20.0, 0.0),
                sketch::Length(6.0),
            )
            .unwrap(),
        };
        let document = Document::new(930);
        let id = document
            .create_sketch(
                &seed,
                Affine::new(0.0, 1.0, -1.0, 0.0, 30.0, -20.0),
                &Style::default(),
            )
            .unwrap();
        let snapshot = document.sketch(id).unwrap();
        let ids: Vec<_> = (0..snapshot.resolved.all_anchors().count())
            .map(|segment_index| curvyo_document_core::AnchorId::new(931, segment_index as u64))
            .collect();
        document.convert_sketch_to_path(id, &ids).unwrap();
        let object = object(&document, id);
        let path = object.drawn_path().unwrap();
        assert_eq!(
            path.subpaths().filter(|p| p.closed).count(),
            usize::from(kind != "arc")
        );
        let mut samples = 0;
        for subpath in path.subpaths() {
            let count = if subpath.closed {
                subpath.anchors.len()
            } else {
                subpath.anchors.len().saturating_sub(1)
            };
            for segment_index in 0..count {
                let start = subpath.anchors[segment_index];
                let end = subpath.anchors[(segment_index + 1) % subpath.anchors.len()];
                for sample in 0..=40 {
                    let t = f64::from(sample) / 40.0;
                    let u = 1.0 - t;
                    let world_x = u.powi(3) * start.point.x
                        + 3.0 * u * u * t * (start.point.x + start.handle_out.x)
                        + 3.0 * u * t * t * (end.point.x + end.handle_in.x)
                        + t.powi(3) * end.point.x;
                    let world_y = u.powi(3) * start.point.y
                        + 3.0 * u * u * t * (start.point.y + start.handle_out.y)
                        + 3.0 * u * t * t * (end.point.y + end.handle_in.y)
                        + t.powi(3) * end.point.y;
                    let x = world_y + 20.0;
                    let y = 30.0 - world_x;
                    let error = match kind {
                        "arc" => (x.hypot(y) - 10.0).abs(),
                        "ellipse" => {
                            let angle = (y / 5.0).atan2(x / 10.0);
                            (x - 10.0 * angle.cos()).hypot(y - 5.0 * angle.sin())
                        }
                        _ => {
                            let spine_x = x.clamp(0.0, 20.0);
                            ((x - spine_x).hypot(y) - 3.0).abs()
                        }
                    };
                    assert!(error <= 0.01, "{kind}: deviation {error} at local{x},{y}");
                    samples += 1;
                }
            }
        }
        assert!(samples > 40);
    }
}

#[test]
fn d01_deep_accepted_formula_must_reopen_and_remain_editable() {
    const CHILD: &str = "CURVYO_0066_DOCUMENT_DEEP_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "d01_deep_accepted_formula_must_reopen_and_remain_editable",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(
            result.success(),
            "isolated document formula lifecycle failed: {result}"
        );
        return;
    }
    let d = Document::new(41);
    let id = d
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let before = d.version();
    let text = format!("width{}", " + 0 mm".repeat(256));
    let Ok(expression) = sketch::Expression::parse(&text, &graph().parameters) else {
        assert_eq!(d.version(), before);
        println!("long formula refused by parser without writes");
        return;
    };
    let mut delta = SketchDelta::default();
    delta.parameters.insert(
        "spacing".into(),
        Some(sketch::Parameter::expression(
            "spacing",
            sketch::Dimension::Length,
            expression,
        )),
    );
    if d.apply_sketch_delta(id, &delta).is_err() {
        assert_eq!(d.version(), before, "refused formula must not write");
        width(&d, id, 60.0);
        println!("long formula refused by document without writes");
        return;
    }
    println!("long formula accepted: verifying actual packed roundtrips and dependent edit");
    width(&d, id, 80.0);
    let restored = unpack(42, &pack(&d, "deep formula acceptance").unwrap()).unwrap();
    width(&restored, id, 80.0);
    restored
        .apply_sketch_delta(id, &edit("width", 100.0))
        .unwrap();
    width(&restored, id, 100.0);
    let again = unpack(43, &pack(&restored, "edited deep formula").unwrap()).unwrap();
    width(&again, id, 100.0);
}

#[test]
#[ignore = "requires frontend/tests/browser/sketch-acceptance.cjs saved artifact"]
fn d01_actual_browser_saved_circle_has_committed_repaired_diameter() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/0066-browser/independent-circle.curvyo");
    let d = unpack(901, &std::fs::read(path).unwrap()).unwrap();
    let objects = d.read_objects();
    let ids: Vec<_> = objects
        .objects
        .iter()
        .filter_map(|o| {
            matches!(o, curvyo_document_core::ObjectSnapshot::Sketch(_)).then_some(o.id())
        })
        .collect();
    assert_eq!(ids.len(), 1);
    let snapshot = d.sketch(ids[0]).unwrap();
    assert!(!snapshot.stale);
    let radii: Vec<_> = snapshot
        .solution
        .entities
        .values()
        .filter_map(|e| {
            if let sketch::Entity::Circle { radius, .. } = e {
                Some(radius.0)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(radii.len(), 1);
    assert!(
        (radii[0] - 12.0).abs() < 1e-6,
        "saved diameter must be committed 24 mm: {radii:?}"
    );
}

#[test]
fn d17_primed_snapshot_tracks_deleted_current_ids_through_undo_redo() {
    let d = Document::new(970);
    let mut g = graph();
    g.entities.insert(
        "temporary".into(),
        sketch::Entity::Point {
            position: sketch::Position::mm(12.0, 34.0),
            fixed: false,
        },
    );
    let id = d
        .create_sketch(&g, Affine::IDENTITY, &Style::default())
        .unwrap();
    prime(&d, id);
    let mut delta = SketchDelta::default();
    delta.entities.insert("temporary".into(), None);
    d.apply_sketch_delta(id, &delta).unwrap();
    let step = d.own_steps().last().unwrap().step;
    for (index, restored) in [false, true, false].into_iter().enumerate() {
        prime(&d, id);
        let snapshot = d.sketch(id).unwrap();
        assert_eq!(
            snapshot.graph.entities.contains_key(&"temporary".into()),
            restored
        );
        assert_eq!(
            snapshot.solution.entities.contains_key(&"temporary".into()),
            restored
        );
        width(&d, id, 60.0);
        if restored {
            d.redo_step(step).unwrap();
        } else if index == 0 {
            d.undo_step(step).unwrap();
        }
    }
}

#[test]
#[ignore = "requires frontend/tests/browser/sketch-clone-acceptance.cjs saved artifact"]
fn d12_actual_browser_clone_entry_edits_original_and_linked_geometry() {
    let bytes = std::fs::read("../target/0066-browser/clone-entry.curvyo").unwrap();
    let d = unpack(980, &bytes).unwrap();
    let read = d.read_objects();
    let sketches: Vec<_> = read
        .objects
        .iter()
        .filter_map(|o| {
            if let curvyo_document_core::ObjectSnapshot::Sketch(s) = o {
                Some(s)
            } else {
                None
            }
        })
        .collect();
    let clones: Vec<_> = read
        .objects
        .iter()
        .filter(|o| matches!(o, curvyo_document_core::ObjectSnapshot::Clone(_)))
        .collect();
    assert_eq!(sketches.len(), 1);
    assert_eq!(clones.len(), 1);
    let circle = sketches[0]
        .solution
        .entities
        .values()
        .find_map(|e| {
            if let sketch::Entity::Circle { radius, .. } = e {
                Some(radius.0)
            } else {
                None
            }
        })
        .unwrap();
    assert!((circle - 15.0).abs() < 1e-6);
    let points: Vec<_> = clones[0]
        .drawn_path()
        .unwrap()
        .all_anchors()
        .map(|a| a.point)
        .collect();
    let min = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    assert!(
        (max - min - 30.0).abs() < 1e-6,
        "clone follows solved diameter30: {}",
        max - min
    );
}

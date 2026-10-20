//! Loading a template from another Curvyo document (`specs/0046-document-shapes` criteria 20,
//! 22, 23, 24 (stage 1), 28, 29 and 33): the golden sources, what comes in, where it goes, the
//! one step, and the limits.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::path::PathBuf;

use curvyo_document_core::{
    AnchorId, Document, EllipseFrame, LOAD_TEMPLATE_COMMIT_LABEL, NewAnchor, NodeId,
    ObjectEditError, ObjectSnapshot, Point, TemplateNode, Vec2, pack, unpack,
};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn square(n: u64, x: f64, y: f64, size: f64) -> Vec<NewAnchor> {
    [(0.0, 0.0), (size, 0.0), (size, size), (0.0, size)]
        .iter()
        .zip(0_u64..)
        .map(|(&(dx, dy), k)| {
            NewAnchor::corner(AnchorId::new(1, n * 10 + k), Point::new(x + dx, y + dy))
        })
        .collect()
}

fn circle(d: &Document, cx: f64, cy: f64, r: f64) -> NodeId {
    d.create_ellipse(EllipseFrame::from_corners(
        Point::new(cx - r, cy - r),
        Point::new(cx + r, cy + r),
    ))
}

fn line(d: &Document, n: u64, a: Point, b: Point) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, n * 10), a),
            NewAnchor::corner(AnchorId::new(1, n * 10 + 1), b),
        ],
        false,
    )
}

/// A key ring: one compound path, an outer square with a hole.
fn key_ring() -> Document {
    let d = Document::new(1);
    let outer = d.create_path(&square(1, 0.0, 0.0, 40.0), true);
    let ring = d
        .replace_with_path(
            &[outer],
            outer,
            &[
                (square(2, 0.0, 0.0, 40.0), true),
                (square(3, 5.0, 5.0, 6.0), true),
            ],
            "combine",
        )
        .unwrap();
    d.rename_node(ring, "Key ring").unwrap();
    d
}

/// Two concentric circles, a donut.
fn two_circles() -> Document {
    let d = Document::new(1);
    // Never two adjacent steps with one label: an older file shows those as one row.
    let outer = circle(&d, 50.0, 50.0, 40.0);
    d.rename_node(outer, "Outer").unwrap();
    circle(&d, 50.0, 50.0, 10.0);
    d
}

/// Four open lines that form a frame.
fn four_lines() -> Document {
    let d = Document::new(1);
    let p = |x, y| Point::new(x, y);
    let ends = [
        (p(0.0, 0.0), p(80.0, 0.0)),
        (p(80.0, 0.0), p(80.0, 50.0)),
        (p(80.0, 50.0), p(0.0, 50.0)),
        (p(0.0, 50.0), p(0.0, 0.0)),
    ];
    // Never two adjacent steps with one label: each line is named after it is drawn.
    for ((a, b), n) in ends.into_iter().zip(1_u64..) {
        let id = line(&d, n, a, b);
        d.rename_node(id, &format!("Side {n}")).unwrap();
    }
    d
}

/// Guides and ordinary paths: only the two guides come in.
fn mixed() -> Document {
    let d = Document::new(1);
    let guide = d.create_path(&square(1, 10.0, 10.0, 30.0), true);
    d.rename_node(guide, "Label").unwrap();
    let _ordinary = d.create_path(&square(2, 60.0, 10.0, 30.0), true);
    let disc = circle(&d, 25.0, 80.0, 15.0);
    d.set_guide(&[guide, disc], true).unwrap();
    d
}

fn sources() -> [(&'static str, Document); 4] {
    [
        ("template_key_ring.curvyo", key_ring()),
        ("template_two_circles.curvyo", two_circles()),
        ("template_four_lines.curvyo", four_lines()),
        ("template_mixed.curvyo", mixed()),
    ]
}

fn source(name: &str) -> Document {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        for (file, document) in sources() {
            std::fs::write(fixture_path(file), pack(&document, "0.1.0").unwrap()).unwrap();
        }
    }
    unpack(2, &std::fs::read(fixture_path(name)).unwrap()).unwrap()
}

fn ids_for(nodes: &[TemplateNode], peer: u64, from: u64) -> Vec<AnchorId> {
    let count: usize = nodes.iter().map(TemplateNode::anchor_count).sum();
    (0..count as u64)
        .map(|k| AnchorId::new(peer, from + k))
        .collect()
}

fn load(target: &Document, source: &Document, from: u64) -> Vec<NodeId> {
    let nodes = source.template_nodes();
    target
        .load_template(&nodes, &ids_for(&nodes, 9, from))
        .unwrap()
}

/// The geometry of an object without ids: points, closed flags, shapes.
fn geometry(object: &ObjectSnapshot) -> String {
    match object {
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Path(path) => {
            let points = |anchors: &[NewAnchor]| -> Vec<(u64, u64)> {
                anchors
                    .iter()
                    .map(|a| (a.point.x.to_bits(), a.point.y.to_bits()))
                    .collect()
            };
            let subs: Vec<_> = path
                .extra_subpaths
                .iter()
                .map(|s| (s.closed, points(&s.anchors)))
                .collect();
            format!(
                "path {} {:?} {:?}",
                path.closed,
                points(&path.anchors),
                subs
            )
        }
        ObjectSnapshot::Primitive(p) => format!("shape {:?}", p.shape),
        ObjectSnapshot::Clone(c) => format!("clone {:?}", c.resolved.anchors),
    }
}

fn leaf_geometries(d: &Document, ids: &[NodeId]) -> Vec<String> {
    ids.iter()
        .map(|&id| geometry(&d.object(id).unwrap()))
        .collect()
}

fn source_geometries(d: &Document) -> Vec<String> {
    d.read_objects().objects.iter().map(geometry).collect()
}

#[test]
fn c20_c24_a_key_ring_comes_in_as_one_guide_with_its_hole_and_name() {
    let s = source("template_key_ring.curvyo");
    let d = Document::new(5);
    let loaded = load(&d, &s, 0);
    assert_eq!(loaded.len(), 1);
    assert_eq!(leaf_geometries(&d, &loaded), source_geometries(&s));
    assert!(d.node_registers(loaded[0]).unwrap().guide);
    assert_eq!(
        d.node_registers(loaded[0]).unwrap().name.as_deref(),
        Some("Key ring")
    );
}

#[test]
fn c20_c22_two_circles_and_four_lines_come_in_unchanged() {
    for name in ["template_two_circles.curvyo", "template_four_lines.curvyo"] {
        let s = source(name);
        let d = Document::new(5);
        let loaded = load(&d, &s, 0);
        assert_eq!(
            leaf_geometries(&d, &loaded),
            source_geometries(&s),
            "{name}"
        );
        assert!(
            loaded.iter().all(|&id| d.node_registers(id).unwrap().guide),
            "{name}"
        );
    }
}

#[test]
fn c20_only_the_guides_come_in_when_the_source_has_some() {
    let s = source("template_mixed.curvyo");
    let d = Document::new(5);
    let loaded = load(&d, &s, 0);
    assert_eq!(loaded.len(), 2);
    let names: Vec<Option<String>> = loaded
        .iter()
        .map(|&id| d.node_registers(id).unwrap().name)
        .collect();
    assert_eq!(names, vec![Some("Label".to_owned()), None]);
    assert!(matches!(
        d.object(loaded[1]),
        Some(ObjectSnapshot::Primitive(_))
    ));
}

#[test]
fn c20_hidden_nodes_stay_out_and_a_clone_comes_in_as_its_path() {
    let s = Document::new(1);
    let a = s.create_path(&square(1, 0.0, 0.0, 10.0), true);
    let hidden = s.create_path(&square(2, 20.0, 0.0, 10.0), true);
    s.set_node_visible(&[hidden], false).unwrap();
    let clone = s.linked_clone(&[a], &[]).unwrap()[0];
    s.translate_objects(&[clone], Vec2::new(0.0, 30.0)).unwrap();
    let nodes = s.template_nodes();
    assert_eq!(nodes.len(), 2);
    assert!(
        nodes
            .iter()
            .all(|n| !matches!(n.object, ObjectSnapshot::Clone(_)))
    );
    let d = Document::new(5);
    let loaded = d.load_template(&nodes, &ids_for(&nodes, 9, 0)).unwrap();
    match d.object(loaded[1]).unwrap() {
        ObjectSnapshot::Path(path) => assert_eq!(path.anchors[0].point, Point::new(0.0, 30.0)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn c23_one_step_in_the_active_layer_at_the_top_and_loading_again_adds_again() {
    let s = source("template_two_circles.curvyo");
    let d = Document::new(5);
    let below = d.create_path(&square(7, 0.0, 0.0, 5.0), true);
    let layer = d.new_layer(None);
    d.set_draw_parent(Some(layer));
    let inside = d.create_path(&square(8, 0.0, 0.0, 5.0), true);
    let steps = d.own_steps().len();
    let first = load(&d, &s, 0);
    assert_eq!(d.own_steps().len(), steps + 1);
    assert_eq!(
        d.own_steps().last().unwrap().label,
        LOAD_TEMPLATE_COMMIT_LABEL
    );
    let mut expected = vec![inside];
    expected.extend(&first);
    assert_eq!(d.children(Some(layer)), expected);
    let second = load(&d, &s, 1000);
    assert_eq!(second.len(), 2);
    assert_eq!(d.children(Some(layer)).len(), 5);
    assert_eq!(d.children(None), vec![below, layer]);
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert_eq!(d.children(Some(layer)).len(), 3);
}

#[test]
fn c23_wrong_anchor_ids_write_nothing() {
    let s = source("template_key_ring.curvyo");
    let nodes = s.template_nodes();
    let d = Document::new(5);
    let steps = d.own_steps().len();
    let good = ids_for(&nodes, 9, 0);
    let mut twice = good.clone();
    twice[1] = twice[0];
    for ids in [&good[1..], &twice[..]] {
        assert!(matches!(
            d.load_template(&nodes, ids),
            Err(ObjectEditError::AnchorIds)
        ));
    }
    let _taken = d.create_path(&[NewAnchor::corner(good[0], Point::new(0.0, 0.0))], false);
    assert!(matches!(
        d.load_template(&nodes, &good),
        Err(ObjectEditError::AnchorIds)
    ));
    assert_eq!(d.own_steps().len(), steps + 1, "only the path above");
    assert_eq!(d.load_template(&[], &[]).unwrap(), Vec::<NodeId>::new());
}

#[test]
fn c29_the_preview_of_a_version_knows_its_guides() {
    let s = source("template_mixed.curvyo");
    let d = Document::new(5);
    let before = d.create_path(&square(7, 0.0, 0.0, 5.0), true);
    let s_before = d.own_steps().last().unwrap().step;
    let loaded = load(&d, &s, 0);
    let s_after = d.own_steps().last().unwrap().step;
    assert!(d.preview_document_at(s_before).unwrap().guides.is_empty());
    let after = d.preview_document_at(s_after).unwrap();
    assert_eq!(after.guides.len(), 2);
    assert!(loaded.iter().all(|id| after.guides.contains(id)));
    assert!(!after.guides.contains(&before));
}

#[test]
fn c33_five_thousand_nodes_and_one_hundred_metres_load() {
    let s = Document::new(1);
    for n in 0..5000_u64 {
        let x = (n % 100) as f64 * 1000.0;
        let _ = s.create_path(&square(n + 1, x, (n / 100) as f64 * 10.0, 5.0), true);
    }
    circle(&s, 50_000.0, 0.0, 50_000.0);
    let d = Document::new(5);
    let loaded = load(&d, &s, 0);
    assert_eq!(loaded.len(), 5001);
    assert_eq!(leaf_geometries(&d, &loaded), source_geometries(&s));
    let wide = d.object(loaded[5000]).unwrap();
    assert!(!format!("{wide:?}").contains("NaN"));
}

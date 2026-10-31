//! How a transform gesture maps a linked clone (`specs/0049-linked-clone` criteria 14 to 16):
//! its drawn outline moves exactly as a path's would, and its matrix is settled at the commit by
//! the one rule of the document core.

use std::collections::HashMap;

use curvyo_document_core::{
    Affine, AnchorId, CloneLinks, CloneSnapshot, NewAnchor, NodeId, ObjectSnapshot, PathSnapshot,
    Point, SINGULAR_DETERMINANT, Style, clone_matrices_after,
};

/// The affine map `f` applies to every point, read from where it sends three probe points:
/// exact for the transforms of a path (move, scale, rotate, skew about any pivot).
/// `id` only names the probe.
pub(crate) fn affine_of(id: NodeId, f: &dyn Fn(&PathSnapshot) -> PathSnapshot) -> Affine {
    let probe = |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(0, k), Point::new(x, y));
    let mapped = f(&PathSnapshot {
        id,
        closed: false,
        style: Style::default(),
        anchors: vec![probe(0, 0.0, 0.0), probe(1, 1.0, 0.0), probe(2, 0.0, 1.0)],
        extra_subpaths: Vec::new(),
        rotation: curvyo_document_core::Angle::from_radians(0.0),
    });
    let [o, x, y] = [0, 1, 2].map(|k| mapped.anchors[k].point);
    Affine::new(x.x - o.x, x.y - o.y, y.x - o.x, y.y - o.y, o.x, o.y)
}

/// `clone` after a gesture that maps the document by `g`: its drawn outline is `g` applied to
/// the outline it had, its matrix `g·M` until the commit settles it. With `unlinked` (the path
/// the clone becomes, prepared at the press for a clone whose chain moves too) and a singular
/// `g`, the clone is unlinked and mapped as that path (criterion 16).
pub(crate) fn map_clone(
    clone: &CloneSnapshot,
    g: Affine,
    unlinked: Option<&PathSnapshot>,
) -> ObjectSnapshot {
    match unlinked {
        Some(path) if g.determinant().abs() < SINGULAR_DETERMINANT => {
            ObjectSnapshot::Path(path.mapped(g))
        }
        _ => ObjectSnapshot::Clone(clone.mapped_by(g)),
    }
}

/// The path a clone becomes if a gesture must unlink it: its drawn outline with placeholder
/// anchor ids, which the session replaces with minted ones at the press.
pub(crate) fn unlinked_placeholder(clone: &CloneSnapshot) -> PathSnapshot {
    let mut path = clone.resolved.clone();
    for (k, anchor) in path.all_anchors_mut().enumerate() {
        anchor.id = AnchorId::new(0, k as u64);
    }
    path
}

/// Whether a gesture on `starts` moves something on the chain of `clone` too: its original or
/// its root is among them.
pub(crate) fn chain_moves(clone: &CloneSnapshot, starts: &[ObjectSnapshot]) -> bool {
    starts
        .iter()
        .any(|object| object.id() == clone.chain.origin || object.id() == clone.chain.root)
}

/// Settles the matrices of the clones among `results` before they are written: when the
/// gesture's map `g` also moves objects on a clone's chain, the matrix is the one that keeps the
/// clone's drawn outline where the gesture put it (`clone_matrices_after`, criterion 16; an
/// unselected clone with own placement on the chain does not move, `0057` criterion 5). `links`
/// is [`curvyo_document_core::Document::clone_links`], `held`
/// [`curvyo_document_core::Document::own_placement_clones`].
pub(crate) fn settle_clone_matrices(
    links: &CloneLinks,
    held: &std::collections::HashSet<NodeId>,
    results: &mut [ObjectSnapshot],
    g: Affine,
) {
    let ids: Vec<NodeId> = results.iter().map(ObjectSnapshot::id).collect();
    let settled: HashMap<NodeId, Affine> = clone_matrices_after(links, held, &ids, g)
        .matrices
        .into_iter()
        .collect();
    for result in results.iter_mut() {
        if let ObjectSnapshot::Clone(clone) = result
            && let Some(&matrix) = settled.get(&clone.id())
        {
            clone.chain.matrix = matrix;
        }
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Angle, Document, Vec2};

    use super::*;

    fn square(document: &Document, n: u64) -> NodeId {
        let corner = |k: u64, x: f64, y: f64| {
            NewAnchor::corner(AnchorId::new(1, n * 10 + k), Point::new(x, y))
        };
        document.create_path(
            &[
                corner(0, 0.0, 0.0),
                corner(1, 10.0, 0.0),
                corner(2, 10.0, 10.0),
                corner(3, 0.0, 10.0),
            ],
            true,
        )
    }

    fn snapshot(document: &Document, id: NodeId) -> ObjectSnapshot {
        document.object(id).unwrap()
    }

    #[test]
    fn the_probe_reads_the_map_of_a_rotation_about_a_pivot() {
        let document = Document::new(1);
        let id = square(&document, 1);
        let pivot = Point::new(3.0, 4.0);
        let angle = Angle::from_radians(0.6);
        let g = affine_of(id, &|path: &PathSnapshot| path.rotated(pivot, angle));
        let wanted = Affine::rotation_about(pivot, angle);
        for (a, b) in g.to_array().iter().zip(wanted.to_array()) {
            assert!((a - b).abs() < 1e-12);
        }
    }

    /// Criterion 16 (c) through the gesture layer: O and B scaled by (2, 1), A not selected.
    #[test]
    fn settled_matrices_keep_each_drawn_outline_where_the_gesture_put_it() {
        let document = Document::new(1);
        let o = square(&document, 1);
        let a = document.linked_clone(&[o], &[]).unwrap()[0];
        document
            .translate_objects(&[a], Vec2::new(30.0, 0.0))
            .unwrap();
        let b = document.linked_clone(&[a], &[]).unwrap()[0];
        document
            .translate_objects(&[b], Vec2::new(0.0, 30.0))
            .unwrap();
        let g = Affine::scale(2.0, 1.0);
        let mut results: Vec<ObjectSnapshot> = [o, b]
            .iter()
            .map(|&id| match snapshot(&document, id) {
                ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
                ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
                ObjectSnapshot::Sketch(_) => unreachable!("no sketch in this test"),
                ObjectSnapshot::Path(path) => ObjectSnapshot::Path(path.mapped(g)),
                ObjectSnapshot::Clone(clone) => map_clone(&clone, g, None),
                other @ ObjectSnapshot::Primitive(_) => other,
            })
            .collect();
        settle_clone_matrices(
            &document.clone_links(),
            &document.own_placement_clones(),
            &mut results,
            g,
        );
        let ObjectSnapshot::Clone(settled) = &results[1] else {
            panic!("a clone")
        };
        assert_eq!(
            settled.chain.matrix,
            Affine::translation(Vec2::new(30.0, 30.0))
        );
        document.transform_objects(&results, false).unwrap();
        let ObjectSnapshot::Clone(drawn) = snapshot(&document, b) else {
            panic!("a clone")
        };
        let xs: Vec<f64> = drawn.resolved.anchors.iter().map(|a| a.point.x).collect();
        assert!((xs[0] - 60.0).abs() < 1e-9 && (xs[1] - 80.0).abs() < 1e-9);
    }

    /// Review F3: with an ill-conditioned stored matrix (1e4 and 1e-4: the clone of a 10 mm square
    /// stays within the limits after the move), settling with the gesture's own map keeps
    /// the drawn outline exact where reading the map back from the result would not.
    #[test]
    fn an_ill_conditioned_matrix_keeps_the_drawn_outline() {
        let document = Document::new(1);
        let o = square(&document, 1);
        let c = document.linked_clone(&[o], &[]).unwrap()[0];
        let ObjectSnapshot::Clone(clone) = snapshot(&document, c) else {
            panic!("a clone")
        };
        let squeezed = clone.mapped_by(Affine::new(1e4, 0.0, 0.0, 1e-4, 0.0, 0.0));
        document
            .transform_objects(&[ObjectSnapshot::Clone(squeezed)], false)
            .unwrap();
        let g = Affine::translation(Vec2::new(100.0, 0.0));
        let ObjectSnapshot::Clone(before) = snapshot(&document, c) else {
            panic!("a clone")
        };
        let mut results: Vec<ObjectSnapshot> = [o, c]
            .iter()
            .map(|&id| match snapshot(&document, id) {
                ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
                ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
                ObjectSnapshot::Sketch(_) => unreachable!("no sketch in this test"),
                ObjectSnapshot::Path(path) => ObjectSnapshot::Path(path.mapped(g)),
                ObjectSnapshot::Clone(clone) => map_clone(&clone, g, None),
                other @ ObjectSnapshot::Primitive(_) => other,
            })
            .collect();
        settle_clone_matrices(
            &document.clone_links(),
            &document.own_placement_clones(),
            &mut results,
            g,
        );
        document.transform_objects(&results, false).unwrap();
        let ObjectSnapshot::Clone(after) = snapshot(&document, c) else {
            panic!("a clone")
        };
        for (a, b) in after.resolved.anchors.iter().zip(&before.resolved.anchors) {
            assert!(
                (a.point.x - (b.point.x + 100.0)).abs() < 1e-6,
                "{a:?} {b:?}"
            );
            assert!((a.point.y - b.point.y).abs() < 1e-6, "{a:?} {b:?}");
        }
    }

    #[test]
    fn a_singular_map_unlinks_only_a_clone_prepared_for_it() {
        let document = Document::new(1);
        let o = square(&document, 1);
        let c = document.linked_clone(&[o], &[]).unwrap()[0];
        let ObjectSnapshot::Clone(clone) = snapshot(&document, c) else {
            panic!("a clone")
        };
        let flat = Affine::scale(0.0, 1.0);
        assert!(matches!(
            map_clone(&clone, flat, None),
            ObjectSnapshot::Clone(_)
        ));
        let placeholder = unlinked_placeholder(&clone);
        assert!(matches!(
            map_clone(&clone, flat, Some(&placeholder)),
            ObjectSnapshot::Path(_)
        ));
        assert!(chain_moves(&clone, &[snapshot(&document, o)]));
        assert!(!chain_moves(&clone, &[]));
    }
}

//! The one rule for the matrices of clones when a command maps a set of objects by one document
//! map G (`specs/0049-linked-clone/adrs.md` decision 4, criteria 15, 16 and 34).
//!
//! Every object's outline changes by a document map T: G if it is selected, the identity if
//! nothing on its chain moves, and for an unselected clone Y with original Z
//! `T_Y = M_Y · T_Z · M_Y⁻¹`, computed root first. A selected clone X with original Y gets
//! `M_X' = G · M_X · T_Y⁻¹`, so its drawn outline is G applied to the outline it had.

use std::collections::{HashMap, HashSet};

use crate::affine::Affine;
use crate::path_model::NodeId;

/// The link of every clone the rule may need: clone id to `(origin, matrix)`. Ids that are not
/// in the map are roots (paths, compound paths) or not clones at all.
pub type CloneLinks = HashMap<NodeId, (NodeId, Affine)>;

/// What [`clone_matrices_after`] decides for the selected clones.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CloneMatrices {
    /// The new matrix of every selected clone that keeps its link.
    pub matrices: Vec<(NodeId, Affine)>,
    /// The selected clones whose invariant needs an inverse that does not exist (G or a matrix
    /// on their chain is singular): they are unlinked first and then mapped as paths
    /// (`0049` criterion 16).
    pub singular: Vec<NodeId>,
}

/// The matrices the selected clones get when every object of `selected` is mapped by `g`
/// (`links` holds at least every selected clone and the clones on their chains). Selected ids
/// that are not clones are mapped by the caller as paths. The result follows the order of
/// `selected_order`.
#[must_use]
pub fn clone_matrices_after(
    links: &CloneLinks,
    selected_order: &[NodeId],
    g: Affine,
) -> CloneMatrices {
    let selected: HashSet<NodeId> = selected_order.iter().copied().collect();
    let mut maps = Maps {
        links,
        selected: &selected,
        g,
        known: HashMap::new(),
    };
    let mut result = CloneMatrices::default();
    for &id in selected_order {
        let Some(&(origin, matrix)) = links.get(&id) else {
            continue;
        };
        match maps.map_of(origin) {
            Some(Moved::Identity) => result.matrices.push((id, g.compose(matrix))),
            Some(Moved::By(t)) => match t.inverse() {
                Some(inverse) => result
                    .matrices
                    .push((id, g.compose(matrix).compose(inverse))),
                None => result.singular.push(id),
            },
            None => result.singular.push(id),
        }
    }
    result
}

/// How an object's outline changes.
#[derive(Debug, Clone, Copy)]
enum Moved {
    /// Not at all.
    Identity,
    /// By this document map.
    By(Affine),
}

/// The maps T of the objects met so far, with what they are computed from.
struct Maps<'a> {
    links: &'a CloneLinks,
    selected: &'a HashSet<NodeId>,
    g: Affine,
    /// T per object; `None` when it needs an inverse that does not exist.
    known: HashMap<NodeId, Option<Moved>>,
}

impl Maps<'_> {
    /// T of `id`, computed root first with an explicit stack (no recursion).
    fn map_of(&mut self, id: NodeId) -> Option<Moved> {
        let mut stack = Vec::new();
        let mut on_stack = HashSet::new();
        let mut current = id;
        let mut base = loop {
            if let Some(known) = self.known.get(&current) {
                break *known;
            }
            if self.selected.contains(&current) {
                break Some(Moved::By(self.g));
            }
            match self.links.get(&current) {
                Some(&(origin, _)) if on_stack.insert(current) => {
                    stack.push(current);
                    current = origin;
                }
                // A root, or a ring (the caller holds no orphans; a ring moves nothing).
                _ => break Some(Moved::Identity),
            }
        };
        if !on_stack.contains(&current) {
            self.known.insert(current, base);
        }
        while let Some(node) = stack.pop() {
            base = match (base, self.links.get(&node)) {
                (Some(Moved::Identity), _) => Some(Moved::Identity),
                (Some(Moved::By(t)), Some(&(_, m))) => m
                    .inverse()
                    .map(|inverse| Moved::By(m.compose(t).compose(inverse))),
                _ => None,
            };
            self.known.insert(node, base);
        }
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{Point, Vec2};

    fn id(n: i32) -> NodeId {
        NodeId::from_parts(1, n)
    }

    fn translate(x: f64, y: f64) -> Affine {
        Affine::translation(Vec2::new(x, y))
    }

    /// O = 1, A = 2 (clone of O, translate (30, 0)), B = 3 (clone of A, translate (0, 30)).
    fn chain() -> CloneLinks {
        HashMap::from([
            (id(2), (id(1), translate(30.0, 0.0))),
            (id(3), (id(2), translate(0.0, 30.0))),
        ])
    }

    fn matrix_of(result: &CloneMatrices, n: i32) -> Affine {
        result
            .matrices
            .iter()
            .find(|(clone, _)| *clone == id(n))
            .map(|(_, m)| *m)
            .unwrap()
    }

    fn close(a: Affine, b: Affine) -> bool {
        a.to_array()
            .iter()
            .zip(b.to_array())
            .all(|(x, y)| (x - y).abs() < 1e-9)
    }

    #[test]
    fn criterion_15_a_clone_alone_gets_g_times_m() {
        let g = Affine::scale(2.0, 1.0);
        let result = clone_matrices_after(&chain(), &[id(3)], g);
        assert!(close(
            matrix_of(&result, 3),
            g.compose(translate(0.0, 30.0))
        ));
    }

    #[test]
    fn criterion_16a_original_and_clone_scaled_together() {
        // The origin square and its clone at (30, 0): M' = G·M·G⁻¹ = translate (60, 0).
        let links = HashMap::from([(id(2), (id(1), translate(30.0, 0.0)))]);
        let result = clone_matrices_after(&links, &[id(1), id(2)], Affine::scale(2.0, 1.0));
        assert!(close(matrix_of(&result, 2), translate(60.0, 0.0)));
    }

    #[test]
    fn criterion_16b_a_and_b_selected() {
        let result = clone_matrices_after(&chain(), &[id(2), id(3)], Affine::scale(2.0, 1.0));
        assert!(close(
            matrix_of(&result, 2),
            Affine::new(2.0, 0.0, 0.0, 1.0, 60.0, 0.0)
        ));
        assert!(close(matrix_of(&result, 3), translate(0.0, 30.0)));
    }

    #[test]
    fn criterion_16c_o_and_b_selected_a_not() {
        let result = clone_matrices_after(&chain(), &[id(1), id(3)], Affine::scale(2.0, 1.0));
        assert!(close(matrix_of(&result, 3), translate(30.0, 30.0)));
        // B's drawn outline: M_B'·M_A·(G·O) equals G·(M_B·M_A·O) at a probe point.
        let p = Point::new(3.0, 4.0);
        let g = Affine::scale(2.0, 1.0);
        let after = matrix_of(&result, 3)
            .compose(translate(30.0, 0.0))
            .compose(g)
            .apply_point(p);
        let wanted = g
            .compose(translate(0.0, 30.0))
            .compose(translate(30.0, 0.0))
            .apply_point(p);
        assert!((after.x - wanted.x).abs() < 1e-9 && (after.y - wanted.y).abs() < 1e-9);
    }

    #[test]
    fn a_singular_map_unlinks_only_the_clones_whose_chain_moves() {
        let flat = Affine::scale(0.0, 1.0);
        let result = clone_matrices_after(&chain(), &[id(1), id(3)], flat);
        assert_eq!(result.singular, vec![id(3)]);
        let alone = clone_matrices_after(&chain(), &[id(3)], flat);
        assert_eq!(alone.singular, Vec::<NodeId>::new());
        assert!(close(
            matrix_of(&alone, 3),
            flat.compose(translate(0.0, 30.0))
        ));
    }

    #[test]
    fn a_singular_matrix_in_the_chain_unlinks() {
        let links = HashMap::from([
            (id(2), (id(1), Affine::scale(0.0, 1.0))),
            (id(3), (id(2), Affine::IDENTITY)),
        ]);
        let result = clone_matrices_after(&links, &[id(1), id(3)], Affine::scale(2.0, 2.0));
        assert_eq!(result.singular, vec![id(3)]);
    }

    #[test]
    fn a_long_chain_needs_no_recursion() {
        let mut links = CloneLinks::new();
        for k in 2..=20_000 {
            links.insert(id(k), (id(k - 1), translate(1.0, 0.0)));
        }
        let result = clone_matrices_after(&links, &[id(1), id(20_000)], translate(5.0, 0.0));
        assert!(close(matrix_of(&result, 20_000), translate(1.0, 0.0)));
    }
}

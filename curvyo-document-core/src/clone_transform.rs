//! The one rule for the matrices of clones when a command maps a set of objects by document maps
//! (`specs/0049-linked-clone/adrs.md` decision 4, criteria 15, 16 and 34; ADR 0016 §2, the
//! held-back rule of `specs/0057-clone-overrides` criteria 3 to 7).
//!
//! Every object's outline changes by a document map T: its own map if it is selected, the
//! identity if nothing on its chain moves, the identity for an unselected clone with own
//! placement (it is held back), and for any other unselected clone Y with original Z
//! `T_Y = M_Y · T_Z · M_Y⁻¹`, computed root first. A selected clone X with original Y gets
//! `M_X' = G_X · M_X · T_Y⁻¹`, so its drawn outline is its map applied to the outline it had; an
//! unselected clone with own placement and original Z gets `M' = M · T_Z⁻¹`, so its drawn outline
//! stays.

use std::collections::{HashMap, HashSet};

use crate::affine::{Affine, SINGULAR_DETERMINANT};
use crate::path_model::NodeId;

/// The link of every clone the rule may need: clone id to `(origin, matrix)`. Ids that are not
/// in the map are roots (paths, compound paths) or not clones at all.
pub type CloneLinks = HashMap<NodeId, (NodeId, Affine)>;

/// What [`clone_matrices_after`] decides.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CloneMatrices {
    /// The new matrix of every selected clone that keeps its link.
    pub matrices: Vec<(NodeId, Affine)>,
    /// The selected clones whose invariant needs an inverse that does not exist (G or a matrix
    /// on their chain is singular): they are unlinked first and then mapped as paths
    /// (`0049` criterion 16).
    pub singular: Vec<NodeId>,
    /// The new matrix of every unselected clone with own placement whose original moves: it is
    /// held back (`0057` criterion 3). In id order.
    pub held_back: Vec<(NodeId, Affine)>,
    /// The unselected clones with own placement whose original's map cannot be inverted: they are
    /// unlinked with the outline they had (`0057` criterion 7). In id order.
    pub held_unlinked: Vec<NodeId>,
}

/// The matrices the clones get when every object of `selected` is mapped by `g` (`links` holds
/// at least every selected clone and the clones on their chains, `held` the clones with own
/// placement). Selected ids that are not clones are mapped by the caller as paths. `matrices`
/// follows the order of `selected_order`.
#[must_use]
#[allow(clippy::implicit_hasher)] // the one set type every caller has
pub fn clone_matrices_after(
    links: &CloneLinks,
    held: &HashSet<NodeId>,
    selected_order: &[NodeId],
    g: Affine,
) -> CloneMatrices {
    let moved: Vec<(NodeId, Affine)> = selected_order.iter().map(|&id| (id, g)).collect();
    clone_matrices_after_maps(links, held, &moved)
}

/// [`clone_matrices_after`] with one map per moved object: what a writer uses that knows the
/// change of each outline rather than one gesture map.
pub(crate) fn clone_matrices_after_maps(
    links: &CloneLinks,
    held: &HashSet<NodeId>,
    moved: &[(NodeId, Affine)],
) -> CloneMatrices {
    let selected: HashMap<NodeId, Affine> = moved.iter().copied().collect();
    let mut maps = Maps {
        links,
        selected: &selected,
        held,
        known: HashMap::new(),
    };
    let mut result = CloneMatrices::default();
    let mut seen = HashSet::new();
    for &(id, g) in moved {
        let Some(&(origin, matrix)) = links.get(&id) else {
            continue;
        };
        if !seen.insert(id) {
            continue;
        }
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
    let mut flagged: Vec<NodeId> = held
        .iter()
        .copied()
        .filter(|id| links.contains_key(id) && !selected.contains_key(id))
        .collect();
    flagged.sort_unstable_by_key(|id| (id.peer, id.counter));
    for id in flagged {
        let (origin, matrix) = links[&id];
        match maps.map_of(origin) {
            Some(Moved::Identity) => {}
            // The original's change needs an inverse on its chain that does not exist (a
            // singular matrix there): the clone cannot be held back by a matrix, so it is
            // unlinked with the outline it had (`0057` criterion 7).
            None => result.held_unlinked.push(id),
            Some(Moved::By(t)) => match t.inverse() {
                Some(inverse) if t.determinant().abs() >= SINGULAR_DETERMINANT => {
                    result.held_back.push((id, matrix.compose(inverse)));
                }
                _ => result.held_unlinked.push(id),
            },
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
    selected: &'a HashMap<NodeId, Affine>,
    held: &'a HashSet<NodeId>,
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
            if let Some(&g) = self.selected.get(&current) {
                break Some(Moved::By(g));
            }
            if self.held.contains(&current) && self.links.contains_key(&current) {
                // An unselected clone with own placement is held back: it does not move.
                break Some(Moved::Identity);
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

    fn none() -> HashSet<NodeId> {
        HashSet::new()
    }

    fn held(ids: &[i32]) -> HashSet<NodeId> {
        ids.iter().map(|&n| id(n)).collect()
    }

    fn held_of(result: &CloneMatrices, n: i32) -> Affine {
        result
            .held_back
            .iter()
            .find(|(clone, _)| *clone == id(n))
            .map(|(_, m)| *m)
            .unwrap()
    }

    #[test]
    fn criterion_3_an_own_placement_clone_is_held_back_and_a_following_one_is_not() {
        // O at 0..10, C = translate (30, 0) with own placement; O moved 5 right.
        let links = HashMap::from([
            (id(2), (id(1), translate(30.0, 0.0))),
            (id(3), (id(1), translate(60.0, 0.0))),
        ]);
        let result = clone_matrices_after(&links, &held(&[2]), &[id(1)], translate(5.0, 0.0));
        assert!(close(held_of(&result, 2), translate(25.0, 0.0)));
        assert_eq!(
            result.held_back.len(),
            1,
            "the following clone is not written"
        );
        assert_eq!(result.matrices, Vec::new());
    }

    #[test]
    fn criterion_5a_a_held_middle_clone_keeps_its_followers_in_place() {
        let result = clone_matrices_after(&chain(), &held(&[2]), &[id(1)], translate(5.0, 0.0));
        assert_eq!(result.held_back.len(), 1);
        assert!(close(held_of(&result, 2), translate(25.0, 0.0)));
    }

    #[test]
    fn criterion_5b_a_held_end_clone_behind_a_following_one() {
        // O moved (5, 0): A follows, B (own placement) gets M_B·T_A⁻¹ = translate (-5, 30).
        let result = clone_matrices_after(&chain(), &held(&[3]), &[id(1)], translate(5.0, 0.0));
        assert!(close(held_of(&result, 3), translate(-5.0, 30.0)));
    }

    #[test]
    fn criterion_5c_and_6_a_moved_clone_holds_back_its_follower_and_a_selected_one_is_not_held() {
        let g = translate(0.0, 7.0);
        let result = clone_matrices_after(&chain(), &held(&[3]), &[id(2)], g);
        assert!(close(held_of(&result, 3), translate(0.0, 23.0)));
        let both = clone_matrices_after(&chain(), &held(&[2, 3]), &[id(1), id(2)], g);
        assert!(both.held_back.iter().all(|(c, _)| *c == id(3)));
        assert!(
            close(matrix_of(&both, 2), translate(30.0, 0.0)),
            "rule of criterion 16"
        );
    }

    #[test]
    fn criterion_7_a_singular_map_unlinks_held_clones() {
        let flat = Affine::scale(0.0, 1.0);
        let result = clone_matrices_after(&chain(), &held(&[2]), &[id(1)], flat);
        assert_eq!(result.held_unlinked, vec![id(2)]);
        assert_eq!(result.held_back, Vec::new());
    }

    #[test]
    fn a_held_clone_behind_a_selected_held_clone_uses_its_map() {
        // A selected (own placement, so the flag does not matter) and B held behind it.
        let g = Affine::scale(2.0, 1.0);
        let result = clone_matrices_after(&chain(), &held(&[2, 3]), &[id(2)], g);
        let wanted = translate(0.0, 30.0).compose(g.inverse().unwrap());
        assert!(close(held_of(&result, 3), wanted));
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
        let result = clone_matrices_after(&chain(), &none(), &[id(3)], g);
        assert!(close(
            matrix_of(&result, 3),
            g.compose(translate(0.0, 30.0))
        ));
    }

    #[test]
    fn criterion_16a_original_and_clone_scaled_together() {
        // The origin square and its clone at (30, 0): M' = G·M·G⁻¹ = translate (60, 0).
        let links = HashMap::from([(id(2), (id(1), translate(30.0, 0.0)))]);
        let result =
            clone_matrices_after(&links, &none(), &[id(1), id(2)], Affine::scale(2.0, 1.0));
        assert!(close(matrix_of(&result, 2), translate(60.0, 0.0)));
    }

    #[test]
    fn criterion_16b_a_and_b_selected() {
        let result =
            clone_matrices_after(&chain(), &none(), &[id(2), id(3)], Affine::scale(2.0, 1.0));
        assert!(close(
            matrix_of(&result, 2),
            Affine::new(2.0, 0.0, 0.0, 1.0, 60.0, 0.0)
        ));
        assert!(close(matrix_of(&result, 3), translate(0.0, 30.0)));
    }

    #[test]
    fn criterion_16c_o_and_b_selected_a_not() {
        let result =
            clone_matrices_after(&chain(), &none(), &[id(1), id(3)], Affine::scale(2.0, 1.0));
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
        let result = clone_matrices_after(&chain(), &none(), &[id(1), id(3)], flat);
        assert_eq!(result.singular, vec![id(3)]);
        let alone = clone_matrices_after(&chain(), &none(), &[id(3)], flat);
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
        let result =
            clone_matrices_after(&links, &none(), &[id(1), id(3)], Affine::scale(2.0, 2.0));
        assert_eq!(result.singular, vec![id(3)]);
    }

    #[test]
    fn a_long_chain_needs_no_recursion() {
        let mut links = CloneLinks::new();
        for k in 2..=20_000 {
            links.insert(id(k), (id(k - 1), translate(1.0, 0.0)));
        }
        let result =
            clone_matrices_after(&links, &none(), &[id(1), id(20_000)], translate(5.0, 0.0));
        assert!(close(matrix_of(&result, 20_000), translate(1.0, 0.0)));
    }

    #[test]
    fn a_held_clone_behind_a_singular_following_clone_is_unlinked() {
        // Tester finding: O, A follows O with a matrix that collapses x, C held behind A.
        let links = HashMap::from([
            (id(2), (id(1), Affine::new(0.0, 0.0, 0.0, 1.0, 5.0, 0.0))),
            (id(3), (id(2), translate(30.0, 0.0))),
        ]);
        let result = clone_matrices_after(&links, &held(&[3]), &[id(1)], translate(5.0, 3.0));
        assert_eq!(result.held_unlinked, vec![id(3)]);
        assert_eq!(result.held_back, Vec::new());
    }
}

//! The one rule for the matrices of clones when a command maps a set of objects by document maps
//! (`specs/0049-linked-clone/adrs.md` decision 4, criteria 15, 16 and 34; ADR 0016 §2, the
//! held-back rule of `specs/0057-clone-overrides` criteria 3 to 7).
//!
//! Every object's outline changes by a document map T: its own map if it is selected, the
//! identity if nothing on its chain moves, the identity for an unselected clone with own
//! placement held whole, and for any other unselected clone Y with original Z
//! `T_Y = M_Y · T_Z · M_Y⁻¹`, computed root first. A selected clone X with original Y gets
//! `M_X' = G_X · M_X · T_Y⁻¹`, so its drawn outline is its map applied to the outline it had; an
//! unselected clone with own placement and original Z gets `M' = M · T_Z⁻¹`, so its drawn outline
//! stays. A tiling instance holds back only the translation `R` that `T_Z` gives its tiling's
//! pivot (ADR 0016 §2 amendment, ADR 0028 §2): `M' = M · R⁻¹` and `T_Y = M · R⁻¹ · T_Z · M⁻¹`.

use std::collections::{HashMap, HashSet};

use crate::affine::{Affine, SINGULAR_DETERMINANT};
use crate::path_model::NodeId;
use crate::tiling_model::TILING_WRITE_TOLERANCE_MM;
use crate::units::{Point, Vec2};

/// How an unselected clone with own placement holds back the change of its original.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hold {
    /// The whole change: its drawn outline stays (`specs/0057-clone-overrides` criterion 3).
    Whole,
    /// Only the translation the change gives `pivot`, the centre of its tiling's cell box before
    /// the change: a tiling instance follows a scale, stretch, flip, rotation or skew of its
    /// source in its own cell (`specs/0069-tiling` criterion 29).
    Translation {
        /// The point whose move is held back, in document space.
        pivot: Point,
    },
}

/// The clones with own placement and how each holds back its original's change.
pub type HeldClones = HashMap<NodeId, Hold>;

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
    /// The new matrix of every unselected clone with own placement whose original moves and
    /// whose drawn outline stays: it is held back (`0057` criterion 3). In id order.
    pub held_back: Vec<(NodeId, Affine)>,
    /// The new matrix of every tiling instance whose original's change is more than a move: it
    /// holds back the move of its pivot and follows the rest ([`Hold::Translation`]). In id
    /// order.
    pub followed: Vec<(NodeId, Affine)>,
    /// The unselected clones with own placement whose original's map cannot be inverted: they are
    /// unlinked with the outline they had (`0057` criterion 7). In id order.
    pub held_unlinked: Vec<NodeId>,
}

/// The matrices the clones get when every object of `selected` is mapped by `g` (`links` holds
/// at least every selected clone and the clones on their chains, `held` the clones with own
/// placement, [`Document::own_placement_clones`](crate::Document::own_placement_clones)).
/// Selected ids that are not clones are mapped by the caller as paths. `matrices`
/// follows the order of `selected_order`.
#[must_use]
#[allow(clippy::implicit_hasher)] // the one set type every caller has
pub fn clone_matrices_after(
    links: &CloneLinks,
    held: &HeldClones,
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
    held: &HeldClones,
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
    let mut flagged: Vec<(NodeId, Hold)> = held
        .iter()
        .map(|(&id, &hold)| (id, hold))
        .filter(|(id, _)| links.contains_key(id) && !selected.contains_key(id))
        .collect();
    flagged.sort_unstable_by_key(|(id, _)| (id.peer, id.counter));
    for (id, hold) in flagged {
        let (origin, matrix) = links[&id];
        match maps.map_of(origin) {
            Some(Moved::Identity) => {}
            // The original's change needs an inverse on its chain that does not exist (a
            // singular matrix there): the clone cannot be held back by a matrix, so it is
            // unlinked with the outline it had (`0057` criterion 7).
            None => result.held_unlinked.push(id),
            Some(Moved::By(t)) if t.determinant().abs() < SINGULAR_DETERMINANT => {
                result.held_unlinked.push(id);
            }
            Some(Moved::By(t)) => match hold {
                Hold::Whole => match t.inverse() {
                    Some(inverse) => result.held_back.push((id, matrix.compose(inverse))),
                    None => result.held_unlinked.push(id),
                },
                Hold::Translation { pivot } => {
                    let back = shift_back(t, pivot);
                    if back
                        .compose(t)
                        .approx_eq(Affine::IDENTITY, TILING_WRITE_TOLERANCE_MM)
                    {
                        result.held_back.push((id, matrix.compose(back)));
                    } else {
                        result.followed.push((id, matrix.compose(back)));
                    }
                }
            },
        }
    }
    result
}

/// The move `t` gives `pivot` (ADR 0028 §2): what a tiling instance holds back, so the rest of
/// `t`, `R⁻¹ · T` with `R` this translation, is its linear part about `pivot`.
fn pivot_shift(t: Affine, pivot: Point) -> Vec2 {
    let moved = t.apply_point(pivot);
    Vec2::new(moved.x - pivot.x, moved.y - pivot.y)
}

/// `R⁻¹`, the translation back by [`pivot_shift`].
fn shift_back(t: Affine, pivot: Point) -> Affine {
    let shift = pivot_shift(t, pivot);
    Affine::translation(Vec2::new(-shift.x, -shift.y))
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
    held: &'a HeldClones,
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
            if self.held.get(&current) == Some(&Hold::Whole) && self.links.contains_key(&current) {
                // An unselected clone with own placement held whole does not move.
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
                (Some(Moved::By(t)), Some(&(_, m))) => {
                    // A tiling instance draws `M · R⁻¹ · T · o`: its change is `M · R⁻¹ · T · M⁻¹`.
                    let drawn = match self.held.get(&node) {
                        Some(&Hold::Translation { pivot }) => shift_back(t, pivot).compose(t),
                        _ => t,
                    };
                    m.inverse()
                        .map(|inverse| Moved::By(m.compose(drawn).compose(inverse)))
                }
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
    use crate::units::{Angle, Point, Vec2};

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

    fn none() -> HeldClones {
        HeldClones::new()
    }

    fn held(ids: &[i32]) -> HeldClones {
        ids.iter().map(|&n| (id(n), Hold::Whole)).collect()
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

    /// O, instance 2 (tiling, pivot (5, 5)) following O by translate (30, 0), clone 3 of the
    /// instance by translate (0, 30); O scaled twice about the pivot.
    fn rigid_chain(hold_3: Option<Hold>) -> (CloneLinks, HeldClones, Affine) {
        let pivot = Point::new(5.0, 5.0);
        let mut held = HeldClones::from([(id(2), Hold::Translation { pivot })]);
        if let Some(hold) = hold_3 {
            held.insert(id(3), hold);
        }
        let scale = translate(5.0, 5.0)
            .compose(Affine::scale(2.0, 2.0))
            .compose(translate(-5.0, -5.0));
        (chain_of_two(), held, scale)
    }

    fn chain_of_two() -> CloneLinks {
        HashMap::from([
            (id(2), (id(1), translate(30.0, 0.0))),
            (id(3), (id(2), translate(0.0, 30.0))),
        ])
    }

    #[test]
    fn a_tiling_instance_follows_a_scale_and_is_not_counted_as_held() {
        let (links, held, scale) = rigid_chain(None);
        let result = clone_matrices_after(&links, &held, &[id(1)], scale);
        assert_eq!(result.held_back, Vec::new());
        assert_eq!(result.followed.len(), 1);
        assert!(close(result.followed[0].1, translate(30.0, 0.0)));
    }

    /// A turn by 0.5 rad about `centre`.
    fn turn_about(centre: Point) -> Affine {
        Affine::rotation_about(centre, Angle::from_radians(0.5))
    }

    #[test]
    fn a_tiling_instance_turns_in_place_when_its_source_rotates() {
        let (links, held, _) = rigid_chain(None);
        // Turned about the origin, the pivot (5, 5) moves too: only that move is held back.
        let turn = turn_about(Point::new(0.0, 0.0));
        let result = clone_matrices_after(&links, &held, &[id(1)], turn);
        assert_eq!(result.held_back, Vec::new());
        assert_eq!(result.followed.len(), 1);
        let drawn = result.followed[0].1.compose(turn);
        let wanted = translate(30.0, 0.0).compose(turn_about(Point::new(5.0, 5.0)));
        assert!(close(drawn, wanted), "{drawn:?} vs {wanted:?}");
    }

    #[test]
    fn a_held_clone_of_a_tiling_instance_holds_the_instance_rotation() {
        let (links, held, _) = rigid_chain(Some(Hold::Whole));
        let turn = turn_about(Point::new(5.0, 5.0));
        let result = clone_matrices_after(&links, &held, &[id(1)], turn);
        let t2 = translate(30.0, 0.0)
            .compose(turn)
            .compose(translate(-30.0, 0.0));
        let wanted = translate(0.0, 30.0).compose(t2.inverse().unwrap());
        assert!(close(held_of(&result, 3), wanted));
    }

    #[test]
    fn a_tiling_instance_holds_a_move_whole() {
        let (links, held, _) = rigid_chain(None);
        let result = clone_matrices_after(&links, &held, &[id(1)], translate(4.0, 7.0));
        assert!(close(held_of(&result, 2), translate(26.0, -7.0)));
        assert_eq!(result.followed, Vec::new());
    }

    #[test]
    fn a_held_clone_of_a_tiling_instance_holds_the_instance_change() {
        let (links, held, scale) = rigid_chain(Some(Hold::Whole));
        let result = clone_matrices_after(&links, &held, &[id(1)], scale);
        // T_2 = M_2 · S · M_2⁻¹; clone 3 draws M_3' · M_2 · S · o = M_3 · M_2 · o.
        let t2 = translate(30.0, 0.0)
            .compose(scale)
            .compose(translate(-30.0, 0.0));
        let wanted = translate(0.0, 30.0).compose(t2.inverse().unwrap());
        assert!(close(held_of(&result, 3), wanted));
    }

    #[test]
    fn a_selected_clone_of_a_tiling_instance_gets_g_m_t_inverse() {
        let (links, held, scale) = rigid_chain(None);
        let g = translate(1.0, 2.0);
        let moved = [(id(1), scale), (id(3), g)];
        let result = clone_matrices_after_maps(&links, &held, &moved);
        let t2 = translate(30.0, 0.0)
            .compose(scale)
            .compose(translate(-30.0, 0.0));
        let wanted = g
            .compose(translate(0.0, 30.0))
            .compose(t2.inverse().unwrap());
        assert!(close(matrix_of(&result, 3), wanted));
    }
}

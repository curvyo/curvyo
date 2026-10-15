//! The read model of a linked clone (`specs/0049-linked-clone/adrs.md` decision 2): what a
//! resolved clone is to the readers that draw, hit, measure and output it.

use serde::Serialize;

use crate::affine::Affine;
use crate::path_model::{NodeId, PathSnapshot};
use crate::units::Angle;

/// How many clones deep Linked clone may make a chain (`0049` criterion 52). A rule of the
/// command only: the reader resolves deeper chains.
pub const MAX_CLONE_DEPTH: usize = 8;

/// One resolved clone. It has no outline of its own: `resolved` is the root's outline under the
/// composed matrix, with the style of its style source (ADR 0016 §3). The chain data sits behind a box so that a
/// clone is no larger than a path or a primitive in every object list.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CloneSnapshot {
    /// What the clone shows: the clone's own id, the root's anchors under
    /// [`CloneChain::composed`] (anchor ids kept), the root's `closed` flags and further outlines,
    /// the style of [`CloneChain::style_source`] (its effective style), rotation 0.
    pub resolved: PathSnapshot,
    /// What the clone follows and how.
    pub chain: Box<CloneChain>,
}

#[cfg(test)]
thread_local! {
    /// How many clone outlines this thread has mapped: the read must map one per clone
    /// (`0049` criterion 63).
    static OUTLINE_TRANSFORMS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The number of clone outlines this thread has mapped so far (test builds only).
#[cfg(test)]
pub(crate) fn outline_transforms() -> usize {
    OUTLINE_TRANSFORMS.with(std::cell::Cell::get)
}

/// The link of a resolved clone and where its chain ends.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CloneChain {
    /// The object it follows directly: a path, a compound path or another clone.
    pub origin: NodeId,
    /// The path or compound path at the end of its chain.
    pub root: NodeId,
    /// The number of clones from this one down to the root, itself included (1 for a clone of
    /// a path).
    pub depth: usize,
    /// Its own matrix M.
    pub matrix: Affine,
    /// The matrix from the root's coordinates to this clone's: M times the original's composed
    /// matrix (`M_B · M_A` for B → A → O).
    pub composed: Affine,
    /// Whether it is held back when its original is transformed (`0057` criterion 3).
    pub own_placement: bool,
    /// Whether it owns its style and opacity (`0057` criterion 11).
    pub own_style: bool,
    /// The node its style and opacity register are read from (ADR 0016 §3): itself with own
    /// style, else the nearest clone on its chain with own style, else the root.
    pub style_source: NodeId,
    /// The root's own snapshot as read, unmapped, one allocation shared by every clone of that
    /// root, so the renderer can tessellate it once (`adrs.md` decision 3, criterion 44).
    /// Invariant: [`CloneSnapshot::resolved`]'s anchors are this outline's under `composed`.
    #[serde(skip)]
    pub root_outline: std::sync::Arc<PathSnapshot>,
}

impl CloneSnapshot {
    /// The clone `id` that shows its chain's root outline under the composed matrix.
    #[must_use]
    pub fn new(id: NodeId, chain: CloneChain) -> Self {
        #[cfg(test)]
        OUTLINE_TRANSFORMS.with(|count| count.set(count.get() + 1));
        let mut resolved = chain.root_outline.mapped(chain.composed);
        resolved.id = id;
        resolved.rotation = Angle::from_radians(0.0);
        Self {
            resolved,
            chain: Box::new(chain),
        }
    }

    /// The clone's own id.
    #[must_use]
    pub const fn id(&self) -> NodeId {
        self.resolved.id
    }

    /// This clone with its drawn outline mapped by `g` while nothing of its chain moves: the
    /// matrix becomes `g·M` (`0049` criterion 15). The live preview of a move or rotate draws it.
    #[must_use]
    pub fn mapped_by(&self, g: Affine) -> Self {
        let mut chain = (*self.chain).clone();
        chain.matrix = g.compose(chain.matrix);
        chain.composed = g.compose(chain.composed);
        Self {
            resolved: self.resolved.mapped(g),
            chain: Box::new(chain),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clone_is_no_larger_than_the_other_kinds() {
        let largest = std::mem::size_of::<PathSnapshot>()
            .max(std::mem::size_of::<crate::PrimitiveSnapshot>());
        assert!(std::mem::size_of::<CloneSnapshot>() <= largest);
    }
}

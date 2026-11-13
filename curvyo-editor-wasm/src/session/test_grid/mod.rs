//! The Test grid tool of a session (`specs/0112-test-grid-and-materials` criteria 1 to 15, 28;
//! ADR 0057; `adrs.md` finding 1): one owned state, the reading of grid objects, their ids and
//! where they lie. The rules are `curvyo-laser-core`'s; each write is one step.

mod bar;
mod canvas;
mod items;
mod place;
mod prepared;
#[cfg(test)]
mod review_tests;
#[cfg(test)]
pub(in crate::session) mod tests;
mod view_json;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};

use curvyo_document_core::{GridItem, NodeId, Point};
use curvyo_laser_core::PROCESS_ID;
use curvyo_laser_core::grids::{Recipe, TestId, TestRecipe, read_test_grid_choice};

use super::Session;

pub use view_json::TestGridView;

/// A typed value the grid did not take: kept in its field with the cause (criterion 5).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Invalid {
    /// The grid it was typed for, `None` for the tool's draft.
    group: Option<NodeId>,
    field: String,
    typed: String,
    cause: String,
}

/// The Test grid tool's state (finding 1): the remembered recipes, the kept invalid value, the
/// spoken status, the label widths and the caller's random bytes for ids.
#[derive(Debug, Default)]
pub(in crate::session) struct TestGridTool {
    /// The last recipe per profile id (`test-grid-choice.json`, this computer).
    choice: BTreeMap<String, TestRecipe>,
    /// The field value the grid did not take, with its cause.
    invalid: Option<Invalid>,
    /// The sr-only status ("New test id 8PL4D."), taken once.
    status: Option<String>,
    /// The refusal of the last press, shown as the Action notice.
    notice: Option<String>,
    /// Label widths in mm at the text height, by face hash and text: shaped once each.
    widths: RefCell<HashMap<(String, String), f64>>,
    /// The pointer over the page while the tool is active: the outline preview.
    hover: Option<Point>,
    /// The material the bar picked while no grid is selected: the next grid uses it.
    draft_material: Option<curvyo_laser_core::grids::MaterialRef>,
    /// Random bytes from the host for new test ids.
    random: Vec<u8>,
    /// A counter mixed into the fallback bytes when the host gave none.
    drawn: Cell<u64>,
}

/// A grid object as read: its group, its readable recipe, its Lasered register and items.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::session) struct GridObject {
    pub(in crate::session) group: NodeId,
    pub(in crate::session) recipe: Recipe,
    pub(in crate::session) lasered: Option<u64>,
    pub(in crate::session) cells: Vec<GridItem>,
    pub(in crate::session) labels: Vec<GridItem>,
    pub(in crate::session) derived: Vec<NodeId>,
}

/// A grid whose derived operations lie on a laser page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::session) struct PlacedGrid {
    pub(in crate::session) group: NodeId,
    pub(in crate::session) id: TestId,
    pub(in crate::session) page_id: String,
    pub(in crate::session) page_name: String,
}

impl Session {
    /// Reads the kept `test-grid-choice.json` (criterion 2); a damaged or format-1 text reads as
    /// empty.
    pub fn load_test_grid_choice(&mut self, text: &str) {
        self.test_grid.choice = read_test_grid_choice(text);
    }

    /// Random bytes from the host (`crypto.getRandomValues`) that new test ids are drawn from
    /// (criterion 9); 64 at a time is plenty.
    pub fn feed_test_grid_random(&mut self, bytes: &[u8]) {
        self.test_grid.random.extend_from_slice(bytes);
    }

    /// The sr-only status of the last commit ("New test id 8PL4D."), once.
    pub fn take_test_grid_status(&mut self) -> Option<String> {
        self.test_grid.status.take()
    }

    /// The refusal of the last press or commit, `None` when it wrote.
    #[must_use]
    pub fn test_grid_notice(&self) -> Option<String> {
        self.test_grid.notice.clone()
    }

    /// The grid object whose group is `group`, `None` when it is no grid or its recipe does not
    /// read (it is then a plain group, ADR 0057 §6).
    pub(in crate::session) fn grid_object(&self, group: NodeId) -> Option<GridObject> {
        let snapshot = self.document.test_grid(group)?;
        let recipe = Recipe::from_value(&snapshot.recipe)?;
        Some(GridObject {
            group,
            recipe,
            lasered: snapshot.lasered,
            cells: snapshot.cells,
            labels: snapshot.labels,
            derived: snapshot.derived,
        })
    }

    /// The grid whose group is `id` or lies above it.
    fn test_grid_above(&self, id: NodeId) -> Option<NodeId> {
        let mut at = Some(id);
        while let Some(node) = at {
            if self.document.test_grid(node).is_some() {
                return Some(node);
            }
            at = self.document.parent_group(node);
        }
        None
    }

    /// The grid the canvas selection is or lies in.
    pub(in crate::session) fn selected_test_grid(&self) -> Option<NodeId> {
        self.selection
            .ids()
            .iter()
            .find_map(|&id| self.test_grid_above(id))
    }

    /// Whether `id` is a grid's group or an item in one: it is never assigned to another
    /// operation.
    pub(in crate::session) fn in_test_grid(&self, id: NodeId) -> bool {
        self.test_grid_above(id).is_some()
    }

    /// The one grid selected as a whole (all its items), with a readable recipe: the Select bar
    /// is its bar then (UX G3).
    pub(in crate::session) fn whole_selected_grid(&self) -> Option<NodeId> {
        let ids = self.selection.ids();
        if ids.is_empty() {
            return None;
        }
        let group = self.selected_test_grid()?;
        let leaves = self.document.leaves_of(group);
        let whole = ids.len() == leaves.len() && ids.iter().all(|id| leaves.contains(id));
        (whole && self.grid_object(group).is_some()).then_some(group)
    }

    /// Whether the selection is one test grid as a whole: it is moved, never resized, rotated or
    /// skewed (criterion 7).
    pub(in crate::session) fn whole_test_grid_selected(&self) -> bool {
        self.whole_selected_grid().is_some()
    }

    /// Whether the Select bar hides Flip, Guide, Object to path and Linked clone and shows the
    /// Test grid bar: the selection is one whole test grid (UX G3, G4).
    #[must_use]
    pub fn select_bar_hides_grid_commands(&self) -> bool {
        self.tool == super::Tool::Select && self.whole_test_grid_selected()
    }

    /// Every grid whose derived operations lie on a laser page, with its id and page.
    pub(in crate::session) fn placed_test_grids(&self) -> Vec<PlacedGrid> {
        let pages = self.document.pages();
        let mut out: Vec<PlacedGrid> = Vec::new();
        for manufacturing in self.document.manufacturings() {
            if manufacturing.process != PROCESS_ID {
                continue;
            }
            let Some(page) = pages
                .iter()
                .find(|p| p.kind.manufacturing() == Some(manufacturing.id))
            else {
                continue;
            };
            let Ok(operations) = self.document.operations(manufacturing.id) else {
                continue;
            };
            for op in operations.iter().filter(|op| op.grid_part.is_some()) {
                let Some(group) = self.document.test_grid_of_derived_row(op.id) else {
                    continue;
                };
                if out.iter().any(|grid| grid.group == group) {
                    continue;
                }
                if let Some(grid) = self.grid_object(group) {
                    out.push(PlacedGrid {
                        group,
                        id: grid.recipe.id,
                        page_id: page.id.clone(),
                        page_name: page.name.clone(),
                    });
                }
            }
        }
        out
    }

    /// The test ids of every grid of the open project (ADR 0058 §4).
    fn project_grid_ids(&self) -> Vec<TestId> {
        self.document
            .child_groups(None)
            .into_iter()
            .filter_map(|(group, _)| self.grid_object(group).map(|grid| grid.recipe.id))
            .collect()
    }

    /// The next 64 random bytes: the host's, else a fallback stream from the peer id (tests and
    /// a host that gave none).
    fn random_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64);
        let mut state = self.peer
            ^ self
                .test_grid
                .drawn
                .get()
                .wrapping_mul(0x9e37_79b9_7f4a_7c15);
        self.test_grid.drawn.set(self.test_grid.drawn.get() + 1);
        while out.len() < 64 {
            // xorshift64*: only a fallback, the host's bytes come first.
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            out.extend_from_slice(&state.wrapping_mul(0x2545_f491_4f6c_dd1d).to_le_bytes());
        }
        out
    }

    /// A new test id, unique against the library's live and retired ids and the project's grid
    /// ids (criterion 9).
    pub(in crate::session) fn next_test_id(&mut self) -> TestId {
        let project = self.project_grid_ids();
        loop {
            let bytes = if self.test_grid.random.len() >= 64 {
                self.test_grid.random.drain(..64).collect()
            } else {
                self.random_bytes()
            };
            let taken = |id: &TestId| project.contains(id) || self.materials.id_taken(*id);
            if let Some(id) = TestId::make(&bytes, &taken) {
                return id;
            }
        }
    }
}

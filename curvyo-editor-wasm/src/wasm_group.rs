//! Groups' `wasm-bindgen` surface (`specs/0023-groups`): what the Grouping card of the rail shows,
//! the two commands, and the context (the entered group) the indicator "Open groups" reads. A
//! further `impl WasmSession` block; every method a pass-through to `Session`, which holds the
//! rules and the sentences of the notices (one command, one message, whether a key or a button
//! caused it).

use wasm_bindgen::prelude::*;

use crate::session::GroupingResult;
use crate::wasm_api::WasmSession;

/// What the Grouping card shows: whether each button acts now (false also when the Select tool is
/// not active).
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct GroupingAvailabilityView {
    /// Two or more objects of the context are selected.
    pub group: bool,
    /// The selection holds a group.
    pub ungroup: bool,
}

/// What a Group or Ungroup did: `kind` is `"ignored"`, `"grouped"`, `"ungrouped"` or `"refused"`
/// (the depth limit), and `notice` the sentence for the canvas notice slot (empty when ignored).
/// A refusal is an alert, the rest a status.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct GroupingResultView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// The one-line notice.
    #[wasm_bindgen(getter_with_clone)]
    pub notice: String,
}

impl From<GroupingResult> for GroupingResultView {
    fn from(result: GroupingResult) -> Self {
        Self {
            kind: result.code().to_owned(),
            notice: result.notice,
        }
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// What the Grouping card of the tool rail shows. Call it with every other state read.
    #[must_use]
    pub fn grouping_availability(&self) -> GroupingAvailabilityView {
        let availability = self.session.grouping_availability();
        GroupingAvailabilityView {
            group: availability.group,
            ungroup: availability.ungroup,
        }
    }

    /// Binds the selected objects into one group, in one commit.
    pub fn group_selection(&mut self) -> GroupingResultView {
        self.session.group_selection().into()
    }

    /// Replaces each selected group by its children, in one commit.
    pub fn ungroup_selection(&mut self) -> GroupingResultView {
        self.session.ungroup_selection().into()
    }

    /// The result of the last Group or Ungroup chord, once (`kind` `"ignored"` when there is none).
    pub fn take_grouping_result(&mut self) -> GroupingResultView {
        self.session.take_grouping_result().map_or_else(
            || GroupingResultView {
                kind: "ignored".to_owned(),
                notice: String::new(),
            },
            Into::into,
        )
    }

    /// How many groups deep the Select tool is; 0 at the document root. The indicator "Open
    /// groups" shows "Document" and one crumb per level.
    #[must_use]
    pub fn context_depth(&self) -> u32 {
        u32::try_from(self.session.context_depth()).unwrap_or(u32::MAX)
    }

    /// A crumb of the indicator: makes the context the first `level` entered groups (0 leaves
    /// every level). Returns whether the context changed.
    pub fn leave_to_level(&mut self, level: u32) -> bool {
        self.session.leave_to_level(level as usize)
    }

    /// What a screen reader hears about the context, once: entering, leaving, a context that
    /// fell back. Empty when there is nothing new.
    pub fn take_context_announcement(&mut self) -> String {
        self.session.take_context_announcement()
    }
}

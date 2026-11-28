//! `Session`'s glue for the Properties panel's tabs
//! (`specs/0043-properties-tabs/`, `adrs.md` decision 4): the active tab as
//! session view state, updated whenever the panel view is read, and the
//! ways a tab is chosen (a press, Shift+Ctrl+F, Shift+Ctrl+D, Shift+Ctrl+H, Shift+Ctrl+L). The rules are
//! `curvyo_ui_core::panel_tabs`'s and `panel_content`'s.

use curvyo_ui_core::{
    PanelContent, PanelTab, PanelTabEntry, PanelTabs, StyleBlocker, output_tabs, panel_body,
    tab_entries_for,
};

use curvyo_document_core::PageKind;

use super::{Session, Tool};

/// Whether the tab `name` is in the strip of a page of this kind (`specs/0098` criterion 8): a
/// sketch page shows the Sketch workbench's tabs as built; every other page the drawing tabs,
/// without Sketches and Parameters. No page shows the old workbench tab.
fn tab_of_kind(name: &str, sketch: bool) -> bool {
    match name {
        "sketches" | "parameters" => sketch,
        "workbench" => false,
        _ => true,
    }
}

/// Whether `tab` is an output-page tab ([`output_tabs`]).
const fn is_output_tab(tab: PanelTab) -> bool {
    matches!(
        tab,
        PanelTab::Operations
            | PanelTab::Materials
            | PanelTab::Fixtures
            | PanelTab::Tools
            | PanelTab::Machines
    )
}

/// Whether `tab` may show on the active page: an output-page tab only while `output` (the active
/// output page's list, ADR 0098 decision 2) holds it, the drawing tabs as `tab_of_kind`.
fn tab_allowed(tab: PanelTab, sketch: bool, output: &[PanelTab]) -> bool {
    (!is_output_tab(tab) || output.contains(&tab)) && tab_of_kind(tab.name(), sketch)
}

/// The strip label of an output-page tab.
const fn output_label(tab: PanelTab) -> &'static str {
    match tab {
        PanelTab::Materials => "Materials",
        PanelTab::Fixtures => "Fixtures",
        PanelTab::Tools => "Tools",
        PanelTab::Machines => "Machines",
        _ => "Operations",
    }
}

/// The key a page kind's last tab is kept under (criterion 9): an output page by its process,
/// every unknown kind as one.
fn kind_key(kind: &PageKind) -> String {
    match kind {
        PageKind::Drawing => "drawing".to_owned(),
        PageKind::Sketch => "sketch".to_owned(),
        PageKind::Output { process, .. } => process.clone(),
        PageKind::Unknown(_) => "unknown".to_owned(),
    }
}

/// What the Properties panel shows: the strip and the body of the active tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelView {
    /// What the body shows.
    pub content: PanelContent,
    /// The strip's tabs, in order.
    pub tabs: Vec<PanelTabEntry>,
}

impl Session {
    /// The output-page tabs of the active page (ADR 0098 decision 2): its process's list while it
    /// is an output page with its extension on, else none.
    pub(in crate::session) fn output_tabs(&self) -> &'static [PanelTab] {
        self.active_output_page()
            .filter(|m| self.process_enabled(&m.process))
            .map_or(&[], |m| output_tabs(&m.process))
    }

    /// Whether the active tool's style scope holds objects: the "selection
    /// state" of the tab rules (`adrs.md` decision 2).
    fn panel_scope_non_empty(&self) -> bool {
        // Clones have no style of their own and stay out of the style scope, but the Style tab
        // shows their clone block (`0049` criteria 22 and 24, UX review must-fix 1).
        // A placed page has no style either; the Style tab shows its block (`specs/0100`
        // criterion 17).
        !self.style_scope().ids.is_empty()
            || self.clone_subject().is_some()
            || self.selected_placement().is_some()
            // A selected picture has its Adjustments group there (`specs/0135` UX notes E1).
            || self.adjust_target().is_some()
    }

    /// Runs `change` on the tab state. The state is a `Cell` because a read of
    /// the panel is what notices a change of selection.
    fn update_panel_tabs<T>(&self, change: impl FnOnce(&mut PanelTabs) -> T) -> T {
        let mut tabs = self.panel_tabs.get();
        let result = change(&mut tabs);
        self.panel_tabs.set(tabs);
        result
    }

    /// Why the Style tab is dimmed, `None` while the scope holds objects: the
    /// tooltip names nothing selected, or the tool that cannot style the
    /// selection.
    fn style_blocker(&self, non_empty: bool) -> Option<StyleBlocker> {
        if non_empty {
            return None;
        }
        Some(match self.tool() {
            Tool::Pen if !self.selection.is_empty() => StyleBlocker::PenTool,
            Tool::Node if !self.selection.is_empty() => StyleBlocker::NodeToolWithoutPath,
            _ => StyleBlocker::NothingSelected,
        })
    }

    /// Reads the panel: lets the tab rule see the scope's current state (so an
    /// edge since the last read switches the tab, in the same frame as the
    /// selection change), then returns the strip and the body.
    pub fn panel_view(&self) -> PanelView {
        let non_empty = self.panel_scope_non_empty();
        let active = self.update_panel_tabs(|tabs| {
            tabs.observe(non_empty);
            tabs.active()
        });
        let sketch = self.workbench() == super::Workbench::Sketch;
        let mut tabs: Vec<PanelTabEntry> = tab_entries_for(active, self.style_blocker(non_empty))
            .into_iter()
            .filter(|tab| tab_of_kind(tab.name, sketch))
            .collect();
        // The Library tab comes right after Fonts (`specs/0137` UX notes P2).
        if let Some(at) = tabs
            .iter()
            .position(|tab| tab.name == PanelTab::Fonts.name())
        {
            tabs.insert(
                at + 1,
                PanelTab::Library.entry_labelled("Library", active == PanelTab::Library),
            );
        }
        // The output-page tabs come right after Document, in their process's order
        // (`specs/0101` criterion 1, ADR 0098 decision 2).
        let at = tabs
            .iter()
            .position(|tab| tab.name == PanelTab::Document.name())
            .map_or(tabs.len(), |index| index + 1);
        for (offset, tab) in self.output_tabs().iter().enumerate() {
            tabs.insert(
                at + offset,
                tab.entry_labelled(output_label(*tab), active == *tab),
            );
        }
        PanelView {
            content: panel_body(active, self.pen_path_unfinished(), non_empty),
            tabs,
        }
    }

    /// A placed test grid (`specs/0112` criterion 12): its rows are what the maker reads next, so
    /// the edge from an empty scope hands Document over to Operations, not to the Style tab of
    /// "Group of 38 objects"; any other tab stays.
    pub(super) fn show_operations_for_new_grid(&self) {
        let non_empty = self.panel_scope_non_empty();
        self.update_panel_tabs(|tabs| {
            let was = tabs.active();
            tabs.observe(non_empty);
            if was == PanelTab::Document {
                let _ = tabs.press(PanelTab::Operations, non_empty);
            }
        });
    }

    /// Hands the tab over when the active page's kind changes (`specs/0098` criterion 9): the
    /// open tab stays when the new kind has it, else the tab last used on a page of that kind,
    /// else Document.
    pub(super) fn follow_tab(&mut self, to: super::Workbench) {
        let kind = kind_key(&self.active_kind());
        let Some(from) = self.tab_kind.replace(kind.clone()) else {
            return;
        };
        if from == kind {
            return;
        }
        let current = self.panel_tabs.get().active();
        self.kind_tabs.insert(from, current);
        let sketch = to == super::Workbench::Sketch;
        let output = self.output_tabs();
        if tab_allowed(current, sketch, output) {
            return;
        }
        let tab = self
            .kind_tabs
            .get(&kind)
            .copied()
            .filter(|tab| tab_allowed(*tab, sketch, output))
            .unwrap_or(PanelTab::Document);
        if !self.press_panel_tab(tab.name()) {
            self.press_panel_tab(PanelTab::Document.name());
        }
    }

    /// What the panel's body shows ([`Session::panel_view`]'s `content`).
    pub fn panel_content(&self) -> PanelContent {
        self.panel_view().content
    }

    /// A press on the tab `name`: `false` for an unknown name and for the
    /// dimmed Style tab, which does nothing (criterion 4).
    pub fn press_panel_tab(&mut self, name: &str) -> bool {
        let Some(tab) = PanelTab::from_name(name) else {
            return false;
        };
        if is_output_tab(tab) && !self.output_tabs().contains(&tab) {
            return false;
        }
        let non_empty = self.panel_scope_non_empty();
        self.update_panel_tabs(|tabs| {
            tabs.observe(non_empty);
            tabs.press(tab, non_empty)
        })
    }

    /// Shift+Ctrl+F (criterion 14): Style with objects in scope, else Document.
    pub fn panel_shortcut_style(&mut self) {
        let non_empty = self.panel_scope_non_empty();
        self.update_panel_tabs(|tabs| {
            tabs.observe(non_empty);
            tabs.shortcut_style(non_empty);
        });
    }

    /// Shift+Ctrl+H (`specs/0020-undo-redo` criterion 46): the History tab.
    pub fn panel_shortcut_history(&mut self) {
        let non_empty = self.panel_scope_non_empty();
        self.update_panel_tabs(|tabs| {
            tabs.observe(non_empty);
            tabs.shortcut_history();
        });
    }

    /// Shift+Ctrl+L (`specs/0050-layers-and-objects-panel` criterion 2): the Layers tab.
    pub fn panel_shortcut_layers(&mut self) {
        let non_empty = self.panel_scope_non_empty();
        self.update_panel_tabs(|tabs| {
            tabs.observe(non_empty);
            tabs.shortcut_layers();
        });
    }

    /// Shift+Ctrl+T or the Font button (`specs/0074-text-objects` criteria 11, 15): the Fonts
    /// tab.
    pub fn panel_shortcut_fonts(&mut self) {
        let non_empty = self.panel_scope_non_empty();
        self.update_panel_tabs(|tabs| {
            tabs.observe(non_empty);
            tabs.shortcut_fonts();
        });
    }

    /// Shift+Ctrl+D (criterion 14): the Document tab.
    pub fn panel_shortcut_document(&mut self) {
        let non_empty = self.panel_scope_non_empty();
        self.update_panel_tabs(|tabs| {
            tabs.observe(non_empty);
            tabs.shortcut_document();
        });
    }
}

/// [`PanelView`] as the flat record the host reads (ADR 0001 §5): the body's
/// name, the active tab and one entry per tab in order.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelTabsRecord {
    /// `"file"`, `"document"`, `"style"`, `"layers"`, `"history"` or `"empty"`.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub content: String,
    /// The active tab's name.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub active: String,
    /// The name of each tab.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub tab_names: Vec<String>,
    /// The accessible name of each tab.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub tab_labels: Vec<String>,
    /// The tooltip of each tab.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub tab_tooltips: Vec<String>,
    /// `1` for a tab that can be pressed, `0` for the dimmed one.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub tab_enabled: Vec<u8>,
}

impl PanelTabsRecord {
    /// The record of `view`.
    #[must_use]
    pub fn new(view: &PanelView) -> Self {
        let content = match view.content {
            PanelContent::File => "file",
            PanelContent::Document => "document",
            PanelContent::Style => "style",
            PanelContent::Layers => "layers",
            PanelContent::History => "history",
            PanelContent::Fonts => "fonts",
            PanelContent::Library => "library",
            PanelContent::Snapping => "snapping",
            PanelContent::Grid => "grid",
            PanelContent::Sketches => "sketches",
            PanelContent::Parameters => "parameters",
            PanelContent::Share => "share",
            PanelContent::Workbench => "workbench",
            PanelContent::Operations => "operations",
            PanelContent::Materials => "materials",
            PanelContent::Fixtures => "fixtures",
            PanelContent::Tools => "tools",
            PanelContent::Machines => "machines",
            PanelContent::Empty => "empty",
        };
        Self {
            content: content.to_string(),
            active: view
                .tabs
                .iter()
                .find(|tab| tab.selected)
                .map_or_else(String::new, |tab| tab.name.to_string()),
            tab_names: view.tabs.iter().map(|t| t.name.to_string()).collect(),
            tab_labels: view.tabs.iter().map(|t| t.label.to_string()).collect(),
            tab_tooltips: view.tabs.iter().map(|t| t.tooltip.to_string()).collect(),
            tab_enabled: view.tabs.iter().map(|t| u8::from(t.enabled)).collect(),
        }
    }
}

#[cfg(test)]
#[path = "panel_tabs_tests.rs"]
mod tests;

//! What the body of the Properties panel shows for the active tab
//! (`specs/0043-properties-tabs/` criteria 3, 10 and 23, `specs/0015-
//! document-size-and-rulers/` criterion 14a): the document's settings, the
//! Style area, the history, or nothing.

use crate::panel_tabs::PanelTab;

/// The panel's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelContent {
    /// The File tab: the project and the file commands (`specs/0051-sidebar-and-no-menu`).
    File,
    /// The Document section: the Document tab is active and the Pen has no
    /// unfinished path.
    Document,
    /// The Style area: the tool's scope holds objects to style.
    Style,
    /// The Layers list (`specs/0050-layers-and-objects-panel` criterion 3).
    Layers,
    /// The History list (`specs/0020-undo-redo` criterion 39).
    History,
    /// The Fonts tab (`specs/0074-text-objects` criteria 15 to 19).
    Fonts,
    /// Document snapping controls.
    Snapping,
    /// The active page's grid controls (`specs/0090-grid`).
    Grid,
    /// Parametric sketch controls.
    Sketches,
    /// Named sketch parameters and formulas.
    Parameters,
    /// Nothing: no heading, no text, no control (`specs/0017-style-panel-
    /// rework` criterion 1). The Document tab with an unfinished Pen path, so
    /// a resize would not move the path being drawn, or the Style tab with
    /// nothing in the tool's scope.
    Empty,
}

/// What the body shows for `active`. `style_has_objects` is whether the active
/// tool's style scope (`style_scope`) holds any object. The panel's width, the
/// canvas and the rulers never change with it.
///
/// Document shows the Document section unless the Pen has an unfinished path
/// (criterion 10). Style shows the Style area when the scope holds objects and
/// is empty otherwise, which only the Pen rule can cause (criterion 23).
#[must_use]
pub fn panel_body(
    active: PanelTab,
    pen_path_unfinished: bool,
    style_has_objects: bool,
) -> PanelContent {
    match active {
        PanelTab::Document if pen_path_unfinished => PanelContent::Empty,
        PanelTab::Document => PanelContent::Document,
        PanelTab::Style if style_has_objects => PanelContent::Style,
        PanelTab::Style => PanelContent::Empty,
        PanelTab::Layers => PanelContent::Layers,
        PanelTab::History => PanelContent::History,
        PanelTab::File => PanelContent::File,
        PanelTab::Fonts => PanelContent::Fonts,
        PanelTab::Snapping => PanelContent::Snapping,
        PanelTab::Grid => PanelContent::Grid,
        PanelTab::Sketches => PanelContent::Sketches,
        PanelTab::Parameters => PanelContent::Parameters,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use PanelContent::{Document, Empty, Style};

    /// Criteria 3, 10 and 23 as a table: (active tab, Pen path unfinished,
    /// objects in scope) and the body.
    #[test]
    fn the_body_follows_the_active_tab_the_pen_and_the_scope() {
        let rows = [
            (PanelTab::Document, false, false, Document),
            (PanelTab::Document, false, true, Document),
            (PanelTab::Document, true, false, Empty),
            (PanelTab::Document, true, true, Empty),
            (PanelTab::Style, false, true, Style),
            (PanelTab::Style, true, true, Style),
            (PanelTab::Style, false, false, Empty),
            (PanelTab::Style, true, false, Empty),
            // History shows its list whatever the tool and the scope are.
            (PanelTab::History, false, false, PanelContent::History),
            (PanelTab::History, true, true, PanelContent::History),
            // So does Layers: the Pen gate is on its actions, not its list (`0050` criterion 53).
            (PanelTab::Layers, false, false, PanelContent::Layers),
            (PanelTab::Layers, true, true, PanelContent::Layers),
            // So does File (`0051`).
            (PanelTab::File, false, false, PanelContent::File),
            (PanelTab::File, true, true, PanelContent::File),
        ];
        for (tab, pen, scope, want) in rows {
            assert_eq!(
                panel_body(tab, pen, scope),
                want,
                "{tab:?} pen {pen} scope {scope}"
            );
        }
    }
}

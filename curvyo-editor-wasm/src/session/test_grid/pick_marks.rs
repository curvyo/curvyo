//! The picks of the selected test grid as view data for the canvas (`specs/0146` criterion 8):
//! one entry per marked cell with its box in canvas CSS pixels. Never written, never output.

use serde::Serialize;

use super::super::Session;

/// The millimetres of the constant gap between two cells of a grid (`0112` criterion 18).
const GAP_MM: f64 = 2.0;

/// One marked cell of the selected grid.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PickMark {
    /// The box's left edge in canvas CSS px.
    pub x: f64,
    /// The box's top edge.
    pub y: f64,
    /// The box's width (the Cell size on screen).
    pub width: f64,
    /// The box's height.
    pub height: f64,
    /// The cell pitch on screen (Cell size plus the gap), for the chip rule of UX 6.3.
    pub pitch: f64,
    /// The gap between two cells on screen, for the frame offset of UX 6.2.
    pub gap: f64,
    /// The cell's picks in the order Best, Second best, Fastest, Deepest.
    pub kinds: Vec<&'static str>,
}

impl Session {
    /// The picks of the grid selected as a whole, `Vec::new()` when none is selected, it names
    /// no material test or the test has no pick: the canvas draws nothing then.
    #[must_use]
    pub fn pick_marks(&self) -> Vec<PickMark> {
        let Some(group) = self.whole_selected_grid() else {
            return Vec::new();
        };
        let Some(grid) = self.grid_object(group) else {
            return Vec::new();
        };
        let Some(link) = &grid.recipe.material else {
            return Vec::new();
        };
        let Some(test) = self
            .materials
            .library()
            .find(&link.id)
            .and_then(|m| m.test(grid.recipe.id))
        else {
            return Vec::new();
        };
        if test.marks.is_empty() {
            return Vec::new();
        }
        let view = self.view();
        let scale = view.scale();
        grid.cells
            .iter()
            .filter_map(|item| {
                let index = |key: &str| {
                    item.tag
                        .get(key)
                        .and_then(serde_json::Value::as_u64)
                        .and_then(|n| u8::try_from(n).ok())
                };
                let cell = curvyo_laser_core::settings::CellIndex {
                    column: index("column")?,
                    row: index("row")?,
                };
                let kinds: Vec<&'static str> = test
                    .marks
                    .kinds_of(cell)
                    .into_iter()
                    .map(curvyo_laser_core::settings::PickKind::as_str)
                    .collect();
                if kinds.is_empty() {
                    return None;
                }
                let (min, max) = curvyo_ui_core::object_bounds(&self.document.object(item.id)?);
                let (x, y) = view.document_to_screen(min);
                let width = (max.x - min.x) * scale;
                Some(PickMark {
                    x,
                    y,
                    width,
                    height: (max.y - min.y) * scale,
                    pitch: width + GAP_MM * scale,
                    gap: GAP_MM * scale,
                    kinds,
                })
            })
            .collect()
    }
}

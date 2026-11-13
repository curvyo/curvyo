//! A grid's items from its recipe (`specs/0112` criteria 7, 10, 32): label widths measured once
//! per distinct text in the included sans, and the laid-out cells and labels at the grid's place.

use curvyo_document_core::{
    Affine, CornerRadii, DrawingText, FontKind, FontRef, GridItems, Length, NewGridCell,
    NewGridLabel, Point, RectBounds, Shape, Style, TextChars, TextLayoutValues, Vec2,
};
use curvyo_laser_core::grids::{Recipe, TEXT_MM, TestLayout, layout};
use curvyo_laser_core::settings::SetKind;

use super::super::Session;
use super::GridObject;

/// The style of a cell: an outline for Cut and Score, a filled square for Engrave.
fn cell_style(kind: SetKind) -> Style {
    let mut style = Style::default();
    style.fill.enabled = kind == SetKind::Engrave;
    style.stroke.enabled = kind != SetKind::Engrave;
    style
}

/// The style of a label: filled text, no stroke (it is engraved).
fn label_style() -> Style {
    let mut style = Style::default();
    style.fill.enabled = true;
    style.stroke.enabled = false;
    style
}

impl Session {
    /// The face the labels are made in: the included sans, once loaded.
    pub(in crate::session) fn grid_label_font(&self) -> Option<FontRef> {
        self.fonts.fallback(FontKind::Outline).map(|(_, font)| font)
    }

    /// The advance width of `text` at the text height in `font`, measured once per distinct
    /// text (criterion 32).
    fn label_width(&self, font: &FontRef, text: &str) -> f64 {
        let key = (font.face_hash.clone(), text.to_owned());
        if let Some(width) = self.test_grid.widths.borrow().get(&key) {
            return *width;
        }
        let height = Length::from_mm(TEXT_MM);
        let layout_values = (&TextLayoutValues::DEFAULT, &TextChars::default());
        let width = self
            .layout_of(font, height, layout_values, text)
            .and_then(|laid| laid.lines.first().map(|line| line.width.0))
            // While the face is not there: a typical sans advance, 0.85 cap heights.
            .unwrap_or_else(|| {
                // a label is a few characters long
                #[allow(clippy::cast_precision_loss)]
                let chars = text.chars().count() as f64;
                chars * TEXT_MM * 0.85
            });
        self.test_grid.widths.borrow_mut().insert(key, width);
        width
    }

    /// The layout of `recipe` with measured label widths.
    pub(in crate::session) fn grid_layout(&self, recipe: &Recipe, font: &FontRef) -> TestLayout {
        layout(recipe, |text| Length::from_mm(self.label_width(font, text)))
    }

    /// Where the grid `grid` sits: the document point of its layout's top-left, from its first
    /// cell (cells are rewritten at every commit, so they hold their layout place); `None` when
    /// it has no cell left.
    pub(in crate::session) fn grid_origin(
        &self,
        grid: &GridObject,
        laid: &TestLayout,
    ) -> Option<Point> {
        let first = laid.cells.first()?;
        let item = grid.cells.iter().find(|item| {
            item.tag.get("column").and_then(serde_json::Value::as_u64)
                == Some(u64::from(first.column))
                && item.tag.get("row").and_then(serde_json::Value::as_u64)
                    == Some(u64::from(first.row))
        })?;
        let (min, _) = curvyo_ui_core::object_bounds(&self.document.object(item.id)?);
        Some(Point::new(min.x - first.min.x, min.y - first.min.y))
    }
}

/// The items of `laid` (laid out from `recipe`) with its top-left corner at `at`, labels in
/// `font` (criterion 10: always burned).
pub(in crate::session) fn grid_items(
    recipe: &Recipe,
    laid: &TestLayout,
    at: Point,
    font: &FontRef,
) -> GridItems {
    let cells = laid
        .cells
        .iter()
        .map(|place| {
            let mut tag = serde_json::Map::new();
            tag.insert("column".to_owned(), place.column.into());
            tag.insert("row".to_owned(), place.row.into());
            NewGridCell {
                shape: Shape::Rect {
                    bounds: RectBounds {
                        origin: Point::new(at.x + place.min.x, at.y + place.min.y),
                        width: place.side,
                        height: place.side,
                    },
                    corner_radii: CornerRadii::uniform(Length::from_mm(0.0)),
                },
                style: cell_style(recipe.test.kind()),
                tag,
            }
        })
        .collect();
    let labels = laid
        .labels
        .iter()
        .map(|place| NewGridLabel {
            text: DrawingText {
                string: place.text.clone(),
                font: font.clone(),
                size: Length::from_mm(TEXT_MM),
                layout: TextLayoutValues::DEFAULT,
                chars: TextChars::default(),
                placement: Affine::translation(Vec2::new(
                    at.x + place.origin.x,
                    at.y + place.origin.y,
                )),
                style: label_style(),
            },
            tag: place.part.as_str().to_owned(),
        })
        .collect();
    GridItems { cells, labels }
}

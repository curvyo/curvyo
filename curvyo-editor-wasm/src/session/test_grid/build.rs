//! A test grid's items from its recipe (`specs/0028` criteria 4, 5, 7, 9, 10): the label widths
//! measured in the included sans, the laid-out cells and labels at the grid's place, and the
//! reading of the built items for the hand-edited check.

use curvyo_document_core::{
    Affine, CornerRadii, DrawingText, FontKind, FontRef, GridItems, Length, NewGridCell,
    NewGridLabel, NodeId, ObjectSnapshot, Point, RectBounds, Shape, Style, TextChars,
    TextLayoutValues, Vec2,
};
use curvyo_laser_core::settings::SetKind;
use curvyo_laser_core::test_grids::{
    BuiltCell, BuiltLabel, CellTag, GridRecipe, LabelLanes, LabelTag, TestGridLayout, UNIT_TEXT,
    hand_edited, layout, speed_texts,
};

use super::super::Session;
use super::GridRead;

/// A rotation counts as none within this many radians.
const TURN_EPS: f64 = 1e-9;

/// The style of a cell: an outline for Cut and Score, a filled square for Engrave (criterion 5).
fn cell_style(kind: SetKind) -> Style {
    let mut style = Style::default();
    style.fill.enabled = kind == SetKind::Engrave;
    style.stroke.enabled = kind != SetKind::Engrave;
    style
}

/// The style of a label: filled text, no stroke (it is engraved or scored by its settings).
fn label_style() -> Style {
    let mut style = Style::default();
    style.fill.enabled = true;
    style.stroke.enabled = false;
    style
}

/// The plain map of a JSON value that is one.
fn plain(value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    match value {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    }
}

impl Session {
    /// The face the labels are made in: the included sans, once loaded.
    pub(in crate::session) fn grid_label_font(&self) -> Option<FontRef> {
        self.fonts.fallback(FontKind::Outline).map(|(_, font)| font)
    }

    /// The advance width of `text` at the cap height `height` in `font`, measured once per
    /// distinct text (criterion 30).
    fn label_width(&self, font: &FontRef, height: Length, text: &str) -> f64 {
        let key = (
            font.face_hash.clone(),
            height.as_mm().to_bits(),
            text.to_owned(),
        );
        if let Some(width) = self.test_grids.widths.borrow().get(&key) {
            return *width;
        }
        let layout_values = (&TextLayoutValues::DEFAULT, &TextChars::default());
        let width = self
            .layout_of(font, height, layout_values, text)
            .and_then(|laid| laid.lines.first().map(|line| line.width.0))
            // While the face is not there: a typical sans advance, 0.6 em or 0.85 cap heights.
            .unwrap_or_else(|| {
                // a label is a few characters long
                #[allow(clippy::cast_precision_loss)]
                let chars = text.chars().count() as f64;
                chars * height.as_mm() * 0.85
            });
        self.test_grids.widths.borrow_mut().insert(key, width);
        width
    }

    /// The measured label lanes of `recipe` (zero with Labels off).
    pub(in crate::session) fn grid_lanes(
        &self,
        recipe: &GridRecipe,
        font: Option<&FontRef>,
    ) -> LabelLanes {
        let (Some(font), true) = (font, recipe.labels.on) else {
            return LabelLanes {
                speed_width: Length::from_mm(0.0),
                unit_width: Length::from_mm(0.0),
            };
        };
        let height = recipe.labels.height;
        let speed = speed_texts(recipe)
            .iter()
            .map(|text| self.label_width(font, height, text))
            .fold(0.0, f64::max);
        LabelLanes {
            speed_width: Length::from_mm(speed),
            unit_width: Length::from_mm(self.label_width(font, height, UNIT_TEXT)),
        }
    }

    /// The layout of `recipe` with measured lanes.
    pub(in crate::session) fn grid_layout(&self, recipe: &GridRecipe) -> TestGridLayout {
        let font = self.grid_label_font();
        layout(recipe, self.grid_lanes(recipe, font.as_ref()))
    }
}

/// The items of `laid` (laid out from `recipe`) with its top-left corner at `at`, labels in
/// `font`.
pub(in crate::session) fn grid_items(
    recipe: &GridRecipe,
    laid: &TestGridLayout,
    at: Point,
    font: Option<&FontRef>,
) -> GridItems {
    let cells = laid
        .cells
        .iter()
        .map(|place| NewGridCell {
            shape: Shape::Rect {
                bounds: RectBounds {
                    origin: Point::new(at.x + place.min.x, at.y + place.min.y),
                    width: place.side,
                    height: place.side,
                },
                corner_radii: CornerRadii::uniform(Length::from_mm(0.0)),
            },
            style: cell_style(recipe.kind),
            tag: plain(
                CellTag {
                    cell: place.cell,
                    power: place.power,
                    speed: place.speed,
                }
                .to_value(),
            ),
        })
        .collect();
    let labels = match font {
        Some(font) if recipe.labels.on => laid
            .labels
            .iter()
            .map(|place| NewGridLabel {
                text: DrawingText {
                    string: place.text.clone(),
                    font: font.clone(),
                    size: recipe.labels.height,
                    layout: TextLayoutValues::DEFAULT,
                    chars: TextChars::default(),
                    placement: Affine::translation(Vec2::new(
                        at.x + place.origin.x,
                        at.y + place.origin.y,
                    )),
                    style: label_style(),
                },
                tag: place.tag.as_str().to_owned(),
            })
            .collect(),
        _ => Vec::new(),
    };
    GridItems { cells, labels }
}

/// The built items of `grid` as the hand-edited rule reads them.
fn built_items(session: &Session, grid: &GridRead) -> (Vec<BuiltCell>, Vec<BuiltLabel>) {
    let object = |id: NodeId| session.document.object(id);
    let cells = grid
        .snapshot
        .cells
        .iter()
        .filter_map(|item| {
            let tag = CellTag::from_value(&item.tag)?;
            let snapshot = object(item.id)?;
            let (min, max) = curvyo_ui_core::object_bounds(&snapshot);
            let square = matches!(
                &snapshot,
                ObjectSnapshot::Primitive(p)
                    if matches!(p.shape, Shape::Rect { .. }) && p.rotation.as_radians().abs() < TURN_EPS
            );
            Some(BuiltCell {
                cell: tag.cell,
                power: tag.power,
                speed: tag.speed,
                min,
                max,
                square,
                filled: snapshot.style().fill.enabled,
            })
        })
        .collect();
    let labels = grid
        .snapshot
        .labels
        .iter()
        .filter_map(|item| {
            let tag = LabelTag::parse(item.tag.as_str()?)?;
            let ObjectSnapshot::Text(text) = object(item.id)? else {
                return None;
            };
            let m = text.placement;
            let plain_move = (m.a - 1.0).abs() < TURN_EPS
                && m.b.abs() < TURN_EPS
                && m.c.abs() < TURN_EPS
                && (m.d - 1.0).abs() < TURN_EPS;
            Some(BuiltLabel {
                tag,
                text: text.string.clone(),
                origin: plain_move.then(|| Point::new(m.e, m.f)),
            })
        })
        .collect();
    (cells, labels)
}

/// Where the grid `grid` laid out as `laid` sits: the document point of its layout's top-left,
/// from a built cell (or label) that has its layout place; `None` when none is left.
pub(in crate::session) fn grid_origin(
    session: &Session,
    grid: &GridRead,
    laid: &TestGridLayout,
) -> Option<Point> {
    let (cells, labels) = built_items(session, grid);
    for place in &laid.cells {
        if let Some(built) = cells.iter().find(|c| c.cell == place.cell) {
            return Some(Point::new(
                built.min.x - place.min.x,
                built.min.y - place.min.y,
            ));
        }
    }
    laid.labels.iter().find_map(|place| {
        let at = labels
            .iter()
            .find(|l| l.tag == place.tag && l.text == place.text)?
            .origin?;
        Some(Point::new(at.x - place.origin.x, at.y - place.origin.y))
    })
}

/// Whether the items of `grid` differ from its recipe's layout `laid` (criterion 10).
pub(in crate::session) fn is_hand_edited(
    session: &Session,
    grid: &GridRead,
    recipe: &GridRecipe,
    laid: &TestGridLayout,
) -> bool {
    let (cells, labels) = built_items(session, grid);
    hand_edited(laid, recipe.kind, &cells, &labels)
}

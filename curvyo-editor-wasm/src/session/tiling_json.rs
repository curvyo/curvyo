//! The tiling bar's recipe and status as JSON in the units the fields show (degrees, percent,
//! millimetres; the circle centre in document coordinates), for the `wasm-bindgen` surface of
//! `specs/0069-tiling` (ADR 0001 §5: strings and scalars only).

use curvyo_document_core::{
    Angle, Channel, CircleOptions, GridOptions, Length, OffsetLines, Percent, SpacingMode,
    TilingLayout, TilingRecipe, TilingResult, Variance, Vec2,
};
use curvyo_ui_core::tiling::CountStatus;
use serde::{Deserialize, Serialize};

use super::Session;

/// One variance channel as the fields show it: random ±, per row, per column (circle: per step).
type Triple = [f64; 3];

/// The recipe in the units of the bar's fields.
#[allow(clippy::struct_excessive_bools)] // the bar's switches, field by field
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct RecipeJson {
    layout: String,
    columns: u32,
    rows: u32,
    spacing_x: f64,
    spacing_y: f64,
    pitch: bool,
    mirror_rows: bool,
    mirror_columns: bool,
    offset: String,
    shift: f64,
    count: u32,
    radius: f64,
    start: f64,
    sweep: f64,
    centre_x: f64,
    centre_y: f64,
    face_centre: bool,
    rotation: Triple,
    scale: Triple,
    offset_x: Triple,
    offset_y: Triple,
    opacity: Triple,
    hue: Triple,
    seed: u32,
    copies: bool,
    keep_source: bool,
}

fn degrees(c: Channel<Angle>) -> Triple {
    [c.random, c.per_row, c.per_column].map(|a| a.as_radians().to_degrees())
}

fn percents(c: Channel<Percent>) -> Triple {
    [c.random, c.per_row, c.per_column].map(Percent::get)
}

fn millimetres(c: Channel<Length>) -> Triple {
    [c.random, c.per_row, c.per_column].map(Length::as_mm)
}

fn angle_channel([r, row, column]: Triple) -> Channel<Angle> {
    let a = |d: f64| Angle::from_radians(d.to_radians());
    Channel {
        random: a(r),
        per_row: a(row),
        per_column: a(column),
    }
}

fn percent_channel([r, row, column]: Triple) -> Channel<Percent> {
    Channel {
        random: Percent::new(r),
        per_row: Percent::new(row),
        per_column: Percent::new(column),
    }
}

fn length_channel([r, row, column]: Triple) -> Channel<Length> {
    let l = Length::from_mm;
    Channel {
        random: l(r),
        per_row: l(row),
        per_column: l(column),
    }
}

/// The circle centre is shown in document coordinates; the recipe keeps it from the box centre.
fn to_json(recipe: &TilingRecipe, box_centre: curvyo_document_core::Point) -> RecipeJson {
    let g = &recipe.grid;
    let c = &recipe.circle;
    let v = &recipe.variance;
    RecipeJson {
        layout: if recipe.layout == TilingLayout::Grid {
            "grid"
        } else {
            "circle"
        }
        .to_owned(),
        columns: g.columns,
        rows: g.rows,
        spacing_x: g.spacing_x.as_mm(),
        spacing_y: g.spacing_y.as_mm(),
        pitch: g.spacing_mode == SpacingMode::Pitch,
        mirror_rows: g.mirror_rows,
        mirror_columns: g.mirror_columns,
        offset: match g.offset {
            OffsetLines::None => "none",
            OffsetLines::Rows => "rows",
            OffsetLines::Columns => "columns",
        }
        .to_owned(),
        shift: g.shift.get(),
        count: c.count,
        radius: c.radius.as_mm(),
        start: c.start.as_radians().to_degrees(),
        sweep: c.sweep.as_radians().to_degrees(),
        centre_x: box_centre.x + c.centre.x,
        centre_y: box_centre.y + c.centre.y,
        face_centre: c.face_centre,
        rotation: degrees(v.rotation),
        scale: percents(v.scale_x),
        offset_x: millimetres(v.offset_x),
        offset_y: millimetres(v.offset_y),
        opacity: percents(v.opacity),
        hue: degrees(v.hue),
        seed: recipe.seed,
        copies: recipe.result == TilingResult::Copies,
        keep_source: recipe.keep_source,
    }
}

fn from_json(json: &RecipeJson, box_centre: curvyo_document_core::Point) -> Option<TilingRecipe> {
    let deg = |d: f64| Angle::from_radians(d.to_radians());
    Some(TilingRecipe {
        layout: match json.layout.as_str() {
            "grid" => TilingLayout::Grid,
            "circle" => TilingLayout::Circle,
            _ => return None,
        },
        grid: GridOptions {
            columns: json.columns,
            rows: json.rows,
            spacing_x: Length::from_mm(json.spacing_x),
            spacing_y: Length::from_mm(json.spacing_y),
            spacing_mode: if json.pitch {
                SpacingMode::Pitch
            } else {
                SpacingMode::Gap
            },
            mirror_rows: json.mirror_rows,
            mirror_columns: json.mirror_columns,
            offset: match json.offset.as_str() {
                "none" => OffsetLines::None,
                "rows" => OffsetLines::Rows,
                "columns" => OffsetLines::Columns,
                _ => return None,
            },
            shift: Percent::new(json.shift),
        },
        circle: CircleOptions {
            count: json.count,
            centre: Vec2::new(json.centre_x - box_centre.x, json.centre_y - box_centre.y),
            radius: Length::from_mm(json.radius),
            start: deg(json.start),
            sweep: deg(json.sweep),
            face_centre: json.face_centre,
        },
        variance: Variance {
            rotation: angle_channel(json.rotation),
            scale_x: percent_channel(json.scale),
            scale_y: percent_channel(json.scale),
            offset_x: length_channel(json.offset_x),
            offset_y: length_channel(json.offset_y),
            opacity: percent_channel(json.opacity),
            hue: angle_channel(json.hue),
            uniform_scale: true,
        },
        seed: json.seed,
        result: if json.copies {
            TilingResult::Copies
        } else {
            TilingResult::Clones
        },
        keep_source: json.keep_source,
    })
}

/// Keeps the stored angle and centre of every field the bar sent back unchanged, so the trip
/// through degrees and document coordinates never moves a value by an ulp (an "edit, change
/// back, Apply" then writes nothing).
#[allow(clippy::float_cmp)] // the same JSON number the bar was given
fn keep_untouched(
    sent: &RecipeJson,
    shown: &RecipeJson,
    current: &TilingRecipe,
    into: &mut TilingRecipe,
) {
    let (c, i) = (&current.circle, &mut into.circle);
    if sent.start == shown.start {
        i.start = c.start;
    }
    if sent.sweep == shown.sweep {
        i.sweep = c.sweep;
    }
    if sent.centre_x == shown.centre_x {
        i.centre.x = c.centre.x;
    }
    if sent.centre_y == shown.centre_y {
        i.centre.y = c.centre.y;
    }
    let keep = |sent: Triple, shown: Triple, current: Channel<Angle>, into: &mut Channel<Angle>| {
        let slots = [
            (&mut into.random, current.random),
            (&mut into.per_row, current.per_row),
            (&mut into.per_column, current.per_column),
        ];
        for ((slot, stored), (a, b)) in slots.into_iter().zip(sent.into_iter().zip(shown)) {
            if a == b {
                *slot = stored;
            }
        }
    };
    keep(
        sent.rotation,
        shown.rotation,
        current.variance.rotation,
        &mut into.variance.rotation,
    );
    keep(
        sent.hue,
        shown.hue,
        current.variance.hue,
        &mut into.variance.hue,
    );
}

impl Session {
    /// The bar's recipe as JSON, empty while the bar is closed.
    #[must_use]
    pub fn tiling_recipe_json(&self) -> String {
        match (self.tiling_recipe(), self.tiling_box_centre()) {
            (Some(recipe), Some(centre)) => {
                serde_json::to_string(&to_json(&recipe, centre)).unwrap_or_default()
            }
            _ => String::new(),
        }
    }

    /// Takes the bar's recipe from JSON; false when it is malformed or out of range (the last
    /// valid preview stays, criterion 34).
    pub fn tiling_set_recipe_json(&mut self, json: &str) -> bool {
        let (Some(centre), Some(current)) = (self.tiling_box_centre(), self.tiling_recipe()) else {
            return false;
        };
        let Ok(sent) = serde_json::from_str::<RecipeJson>(json) else {
            return false;
        };
        let Some(mut recipe) = from_json(&sent, centre) else {
            return false;
        };
        keep_untouched(&sent, &to_json(&current, centre), &current, &mut recipe);
        self.tiling_set_recipe(recipe)
    }

    /// The status as JSON: `count`, `level` ("fine", "simplified", "refused"), `images_copies`,
    /// `refusal` (the core's sentence or empty), `reopened`; empty while the bar is closed.
    #[must_use]
    pub fn tiling_status_json(&self) -> String {
        let Some(status) = self.tiling_status() else {
            return String::new();
        };
        let (count, level) = match status.count {
            CountStatus::Fine(n) => (n, "fine"),
            CountStatus::Simplified(n) => (n, "simplified"),
            CountStatus::Refused(n) => (n, "refused"),
        };
        serde_json::json!({
            "count": count,
            "level": level,
            "images_copies": status.images_become_copies,
            "refusal": status.refusal.unwrap_or_default(),
            "reopened": status.reopened,
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_json_round_trips_a_recipe() {
        let centre = curvyo_document_core::Point::new(5.0, 2.5);
        let cell_box = curvyo_ui_core::tiling::CellBox {
            min: curvyo_document_core::Point::new(0.0, 0.0),
            max: curvyo_document_core::Point::new(10.0, 5.0),
        };
        let mut recipe = curvyo_ui_core::tiling::default_recipe(cell_box, 7);
        recipe.variance.rotation.random = Angle::from_radians(0.25_f64.to_radians() * 4.0);
        let back = from_json(&to_json(&recipe, centre), centre).unwrap();
        assert_eq!(back.seed, 7);
        assert!((back.variance.rotation.random.as_radians() - 1f64.to_radians()).abs() < 1e-12);
        assert!((back.circle.centre.x - recipe.circle.centre.x).abs() < 1e-12);
    }

    #[test]
    fn a_field_sent_back_unchanged_keeps_its_stored_value() {
        let mut session = Session::new(1);
        let id = session.document.create_path(
            &[
                curvyo_document_core::NewAnchor::corner(
                    curvyo_document_core::AnchorId::new(1, 1),
                    curvyo_document_core::Point::new(0.3, 0.7),
                ),
                curvyo_document_core::NewAnchor::corner(
                    curvyo_document_core::AnchorId::new(1, 2),
                    curvyo_document_core::Point::new(10.1, 3.3),
                ),
            ],
            false,
        );
        session.selection.set(&[id]);
        assert!(session.tiling_enter(9));
        let mut recipe = session.tiling_recipe().unwrap();
        recipe.circle.start = Angle::from_radians(0.1);
        recipe.variance.hue.random = Angle::from_radians(0.3);
        assert!(session.tiling_set_recipe(recipe));
        let json = session.tiling_recipe_json();
        assert!(session.tiling_set_recipe_json(&json));
        assert_eq!(session.tiling_recipe(), Some(recipe), "bit for bit");
    }
}

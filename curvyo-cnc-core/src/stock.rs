//! The stock of a CNC page (`specs/0161-cnc-output` criterion 3, `adrs.md` CN1): one job fixture
//! of kind `curvyo.cnc/stock` whose parameters are its width, height, thickness and material,
//! anchored at the page origin.

use curvyo_document_core::Length;
use serde_json::{Map, Value};

use crate::profile::CncValues;

/// The stock fixture's kind.
pub const STOCK_KIND: &str = "curvyo.cnc/stock";
/// The longest material name.
pub const MAX_MATERIAL_CHARS: usize = 40;

const WIDTH: &str = "width_mm";
const HEIGHT: &str = "height_mm";
const THICKNESS: &str = "thickness_mm";
const MATERIAL: &str = "material";

/// The block to mill.
#[derive(Debug, Clone, PartialEq)]
pub struct Stock {
    /// Its width along X.
    pub width: Length,
    /// Its height along the page's Y.
    pub height: Length,
    /// Its thickness along Z.
    pub thickness: Length,
    /// Its material name.
    pub material: String,
}

impl Default for Stock {
    /// 200 x 150 x 10 mm plywood (criterion 3, Proposal).
    fn default() -> Self {
        Self {
            width: Length::from_mm(200.0),
            height: Length::from_mm(150.0),
            thickness: Length::from_mm(10.0),
            material: "Plywood".to_owned(),
        }
    }
}

/// The ranges of the stock's fields on a machine of `values`: width and height 1 mm to the work
/// area, thickness 0.1 mm to the Z travel.
fn range(field: &str, values: &CncValues) -> Option<(f64, f64)> {
    match field {
        WIDTH => Some((1.0, values.work_area.width.as_mm())),
        HEIGHT => Some((1.0, values.work_area.height.as_mm())),
        THICKNESS => Some((0.1, values.z_travel.as_mm())),
        _ => None,
    }
}

/// The field message of a value outside its range (UX 11).
fn message(field: &str, (min, max): (f64, f64)) -> String {
    match field {
        WIDTH => format!("Width is 1 to {max} mm."),
        HEIGHT => format!("Height is 1 to {max} mm."),
        _ => format!("Thickness is {min} to {max:.1} mm."),
    }
}

impl Stock {
    /// The stock the fixture parameters `params` hold on a machine of `values`, with how many
    /// values were damaged: a missing or damaged value reads as the default.
    #[must_use]
    pub fn read(params: &Map<String, Value>, values: &CncValues) -> (Self, usize) {
        let mut stock = Self::default();
        let mut damaged = 0;
        for field in [WIDTH, HEIGHT, THICKNESS] {
            let Some(value) = params.get(field) else {
                continue;
            };
            let read = value
                .as_f64()
                .zip(range(field, values))
                .and_then(|(v, (min, max))| (min..=max).contains(&v).then(|| Length::from_mm(v)));
            match (read, field) {
                (Some(l), WIDTH) => stock.width = l,
                (Some(l), HEIGHT) => stock.height = l,
                (Some(l), _) => stock.thickness = l,
                (None, _) => damaged += 1,
            }
        }
        if let Some(value) = params.get(MATERIAL) {
            match value.as_str().filter(|m| material_fits(m)) {
                Some(m) => m.clone_into(&mut stock.material),
                None => damaged += 1,
            }
        }
        (stock, damaged)
    }

    /// The fixture parameters of the stock.
    #[must_use]
    pub fn params(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert(WIDTH.to_owned(), Value::from(self.width.as_mm()));
        map.insert(HEIGHT.to_owned(), Value::from(self.height.as_mm()));
        map.insert(THICKNESS.to_owned(), Value::from(self.thickness.as_mm()));
        map.insert(MATERIAL.to_owned(), Value::from(self.material.as_str()));
        map
    }

    /// The stock with `field` (`width_mm`, `height_mm`, `thickness_mm`, `material`) set to the
    /// typed `text` on a machine of `values`.
    ///
    /// # Errors
    /// The field message for a value that does not read or lies outside its range; an empty
    /// message for an unknown field.
    pub fn edit(&self, field: &str, text: &str, values: &CncValues) -> Result<Self, String> {
        let mut out = self.clone();
        if field == MATERIAL {
            let text = text.trim();
            if !material_fits(text) {
                return Err(format!("Material is 1 to {MAX_MATERIAL_CHARS} characters."));
            }
            text.clone_into(&mut out.material);
            return Ok(out);
        }
        let range = range(field, values).ok_or_else(String::new)?;
        let value = text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| (range.0..=range.1).contains(v))
            .ok_or_else(|| message(field, range))?;
        let l = Length::from_mm(value);
        match field {
            WIDTH => out.width = l,
            HEIGHT => out.height = l,
            _ => out.thickness = l,
        }
        Ok(out)
    }
}

fn material_fits(text: &str) -> bool {
    (1..=MAX_MATERIAL_CHARS).contains(&text.trim().chars().count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::DEFAULT_BUILTIN;

    #[test]
    fn stock_reads_back_and_refuses_values_beyond_the_machine_with_the_limit() {
        let values = DEFAULT_BUILTIN.profile().values;
        let stock = Stock::default();
        assert_eq!(Stock::read(&stock.params(), &values), (stock.clone(), 0));
        assert_eq!(
            stock.edit("width_mm", "301", &values),
            Err("Width is 1 to 300 mm.".to_owned())
        );
        assert_eq!(
            stock.edit("thickness_mm", "61", &values),
            Err("Thickness is 0.1 to 60.0 mm.".to_owned())
        );
        let edited = stock.edit("height_mm", "120", &values).unwrap();
        assert_eq!(edited.height.as_mm(), 120.0);
        let mut damaged = stock.params();
        damaged.insert("width_mm".into(), Value::from(9_999.0));
        assert_eq!(Stock::read(&damaged, &values).1, 1);
    }
}

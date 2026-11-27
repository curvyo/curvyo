//! The tool list of a CNC page (`specs/0161-cnc-output` criterion 4, `adrs.md` CN2): flat end
//! mills with their feeds, stored as one atomic register `x.curvyo.cnc.tools`.

use serde_json::{Value, json};

use crate::units::{FeedMmMin, Flutes, Rpm, StepDown, ToolDiameter};

/// The register field of the tool list (`x.curvyo.cnc.tools`).
pub const TOOLS_FIELD: &str = "tools";
/// The id of the tool a page without a stored list reads.
pub const DEFAULT_TOOL_ID: &str = "default";

/// One end mill.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    /// Its id (a UUID, or [`DEFAULT_TOOL_ID`]).
    pub id: String,
    /// Its diameter.
    pub diameter: ToolDiameter,
    /// Its flutes.
    pub flutes: Flutes,
    /// The cutting feed.
    pub feed: FeedMmMin,
    /// The plunge feed.
    pub plunge: FeedMmMin,
    /// The spindle speed.
    pub rpm: Rpm,
    /// The depth of one pass.
    pub step_down: StepDown,
}

impl Tool {
    /// The default tool with id `id` (criterion 4, Proposal): flat 3.175 mm, 2 flutes, feed
    /// 800 mm/min, plunge 300 mm/min, 12 000 rpm, step-down 1 mm.
    #[must_use]
    pub fn default_with_id(id: &str) -> Self {
        // invariant: each default lies within its unit's range
        Self {
            id: id.to_owned(),
            diameter: ToolDiameter::new(3.175).unwrap_or_else(|| unreachable!()),
            flutes: Flutes::new(2).unwrap_or_else(|| unreachable!()),
            feed: FeedMmMin::new(800.0).unwrap_or_else(|| unreachable!()),
            plunge: FeedMmMin::new(300.0).unwrap_or_else(|| unreachable!()),
            rpm: Rpm::new(12_000).unwrap_or_else(|| unreachable!()),
            step_down: StepDown::new(1.0).unwrap_or_else(|| unreachable!()),
        }
    }

    /// The tool's name ("Flat 3.175 mm"): it follows the diameter (UX N2).
    #[must_use]
    pub fn name(&self) -> String {
        format!("Flat {} mm", self.diameter.get())
    }

    /// The tool's radius in mm.
    #[must_use]
    pub fn radius_mm(&self) -> f64 {
        self.diameter.get() / 2.0
    }

    /// The tool as stored.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "kind": "flat",
            "diameter_mm": self.diameter.get(),
            "flutes": self.flutes.get(),
            "feed_mm_min": self.feed.get(),
            "plunge_mm_min": self.plunge.get(),
            "rpm": self.rpm.get(),
            "step_down_mm": self.step_down.get(),
        })
    }

    /// The tool `value` holds with how many values were damaged (each reads as the default's);
    /// `None` for no object with an id.
    fn of_json(value: &Value, spindle_max: Rpm) -> Option<(Self, usize)> {
        let map = value.as_object()?;
        let id = map.get("id").and_then(Value::as_str)?;
        let mut tool = Self::default_with_id(id);
        let mut damaged = usize::from(map.get("kind").and_then(Value::as_str) != Some("flat"));
        let f = |key: &str| map.get(key).and_then(Value::as_f64);
        let u = |key: &str| map.get(key).and_then(Value::as_u64);
        let mut count = |ok: bool| damaged += usize::from(!ok);
        count(
            f("diameter_mm")
                .and_then(ToolDiameter::new)
                .map(|v| tool.diameter = v)
                .is_some(),
        );
        count(
            u("flutes")
                .and_then(Flutes::new)
                .map(|v| tool.flutes = v)
                .is_some(),
        );
        count(
            f("feed_mm_min")
                .and_then(FeedMmMin::new)
                .map(|v| tool.feed = v)
                .is_some(),
        );
        count(
            f("plunge_mm_min")
                .and_then(FeedMmMin::new)
                .map(|v| tool.plunge = v)
                .is_some(),
        );
        let rpm = u("rpm").and_then(Rpm::new).filter(|r| *r <= spindle_max);
        count(rpm.map(|v| tool.rpm = v).is_some());
        count(
            f("step_down_mm")
                .and_then(StepDown::new)
                .map(|v| tool.step_down = v)
                .is_some(),
        );
        Some((tool, damaged))
    }

    /// The tool with `field` (`diameter_mm`, `flutes`, `feed_mm_min`, `plunge_mm_min`, `rpm`,
    /// `step_down_mm`) set to the typed `text`; the spindle speed is at most `spindle_max`.
    ///
    /// # Errors
    /// The field message for a value that does not read or is out of its range; an empty
    /// message for an unknown field.
    pub fn edit(&self, field: &str, text: &str, spindle_max: Rpm) -> Result<Self, String> {
        let text = text.trim().replace(',', "");
        let number = text.parse::<f64>().ok();
        let whole = text.parse::<u64>().ok();
        let mut out = self.clone();
        let feed = |name: &str| format!("{name} is 1 to 50,000 mm/min.");
        match field {
            "diameter_mm" => {
                out.diameter = number
                    .and_then(ToolDiameter::new)
                    .ok_or("Diameter is 0.1 to 25.0 mm.")?;
            }
            "flutes" => out.flutes = whole.and_then(Flutes::new).ok_or("Flutes is 1 to 8.")?,
            "feed_mm_min" => out.feed = number.and_then(FeedMmMin::new).ok_or(feed("Feed"))?,
            "plunge_mm_min" => {
                out.plunge = number.and_then(FeedMmMin::new).ok_or(feed("Plunge"))?;
            }
            "rpm" => {
                out.rpm = whole
                    .and_then(Rpm::new)
                    .filter(|r| *r <= spindle_max)
                    .ok_or_else(|| format!("Spindle is 0 to {} rpm.", grouped(spindle_max)))?;
            }
            "step_down_mm" => {
                out.step_down = number
                    .and_then(StepDown::new)
                    .ok_or("Step-down is 0.1 to 25.0 mm.")?;
            }
            _ => return Err(String::new()),
        }
        Ok(out)
    }
}

fn grouped(rpm: Rpm) -> String {
    let n = rpm.get();
    if n >= 1_000 {
        format!("{},{:03}", n / 1_000, n % 1_000)
    } else {
        n.to_string()
    }
}

/// The tool list `stored` holds (the register's value) with how many values were damaged; a
/// missing or empty list reads as the one default tool. An entry that is no tool is skipped and
/// counted.
#[must_use]
pub fn read_tools(stored: Option<&Value>, spindle_max: Rpm) -> (Vec<Tool>, usize) {
    let mut damaged = 0;
    let mut tools = Vec::new();
    if let Some(stored) = stored {
        match stored.as_array() {
            Some(items) => {
                for item in items {
                    match Tool::of_json(item, spindle_max) {
                        Some((tool, bad)) if !tools.iter().any(|t: &Tool| t.id == tool.id) => {
                            damaged += bad;
                            tools.push(tool);
                        }
                        _ => damaged += 1,
                    }
                }
            }
            None => damaged += 1,
        }
    }
    if tools.is_empty() {
        tools.push(Tool::default_with_id(DEFAULT_TOOL_ID));
    }
    (tools, damaged)
}

/// The register value of `tools`.
#[must_use]
pub fn write_tools(tools: &[Tool]) -> Value {
    Value::Array(tools.iter().map(Tool::to_json).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn max() -> Rpm {
        Rpm::new(12_000).unwrap()
    }

    #[test]
    fn the_default_tool_and_a_damaged_diameter_read_as_criterion_4_says() {
        let (tools, damaged) = read_tools(None, max());
        assert_eq!(damaged, 0);
        assert_eq!(tools[0].name(), "Flat 3.175 mm");
        assert_eq!(tools[0].feed.get(), 800.0);
        let mut stored = write_tools(&tools);
        stored[0]["diameter_mm"] = Value::from(0.0);
        let (tools, damaged) = read_tools(Some(&stored), max());
        assert_eq!((tools[0].diameter.get(), damaged), (3.175, 1));
        assert_eq!(read_tools(Some(&write_tools(&tools)), max()), (tools, 0));
    }

    #[test]
    fn every_tool_value_is_refused_outside_its_range() {
        let tool = Tool::default_with_id("t");
        assert_eq!(
            tool.edit("diameter_mm", "0", max()),
            Err("Diameter is 0.1 to 25.0 mm.".to_owned())
        );
        assert_eq!(
            tool.edit("rpm", "13000", max()),
            Err("Spindle is 0 to 12,000 rpm.".to_owned())
        );
        assert_eq!(tool.edit("flutes", "4", max()).unwrap().flutes.get(), 4);
        assert!(tool.edit("step_down_mm", "30", max()).is_err());
    }
}

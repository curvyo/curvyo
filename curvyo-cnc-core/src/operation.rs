//! The Profile operation's numbers (`specs/0161-cnc-output` criterion 5, `adrs.md` CN2): side,
//! depth, tool and direction in `op_options`, the typed feeds in `op_settings`.

use serde_json::{Map, Value};

use crate::tools::Tool;
use crate::units::{Depth, FeedMmMin};

/// The stored kind of a Profile operation.
pub const PROFILE_KIND: &str = "profile";

/// Which side of the outline the tool centre runs (criterion 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Side {
    /// Outside by the tool radius: the part keeps its size.
    #[default]
    Outside,
    /// Inside by the tool radius: a hole keeps its size.
    Inside,
}

impl Side {
    /// Both sides.
    pub const ALL: [Self; 2] = [Self::Outside, Self::Inside];

    /// The stored spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Outside => "outside",
            Self::Inside => "inside",
        }
    }

    /// The word ("Outside").
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Outside => "Outside",
            Self::Inside => "Inside",
        }
    }

    /// The side a stored spelling names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.key() == key)
    }
}

/// The milling direction with the spindle clockwise (criterion 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    /// Conventional (up) milling: the default (Proposal).
    #[default]
    Conventional,
    /// Climb (down) milling: the finished wall on the tool's right.
    Climb,
}

impl Direction {
    /// Both directions.
    pub const ALL: [Self; 2] = [Self::Conventional, Self::Climb];

    /// The stored spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Conventional => "conventional",
            Self::Climb => "climb",
        }
    }

    /// The word ("Conventional").
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Conventional => "Conventional",
            Self::Climb => "Climb",
        }
    }

    /// The direction a stored spelling names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.key() == key)
    }
}

/// The numbers of one Profile operation as read.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileSettings {
    /// The side.
    pub side: Side,
    /// The depth below the stock top.
    pub depth: Depth,
    /// The id of its tool in the page's list.
    pub tool: String,
    /// The direction.
    pub direction: Direction,
    /// The typed cutting feed; `None` follows the tool.
    pub feed: Option<FeedMmMin>,
    /// The typed plunge feed; `None` follows the tool.
    pub plunge: Option<FeedMmMin>,
}

/// The default depth on a stock `thickness_mm` thick: the whole thickness.
fn full_depth(thickness_mm: f64) -> Depth {
    Depth::new(thickness_mm)
        .or_else(|| Depth::new(Depth::MIN))
        // invariant: Depth::MIN lies within the range of Depth
        .unwrap_or_else(|| unreachable!())
}

impl ProfileSettings {
    /// A new operation's numbers: Outside, the whole stock thickness, `tool`, Conventional, the
    /// tool's feeds.
    #[must_use]
    pub fn new(tool: &Tool, thickness_mm: f64) -> Self {
        Self {
            side: Side::Outside,
            depth: full_depth(thickness_mm),
            tool: tool.id.clone(),
            direction: Direction::Conventional,
            feed: None,
            plunge: None,
        }
    }

    /// The numbers `options` and `settings` hold on a page of `tools` and a stock `thickness_mm`
    /// thick, with how many values were damaged: each reads as the default (an unknown tool as
    /// the first, a depth beyond the stock as the stock thickness).
    #[must_use]
    pub fn read(
        options: &Map<String, Value>,
        settings: &Map<String, Value>,
        tools: &[Tool],
        thickness_mm: f64,
    ) -> (Self, usize) {
        let first = tools.first().map(|t| t.id.clone()).unwrap_or_default();
        let mut out = Self {
            tool: first,
            ..Self::new(&Tool::default_with_id(""), thickness_mm)
        };
        let mut damaged = 0;
        let mut take = |map: &Map<String, Value>, key: &str, ok: &mut dyn FnMut(&Value) -> bool| {
            if let Some(v) = map.get(key)
                && !ok(v)
            {
                damaged += 1;
            }
        };
        take(options, "side", &mut |v| {
            v.as_str()
                .and_then(Side::parse)
                .map(|s| out.side = s)
                .is_some()
        });
        take(options, "direction", &mut |v| {
            v.as_str()
                .and_then(Direction::parse)
                .map(|d| out.direction = d)
                .is_some()
        });
        take(options, "depth_mm", &mut |v| {
            v.as_f64()
                .filter(|d| *d <= thickness_mm + 1e-9)
                .and_then(Depth::new)
                .map(|d| out.depth = d)
                .is_some()
        });
        take(options, "tool", &mut |v| {
            v.as_str()
                .filter(|id| tools.iter().any(|t| t.id == *id))
                .map(|id| id.clone_into(&mut out.tool))
                .is_some()
        });
        take(settings, "feed_mm_min", &mut |v| {
            v.as_f64()
                .and_then(FeedMmMin::new)
                .map(|f| out.feed = Some(f))
                .is_some()
        });
        take(settings, "plunge_mm_min", &mut |v| {
            v.as_f64()
                .and_then(FeedMmMin::new)
                .map(|f| out.plunge = Some(f))
                .is_some()
        });
        (out, damaged)
    }

    /// The `op_options` map.
    #[must_use]
    pub fn options(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("side".into(), Value::from(self.side.key()));
        map.insert("depth_mm".into(), Value::from(self.depth.get()));
        map.insert("tool".into(), Value::from(self.tool.as_str()));
        map.insert("direction".into(), Value::from(self.direction.key()));
        map
    }

    /// The `op_settings` map: the typed feeds only.
    #[must_use]
    pub fn settings(&self) -> Map<String, Value> {
        let mut map = Map::new();
        if let Some(f) = self.feed {
            map.insert("feed_mm_min".into(), Value::from(f.get()));
        }
        if let Some(f) = self.plunge {
            map.insert("plunge_mm_min".into(), Value::from(f.get()));
        }
        map
    }

    /// The numbers with `field` (`side`, `direction`, `depth_mm`, `tool`, `feed_mm_min`,
    /// `plunge_mm_min`) set to `text` on a page of `tools` and a stock `thickness_mm` thick; an
    /// empty feed follows the tool again (Reset to tool).
    ///
    /// # Errors
    /// The field message for a value that does not read or is out of its range; an empty
    /// message for an unknown field or tool.
    pub fn edit(
        &self,
        field: &str,
        text: &str,
        tools: &[Tool],
        thickness_mm: f64,
    ) -> Result<Self, String> {
        let text = text.trim().replace(',', "");
        let mut out = self.clone();
        let feed = |name: &str| -> Result<Option<FeedMmMin>, String> {
            if text.is_empty() {
                return Ok(None);
            }
            text.parse::<f64>()
                .ok()
                .and_then(FeedMmMin::new)
                .map(Some)
                .ok_or_else(|| format!("{name} is 1 to 50,000 mm/min."))
        };
        match field {
            "side" => out.side = Side::parse(&text).ok_or_else(String::new)?,
            "direction" => out.direction = Direction::parse(&text).ok_or_else(String::new)?,
            "depth_mm" => {
                out.depth = text
                    .parse::<f64>()
                    .ok()
                    .filter(|d| *d <= thickness_mm + 1e-9)
                    .and_then(Depth::new)
                    .ok_or_else(|| format!("Depth is 0.1 to {thickness_mm:.1} mm."))?;
            }
            "tool" if tools.iter().any(|t| t.id == text) => out.tool.clone_from(&text),
            "feed_mm_min" => out.feed = feed("Feed")?,
            "plunge_mm_min" => out.plunge = feed("Plunge")?,
            _ => return Err(String::new()),
        }
        Ok(out)
    }

    /// The number of passes on a tool of step-down `tool`: ceil(depth / step-down), counted in
    /// whole micrometres so 1.1 / 0.1 is 11 (criterion 8).
    #[must_use]
    pub fn passes(&self, tool: &Tool) -> u32 {
        let (d, s) = (self.depth.um(), tool.step_down.um().max(1));
        u32::try_from((d + s - 1) / s).unwrap_or(u32::MAX).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_read_back_and_damage_reads_as_the_defaults() {
        let tools = vec![Tool::default_with_id("a"), Tool::default_with_id("b")];
        let mut s = ProfileSettings::new(&tools[1], 10.0);
        s.side = Side::Inside;
        s.feed = FeedMmMin::new(600.0);
        assert_eq!(
            ProfileSettings::read(&s.options(), &s.settings(), &tools, 10.0),
            (s.clone(), 0)
        );
        let mut options = s.options();
        options.insert("depth_mm".into(), Value::from(25.0));
        options.insert("tool".into(), Value::from("gone"));
        let (read, damaged) = ProfileSettings::read(&options, &Map::new(), &tools, 10.0);
        assert_eq!(
            (read.depth.get(), read.tool.as_str(), damaged),
            (10.0, "a", 2)
        );
        assert_eq!(
            s.edit("depth_mm", "11", &tools, 10.0),
            Err("Depth is 0.1 to 10.0 mm.".to_owned())
        );
        assert_eq!(s.edit("feed_mm_min", "", &tools, 10.0).unwrap().feed, None);
    }

    #[test]
    fn passes_are_counted_in_micrometres() {
        let mut tool = Tool::default_with_id("t");
        tool.step_down = crate::units::StepDown::new(0.1).unwrap();
        let mut s = ProfileSettings::new(&tool, 10.0);
        s.depth = Depth::new(1.1).unwrap();
        assert_eq!(s.passes(&tool), 11);
        tool.step_down = crate::units::StepDown::new(3.0).unwrap();
        s.depth = Depth::new(10.0).unwrap();
        assert_eq!(s.passes(&tool), 4);
    }
}

//! The serialisable shapes of the Materials tab view.

use curvyo_laser_core::settings::{SetKind, Settings};
use serde::Serialize;

/// The Materials tab of the active laser page.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaterialsView {
    /// The page's laser: profile id and name.
    pub page_laser: Option<LaserView>,
    /// The rows in the maker's order.
    pub rows: Vec<MaterialRowView>,
    /// The unfolded material's body, `None` when none is unfolded or it does not read.
    pub unfolded: Option<MaterialBodyView>,
    /// Whether the New material form is open.
    pub form_open: bool,
    /// The default name of a new material, "Material `<N>`".
    pub next_name: String,
    /// One notice per damaged, newer or older record of the load, and the retired file's.
    pub load_notices: Vec<String>,
}

/// A laser as the tab names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LaserView {
    /// The profile id.
    pub id: String,
    /// The profile's name.
    pub name: String,
}

/// One row of the list.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum MaterialRowView {
    /// A readable material.
    Loaded {
        /// Its id.
        id: String,
        /// Its name.
        name: String,
        /// Its thickness in mm.
        thickness_mm: f64,
        /// Line 2: "2 lasers · 3 sets · 4 tests", or "No sets".
        line: String,
        /// How many tests it holds (the delete status).
        tests: usize,
    },
    /// A damaged record.
    Damaged {
        /// Its id.
        id: String,
        /// The reason.
        reason: String,
    },
    /// A record of a newer format.
    Newer {
        /// Its id.
        id: String,
        /// The name its JSON names, when readable.
        name: Option<String>,
    },
    /// A record of format 1, skipped in this build (plan Part F).
    Older {
        /// Its id.
        id: String,
        /// The name its JSON names, when readable.
        name: Option<String>,
    },
}

/// The kinds of a set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SetKindView {
    /// Cut.
    Cut,
    /// Score.
    Score,
    /// Engrave.
    Engrave,
}

impl From<SetKind> for SetKindView {
    fn from(kind: SetKind) -> Self {
        match kind {
            SetKind::Cut => Self::Cut,
            SetKind::Score => Self::Score,
            SetKind::Engrave => Self::Engrave,
        }
    }
}

/// The three numbers of a set.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct NumbersView {
    /// Power in percent.
    pub power_pct: f64,
    /// Speed in mm/min.
    pub speed_mm_min: u64,
    /// Passes.
    pub passes: u8,
}

impl From<Settings> for NumbersView {
    fn from(set: Settings) -> Self {
        Self {
            power_pct: set.power.percent(),
            speed_mm_min: u64::from(set.speed.get()),
            passes: set.passes.get(),
        }
    }
}

/// The unfolded material (UX 7.4), or its open test (UX 8.2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaterialBodyView {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its thickness in mm.
    pub thickness_mm: f64,
    /// The laser blocks: the page's first, then the others in the order they were added.
    pub lasers: Vec<LaserBlockView>,
    /// Whether the "+ Sets for another laser" list is unfolded.
    pub laser_pick_open: bool,
    /// The library's profiles not yet in the material, for that list.
    pub other_lasers: Vec<LaserView>,
    /// The tests, newest first.
    pub tests: Vec<TestRowView>,
    /// The open test, `None` for the material's body.
    pub test: Option<TestDetailView>,
}

/// One laser block.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LaserBlockView {
    /// The laser.
    pub laser: LaserView,
    /// Whether it is the page's laser ("This page").
    pub this_page: bool,
    /// Whether the block is unfolded.
    pub open: bool,
    /// Its sets in the order Cut, Score, Engrave.
    pub sets: Vec<SetView>,
    /// The kinds without a set, one "+ `<kind>` set" button each.
    pub missing: Vec<SetKindView>,
}

/// One set of a laser.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SetView {
    /// Its kind.
    pub kind: SetKindView,
    /// Its numbers.
    pub numbers: NumbersView,
    /// Its interval in mm, Engrave only.
    pub interval_mm: Option<f64>,
    /// "Test K7Q2M" or "Typed".
    pub source: String,
}

/// One test row (UX 8.1).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TestRowView {
    /// Its id.
    pub id: String,
    /// Its kind.
    pub kind: SetKindView,
    /// Whether it is Lasered.
    pub lasered: bool,
    /// Line 2: "Lasered 12 Nov · Diode 20 W", "Not lasered · Birch sheet" or "Not lasered · not
    /// on a page".
    pub line: String,
    /// Whether it has a photo.
    pub photo: bool,
}

/// One axis of the judge table.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AxisView {
    /// Its title ("Power %").
    pub title: String,
    /// The unit after a value in a spoken text (" %", " mm/min", " ×", " mm").
    pub unit: &'static str,
    /// The value texts of its cells.
    pub values: Vec<String>,
}

/// One cell's place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CellView {
    /// Its column, from 0 at the left.
    pub column: u8,
    /// Its row, from 0 at the top.
    pub row: u8,
}

/// Where "Place on `<page>`" or "Select on page" goes (criterion 28).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlaceView {
    /// Whether the grid is on a laser page ("Select on page").
    pub on_page: bool,
    /// The button text.
    pub label: String,
}

/// The open test (UX 8.2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TestDetailView {
    /// Its id.
    pub id: String,
    /// Its kind.
    pub kind: SetKindView,
    /// "Cut · Diode 20 W".
    pub head: String,
    /// "Lasered 12 Nov 2026 · not on a page" or "Not lasered · on Birch sheet".
    pub state_line: String,
    /// "Power 20–100 % × Speed 600–6000 mm/min · 1 × · cell 10 mm · 12 Nov 2026".
    pub axes_line: String,
    /// The columns.
    pub columns: AxisView,
    /// The rows.
    pub rows: AxisView,
    /// The best cell.
    pub best: Option<CellView>,
    /// "Cut set of Diode 20 W: 60 %, 1950 mm/min, 1 ×", `None` without a best cell.
    pub set_line: Option<String>,
    /// The note.
    pub note: String,
    /// The photo's file name, `None` without a photo.
    pub photo: Option<String>,
    /// The place button.
    pub place: PlaceView,
}

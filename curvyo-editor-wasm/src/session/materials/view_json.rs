//! Builds the Materials tab view from the session (`specs/0112` criteria 16 to 28; UX notes 7, 8; `adrs.md`
//! finding 3): the one producer of `materials_view`, pinned by the shared fixtures in
//! `curvyo-editor-wasm/tests/fixtures/materials_view/`.

use curvyo_laser_core::grids::{Param, Side, TestId};
use curvyo_laser_core::library::{
    LaserSets, MAX_MATERIALS, Material, MaterialRead, Set, SetSource, TestRecord, photo_file_name,
};
use curvyo_laser_core::settings::SetKind;
use std::fmt::Write as _;

use super::super::Session;

use super::view_types::{
    AxisView, CellView, LaserBlockView, LaserView, MaterialBodyView, MaterialRowView,
    MaterialsView, PlaceView, SetKindView, SetView, TestDetailView, TestRowView,
};

const KINDS: [SetKind; 3] = [SetKind::Cut, SetKind::Score, SetKind::Engrave];

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The civil date of Unix milliseconds `ms` (UTC): year, month 1 to 12, day.
fn civil(ms: u64) -> (i64, usize, i64) {
    let days = i64::try_from(ms / 86_400_000).unwrap_or(0);
    // Howard Hinnant's days-to-civil.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, usize::try_from(month).unwrap_or(1), day)
}

/// "12 Nov".
pub(in crate::session) fn short_date(ms: u64) -> String {
    let (_, month, day) = civil(ms);
    format!("{day} {}", MONTHS[month.clamp(1, 12) - 1])
}

/// "12 Nov 2026".
pub(in crate::session) fn long_date(ms: u64) -> String {
    let (year, month, day) = civil(ms);
    format!("{day} {} {year}", MONTHS[month.clamp(1, 12) - 1])
}

/// "1 laser", "2 lasers"; `None` at 0.
fn count(n: usize, word: &str) -> Option<String> {
    match n {
        0 => None,
        1 => Some(format!("1 {word}")),
        n => Some(format!("{n} {word}s")),
    }
}

/// Line 2 of a material row (UX 7.3).
fn row_line(material: &Material) -> String {
    let parts: Vec<String> = [
        count(material.lasers.len(), "laser"),
        count(material.set_count(), "set"),
        count(material.tests.len(), "test"),
    ]
    .into_iter()
    .flatten()
    .collect();
    if parts.is_empty() {
        "No sets".to_owned()
    } else {
        parts.join(" · ")
    }
}

fn source_text(set: &Set) -> String {
    match &set.source {
        SetSource::Test(id) => format!("Test {id}"),
        SetSource::Typed => "Typed".to_owned(),
        SetSource::History(_) => "Test".to_owned(),
    }
}

fn set_view(kind: SetKind, set: &Set) -> SetView {
    SetView {
        kind: kind.into(),
        numbers: set.numbers.settings.into(),
        interval_mm: set
            .numbers
            .interval
            .map(curvyo_laser_core::settings::Interval::mm),
        source: source_text(set),
    }
}

/// "60 %, 1950 mm/min, 1 ×" (and ", 0.08 mm" with an interval).
fn numbers_text(set: &Set) -> String {
    let s = set.numbers.settings;
    let mut text = format!(
        "{} %, {} mm/min, {} ×",
        s.power,
        s.speed.get(),
        s.passes.get()
    );
    if let Some(interval) = set.numbers.interval {
        let _ = write!(text, ", {:.2} mm", interval.mm());
    }
    text
}

fn axis_view(test: &TestRecord, side: Side) -> AxisView {
    let axis = test.recipe.axis(side);
    AxisView {
        title: axis.param().title().to_owned(),
        unit: param_words(axis.param()).1,
        values: (0..axis.count())
            .map(|i| axis.value(i).to_string())
            .collect(),
    }
}

/// The word and unit of a parameter in the axes line.
pub(in crate::session) const fn param_words(param: Param) -> (&'static str, &'static str) {
    match param {
        Param::Power => ("Power", " %"),
        Param::Speed => ("Speed", " mm/min"),
        Param::Passes => ("Passes", " ×"),
        Param::Interval => ("Interval", " mm"),
    }
}

/// "Power 20–100 % × Speed 600–6000 mm/min · 1 × · cell 10 mm · 12 Nov 2026".
fn axes_line(test: &TestRecord) -> String {
    let side = |side| {
        let axis = test.recipe.axis(side);
        let (min, max) = (axis.min(), axis.max());
        let (word, unit) = param_words(axis.param());
        format!("{word} {min}–{max}{unit}")
    };
    let mut parts = vec![format!("{} × {}", side(Side::Columns), side(Side::Rows))];
    parts.extend(
        test.recipe
            .fixed_values()
            .into_iter()
            .map(|value| format!("{value}{}", param_words(value.param).1)),
    );
    parts.push(format!("cell {} mm", test.recipe.cell().as_mm()));
    parts.push(long_date(test.created));
    parts.join(" · ")
}

impl Session {
    /// The page name of the laser page holding a grid with the test id `id`, if any.
    fn test_page(&self, id: TestId) -> Option<String> {
        self.placed_test_grids()
            .into_iter()
            .find(|grid| grid.id == id)
            .map(|grid| grid.page_name)
    }

    fn test_row(&self, test: &TestRecord) -> TestRowView {
        let line = match test.lasered {
            Some(at) => format!("Lasered {} · {}", short_date(at), test.machine.name),
            None => format!(
                "Not lasered · {}",
                self.test_page(test.id)
                    .unwrap_or_else(|| "not on a page".to_owned())
            ),
        };
        TestRowView {
            id: test.id.to_string(),
            kind: test.recipe.kind().into(),
            lasered: test.lasered.is_some(),
            line,
            photo: test.photo.is_some(),
        }
    }

    fn test_detail(&self, material: &Material, test: &TestRecord) -> TestDetailView {
        let page = self.test_page(test.id);
        let on = page
            .as_ref()
            .map_or_else(|| "not on a page".to_owned(), |p| format!("on {p}"));
        let state_line = match test.lasered {
            Some(at) => format!("Lasered {} · {on}", long_date(at)),
            None => format!("Not lasered · {on}"),
        };
        let kind = test.recipe.kind();
        let set_line = test.best.and(
            material
                .set_for(&test.machine.id, kind)
                .filter(|set| set.source == SetSource::Test(test.id))
                .map(|set| {
                    format!(
                        "{} set of {}: {}",
                        kind.label(),
                        test.machine.name,
                        numbers_text(set)
                    )
                }),
        );
        let page_name = self.active_page().name;
        TestDetailView {
            id: test.id.to_string(),
            kind: kind.into(),
            head: format!("{} · {}", kind.label(), test.machine.name),
            state_line,
            axes_line: axes_line(test),
            columns: axis_view(test, Side::Columns),
            rows: axis_view(test, Side::Rows),
            best: test.best.map(|c| CellView {
                column: c.column,
                row: c.row,
            }),
            set_line,
            note: test.note.clone(),
            photo: test
                .photo
                .map(|kind| photo_file_name(&material.id, test.id, kind)),
            place: PlaceView {
                on_page: page.is_some(),
                label: if page.is_some() {
                    "Select on page".to_owned()
                } else {
                    format!("Place on {page_name}")
                },
            },
        }
    }

    fn laser_block(&self, laser: &LaserSets, this_page: bool) -> LaserBlockView {
        let tab = &self.materials.tab;
        LaserBlockView {
            laser: LaserView {
                id: laser.machine.id.clone(),
                name: laser.machine.name.clone(),
            },
            this_page,
            open: if this_page {
                !tab.page_laser_folded
            } else {
                tab.open_lasers.contains(&laser.machine.id)
            },
            sets: KINDS
                .into_iter()
                .filter_map(|kind| laser.sets.get(&kind).map(|set| set_view(kind, set)))
                .collect(),
            missing: KINDS
                .into_iter()
                .filter(|kind| !laser.sets.contains_key(kind))
                .map(SetKindView::from)
                .collect(),
        }
    }

    fn material_body(&self, material: &Material) -> MaterialBodyView {
        let page = self.active_laser();
        let page_id = page.as_ref().map(|l| l.id.clone());
        let mut lasers: Vec<LaserSets> = material.lasers.clone();
        let picked = self.materials.tab.picked_lasers.get(&material.id);
        for laser in page.iter().chain(picked.into_iter().flatten()) {
            if !lasers.iter().any(|l| l.machine.id == laser.id) {
                lasers.push(LaserSets {
                    machine: laser.clone(),
                    sets: std::collections::BTreeMap::new(),
                });
            }
        }
        lasers.sort_by_key(|l| Some(&l.machine.id) != page_id.as_ref());
        let blocks = lasers
            .iter()
            .map(|l| self.laser_block(l, Some(&l.machine.id) == page_id.as_ref()))
            .collect();
        let other_lasers = self
            .machine_profiles()
            .listed()
            .into_iter()
            .filter(|p| !lasers.iter().any(|l| l.machine.id == p.id))
            .map(|p| LaserView {
                id: p.id.clone(),
                name: p.values.name.clone(),
            })
            .collect();
        let test = self
            .materials
            .tab
            .open_test
            .and_then(|id| material.test(id))
            .map(|test| self.test_detail(material, test));
        MaterialBodyView {
            id: material.id.clone(),
            name: material.name.clone(),
            thickness_mm: material.thickness.mm(),
            lasers: blocks,
            laser_pick_open: self.materials.tab.laser_pick_open,
            other_lasers,
            tests: material
                .tests
                .iter()
                .rev()
                .map(|t| self.test_row(t))
                .collect(),
            test,
        }
    }

    /// The default name of a new material: "Material `<N>`" with the first free number.
    fn next_material_name(&self) -> String {
        (1..=MAX_MATERIALS + 1)
            .map(|n| format!("Material {n}"))
            .find(|name| {
                !self
                    .materials
                    .library
                    .reads()
                    .iter()
                    .any(|read| matches!(read, MaterialRead::Loaded(m) if &m.name == name))
            })
            .unwrap_or_default()
    }

    /// The Materials tab, `None` off a laser page.
    #[must_use]
    pub fn materials_view(&self) -> Option<MaterialsView> {
        self.active_copy()?;
        let library = &self.materials.library;
        let rows = library
            .listed()
            .into_iter()
            .map(|read| match read {
                MaterialRead::Loaded(material) => MaterialRowView::Loaded {
                    id: material.id.clone(),
                    name: material.name.clone(),
                    thickness_mm: material.thickness.mm(),
                    line: row_line(material),
                    tests: material.tests.len(),
                },
                MaterialRead::Damaged { id, reason } => MaterialRowView::Damaged {
                    id: id.clone(),
                    reason: reason.clone(),
                },
                MaterialRead::Newer { id } => MaterialRowView::Newer {
                    id: id.clone(),
                    name: library.name_of(id).map(str::to_owned),
                },
                MaterialRead::Older { id } => MaterialRowView::Older {
                    id: id.clone(),
                    name: library.name_of(id).map(str::to_owned),
                },
            })
            .collect();
        let unfolded = self
            .materials
            .tab
            .unfolded
            .as_deref()
            .and_then(|id| library.find(id))
            .map(|material| self.material_body(material));
        let mut load_notices = library.load_notices().to_vec();
        load_notices.extend(self.materials.retired_notice.map(str::to_owned));
        Some(MaterialsView {
            page_laser: self.active_laser().map(|l| LaserView {
                id: l.id,
                name: l.name,
            }),
            rows,
            unfolded,
            form_open: self.materials.tab.form_open,
            next_name: self.next_material_name(),
            load_notices,
        })
    }

    /// The Materials tab as JSON, `null` off a laser page.
    #[must_use]
    pub fn materials_view_json(&self) -> String {
        serde_json::to_string(&self.materials_view()).unwrap_or_else(|_| "null".to_owned())
    }
}

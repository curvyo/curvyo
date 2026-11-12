//! The Materials section's view as JSON (`specs/0028` criteria 19 to 23, 29; UX notes §10;
//! `adrs.md` §6): the one producer of `materials_view`, pinned by the shared fixtures in
//! `curvyo-editor-wasm/tests/fixtures/materials_view/`.

use curvyo_laser_core::material_library::Materials;
use curvyo_laser_core::settings::{HistoryEvent, Material, MaterialRead, SetKind, SetSource};
use serde::Serialize;

use super::super::Session;

/// The Materials section of the active laser page.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaterialsView {
    /// The name of the page's profile copy ("Materials of `<profile name>`").
    pub profile_name: String,
    /// The rows of this machine in the maker's order.
    pub rows: Vec<MaterialRowView>,
    /// The selected material's fields, `None` when no readable row is selected.
    pub selected: Option<MaterialFieldsView>,
    /// The default name of a new material, "Material `<N>`".
    pub next_name: String,
    /// One notice per damaged or newer record of the load.
    pub load_notices: Vec<String>,
}

/// One row of the section.
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
        /// The kinds it has sets for, in the order Cut, Score, Engrave.
        kinds: Vec<SetKindView>,
        /// How many operations of the open project point to it.
        uses: usize,
    },
    /// A damaged record (criterion 29).
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

impl From<curvyo_laser_core::settings::Settings> for NumbersView {
    fn from(set: curvyo_laser_core::settings::Settings) -> Self {
        Self {
            power_pct: set.power.percent(),
            speed_mm_min: u64::from(set.speed.get()),
            passes: set.passes.get(),
        }
    }
}

/// The fields of the selected material (UX notes §10.3).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaterialFieldsView {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its thickness in mm.
    pub thickness_mm: f64,
    /// How many operations of the open project point to it.
    pub uses: usize,
    /// Its sets in the order Cut, Score, Engrave.
    pub sets: Vec<SetFieldsView>,
}

/// One set of the selected material.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SetFieldsView {
    /// Its kind.
    pub kind: SetKindView,
    /// Its numbers.
    pub numbers: NumbersView,
    /// The source line ("Tested 2026-11-10, 5 × 5 grid, cell 60 %, 3000 mm/min").
    pub source: String,
}

/// The civil date of Unix milliseconds `ms`, "2026-11-10" (UTC).
pub(in crate::session) fn date_text(ms: u64) -> String {
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
    format!("{year:04}-{month:02}-{day:02}")
}

/// The source line of the `kind` set of `material` (UX notes §10.3).
fn source_line(material: &Material, kind: SetKind) -> String {
    let Some(set) = material.sets.get(&kind) else {
        return String::new();
    };
    match &set.source {
        SetSource::Test(entry) => material
            .history
            .iter()
            .find(|e| &e.id == entry)
            .and_then(|e| match &e.event {
                HistoryEvent::Tested { grid, set, .. } => Some(format!(
                    "Tested {}, {} × {} grid, cell {} %, {} mm/min",
                    date_text(e.date),
                    grid.columns,
                    grid.rows,
                    set.power,
                    set.speed.get()
                )),
                _ => None,
            })
            .unwrap_or_default(),
        SetSource::Edited => material
            .history
            .iter()
            .rev()
            .find(|e| e.kind == kind && matches!(e.event, HistoryEvent::Edited { .. }))
            .map_or_else(
                || "Edited by hand".to_owned(),
                |e| format!("Edited by hand {}", date_text(e.date)),
            ),
    }
}

const KINDS: [SetKind; 3] = [SetKind::Cut, SetKind::Score, SetKind::Engrave];

/// The listed rows of the library for `profile_id`.
fn material_rows(
    library: &Materials,
    profile_id: &str,
    uses: &dyn Fn(&str) -> usize,
) -> Vec<MaterialRowView> {
    library
        .listed(profile_id)
        .into_iter()
        .map(|read| match read {
            MaterialRead::Loaded(material) => MaterialRowView::Loaded {
                id: material.id.clone(),
                name: material.name.clone(),
                thickness_mm: material.thickness.mm(),
                kinds: KINDS
                    .into_iter()
                    .filter(|kind| material.sets.contains_key(kind))
                    .map(SetKindView::from)
                    .collect(),
                uses: uses(&material.id),
            },
            MaterialRead::Damaged { id, reason } => MaterialRowView::Damaged {
                id: id.clone(),
                reason: reason.clone(),
            },
            MaterialRead::Newer { id } => MaterialRowView::Newer {
                id: id.clone(),
                name: library.name_of(id).map(str::to_owned),
            },
        })
        .collect()
}

/// The fields of the selected material, `uses` operations linking it.
fn selected_fields(material: &Material, uses: usize) -> MaterialFieldsView {
    MaterialFieldsView {
        id: material.id.clone(),
        name: material.name.clone(),
        thickness_mm: material.thickness.mm(),
        uses,
        sets: KINDS
            .into_iter()
            .filter_map(|kind| {
                let set = material.sets.get(&kind)?;
                Some(SetFieldsView {
                    kind: kind.into(),
                    numbers: set.settings.into(),
                    source: source_line(material, kind),
                })
            })
            .collect(),
    }
}

/// The first `n` for which "Material n" is free among the loaded names of `profile_id`.
fn next_material_number(library: &Materials, profile_id: &str) -> usize {
    let names: Vec<&str> = library
        .listed(profile_id)
        .into_iter()
        .filter_map(|read| match read {
            MaterialRead::Loaded(material) => Some(material.name.as_str()),
            _ => None,
        })
        .collect();
    (1..=names.len() + 1)
        .find(|n| !names.contains(&format!("Material {n}").as_str()))
        .unwrap_or(1)
}

impl Session {
    /// How many laser operations link each material id.
    fn material_use_counts(&self) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::<String, usize>::new();
        for op in self.laser_operations() {
            if let Some(id) = op.material_id() {
                *counts.entry(id.to_owned()).or_default() += 1;
            }
        }
        counts
    }

    /// The Materials section of the active laser page, `None` on another page.
    #[must_use]
    pub fn materials_view(&self) -> Option<MaterialsView> {
        if !self.operations_available() {
            return None;
        }
        let copy = self.active_copy()?;
        let profile_id = copy.read.copy.id.clone().unwrap_or_default();
        let library = self.materials.library();
        let counts = self.material_use_counts();
        let uses = |id: &str| counts.get(id).copied().unwrap_or(0);
        let rows = material_rows(library, &profile_id, &uses);
        let selected = self
            .materials
            .selected
            .as_deref()
            .and_then(|id| library.find(id))
            .filter(|material| material.machine.id == profile_id)
            .map(|material| selected_fields(material, uses(&material.id)));
        let next = next_material_number(library, &profile_id);
        Some(MaterialsView {
            profile_name: copy.read.copy.values.name,
            rows,
            selected,
            next_name: format!("Material {next}"),
            load_notices: library.load_notices().to_vec(),
        })
    }

    /// The Materials section as JSON, `null` on another page.
    #[must_use]
    pub fn materials_view_json(&self) -> String {
        serde_json::to_string(&self.materials_view()).unwrap_or_else(|_| "null".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::date_text;

    #[test]
    fn dates_are_civil_utc_days() {
        assert_eq!(date_text(0), "1970-01-01");
        assert_eq!(date_text(1_762_862_400_000), "2025-11-11");
        assert_eq!(date_text(1_709_164_800_000), "2024-02-29");
    }
}

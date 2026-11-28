//! The Machines tab's view (ADR 0098 decision 4, UX unify 4.2 to 4.4): one section per enabled
//! family with its machines, built-ins first, each row's line 2 and body fields, and the inline
//! offer to update the pages after an edit.

mod block;
mod edit;
#[cfg(test)]
mod tests;

use serde::Serialize;

use super::Session;

pub(in crate::session) use edit::FollowOffer;

/// A machine family as the Machines tab sorts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum Family {
    /// Laser cutters and engravers.
    Laser,
    /// Cutting plotters.
    Plotter,
    /// Embroidery machines.
    Embroidery,
    /// CNC routers.
    Cnc,
}

impl Family {
    /// Every family in tab order.
    pub(in crate::session) const ALL: [Self; 4] =
        [Self::Laser, Self::Plotter, Self::Embroidery, Self::Cnc];

    /// The process id of its pages.
    pub(in crate::session) const fn process(self) -> &'static str {
        match self {
            Self::Laser => "curvyo.laser",
            Self::Plotter => curvyo_cutter_core::PROCESS_ID,
            Self::Embroidery => curvyo_embroidery_core::PROCESS_ID,
            Self::Cnc => curvyo_cnc_core::PROCESS_ID,
        }
    }

    /// The family of a process id.
    pub(in crate::session) fn of_process(process: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.process() == process)
    }

    /// The family of a section key (`laser`, `plotter`, `embroidery`, `cnc`).
    pub(in crate::session) fn of_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| family_key(*f) == key)
    }

    /// The kind word of its sections and chips.
    const fn word(self) -> &'static str {
        match self {
            Self::Laser => "Laser",
            Self::Plotter => "Plotter",
            Self::Embroidery => "Embroidery",
            Self::Cnc => "CNC",
        }
    }
}

/// How a body field is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    /// A number field with its unit.
    Number,
    /// A switch.
    Switch,
    /// The origin corner control.
    Corner,
    /// A plain read-only line.
    Text,
    /// A free text field (the connector address).
    Entry,
}

/// One field of a machine's body, in the order of UX unify 4.3.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MachineFieldView {
    /// The name `set_machine_value` takes.
    pub key: &'static str,
    /// The label inside the field.
    pub label: &'static str,
    /// The value as text ("6000", "on", `bottom_left`, "12 × 12 in, 12 × 24 in").
    pub value: String,
    /// The unit inside the field, empty for none.
    pub unit: &'static str,
    /// How it is drawn; every field of a built-in is [`FieldKind::Text`].
    pub kind: FieldKind,
}

/// One machine row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MachineRowView {
    /// The record id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Line 2: "400 × 300 mm · GRBL".
    pub line2: String,
    /// A built-in: Duplicate only, its body plain lines.
    pub builtin: bool,
    /// The body fields.
    pub fields: Vec<MachineFieldView>,
}

/// The inline question after an edit (Frage 434 C, UX unify 4.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FollowView {
    /// The machine it asks about.
    pub id: String,
    /// "2 pages use Bed laser. Update them too?"
    pub sentence: String,
    /// How many pages Update pages writes.
    pub pages: usize,
}

/// One family's section.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MachineSectionView {
    /// `laser`, `plotter`, `embroidery`, `cnc`.
    pub family: &'static str,
    /// "Laser".
    pub word: &'static str,
    /// Its machines, built-ins first, then by name.
    pub rows: Vec<MachineRowView>,
    /// Whether it is the active page's family (first and unfolded).
    pub active: bool,
    /// The library notices of this family ("1 machine could not be read. …").
    pub notices: Vec<String>,
}

/// The whole Machines tab.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MachinesView {
    /// The sections, the active page's family first.
    pub sections: Vec<MachineSectionView>,
    /// The offer to update the pages after the last edit, if any.
    pub follow: Option<FollowView>,
}

const fn family_key(family: Family) -> &'static str {
    match family {
        Family::Laser => "laser",
        Family::Plotter => "plotter",
        Family::Embroidery => "embroidery",
        Family::Cnc => "cnc",
    }
}

/// A number without a trailing ".0".
fn number(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{n:.0}")
    } else {
        format!("{n}")
    }
}

impl Session {
    /// The Machines tab of every enabled family, `None` off an output page.
    #[must_use]
    pub fn machines_view(&self) -> Option<MachinesView> {
        let active = self.active_output_page()?;
        let first = Family::of_process(&active.process);
        let mut families: Vec<Family> = Family::ALL
            .into_iter()
            .filter(|f| self.process_enabled(f.process()))
            .collect();
        families.sort_by_key(|f| Some(*f) != first);
        let sections = families
            .into_iter()
            .map(|family| {
                let mut rows = self.machine_rows(family);
                rows.sort_by_key(|r| (!r.builtin, r.name.to_lowercase()));
                MachineSectionView {
                    family: family_key(family),
                    word: family.word(),
                    rows,
                    active: Some(family) == first,
                    notices: if family == Family::Laser {
                        self.manufacturing.machine.profiles.load_notices()
                    } else {
                        Vec::new()
                    },
                }
            })
            .collect();
        Some(MachinesView {
            sections,
            follow: self.follow_view(),
        })
    }

    /// The Machines tab as JSON, `null` off an output page.
    #[must_use]
    pub fn machines_view_json(&self) -> String {
        serde_json::to_string(&self.machines_view()).unwrap_or_else(|_| "null".to_owned())
    }

    /// The rows of `family`'s library, built-ins included.
    fn machine_rows(&self, family: Family) -> Vec<MachineRowView> {
        match family {
            Family::Laser => self
                .manufacturing
                .machine
                .profiles
                .listed()
                .into_iter()
                .map(|p| {
                    let builtin = p.is_builtin();
                    MachineRowView {
                        id: p.id.clone(),
                        name: p.values.name.clone(),
                        line2: format!(
                            "{} × {} mm · {}",
                            number(p.work_area.width.as_mm()),
                            number(p.work_area.height.as_mm()),
                            curvyo_laser_core::job::name(p.values.output_format)
                        ),
                        builtin,
                        fields: edit::laser_fields(p, builtin),
                    }
                })
                .collect(),
            Family::Plotter => self
                .plotter_profiles()
                .iter()
                .map(|p| {
                    let builtin = curvyo_cutter_core::builtin(&p.id).is_some();
                    MachineRowView {
                        id: p.id.clone(),
                        name: p.values.name.clone(),
                        line2: format!("{} · {}", p.values.name, p.values.output_format.label()),
                        builtin,
                        fields: edit::plotter_fields(&p.values, builtin),
                    }
                })
                .collect(),
            Family::Embroidery => self
                .embroidery_profiles()
                .iter()
                .map(|p| {
                    let builtin = curvyo_embroidery_core::builtin(&p.id).is_some();
                    let (w, h) = p.values.default_hoop().field_mm();
                    MachineRowView {
                        id: p.id.clone(),
                        name: p.values.name.clone(),
                        line2: format!("{w} × {h} mm · {}", p.values.output_format.label()),
                        builtin,
                        fields: edit::embroidery_fields(&p.values, builtin),
                    }
                })
                .collect(),
            Family::Cnc => self
                .cnc_profiles()
                .iter()
                .map(|p| {
                    let builtin = curvyo_cnc_core::builtin(&p.id).is_some();
                    MachineRowView {
                        id: p.id.clone(),
                        name: p.values.name.clone(),
                        line2: format!(
                            "{} × {} mm · {}",
                            number(p.values.work_area.width.as_mm()),
                            number(p.values.work_area.height.as_mm()),
                            p.values.output_format.label()
                        ),
                        builtin,
                        fields: edit::cnc_fields(&p.values, builtin),
                    }
                })
                .collect(),
        }
    }
}

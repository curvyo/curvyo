//! The `settings`, `source` and `pick` parts of the Operations tab's JSON (`specs/0028` criteria
//! 24 to 27; `specs/0112` criteria 12, 21; UX notes 6): what one row's Settings block (the
//! unfolded row's body of `specs/0111` criteria 18 to 21) and a derived row's source line show.

use std::collections::HashMap;

use curvyo_document_core::{NodeId, OperationSnapshot};
use curvyo_laser_core::FeedRate;
use curvyo_laser_core::grids::{GridPart, Recipe};
use curvyo_laser_core::library::MaterialRead;
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::{
    Field, Interval, OperationOptions, Role, SetKind, SetOverride, Settings, SettingsRead,
    SettingsState, hatches, main_set, read_options, resolve,
};
use serde::Serialize;

use super::super::Session;
use super::super::materials::{NumbersView, SetKindView};

/// The state of an operation's settings (criterion 26), shown by symbol and text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsStateView {
    /// No settings.
    NoSettings,
    /// Damaged settings.
    Damaged,
    /// Own numbers.
    Own,
    /// Linked, the material found with its set.
    Live,
    /// Linked, the stored numbers stand in.
    Stored,
}

/// The material an operation points to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkView {
    /// Its id.
    pub id: String,
    /// Its current name (live) or the stored one.
    pub name: String,
}

/// A number field of a set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldView {
    /// Power.
    Power,
    /// Speed.
    Speed,
    /// Passes.
    Passes,
}

/// The two sets of an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleView {
    /// The kind's set (the hatch of Engrave + Outline).
    Main,
    /// The outline of Engrave + Outline.
    Outline,
}

/// The numbers a set's fields show; a field is `None` only in the No settings draft.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FieldNumbersView {
    /// Power in percent.
    pub power_pct: Option<f64>,
    /// Speed in mm/min.
    pub speed_mm_min: Option<u64>,
    /// Passes.
    pub passes: Option<u8>,
}

impl From<Settings> for FieldNumbersView {
    fn from(set: Settings) -> Self {
        Self {
            power_pct: Some(set.power.percent()),
            speed_mm_min: Some(u64::from(set.speed.get())),
            passes: Some(set.passes.get()),
        }
    }
}

impl From<SetOverride> for FieldNumbersView {
    fn from(draft: SetOverride) -> Self {
        Self {
            power_pct: draft
                .power
                .map(curvyo_laser_core::settings::PowerPercent::percent),
            speed_mm_min: draft.speed.map(|s| u64::from(s.get())),
            passes: draft.passes.map(curvyo_laser_core::settings::Passes::get),
        }
    }
}

/// One set of the block: the numbers that run, the material's numbers and the typed fields.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SetView {
    /// Which set.
    pub role: RoleView,
    /// What the fields show: the numbers that run.
    pub run: FieldNumbersView,
    /// The material's numbers (live or stored) of a linked operation, shown beside each typed
    /// number ("Material 60").
    pub material: Option<NumbersView>,
    /// The fields typed over the material.
    pub typed: Vec<FieldView>,
}

/// A material the pick shows disabled for this operation, with the words in place of the symbols.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DisabledPickView {
    /// The material's id.
    pub id: String,
    /// "No Score set".
    pub reason: String,
}

/// One row's Settings block (UX notes §8).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SettingsView {
    /// The state.
    pub state: SettingsStateView,
    /// The material pointed to.
    pub material: Option<LinkView>,
    /// Why the stored numbers stand in (state stored), the line of UX 8.4.
    pub reason: Option<String>,
    /// "Differs from `<name>`" while a number is typed over the material.
    pub differs: Option<String>,
    /// "Speed 7000 is above this machine's 6000 mm/min." (not damage).
    pub over_feed: Option<String>,
    /// The sets: main, and the outline for Engrave + Outline; empty for Damaged settings.
    pub sets: Vec<SetView>,
    /// Whether "Reset to material" shows (a typed number exists).
    pub reset: bool,
    /// The set kind this operation takes from a material.
    pub needs: SetKindView,
    /// The pick's materials this operation cannot take.
    pub disabled: Vec<DisabledPickView>,
    /// The Interval in mm of Engrave and Engrave + Outline (`specs/0111` criterion 19): the one
    /// that runs, or with No settings the draft's (0.10 until typed); `None` otherwise.
    pub interval_mm: Option<f64>,
    /// The operation-only settings of the lower part (`specs/0111` criterion 20); `None` for
    /// Damaged settings.
    pub options: Option<OptionsView>,
    /// Whether Z offset is shown: the machine has a motorised Z axis (`specs/0111` criterion 21).
    pub z_offset_shown: bool,
}

/// The lower part of an unfolded row (`specs/0111` criterion 20).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct OptionsView {
    /// The hatch options, for Engrave and Engrave + Outline only.
    pub hatch: Option<HatchOptionsView>,
    /// The stored Z offset in mm (shown only with `z_offset_shown`, kept either way).
    pub z_offset_mm: f64,
}

/// The four hatch options.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct HatchOptionsView {
    /// Crosshatch.
    pub crosshatch: bool,
    /// Bidirectional fill.
    pub bidirectional: bool,
    /// Scan angle in degrees.
    pub scan_angle_deg: f64,
    /// Overscanning in percent.
    pub overscan_pct: u8,
}

fn options_view(options: OperationOptions, kind: OperationKind) -> OptionsView {
    OptionsView {
        hatch: hatches(kind).then(|| HatchOptionsView {
            crosshatch: options.crosshatch,
            bidirectional: options.bidirectional,
            scan_angle_deg: options.scan_angle.degrees(),
            overscan_pct: options.overscan.percent(),
        }),
        z_offset_mm: options.z_offset.mm(),
    }
}

/// A material of the pick.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PickView {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its thickness in mm.
    pub thickness_mm: f64,
    /// The kinds it has sets for, in the order Cut, Score, Engrave.
    pub kinds: Vec<SetKindView>,
}

/// The source line of a grid's derived row (UX 6): which grid and cell it runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceView {
    /// The grid's group.
    pub grid: NodeId,
    /// Whether it is a cell row (it has no material pick, G10), else the labels row.
    pub cell: bool,
    /// "Birch 3 mm · K7Q2M column 3, row 2", "K7Q2M labels" without a material.
    pub text: String,
}

fn field_view(field: Field) -> FieldView {
    match field {
        Field::Power => FieldView::Power,
        Field::Speed => FieldView::Speed,
        Field::Passes => FieldView::Passes,
    }
}

fn role_view(role: Role) -> RoleView {
    match role {
        Role::Main => RoleView::Main,
        Role::Outline => RoleView::Outline,
    }
}

fn roles(kind: OperationKind) -> &'static [Role] {
    if kind == OperationKind::EngraveOutline {
        &[Role::Main, Role::Outline]
    } else {
        &[Role::Main]
    }
}

/// The pick: the readable materials in the tab's order, each with the kinds it has sets for on
/// the page's laser (ADR 0058 §7).
pub(super) fn pick_views(session: &Session) -> Vec<PickView> {
    let profile = session.active_profile_id().unwrap_or_default();
    session
        .materials()
        .listed()
        .into_iter()
        .filter_map(|read| match read {
            MaterialRead::Loaded(material) => Some(PickView {
                id: material.id.clone(),
                name: material.name.clone(),
                thickness_mm: material.thickness.mm(),
                kinds: [SetKind::Cut, SetKind::Score, SetKind::Engrave]
                    .into_iter()
                    .filter(|&kind| material.set_for(&profile, kind).is_some())
                    .map(SetKindView::from)
                    .collect(),
            }),
            _ => None,
        })
        .collect()
}

/// The picks whose material has no set for the row's main kind `needs`.
fn disabled_picks(pick: &[PickView], needs: SetKind) -> Vec<DisabledPickView> {
    let needs_view = SetKindView::from(needs);
    pick.iter()
        .filter(|material| !material.kinds.contains(&needs_view))
        .map(|material| DisabledPickView {
            id: material.id.clone(),
            reason: format!("No {} set", needs.label()),
        })
        .collect()
}

/// The set blocks of the row: none when damaged, the draft for no settings, else one per role.
fn set_views(
    state: &SettingsState,
    draft: Option<SetOverride>,
    kind: OperationKind,
    overridden: &[(Role, Field)],
) -> Vec<SetView> {
    match (state, draft) {
        (SettingsState::Damaged(_), _) => Vec::new(),
        (SettingsState::NoSettings, draft) => vec![SetView {
            role: RoleView::Main,
            run: draft.unwrap_or_default().into(),
            material: None,
            typed: Vec::new(),
        }],
        _ => roles(kind)
            .iter()
            .filter_map(|&role| {
                let run = state.run()?.of(role)?;
                Some(SetView {
                    role: role_view(role),
                    run: run.into(),
                    material: state.material().and_then(|m| m.of(role)).map(Into::into),
                    typed: overridden
                        .iter()
                        .filter(|(r, _)| *r == role)
                        .map(|(_, field)| field_view(*field))
                        .collect(),
                })
            })
            .collect(),
    }
}

/// The warning when a set's speed is above the machine's `max_feed`.
fn over_feed_text(state: &SettingsState, max_feed: FeedRate) -> Option<String> {
    state
        .run()
        .and_then(|run| {
            std::iter::once(run.main)
                .chain(run.outline)
                .find(|set| set.speed.get() > max_feed.get())
        })
        .map(|set| {
            format!(
                "Speed {} is above this machine's {} mm/min.",
                set.speed.get(),
                max_feed.get()
            )
        })
}

/// The Interval field of a hatching row: the one that runs, or the draft's with No settings.
fn interval_view(
    session: &Session,
    id: NodeId,
    state: &SettingsState,
    kind: OperationKind,
) -> Option<f64> {
    let interval = match state {
        SettingsState::NoSettings => {
            hatches(kind).then(|| session.draft_interval(id).unwrap_or(Interval::DEFAULT))
        }
        _ => state.run().and_then(|run| run.interval),
    };
    interval.map(Interval::mm)
}

/// What every row of one view reads of the page's profile copy, read once per view (`specs/0112`
/// criterion 32: the copy is parsed once, not per row).
pub(super) struct PageFacts {
    /// The copy's profile id, empty when it names none.
    pub(super) profile: String,
    /// The machine's maximum feed.
    pub(super) max_feed: FeedRate,
    /// Whether the machine has a motorised Z axis.
    pub(super) motorised_z: bool,
}

impl PageFacts {
    /// The facts of the active laser page.
    pub(super) fn of(session: &Session) -> Self {
        Self {
            profile: session.active_profile_id().unwrap_or_default(),
            max_feed: session.active_max_feed(),
            motorised_z: session.active_motorised_z(),
        }
    }
}

/// The Settings block of `op`, `None` for a grid row and an unknown kind.
pub(super) fn settings_view(
    session: &Session,
    op: &OperationSnapshot,
    pick: &[PickView],
    page: &PageFacts,
) -> Option<SettingsView> {
    if op.grid {
        return None;
    }
    let kind = OperationKind::parse(op.kind.as_deref()?)?;
    let read = super::settings::operation_read(op, kind);
    let options = read_options(op.options.as_ref(), kind).ok();
    let state = resolve(&read, kind, session.materials().reads(), &page.profile);
    let needs = main_set(kind);
    let needs_view = SetKindView::from(needs);
    let disabled = disabled_picks(pick, needs);
    let link = match &read {
        SettingsRead::Stored(stored) => stored.material.as_ref(),
        SettingsRead::None | SettingsRead::Damaged(_) => None,
    };
    let (state_view, reason, name) = match &state {
        SettingsState::NoSettings => (SettingsStateView::NoSettings, None, None),
        SettingsState::Damaged(_) => (SettingsStateView::Damaged, None, None),
        SettingsState::Own(_) => (SettingsStateView::Own, None, None),
        SettingsState::Live { name, .. } => (SettingsStateView::Live, None, Some(name.clone())),
        SettingsState::Stored { name, reason, .. } => (
            SettingsStateView::Stored,
            Some(reason.text()),
            Some(name.clone()),
        ),
    };
    let overridden: Vec<(Role, Field)> = match &state {
        SettingsState::Live { overridden, .. } | SettingsState::Stored { overridden, .. } => {
            overridden.clone()
        }
        _ => Vec::new(),
    };
    let sets = set_views(&state, session.settings_draft(op.id), kind, &overridden);
    let over_feed = over_feed_text(&state, page.max_feed);
    Some(SettingsView {
        state: state_view,
        material: link.map(|link| LinkView {
            id: link.id.clone(),
            name: name.unwrap_or_else(|| link.name.clone()),
        }),
        reason,
        differs: state.differs_text(),
        over_feed,
        sets,
        reset: !overridden.is_empty(),
        needs: needs_view,
        disabled,
        interval_mm: interval_view(session, op.id, &state, kind),
        options: options.map(|options| options_view(options, kind)),
        z_offset_shown: page.motorised_z,
    })
}

/// The test id and material name of each grid the derived rows of one view name, read once per
/// grid (`specs/0112` criterion 32: 145 rows of one grid).
pub(super) type SourceCache = HashMap<NodeId, Option<(String, Option<String>)>>;

/// The source line of a derived row, `None` for any other row (UX 6).
pub(super) fn source_view(
    session: &Session,
    op: &OperationSnapshot,
    cache: &mut SourceCache,
) -> Option<SourceView> {
    let (_, part) = GridPart::read(op.grid_part.as_ref()?)?;
    let grid = session.document.test_grid_of_derived_row(op.id)?;
    let (id, material) = cache
        .entry(grid)
        .or_insert_with(|| {
            let recipe = Recipe::from_value(&session.document.test_grid(grid)?.recipe)?;
            let material = recipe.material.map(|m| {
                session
                    .materials()
                    .find(&m.id)
                    .map_or(m.name, |found| found.name.clone())
            });
            Some((recipe.id.to_string(), material))
        })
        .clone()?;
    let what = match part {
        GridPart::Labels => format!("{id} labels"),
        GridPart::Cell { column, row } => format!("{id} column {}, row {}", column + 1, row + 1),
    };
    Some(SourceView {
        grid,
        cell: matches!(part, GridPart::Cell { .. }),
        text: material.map_or_else(|| what.clone(), |name| format!("{name} · {what}")),
    })
}

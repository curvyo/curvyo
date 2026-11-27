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
    Field, Interval, OperationOptions, PickKind, Role, SetKind, Settings, SettingsRead,
    SettingsState, cuts_through, hatches, main_set, read_options, resolve, scans,
};
use serde::Serialize;

use super::super::Session;
use super::super::materials::{NumbersView, SetKindView};
use super::image_view::{ImageSettingsView, RasterFacts, image_settings_view};
use curvyo_laser_core::job::{LeafGeometry, burns_grey};

/// The state of an operation's settings (criterion 26), shown by symbol and text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsStateView {
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

/// The numbers a set's fields show: never empty (`specs/0150` criterion 27).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FieldNumbersView {
    /// Power in percent.
    pub power_pct: f64,
    /// Speed in mm/min.
    pub speed_mm_min: u64,
    /// Passes.
    pub passes: u8,
}

impl From<Settings> for FieldNumbersView {
    fn from(set: Settings) -> Self {
        Self {
            power_pct: set.power.percent(),
            speed_mm_min: u64::from(set.speed.get()),
            passes: set.passes.get(),
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

/// One of the four picks the operation can follow (`specs/0146` criteria 13, 15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PickChoiceView {
    /// The stored name: `best`, `second_best`, `fastest` or `deepest`.
    pub pick: &'static str,
    /// The word.
    pub label: &'static str,
    /// Whether the linked material has this pick's set for the page's laser.
    pub available: bool,
    /// "No Fastest set for this laser" while it has not.
    pub reason: Option<String>,
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
    /// The pick the link follows, `best` while no `pick` is stored.
    pub pick: &'static str,
    /// The four picks in order with their availability; empty while the operation has no link.
    pub picks: Vec<PickChoiceView>,
    /// The set kind this operation takes from a material.
    pub needs: SetKindView,
    /// The pick's materials this operation cannot take.
    pub disabled: Vec<DisabledPickView>,
    /// The Interval in mm of Engrave and Engrave + Outline (`specs/0111` criterion 19): the one
    /// that runs; `None` otherwise.
    pub interval_mm: Option<f64>,
    /// The operation-only settings of the lower part (`specs/0111` criterion 20); `None` for
    /// Damaged settings.
    pub options: Option<OptionsView>,
    /// Whether Z offset is shown: the machine has a motorised Z axis (`specs/0111` criterion 21).
    pub z_offset_shown: bool,
}

/// The lower part of an unfolded row (`specs/0111` criterion 20).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OptionsView {
    /// The hatch options, for Engrave and Engrave + Outline only.
    pub hatch: Option<HatchOptionsView>,
    /// The stored Z offset in mm (shown only with `z_offset_shown`, kept either way).
    pub z_offset_mm: f64,
    /// The Image settings group, only while the Engrave operation holds a raster item
    /// (`specs/0123` criterion 11); the settings stay stored while it is `None`.
    pub image: Option<ImageSettingsView>,
    /// The Tabs group, Cut and Cut-out only (`specs/0139` criteria 1, 2).
    pub tabs: Option<TabsView>,
}

/// The Tabs group of a Cut or Cut-out row (`specs/0139` criteria 1 to 3).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TabsView {
    /// Tabs per path.
    pub count: u8,
    /// The tab width in mm.
    pub width_mm: f64,
    /// Skip inner shapes.
    pub skip_inner: bool,
    /// The muted line under Count: "Placed: 3 per path", only when some path gets fewer.
    pub placed: Option<String>,
}

/// The four hatch options.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct HatchOptionsView {
    /// Crosshatch, Engrave kinds only (`None` on Image, ADR 0069 §13).
    pub crosshatch: Option<bool>,
    /// Bidirectional fill.
    pub bidirectional: bool,
    /// Scan angle in degrees.
    pub scan_angle_deg: f64,
    /// Overscanning in percent.
    pub overscan_pct: u8,
}

fn options_view(
    options: OperationOptions,
    kind: OperationKind,
    facts: RasterFacts,
    placed: Option<String>,
) -> OptionsView {
    OptionsView {
        tabs: cuts_through(kind).then(|| TabsView {
            count: options.cut.tab_count.get(),
            width_mm: options.cut.tab_width.mm(),
            skip_inner: options.cut.tab_skip_inner,
            placed,
        }),
        image: (kind == OperationKind::Image).then(|| image_settings_view(&options.image, facts)),
        hatch: scans(kind).then(|| HatchOptionsView {
            crosshatch: hatches(kind).then_some(options.crosshatch),
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
                kinds: SetKind::ALL
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

/// The set blocks of the row: none when damaged, else one per role.
fn set_views(
    state: &SettingsState,
    kind: OperationKind,
    overridden: &[(Role, Field)],
) -> Vec<SetView> {
    match state {
        SettingsState::Damaged(_) => Vec::new(),
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

/// The Interval field of a hatching row: the one that runs.
fn interval_view(state: &SettingsState) -> Option<f64> {
    state.run().and_then(|run| run.interval).map(Interval::mm)
}

/// What every row of one view reads of the page's profile copy, read once per view (`specs/0112`
/// criterion 32: the copy is parsed once, not per row).
pub(super) struct PageFacts {
    /// The copy's profile id, empty when it names none.
    pub(super) profile: String,
    /// The machine's maximum feed.
    pub(super) max_feed: FeedRate,
    /// The copy's default pair, what an operation without `op_settings` reads (`specs/0150`).
    pub(super) defaults: curvyo_laser_core::DefaultNumbers,
    /// Whether the machine has a motorised Z axis.
    pub(super) motorised_z: bool,
    /// What each Image operation's items need of its group, by operation.
    pub(super) raster: HashMap<NodeId, RasterFacts>,
    /// The placed line of each Cut or Cut-out whose paths get fewer tabs than Count
    /// (`specs/0139` criterion 2), by operation.
    pub(super) placed: HashMap<NodeId, String>,
}

impl PageFacts {
    /// The facts of the active laser page.
    pub(super) fn of(session: &Session) -> Self {
        Self {
            profile: session.active_profile_id().unwrap_or_default(),
            max_feed: session.active_max_feed(),
            defaults: session.active_defaults(),
            motorised_z: session.active_motorised_z(),
            raster: session.raster_facts(),
            placed: session.placed_lines(),
        }
    }
}

impl Session {
    /// What each Image operation of the active laser page needs of its group: whether it holds a
    /// gradient-filled leaf, whether one of its items burns Grey (a picture with its own dither,
    /// ADR 0072 amendment A4).
    fn raster_facts(&self) -> HashMap<NodeId, RasterFacts> {
        self.with_page_job_input(|page| {
            page.blocks
                .iter()
                .filter_map(|block| {
                    let curvyo_laser_core::job::BlockSource::Operation {
                        options: Ok(options),
                        ..
                    } = &block.source
                    else {
                        return None;
                    };
                    let facts = RasterFacts {
                        gradient: block
                            .leaves
                            .iter()
                            .any(|l| matches!(l.geometry, LeafGeometry::Gradient { .. })),
                        grey: burns_grey(&options.image, &block.leaves),
                    };
                    Some((block.id, facts))
                })
                .collect()
        })
        .unwrap_or_default()
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
    let read = super::settings::operation_read(op, kind, page.defaults);
    let options = read_options(op.options.as_ref(), kind).ok();
    let state = resolve(&read, kind, session.materials().reads(), &page.profile);
    let needs = main_set(kind);
    let needs_view = SetKindView::from(needs);
    let disabled = disabled_picks(pick, needs);
    let link = match &read {
        SettingsRead::Stored(stored) => stored.material.as_ref(),
        SettingsRead::Damaged(_) => None,
    };
    let (state_view, reason, name) = match &state {
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
    let picks = link.map_or_else(Vec::new, |link| {
        pick_choices(session, link.id.as_str(), kind, page)
    });
    let sets = set_views(&state, kind, &overridden);
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
        pick: link.map_or(PickKind::Best, |link| link.pick).as_str(),
        picks,
        needs: needs_view,
        disabled,
        interval_mm: interval_view(&state),
        options: options.map(|options| {
            let facts = page.raster.get(&op.id).copied().unwrap_or_default();
            options_view(options, kind, facts, page.placed.get(&op.id).cloned())
        }),
        z_offset_shown: page.motorised_z,
    })
}

/// The four picks of the material `id` for an operation of kind `kind` on the page's laser: each
/// available when the material holds that pick's main set (`specs/0146` criterion 15).
fn pick_choices(
    session: &Session,
    id: &str,
    kind: OperationKind,
    page: &PageFacts,
) -> Vec<PickChoiceView> {
    let material = session.materials().find(id);
    PickKind::ALL
        .into_iter()
        .map(|pick| {
            let available = material
                .and_then(|m| m.pick_set(&page.profile, main_set(kind), pick))
                .is_some();
            PickChoiceView {
                pick: pick.as_str(),
                label: pick.label(),
                available,
                reason: (!available).then(|| format!("No {} set for this laser", pick.label())),
            }
        })
        .collect()
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

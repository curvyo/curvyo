//! The numbers each block of the job runs with (ADR 0064 §3): an operation's effective numbers by
//! the settings rule, a fixture block's by the page's material pick.

use crate::job_input::{BlockInput, BlockSource};
use crate::machine_profile::FeedRate;
use crate::material::{Interval, Passes, PowerPercent, SetKind, Settings};
use crate::material_record::MaterialRead;
use crate::operation_kind::OperationKind;
use crate::operation_options::{OperationOptions, Overscan};
use crate::operation_settings::{Field, Role};
use crate::operation_settings_rule::{SettingsState, StoredReason, main_set, resolve};

/// The numbers of one set as they run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunNumbers {
    /// Power in percent of the machine's maximum.
    pub power: PowerPercent,
    /// The speed as set (shown in the gate).
    pub speed: FeedRate,
    /// The written feed: the speed, at most the machine's maximum feed (criterion 21).
    pub feed: FeedRate,
    /// Passes.
    pub passes: Passes,
}

impl RunNumbers {
    fn of(settings: Settings, max_feed: FeedRate) -> Self {
        Self {
            power: settings.power,
            speed: settings.speed,
            feed: if settings.speed.get() > max_feed.get() {
                max_feed
            } else {
                settings.speed
            },
            passes: settings.passes,
        }
    }

    /// Whether the speed lies above the machine's maximum feed.
    #[must_use]
    pub fn over_feed(&self) -> bool {
        self.speed != self.feed
    }
}

/// The numbers a block runs with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockNumbers {
    /// The kind.
    pub kind: OperationKind,
    /// The kind's set.
    pub main: RunNumbers,
    /// The outline set of Engrave + Outline.
    pub outline: Option<RunNumbers>,
    /// The hatch interval of the Engrave kinds.
    pub interval: Interval,
    /// The Interval a picture's preset is derived at (`specs/0135` criterion 60): the
    /// operation's own, also while a coarse preview reads the dots at a larger `interval`.
    pub picture_interval: Interval,
    /// The operation-only settings (defaults for fixture blocks).
    pub options: OperationOptions,
}

impl BlockNumbers {
    /// The numbers of `role`; the outline falls back to the main set.
    #[must_use]
    pub fn of(&self, role: Role) -> RunNumbers {
        match role {
            Role::Main => self.main,
            Role::Outline => self.outline.unwrap_or(self.main),
        }
    }
}

/// Where the numbers of a running block come from, for the gate's source words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumbersSource {
    /// The operation's own numbers.
    Own,
    /// A live material, with the typed fields.
    Live {
        /// The material name.
        material: String,
        /// The typed fields per role.
        typed: Vec<(Role, Field)>,
    },
    /// The stored numbers of a material link, with the reason and the typed fields.
    Stored {
        /// The name stored with the link.
        material: String,
        /// Why the stored numbers run.
        reason: StoredReason,
        /// The typed fields per role.
        typed: Vec<(Role, Field)>,
    },
    /// The page's fixture material pick (always live).
    Fixture {
        /// The material name.
        material: String,
    },
}

/// The numbers of one block as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumbersRead {
    /// The block runs.
    Runs {
        /// Its numbers.
        numbers: BlockNumbers,
        /// Where they come from.
        source: NumbersSource,
    },
    /// An operation with no settings: blocks (4 a).
    NoSettings,
    /// A fixture block with no material pick, or a pick not loaded here: blocks (4 a).
    FixtureNoMaterial,
    /// A fixture block whose material has no set of its kind for this laser: blocks (4 a).
    FixtureNoSet {
        /// The material name.
        material: String,
        /// The set kind it lacks.
        kind: SetKind,
    },
    /// An Unknown kind, damaged settings or damaged options: runs nothing, warns (criterion 33).
    Damaged,
}

impl NumbersRead {
    /// The numbers when the block runs.
    #[must_use]
    pub const fn numbers(&self) -> Option<&BlockNumbers> {
        match self {
            Self::Runs { numbers, .. } => Some(numbers),
            _ => None,
        }
    }

    /// Whether the block blocks Export (4 a).
    #[must_use]
    pub const fn blocks(&self) -> bool {
        matches!(
            self,
            Self::NoSettings | Self::FixtureNoMaterial | Self::FixtureNoSet { .. }
        )
    }
}

/// What a block's numbers depend on besides the block.
#[derive(Debug, Clone, Copy)]
pub struct NumbersContext<'a> {
    /// The loaded material library.
    pub materials: &'a [MaterialRead],
    /// The page's profile id.
    pub profile_id: &'a str,
    /// The page's fixture material pick.
    pub fixture_material: Option<&'a str>,
    /// The machine's maximum feed.
    pub max_feed: FeedRate,
}

/// The numbers `block` runs with.
#[must_use]
pub fn block_numbers(block: &BlockInput, context: NumbersContext<'_>) -> NumbersRead {
    let Some(kind) = block.kind.known() else {
        return NumbersRead::Damaged;
    };
    match &block.source {
        BlockSource::Operation {
            settings, options, ..
        } => {
            let Ok(options) = options else {
                return NumbersRead::Damaged;
            };
            let state = resolve(settings, kind, context.materials, context.profile_id);
            operation_numbers(kind, *options, &state, context.max_feed)
        }
        BlockSource::Fixture { .. } => fixture_numbers(kind, context),
    }
}

fn operation_numbers(
    kind: OperationKind,
    options: OperationOptions,
    state: &SettingsState,
    max_feed: FeedRate,
) -> NumbersRead {
    let source = match state {
        SettingsState::NoSettings => return NumbersRead::NoSettings,
        SettingsState::Damaged(_) => return NumbersRead::Damaged,
        SettingsState::Own(_) => NumbersSource::Own,
        SettingsState::Live {
            name, overridden, ..
        } => NumbersSource::Live {
            material: name.clone(),
            typed: overridden.clone(),
        },
        SettingsState::Stored {
            name,
            reason,
            overridden,
            ..
        } => NumbersSource::Stored {
            material: name.clone(),
            reason: *reason,
            typed: overridden.clone(),
        },
    };
    let Some(run) = state.run() else {
        return NumbersRead::NoSettings;
    };
    NumbersRead::Runs {
        numbers: BlockNumbers {
            kind,
            main: RunNumbers::of(run.main, max_feed),
            outline: (kind == OperationKind::EngraveOutline)
                .then(|| RunNumbers::of(run.outline.unwrap_or(run.main), max_feed)),
            interval: run.interval.unwrap_or_default(),
            picture_interval: run.interval.unwrap_or_default(),
            options,
        },
        source,
    }
}

fn fixture_numbers(kind: OperationKind, context: NumbersContext<'_>) -> NumbersRead {
    let material = context.fixture_material.and_then(|id| {
        context.materials.iter().find_map(|read| match read {
            MaterialRead::Loaded(material) if material.id == id => Some(material),
            _ => None,
        })
    });
    let Some(material) = material else {
        return NumbersRead::FixtureNoMaterial;
    };
    let set_kind = main_set(kind);
    let Some(set) = material.set_for(context.profile_id, set_kind) else {
        return NumbersRead::FixtureNoSet {
            material: material.name.clone(),
            kind: set_kind,
        };
    };
    let main = RunNumbers::of(set.numbers.settings, context.max_feed);
    NumbersRead::Runs {
        numbers: BlockNumbers {
            kind,
            main,
            outline: (kind == OperationKind::EngraveOutline).then_some(main),
            interval: set.numbers.interval.unwrap_or_default(),
            picture_interval: set.numbers.interval.unwrap_or_default(),
            options: fixture_options(),
        },
        source: NumbersSource::Fixture {
            material: material.name.clone(),
        },
    }
}

/// The options of a fixture block (ADR 0064 §3): bidirectional, 0 degrees, no Crosshatch, no
/// Overscanning, no Z offset.
fn fixture_options() -> OperationOptions {
    OperationOptions {
        overscan: Overscan::new(0).unwrap_or(Overscan::DEFAULT),
        ..OperationOptions::default()
    }
}

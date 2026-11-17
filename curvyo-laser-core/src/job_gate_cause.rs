//! The causes of the gate's checks (ADR 0064 §8, amended 2026-11-17): text-free, in the display
//! order of criteria 4 and 5.

use curvyo_document_core::NodeId;

use crate::material::SetKind;
use crate::operation_settings::Field;

/// Whether a cause disables Export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Disables Export (`OctagonX`).
    Block,
    /// Shown, Export stays enabled (`TriangleAlert`).
    Warn,
}

/// One cause of the gate, text-free: ids, kinds and numbers, never a sentence. Material names are
/// the settings rule's (the live record's, or the one stored with the link). The variant order is
/// the display order of criteria 4 and 5: blocks first, then warns, damaged settings (33) last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cause {
    /// 4 a: an operation that runs has no settings.
    NoSettings {
        /// The operation.
        op: NodeId,
    },
    /// 4 a: running fixture jobs and no (loaded) material pick.
    FixtureNoMaterial,
    /// 4 a: the fixture material has no set of a kind for this laser.
    FixtureNoSet {
        /// The material name.
        material: String,
        /// The set kind.
        kind: SetKind,
    },
    /// 4 b: burn geometry outside the bed.
    OutsideBed {
        /// The bed's width in µm.
        width_um: i64,
        /// The bed's height in µm.
        height_um: i64,
    },
    /// 4 c: the profile copy is damaged or unreadable.
    MachineDamaged,
    /// 4 e: the Start register holds a value this build does not read.
    StartDamaged,
    /// 4 d: nothing runs.
    NothingRuns,
    /// The machine is the built-in Generic laser.
    GenericLaser,
    /// A linked material is not found; the stored numbers run.
    MaterialNotFound {
        /// The operation.
        op: NodeId,
        /// The material name stored with the link.
        material: String,
    },
    /// A linked material has no set of the kind for this laser; the stored numbers run.
    NoSetForLaser {
        /// The operation.
        op: NodeId,
        /// The material name.
        material: String,
        /// The set kind.
        kind: SetKind,
    },
    /// A typed number runs over a material number.
    TypedOverride {
        /// The operation.
        op: NodeId,
        /// The material name.
        material: String,
        /// The typed fields, in field order, each once.
        fields: Vec<Field>,
    },
    /// A speed above the machine's maximum feed runs at the maximum (criterion 21).
    OverFeed {
        /// The operation.
        op: NodeId,
        /// The maximum feed in mm/min.
        feed: u32,
    },
    /// A Z offset is stored and the machine has no motorised Z axis (criterion 19).
    ZWithoutAxis {
        /// The operation.
        op: NodeId,
    },
    /// Items on the page are Not assigned.
    NotAssigned,
    /// Items run while hidden.
    HiddenRuns,
    /// Operations have Output off; `op` when there is one.
    OutputOff {
        /// The one operation.
        op: Option<NodeId>,
    },
    /// Cut-out items gave no closed contour.
    CutOutNoContour {
        /// The operation.
        op: NodeId,
    },
    /// Operations of an Unknown kind or with damaged settings run nothing (criterion 33).
    DamagedSettings,
}

impl Cause {
    /// The display rank: the variant's place in the declaration.
    #[must_use]
    pub const fn rank(&self) -> u8 {
        match self {
            Self::NoSettings { .. } => 0,
            Self::FixtureNoMaterial => 1,
            Self::FixtureNoSet { .. } => 2,
            Self::OutsideBed { .. } => 3,
            Self::MachineDamaged => 4,
            Self::StartDamaged => 5,
            Self::NothingRuns => 6,
            Self::GenericLaser => 7,
            Self::MaterialNotFound { .. } => 8,
            Self::NoSetForLaser { .. } => 9,
            Self::TypedOverride { .. } => 10,
            Self::OverFeed { .. } => 11,
            Self::ZWithoutAxis { .. } => 12,
            Self::NotAssigned => 13,
            Self::HiddenRuns => 14,
            Self::OutputOff { .. } => 15,
            Self::CutOutNoContour { .. } => 16,
            Self::DamagedSettings => 17,
        }
    }

    /// Block or warn.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        if self.rank() <= Self::NothingRuns.rank() {
            Severity::Block
        } else {
            Severity::Warn
        }
    }
}

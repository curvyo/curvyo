//! The five operation kinds of `specs/0101-laser-operations`, as stored (ADR 0046 §1).

/// What one operation does (criterion 2 and "Words").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationKind {
    /// Every outline as a line.
    Cut,
    /// Every outline as a line with surface power, in the Marking section.
    Score,
    /// Closed areas filled.
    Engrave,
    /// Engrave plus every closed outline traced.
    EngraveOutline,
    /// The outer contour of each item.
    CutOut,
}

/// The two sections of the list with Cuts last on (criterion 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// Engrave, Engrave + Outline, Score.
    Marking,
    /// Cut, Cut-out.
    Cutting,
}

/// A stored kind as read: one of the five, or Unknown (criterion 26).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KindRead {
    /// A kind this build knows.
    Known(OperationKind),
    /// An unknown kind text or an unreadable record: runs nothing, counted nowhere.
    Unknown,
}

impl OperationKind {
    /// Every kind, in the order of the toolbar's kind buttons (criterion 1).
    pub const ALL: [Self; 5] = [
        Self::Cut,
        Self::Score,
        Self::Engrave,
        Self::EngraveOutline,
        Self::CutOut,
    ];

    /// The stored text (a published file key, ADR 0046 Consequences).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cut => "cut",
            Self::Score => "score",
            Self::Engrave => "engrave",
            Self::EngraveOutline => "engrave_outline",
            Self::CutOut => "cut_out",
        }
    }

    /// The kind a stored text names, `None` for any other text.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == text)
    }

    /// The word shown in the UI and used in a new name ("Engrave + Outline 1").
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cut => "Cut",
            Self::Score => "Score",
            Self::Engrave => "Engrave",
            Self::EngraveOutline => "Engrave + Outline",
            Self::CutOut => "Cut-out",
        }
    }

    /// The section the kind belongs to.
    #[must_use]
    pub const fn section(self) -> Section {
        match self {
            Self::Cut | Self::CutOut => Section::Cutting,
            Self::Score | Self::Engrave | Self::EngraveOutline => Section::Marking,
        }
    }
}

impl KindRead {
    /// The kind of a stored record's kind text; `None` (an unreadable record) is Unknown.
    #[must_use]
    pub fn of(stored: Option<&str>) -> Self {
        stored
            .and_then(OperationKind::parse)
            .map_or(Self::Unknown, Self::Known)
    }

    /// The known kind, `None` for Unknown.
    #[must_use]
    pub const fn known(self) -> Option<OperationKind> {
        match self {
            Self::Known(kind) => Some(kind),
            Self::Unknown => None,
        }
    }

    /// The section; an Unknown operation keeps its place, so it sits with Marking and is never
    /// moved below the line (ADR 0046 §5).
    #[must_use]
    pub const fn section(self) -> Section {
        match self {
            Self::Known(kind) => kind.section(),
            Self::Unknown => Section::Marking,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_five_texts_read_as_kinds_and_everything_else_as_unknown() {
        for kind in OperationKind::ALL {
            assert_eq!(KindRead::of(Some(kind.as_str())), KindRead::Known(kind));
        }
        for other in ["Cut", "dot_mode", "", "ignore"] {
            assert_eq!(KindRead::of(Some(other)), KindRead::Unknown, "{other}");
        }
        assert_eq!(
            KindRead::of(None),
            KindRead::Unknown,
            "an unreadable record"
        );
    }

    #[test]
    fn cut_and_cut_out_are_cutting_and_the_rest_marking() {
        use OperationKind::{Cut, CutOut, Engrave, EngraveOutline, Score};
        assert_eq!(Cut.section(), Section::Cutting);
        assert_eq!(CutOut.section(), Section::Cutting);
        for kind in [Score, Engrave, EngraveOutline] {
            assert_eq!(kind.section(), Section::Marking);
        }
        assert_eq!(KindRead::Unknown.section(), Section::Marking);
        assert_eq!(EngraveOutline.label(), "Engrave + Outline");
        assert_eq!(CutOut.label(), "Cut-out");
    }
}

//! The state of one click-to-create placement, from its click until it is placed or cancelled
//! (`specs/0052-click-to-create-shapes/`, `adrs.md` decision 1).
//!
//! Ephemeral (ADR 0009 §2): nothing here touches the document. The session computes the live
//! shape with the active tool's `shape_for` and passes it in; the placement shows its values,
//! freezes them on the first edit, and resolves the typed and kept values into the shape to
//! commit.

mod fields;
mod resolve;
mod text;

use curvyo_document_core::{Angle, DisplayUnit, Length, Point, Shape, Tolerance};

use crate::PolyStarSettings;
use crate::conversion::{GEOMETRIC_TOLERANCE_MM, is_circle};
use crate::display_unit_text::parse_length;
use crate::modifiers::Modifiers;
use crate::shape_tool_common::CreatePreview;
use crate::transform_commit::shape_is_sane;
use crate::transform_entry::{InvalidReason, parse_entry_number};
use resolve::{BoxAtA, Direction, placed_ellipse, placed_rect, placed_star_frame};
pub use text::{
    CANCELLED_SENTENCE, PlacementNoun, field_label, field_name, field_suffix, message,
    placed_sentence,
};

/// A size at or below this is "Must be more than 0" (criterion 23).
const SIZE_TOLERANCE: Tolerance = Tolerance::from_mm(GEOMETRIC_TOLERANCE_MM);

/// Which creation tool the placement belongs to: one placement type for all three, no trait
/// and no generic (`adrs.md` decision 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementKind {
    /// The Rectangle tool.
    Rect,
    /// The Ellipse tool.
    Ellipse,
    /// The Polygon/Star tool.
    PolyStar,
}

impl PlacementKind {
    /// The chip's fields in Tab order (criteria 11 and 13).
    #[must_use]
    pub const fn fields(self) -> &'static [PlacementField] {
        match self {
            Self::Rect => &[
                PlacementField::Width,
                PlacementField::Height,
                PlacementField::Angle,
            ],
            Self::Ellipse | Self::PolyStar => &[PlacementField::Size, PlacementField::Angle],
        }
    }
}

/// One field of the placement chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementField {
    /// The rectangle's width.
    Width,
    /// The rectangle's height.
    Height,
    /// The ellipse's radius or the polygon's or star's outer radius, or their diameter.
    Size,
    /// The angle, in degrees, clockwise on screen.
    Angle,
}

/// What a placement commits: the shape and its `rotation` register (`adrs.md` decision 2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// The unturned shape, its frame moved so the turn about A comes out right.
    pub shape: Shape,
    /// The `rotation` register; always 0 for a polygon or star, whose angle is in the frame.
    pub rotation: Angle,
}

/// Why a placement cannot be placed: the first refused field in Tab order (criterion 23).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacementError {
    /// The field that gets focus and the message.
    pub field: PlacementField,
    /// "Enter a number", "Must be more than 0" or "Too large".
    pub reason: InvalidReason,
}

/// What the pointer and the modifiers were at the freeze, and the live shape they gave.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Kept {
    pointer: Point,
    modifiers: Modifiers,
    live: Option<CreatePreview>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    /// No field edited: the pointer and the modifiers decide (criteria 2, 3, 12).
    Following {
        pointer: Point,
        modifiers: Modifiers,
    },
    /// A field was edited: the pointer and the modifiers change nothing (criterion 14).
    Frozen(Kept),
}

/// One click-to-create placement.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapePlacement {
    kind: PlacementKind,
    a: Point,
    phase: Phase,
    /// The text of each edited field, by its position in [`PlacementKind::fields`]; `None` for an
    /// untouched field.
    edited: [Option<String>; 3],
    diameter: bool,
    error: Option<PlacementError>,
    shown: Option<Placed>,
}

impl ShapePlacement {
    /// A placement of `kind` that starts at `a` with the pointer at `a` (criteria 1 and 26).
    #[must_use]
    pub const fn new(kind: PlacementKind, a: Point, modifiers: Modifiers) -> Self {
        Self {
            kind,
            a,
            phase: Phase::Following {
                pointer: a,
                modifiers,
            },
            edited: [None, None, None],
            diameter: false,
            error: None,
            shown: None,
        }
    }

    /// Which tool this placement belongs to.
    #[must_use]
    pub const fn kind(&self) -> PlacementKind {
        self.kind
    }

    /// Point A.
    #[must_use]
    pub const fn a(&self) -> Point {
        self.a
    }

    /// Whether a field was edited (criterion 14).
    #[must_use]
    pub const fn is_frozen(&self) -> bool {
        matches!(self.phase, Phase::Frozen(_))
    }

    /// Whether the size field is in diameter mode (criterion 20).
    #[must_use]
    pub const fn diameter(&self) -> bool {
        self.diameter
    }

    /// The pointer and the modifiers the live shape follows, or `None` once frozen.
    #[must_use]
    pub const fn following(&self) -> Option<(Point, Modifiers)> {
        match self.phase {
            Phase::Following { pointer, modifiers } => Some((pointer, modifiers)),
            Phase::Frozen(_) => None,
        }
    }

    /// The pointer position the chip is placed from: the live pointer while following, the
    /// pointer at the freeze afterwards (criterion 11).
    #[must_use]
    pub const fn pointer(&self) -> Point {
        match self.phase {
            Phase::Following { pointer, .. } => pointer,
            Phase::Frozen(kept) => kept.pointer,
        }
    }

    /// The pointer moved to `pointer` with `modifiers`; nothing once frozen.
    pub const fn follow(&mut self, pointer: Point, modifiers: Modifiers) {
        if let Phase::Following { .. } = self.phase {
            self.phase = Phase::Following { pointer, modifiers };
        }
    }

    /// The modifiers changed with the pointer still (criterion 3); nothing once frozen.
    pub const fn set_modifiers(&mut self, modifiers: Modifiers) {
        if let Phase::Following { pointer, .. } = self.phase {
            self.phase = Phase::Following { pointer, modifiers };
        }
    }

    /// Whether W and H are linked: the rectangle with Ctrl held, or kept at the freeze
    /// (criterion 17).
    #[must_use]
    pub const fn linked(&self) -> bool {
        let modifiers = match self.phase {
            Phase::Following { modifiers, .. } => modifiers,
            Phase::Frozen(kept) => kept.modifiers,
        };
        matches!(self.kind, PlacementKind::Rect) && modifiers.ctrl
    }

    /// The maker edited `field` to `text`. The first edit freezes the placement with the
    /// current pointer, modifiers and `live` shape (criterion 14); with W and H linked, both get
    /// the text (criterion 17). Clears a refusal's message.
    pub fn edit(&mut self, field: PlacementField, text: &str, live: Option<CreatePreview>) {
        let Some(index) = self.index(field) else {
            return;
        };
        if let Phase::Following { pointer, modifiers } = self.phase {
            self.phase = Phase::Frozen(Kept {
                pointer,
                modifiers,
                live,
            });
        }
        self.edited[index] = Some(text.to_owned());
        if self.linked() {
            for other in [PlacementField::Width, PlacementField::Height] {
                if let Some(i) = self.index(other) {
                    self.edited[i] = Some(text.to_owned());
                }
            }
        }
        self.error = None;
    }

    /// Space or the r/⌀ button: switches the size field between radius and diameter
    /// (criterion 20). Does not freeze; nothing in the rectangle chip.
    pub const fn toggle_diameter(&mut self) {
        if !matches!(self.kind, PlacementKind::Rect) {
            self.diameter = !self.diameter;
        }
    }

    /// Records the refusal of an Enter or a second click (criterion 23).
    pub const fn refuse(&mut self, error: PlacementError) {
        self.error = Some(error);
    }

    /// The refusal shown under the fields, until the next edit.
    #[must_use]
    pub const fn error(&self) -> Option<PlacementError> {
        self.error
    }

    /// What Enter or a second click places (criteria 16 to 23): with no field edited the
    /// `live` shape (criterion 21); otherwise the typed and kept values. The polygon or star
    /// takes the bar's `settings` (criterion 25).
    ///
    /// # Errors
    /// The first refused field in Tab order (criterion 23).
    pub fn result(
        &self,
        live: Option<CreatePreview>,
        settings: PolyStarSettings,
        unit: DisplayUnit,
    ) -> Result<Placed, PlacementError> {
        let first = self.kind.fields()[0];
        let Phase::Frozen(kept) = self.phase else {
            return live
                .map(|preview| Placed {
                    shape: preview.shape,
                    rotation: Angle::from_radians(0.0),
                })
                .ok_or(PlacementError {
                    field: first,
                    reason: InvalidReason::NotPositive,
                });
        };
        let placed = self.typed(kept, settings, unit)?;
        if shape_is_sane(&placed.shape) {
            return Ok(placed);
        }
        let field = self
            .kind
            .fields()
            .iter()
            .copied()
            .find(|&f| {
                f != PlacementField::Angle
                    && self.index(f).is_some_and(|i| self.edited[i].is_some())
            })
            .unwrap_or(first);
        Err(PlacementError {
            field,
            reason: InvalidReason::TooLarge,
        })
    }

    /// Brings the preview up to date after any change: the live shape while following, the
    /// result once frozen, or the last valid result while a field is refused (criterion 15).
    pub fn refresh(
        &mut self,
        live: Option<CreatePreview>,
        settings: PolyStarSettings,
        unit: DisplayUnit,
    ) {
        match self.phase {
            Phase::Following { .. } => {
                self.shown = live.map(|preview| Placed {
                    shape: preview.shape,
                    rotation: Angle::from_radians(0.0),
                });
            }
            Phase::Frozen(_) => {
                if let Ok(placed) = self.result(live, settings, unit) {
                    self.shown = Some(placed);
                }
            }
        }
    }

    /// The shape the preview draws, as of the last [`ShapePlacement::refresh`].
    #[must_use]
    pub const fn shown(&self) -> Option<Placed> {
        self.shown
    }

    /// The typed and kept values resolved into a shape.
    fn typed(
        &self,
        kept: Kept,
        settings: PolyStarSettings,
        unit: DisplayUnit,
    ) -> Result<Placed, PlacementError> {
        let kept_shape = kept.live.map(|p| p.shape);
        let angle = self.angle(kept_shape.as_ref())?;
        let at = |width: Length, height: Length| BoxAtA {
            a: self.a,
            width,
            height,
            direction: Direction::of(self.a, kept.pointer),
            centred: kept.modifiers.shift,
        };
        Ok(match self.kind {
            PlacementKind::Rect => {
                let width = self.length(PlacementField::Width, kept_shape.as_ref(), unit)?;
                let height = self.length(PlacementField::Height, kept_shape.as_ref(), unit)?;
                Placed {
                    shape: placed_rect(at(width, height), angle),
                    rotation: angle,
                }
            }
            PlacementKind::Ellipse => {
                let (rx, ry) = if self.is_edited(PlacementField::Size) {
                    let r = self.length(PlacementField::Size, None, unit)?;
                    (r, r)
                } else {
                    match kept_shape {
                        Some(Shape::Ellipse { frame, .. })
                            if positive(frame.rx) && positive(frame.ry) =>
                        {
                            (frame.rx, frame.ry)
                        }
                        _ => return Err(self.not_positive(PlacementField::Size)),
                    }
                };
                let twice = |r: Length| Length::from_mm(2.0 * r.as_mm());
                Placed {
                    shape: placed_ellipse(at(twice(rx), twice(ry)), angle),
                    rotation: angle,
                }
            }
            PlacementKind::PolyStar => {
                let radius = self.length(PlacementField::Size, kept_shape.as_ref(), unit)?;
                Placed {
                    shape: settings.shape(placed_star_frame(self.a, radius, angle)),
                    rotation: Angle::from_radians(0.0),
                }
            }
        })
    }

    /// A length field's value in millimetres, typed (a diameter halved) or kept, refused when
    /// it is not a number or not more than 0.
    fn length(
        &self,
        field: PlacementField,
        kept: Option<&Shape>,
        unit: DisplayUnit,
    ) -> Result<Length, PlacementError> {
        let length = match self.index(field).and_then(|i| self.edited[i].as_ref()) {
            Some(text) => {
                let typed = parse_length(text, unit).ok_or(PlacementError {
                    field,
                    reason: InvalidReason::NotANumber,
                })?;
                if field == PlacementField::Size && self.diameter {
                    Length::from_mm(typed.as_mm() / 2.0)
                } else {
                    typed
                }
            }
            None => self.kept_length(field, kept),
        };
        if positive(length) {
            Ok(length)
        } else {
            Err(self.not_positive(field))
        }
    }

    /// The angle, typed or kept.
    fn angle(&self, kept: Option<&Shape>) -> Result<Angle, PlacementError> {
        let degrees = match self
            .index(PlacementField::Angle)
            .and_then(|i| self.edited[i].as_ref())
        {
            Some(text) => parse_entry_number(text, true).ok_or(PlacementError {
                field: PlacementField::Angle,
                reason: InvalidReason::NotANumber,
            })?,
            None => untouched_degrees(kept),
        };
        Ok(Angle::from_radians(degrees.to_radians()))
    }

    const fn not_positive(&self, field: PlacementField) -> PlacementError {
        let _ = self;
        PlacementError {
            field,
            reason: InvalidReason::NotPositive,
        }
    }

    /// The untouched value of a length field in radius terms, `Length` zero without a shape.
    fn kept_length(&self, field: PlacementField, shape: Option<&Shape>) -> Length {
        let zero = Length::from_mm(0.0);
        match (field, shape) {
            (PlacementField::Width, Some(Shape::Rect { bounds, .. })) => bounds.width,
            (PlacementField::Height, Some(Shape::Rect { bounds, .. })) => bounds.height,
            (PlacementField::Size, Some(Shape::Ellipse { frame, .. })) if is_circle(frame) => {
                frame.rx
            }
            (
                PlacementField::Size,
                Some(Shape::Polygon { frame, .. } | Shape::Star { frame, .. }),
            ) => frame.radius,
            _ => {
                let _ = self;
                zero
            }
        }
    }

    fn kept_or(&self, live: Option<CreatePreview>) -> Option<CreatePreview> {
        match self.phase {
            Phase::Following { .. } => live,
            Phase::Frozen(kept) => kept.live,
        }
    }

    fn is_edited(&self, field: PlacementField) -> bool {
        self.index(field).is_some_and(|i| self.edited[i].is_some())
    }

    fn index(&self, field: PlacementField) -> Option<usize> {
        self.kind.fields().iter().position(|&f| f == field)
    }
}

/// More than the geometric tolerance (criterion 23).
fn positive(length: Length) -> bool {
    length.as_mm() > SIZE_TOLERANCE.as_mm()
}

/// The angle an untouched Angle field shows (criterion 12): 0 for a rectangle or ellipse, the
/// first vertex's angle (after a Ctrl snap) for a polygon or star, in (-180°, 180°].
fn untouched_degrees(shape: Option<&Shape>) -> f64 {
    match shape {
        Some(Shape::Polygon { frame, .. } | Shape::Star { frame, .. }) => {
            frame.angle.normalized().as_radians().to_degrees()
        }
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests;

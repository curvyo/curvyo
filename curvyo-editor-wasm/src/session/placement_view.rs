//! What the placement chip shows, read in one call per interaction
//! (`specs/0052-click-to-create-shapes/` criteria 11, 12, 17, 20, 23, 29; ADR 0001 §5).

use curvyo_document_core::Point;
use curvyo_ui_core::{
    Modifiers, PlacementKind, PlacementNoun, field_label, field_name, field_suffix,
    placement_message,
};

use super::Session;

/// One field of the placement chip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementFieldView {
    /// The visible label: "W", "H", "r", "⌀" or "Angle".
    pub label: &'static str,
    /// The accessible name.
    pub name: &'static str,
    /// The text: the maker's, or the live value.
    pub text: String,
    /// The muted placeholder of an empty ellipse size field, or empty.
    pub placeholder: String,
    /// The unit symbol or "°".
    pub suffix: &'static str,
    /// The text is refused right now (criterion 15).
    pub invalid: bool,
    /// The size field, whose label is the r/⌀ button (criterion 20).
    pub size: bool,
}

/// One modifier check of the chip (`0052` criterion 32).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifierCheckView {
    /// The visible label with its key: "From centre (Shift)".
    pub label: &'static str,
    /// Shift, not Ctrl.
    pub shift: bool,
    /// The modifier is in effect now, from a tap, a held key or both inverted.
    pub on: bool,
}

/// The placement chip as a whole.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementChipView {
    /// "New rectangle", "New ellipse", "New polygon" or "New star".
    pub group_name: String,
    /// W and H are linked: the chain glyph shows (criterion 17).
    pub linked: bool,
    /// A field was edited: the chip stands still (criterion 14).
    pub frozen: bool,
    /// The modifier checks: from the centre (Shift) and square (rectangle) or circle (ellipse) (Ctrl)
    /// or ellipse, the angle snap (Ctrl) for a polygon or star. None once frozen.
    pub checks: Vec<ModifierCheckView>,
    /// The fields in Tab order.
    pub fields: Vec<PlacementFieldView>,
    /// The refused field's Tab position and its message, after an Enter or a click.
    pub error: Option<(usize, &'static str)>,
    /// Point A, in document space.
    pub a: Point,
    /// The pointer the chip is placed from, in document space.
    pub pointer: Point,
}

impl Session {
    /// The chip of the running placement, or `None`.
    #[must_use]
    pub fn placement_chip(&self) -> Option<PlacementChipView> {
        let placement = self.placement()?;
        let kind = placement.kind();
        let unit = self.display_unit();
        let live = self.placement_live();
        let error = placement.error().and_then(|error| {
            let index = kind.fields().iter().position(|&f| f == error.field)?;
            Some((index, placement_message(error.reason)))
        });
        let fields = kind
            .fields()
            .iter()
            .map(|&field| PlacementFieldView {
                label: field_label(field, placement.diameter()),
                name: field_name(kind, field, placement.diameter()),
                text: placement.field_text(field, live, unit),
                placeholder: placement.placeholder(field, live, unit).unwrap_or_default(),
                suffix: field_suffix(field, unit),
                invalid: placement.field_problem(field, unit).is_some(),
                size: field == curvyo_ui_core::PlacementField::Size,
            })
            .collect();
        let effective = placement
            .following()
            .map_or_else(|| self.placement_effective(Modifiers::NONE), |(_, m)| m);
        let checks = if placement.is_frozen() {
            Vec::new()
        } else if matches!(kind, PlacementKind::PolyStar) {
            vec![ModifierCheckView {
                label: "Snap angle (Ctrl)",
                shift: false,
                on: effective.ctrl,
            }]
        } else {
            vec![
                ModifierCheckView {
                    label: "From centre (Shift)",
                    shift: true,
                    on: effective.shift,
                },
                ModifierCheckView {
                    label: if matches!(kind, PlacementKind::Rect) {
                        "Square (Ctrl)"
                    } else {
                        "Circle (Ctrl)"
                    },
                    shift: false,
                    on: effective.ctrl,
                },
            ]
        };
        Some(PlacementChipView {
            checks,
            group_name: PlacementNoun::of(kind, self.poly_star.mode()).group_name(),
            linked: placement.linked(),
            frozen: placement.is_frozen(),
            fields,
            error,
            a: placement.a(),
            pointer: placement.pointer(),
        })
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::Point;

    use super::super::{Session, Tool};

    /// Criteria 11, 12, 17, 23, 29: the rectangle chip.
    #[test]
    fn the_rectangle_chip_shows_labels_values_link_hint_and_refusal() {
        let mut s = Session::new(1);
        s.viewport = curvyo_ui_core::Viewport::new();
        s.set_tool(Tool::Rectangle);
        assert_eq!(s.placement_chip(), None);
        let a = Point::new(100.0, 50.0);
        s.pointer_down(a, false);
        s.pointer_up(a, false, false);
        s.modifiers_changed(false, true, false);
        s.pointer_hover(Point::new(130.0, 40.0), false, true);
        let chip = s.placement_chip().expect("a chip");
        assert_eq!(chip.group_name, "New rectangle");
        let labels: Vec<_> = chip.fields.iter().map(|f| f.label).collect();
        assert_eq!(labels, ["W", "H", "Angle"]);
        assert_eq!(chip.fields[0].text, "30.0");
        assert_eq!(chip.fields[1].text, "30.0");
        assert_eq!(chip.fields[0].suffix, "mm");
        assert!(chip.linked);
        s.placement_edit(0, "abc");
        assert!(!s.place_placement());
        let chip = s.placement_chip().expect("a chip");
        assert!(chip.frozen && chip.fields[0].invalid && chip.fields[1].invalid);
        assert_eq!(chip.error, Some((0, "Enter a number")));
        assert_eq!(chip.pointer, Point::new(130.0, 40.0));
    }

    /// Criteria 12, 20: the ellipse chip's placeholder and the r/⌀ label.
    #[test]
    fn the_ellipse_chip_shows_a_placeholder_and_the_diameter_label() {
        let mut s = Session::new(1);
        s.viewport = curvyo_ui_core::Viewport::new();
        s.set_tool(Tool::Ellipse);
        let a = Point::new(50.0, 50.0);
        s.pointer_down(a, false);
        s.pointer_up(a, false, false);
        s.pointer_hover(Point::new(80.0, 60.0), false, false);
        let chip = s.placement_chip().expect("a chip");
        assert_eq!(chip.fields[0].text, "");
        assert_eq!(chip.fields[0].placeholder, "15.0 \u{d7} 5.0");
        assert!(chip.fields[0].size);
        s.placement_toggle_diameter();
        let chip = s.placement_chip().expect("a chip");
        assert_eq!(
            (chip.fields[0].label, chip.fields[0].name),
            ("\u{2300}", "Diameter")
        );
    }
}

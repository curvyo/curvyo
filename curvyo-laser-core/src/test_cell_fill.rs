//! What the cells of a test grid are drawn with (`specs/0123` criteria 44, 45; ADR 0069 §8,
//! §13): an outline for Cut and Score, a filled square for Engrave, a black-to-white Gradient
//! rectangle for Image.

use curvyo_document_core::{Color, FillPaint, GradientFill, Style};

use crate::material::SetKind;
use crate::test_recipe::TestRecipe;

/// The gradient of an Image cell (ADR 0066 default linear, `#000000FF` to `#FFFFFFFF`, so the
/// burn ramps from full at the left to none at the right).
#[must_use]
pub fn cell_gradient() -> GradientFill {
    GradientFill::first_for(Color::BLACK, 255)
}

/// The style of a cell of `test`: an outline for Cut and Score, a filled square for Engrave, a
/// square painted with [`cell_gradient`] for Image.
#[must_use]
pub fn cell_style(test: &TestRecipe) -> Style {
    let kind = test.kind();
    let filled = matches!(kind, SetKind::Engrave | SetKind::Image);
    let mut style = Style::default();
    style.fill.enabled = filled;
    style.stroke.enabled = !filled;
    if kind == SetKind::Image {
        style.fill.paint = FillPaint::Gradient;
        style.fill.gradient = Some(cell_gradient());
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine_profile::FeedRate;
    use crate::test_recipe::RecipeDefaults;

    fn defaults() -> RecipeDefaults {
        RecipeDefaults {
            max_feed: FeedRate::new(6000).unwrap(),
            set: None,
        }
    }

    #[test]
    fn an_image_cell_paints_black_to_white_and_an_engrave_cell_its_colour() {
        let recipe = TestRecipe::new(&defaults());
        let solid = cell_style(&recipe.with_kind(SetKind::Engrave, &defaults()));
        assert!(solid.fill.enabled && !solid.stroke.enabled);
        assert_eq!(solid.fill.painted_gradient(), None);
        let image = cell_style(&recipe.with_kind(SetKind::Image, &defaults()));
        assert!(image.fill.enabled && !image.stroke.enabled);
        let painted = image.fill.painted_gradient().unwrap();
        assert_eq!(painted, &cell_gradient());
        let stops = painted.stops.as_slice();
        assert_eq!((stops[0].colour, stops[0].alpha), (Color::BLACK, 255));
        assert_eq!((stops[1].colour, stops[1].alpha), (Color::WHITE, 255));
        let cut = cell_style(&recipe);
        assert!(!cut.fill.enabled && cut.stroke.enabled);
    }
}

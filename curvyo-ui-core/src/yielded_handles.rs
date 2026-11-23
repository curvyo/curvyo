//! The set of a box's own transform handles that yield to a gradient handle drawn near them
//! (`specs/0124-gradient-fill` criterion 6, UX notes §6.3), one bit per handle kind and place.

use crate::ResizeDirection;
use crate::transform_handle_layout::{EditHandle, Side};

/// A set of a box's own handles, by kind and place: the ones a gradient handle covers
/// (`specs/0124-gradient-fill` criterion 6, UX notes §6.3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct YieldedHandles(u32);

/// The place of `d` among the eight directions.
const fn direction_index(d: ResizeDirection) -> u32 {
    match d {
        ResizeDirection::N => 0,
        ResizeDirection::Ne => 1,
        ResizeDirection::E => 2,
        ResizeDirection::Se => 3,
        ResizeDirection::S => 4,
        ResizeDirection::Sw => 5,
        ResizeDirection::W => 6,
        ResizeDirection::Nw => 7,
    }
}

/// The bit of `handle` in a [`YieldedHandles`].
const fn handle_bit(handle: EditHandle) -> u32 {
    use crate::param_handles::ParamHandle;
    use curvyo_document_core::Corner;
    let index = match handle {
        EditHandle::Resize(d) => direction_index(d),
        EditHandle::Rotate(d) => 8 + direction_index(d),
        EditHandle::Skew(side) => match side {
            Side::Top => 16,
            Side::Right => 17,
            Side::Bottom => 18,
            Side::Left => 19,
        },
        EditHandle::Move => 20,
        EditHandle::Param(param) => match param {
            ParamHandle::CornerRadius(Corner::Tl) => 21,
            ParamHandle::CornerRadius(Corner::Tr) => 22,
            ParamHandle::CornerRadius(Corner::Br) => 23,
            ParamHandle::CornerRadius(Corner::Bl) => 24,
            ParamHandle::InnerRadius => 25,
            ParamHandle::ArcStart => 26,
            ParamHandle::ArcEnd => 27,
            ParamHandle::Curve => 28,
            ParamHandle::Rounded => 29,
        },
    };
    1 << index
}

impl YieldedHandles {
    /// No handle yields.
    pub const NONE: Self = Self(0);

    /// This set and `handle`.
    #[must_use]
    pub const fn with(self, handle: EditHandle) -> Self {
        Self(self.0 | handle_bit(handle))
    }

    /// Whether `handle` yields.
    #[must_use]
    pub const fn contains(self, handle: EditHandle) -> bool {
        self.0 & handle_bit(handle) != 0
    }
}

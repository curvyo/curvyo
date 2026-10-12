//! The editor's record of one pointer reading as the webview delivered it
//! (`specs/0032-pen-tablet-input` criterion 1, `adrs.md` "the input record").

use curvyo_document_core::Point;

/// The `buttons` bit a pen sets while its eraser end is in contact or pressed.
pub const ERASER_BUTTON: u32 = 32;

/// Which kind of device a reading comes from (the DOM's `pointerType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerKind {
    /// A mouse, or a `pointerType` the editor does not know.
    Mouse,
    /// A pen or stylus.
    Pen,
    /// A finger on a touch screen.
    Touch,
}

impl PointerKind {
    /// The kind for a DOM `pointerType` string; anything but "pen" and "touch" is a mouse.
    #[must_use]
    pub fn from_dom(pointer_type: &str) -> Self {
        match pointer_type {
            "pen" => Self::Pen,
            "touch" => Self::Touch,
            _ => Self::Mouse,
        }
    }
}

/// One pointer reading. No pointer id, device id or device name is part of it (criterion 16).
/// Tilt and the eraser flag are carried and only shown by the readout (criterion 15).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerSample {
    /// The position in document millimetres.
    pub position: Point,
    /// The device kind.
    pub kind: PointerKind,
    /// The pressure from 0 to 1, exactly as delivered (0.5 for a device without pressure).
    pub pressure: f64,
    /// The tilt along X in degrees, from -90 to 90.
    pub tilt_x_deg: f64,
    /// The tilt along Y in degrees, from -90 to 90.
    pub tilt_y_deg: f64,
    /// Whether the eraser end is in use (`buttons` bit 32).
    pub eraser: bool,
    /// The event time in milliseconds, as the webview delivered it.
    pub time_ms: f64,
}

impl PointerSample {
    /// The record of a reading passed as `[x, y, pressure, tilt_x, tilt_y, time_ms]` (the layout
    /// of one sample in the wasm call), with its `pointerType` and `buttons`. The position is
    /// taken as given; the caller converts it to document millimetres first.
    #[must_use]
    pub fn from_values(pointer_type: &str, buttons: u32, values: [f64; 6]) -> Self {
        Self {
            position: Point::new(values[0], values[1]),
            kind: PointerKind::from_dom(pointer_type),
            pressure: values[2],
            tilt_x_deg: values[3],
            tilt_y_deg: values[4],
            eraser: buttons & ERASER_BUTTON != 0,
            time_ms: values[5],
        }
    }
}

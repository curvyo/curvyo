//! The four circle handles of the open tiling bar (`specs/0069-tiling` criterion 14, UX notes
//! §12.4): picking, dragging and the readout of a drag, the cursor over them, and the entry box a
//! double-click or the keys C, R, A and S open.

use curvyo_document_core::{Length, Point};
use curvyo_ui_core::tiling::{CircleHandle, CircleView};

use super::Session;
use super::keys::KeyInput;
use super::shapes::LiveReadout;
use super::tiling::HandleEntry;

/// The pick distance around each handle's centre, screen pixels (UX notes §12.4).
pub(super) const HANDLE_HIT_PX: f64 = 12.0;
/// The smallest radius a drag gives.
const MIN_RADIUS_MM: f64 = 0.01;

/// One decimal with the real minus sign (the "Live transform readout").
fn one(value: f64) -> String {
    format!("{value:.1}").replace('-', "\u{2212}")
}

/// `degrees` brought into -180° to 180° for a readout.
fn shown_degrees(degrees: f64) -> f64 {
    let wrapped = (degrees + 180.0).rem_euclid(360.0) - 180.0;
    if wrapped <= -180.0 { 180.0 } else { wrapped }
}

/// The handle named by `code` (`"centre"`, `"radius"`, `"start"`, `"sweep"`).
#[must_use]
pub fn circle_handle(code: &str) -> Option<CircleHandle> {
    Some(match code {
        "centre" => CircleHandle::Centre,
        "radius" => CircleHandle::Radius,
        "start" => CircleHandle::Start,
        "sweep" => CircleHandle::Sweep,
        _ => return None,
    })
}

/// The code of `handle`, the inverse of [`circle_handle`].
const fn handle_code(handle: CircleHandle) -> &'static str {
    match handle {
        CircleHandle::Centre => "centre",
        CircleHandle::Radius => "radius",
        CircleHandle::Start => "start",
        CircleHandle::Sweep => "sweep",
    }
}

impl Session {
    /// The readout chip of a handle drag (UX notes §12.4): "Centre 120.0, 80.0 mm", "r 42.0 mm",
    /// "start 30.0° · sweep 120.0°" or "end 150.0° · sweep 120.0°", at the handle.
    pub(super) fn tiling_readout(&self) -> Option<LiveReadout> {
        let drag = self.tiling.as_ref()?.drag?;
        let circle = self.tiling_circle()?;
        let start = circle.start.as_radians().to_degrees();
        let sweep = circle.sweep.as_radians().to_degrees();
        let text = match drag {
            CircleHandle::Centre => {
                format!(
                    "Centre {}, {} mm",
                    one(circle.centre.x),
                    one(circle.centre.y)
                )
            }
            CircleHandle::Radius => format!("r {} mm", one(circle.radius_mm)),
            CircleHandle::Start => format!("start {}° · sweep {}°", one(start), one(sweep)),
            CircleHandle::Sweep => format!(
                "end {}° · sweep {}°",
                one(shown_degrees(start + sweep)),
                one(sweep)
            ),
        };
        Some(LiveReadout {
            text,
            anchor: circle.handle_point(drag),
        })
    }

    /// A press while the bar is open: closes an open entry box as Escape does, then picks a
    /// circle handle; otherwise does nothing (canvas selection is locked, UX notes §2). Returns
    /// whether the bar is open.
    pub(super) fn tiling_press(&mut self, point: Point) -> bool {
        if self.tiling.is_none() {
            return false;
        }
        self.tiling_close_entry(true);
        let drag = self
            .tiling_circle()
            .and_then(|circle| circle.pick(point, HANDLE_HIT_PX));
        if let Some(mode) = self.tiling.as_mut() {
            mode.drag = drag;
        }
        true
    }

    /// A move during a handle drag (criterion 14): the centre follows the snapped pointer, the
    /// radius its distance from the centre, the start and sweep its angle (Ctrl: the 15° and
    /// 22.5° stops); the preview follows.
    pub(super) fn tiling_drag(&mut self, point: Point) -> bool {
        let Some(drag) = self.tiling.as_ref().and_then(|mode| mode.drag) else {
            return self.tiling.is_some();
        };
        let Some(circle) = self.tiling_circle() else {
            return true;
        };
        let ctrl = self.held.ctrl;
        let snapped = match drag {
            CircleHandle::Centre | CircleHandle::Radius => self.snap_creation_point(point),
            CircleHandle::Start | CircleHandle::Sweep => point,
        };
        let Some(mode) = self.tiling.as_ref() else {
            return false;
        };
        let mut recipe = mode.recipe;
        match drag {
            CircleHandle::Centre => {
                recipe.circle.centre = mode.cell_box.centre().vector_to(snapped);
            }
            CircleHandle::Radius => {
                recipe.circle.radius =
                    Length::from_mm(circle.centre.vector_to(snapped).length().max(MIN_RADIUS_MM));
            }
            CircleHandle::Start => recipe.circle.start = circle.dragged_start(point, ctrl),
            CircleHandle::Sweep => recipe.circle.sweep = circle.dragged_sweep(point, ctrl),
        }
        self.tiling_set_recipe(recipe);
        true
    }

    /// The release of a handle drag. Returns whether the bar is open.
    pub(super) fn tiling_release_press(&mut self) -> bool {
        let open = self.tiling.is_some();
        if let Some(mode) = self.tiling.as_mut()
            && mode.drag.take().is_some()
        {
            self.end_snapping_drag();
        }
        open
    }

    /// A double-click while the bar is open: on a circle handle it opens that handle's entry box
    /// (criterion 14). Returns whether the bar is open.
    pub(super) fn tiling_double_click(&mut self, point: Point) -> bool {
        if self.tiling.is_none() {
            return false;
        }
        if let Some(handle) = self
            .tiling_circle()
            .and_then(|circle| circle.pick(point, HANDLE_HIT_PX))
        {
            self.tiling_open_entry(handle);
        }
        true
    }

    /// Opens the entry box of `handle` (UX notes §12.4), remembering the circle values it opens
    /// with; another open box is closed first, keeping its values. False outside a circle tiling.
    pub fn tiling_open_entry(&mut self, handle: CircleHandle) -> bool {
        if self.tiling_circle().is_none() {
            return false;
        }
        self.end_snapping_drag();
        let Some(mode) = self.tiling.as_mut() else {
            return false;
        };
        mode.drag = None;
        mode.entry = Some(HandleEntry {
            handle,
            opened_with: mode.recipe.circle,
        });
        true
    }

    /// The handle whose entry box is open.
    #[must_use]
    pub fn tiling_entry(&self) -> Option<CircleHandle> {
        self.tiling.as_ref()?.entry.map(|entry| entry.handle)
    }

    /// Closes the entry box: Enter keeps the typed values (`restore` false); Escape, a press
    /// elsewhere, a blur restore the values it opened with (UX notes §12.4). Never cancels the
    /// tiling; nothing happens without an open box.
    pub fn tiling_close_entry(&mut self, restore: bool) {
        let Some(mode) = self.tiling.as_mut() else {
            return;
        };
        let Some(entry) = mode.entry.take() else {
            return;
        };
        if restore {
            let mut recipe = mode.recipe;
            let (to, from) = (&mut recipe.circle, entry.opened_with);
            to.centre = from.centre;
            to.radius = from.radius;
            to.start = from.start;
            to.sweep = from.sweep;
            self.tiling_set_recipe(recipe);
        }
    }

    /// The open entry box as JSON for its placement: `handle` (`"centre"`, `"radius"`,
    /// `"start"`, `"sweep"`), the handle `x`, `y` and the circle centre `cx`, `cy` in canvas CSS
    /// pixels; empty while no box is open.
    #[must_use]
    pub fn tiling_entry_json(&self) -> String {
        let (Some(handle), Some(circle)) = (self.tiling_entry(), self.tiling_circle()) else {
            return String::new();
        };
        let view = self.view();
        let (x, y) = view.document_to_screen(circle.handle_point(handle));
        let (cx, cy) = view.document_to_screen(circle.centre);
        serde_json::json!({ "handle": handle_code(handle), "x": x, "y": y, "cx": cx, "cy": cy })
            .to_string()
    }

    /// The keys C, R, A and S on the canvas of an open circle tiling open the entry box of the
    /// Centre, Radius, Start and Sweep handle (UX notes §12.4, keyboard route). Returns whether
    /// the key opened one.
    pub(super) fn tiling_entry_key(&mut self, input: &KeyInput<'_>) -> bool {
        if input.dom_blocked || input.ctrl || input.alt || input.repeat {
            return false;
        }
        let handle = match input.key {
            "c" | "C" => CircleHandle::Centre,
            "r" | "R" => CircleHandle::Radius,
            "a" | "A" => CircleHandle::Start,
            "s" | "S" => CircleHandle::Sweep,
            _ => return false,
        };
        self.tiling_open_entry(handle)
    }

    /// The cursor over a circle handle (UX notes §12.4): `move` on the centre, the resize arrow
    /// nearest the radial direction on the radius, `pointer` on start and sweep; `None` away
    /// from them.
    pub(super) fn tiling_cursor(&self) -> Option<String> {
        let circle = self.tiling_circle()?;
        let mode = self.tiling.as_ref()?;
        let handle = mode
            .drag
            .or_else(|| circle.pick(self.pointer_position?, HANDLE_HIT_PX))?;
        Some(match handle {
            CircleHandle::Centre => "move".to_owned(),
            CircleHandle::Radius => format!("resize:{:.1}", radial_cursor_degrees(&circle)),
            CircleHandle::Start | CircleHandle::Sweep => "pointer".to_owned(),
        })
    }
}

/// The radius handle's arrow: the radial direction, degrees clockwise on screen, 0 to 180.
fn radial_cursor_degrees(circle: &CircleView) -> f64 {
    let at = circle.handle_point(CircleHandle::Radius);
    let d = circle.centre.vector_to(at);
    d.y.atan2(d.x).to_degrees().rem_euclid(180.0)
}

//! The page's user origin in the session (`specs/0118` slice 2c, adrs O6, O7; UX notes W41, W43,
//! W46, W47): Set origin here and the typed pair, and the canvas cross and dashed burn box.

use curvyo_document_core::Point;
use curvyo_laser_core::job::{
    Frame, Job, MoveKind, PointUm, START_FIELD, Start, USER_ORIGIN_LIMIT_UM, burn_box, mm_of_um,
    read_output_settings, register_key, um_of_mm,
};
use serde::Serialize;

use super::super::Session;

/// A stored point within this many µm of the new one is the same point (W41: 0.01 mm).
const SAME_POINT_UM: i64 = 10;
/// The page's shorter side under this many screen px hides the cross (W46).
const SMALLEST_PAGE_PX: f64 = 48.0;

/// The User origin row under the Start pick (W43): the stored point in bed millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct UserOriginView {
    /// X in mm from the machine origin.
    pub x: f64,
    /// Y in mm from the machine origin.
    pub y: f64,
}

/// The canvas marks of the user origin in canvas CSS px (W46, W47).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct UserOriginMarks {
    /// The cross's centre, while User origin is picked.
    pub cross: Option<(f64, f64)>,
    /// The dashed burn box `[x, y, width, height]`, while the job is shifted and the Toolpath
    /// view is not pressed.
    #[serde(rename = "box")]
    pub burn_box: Option<[f64; 4]>,
}

/// The page box of the burn moves of `job`, placed by its shift, in page µm.
fn placed_box(job: &Job) -> Option<(PointUm, PointUm)> {
    let mut from = job.home;
    let mut points = Vec::new();
    for m in &job.moves {
        if matches!(m.kind, MoveKind::Burn { .. } | MoveKind::Sweep { .. }) {
            points.extend([from, m.to]);
        }
        from = m.to;
    }
    let b = burn_box(points)?;
    let shift = |p: PointUm| PointUm::new(p.x + job.shift.x, p.y + job.shift.y);
    Some((shift(b.lo), shift(b.hi)))
}

impl Session {
    /// Set origin here and the typed X, Y pair (criterion 29): stores `(x_mm, y_mm)`, the head's
    /// position in the job file's coordinates, as the page's user origin with Start User origin,
    /// one step "Set origin here". Nothing goes to the controller. `false` off a laser page,
    /// beyond ±10 m, or at the stored point (within 0.01 mm) with User origin picked.
    pub fn set_user_origin(&mut self, x_mm: f64, y_mm: f64) -> bool {
        self.store_user_origin(x_mm, y_mm, false)
    }

    /// The typed X, Y pair of the User origin row (W43): as [`Self::set_user_origin`], its own
    /// step "User origin".
    pub fn type_user_origin(&mut self, x_mm: f64, y_mm: f64) -> bool {
        self.store_user_origin(x_mm, y_mm, true)
    }

    fn store_user_origin(&mut self, x_mm: f64, y_mm: f64, typed: bool) -> bool {
        let Some(manufacturing) = self.active_manufacturing() else {
            return false;
        };
        let limit = mm_of_um(USER_ORIGIN_LIMIT_UM);
        if !(x_mm.abs() <= limit && y_mm.abs() <= limit) {
            return false;
        }
        let point = PointUm::new(um_of_mm(x_mm), um_of_mm(y_mm));
        let settings = read_output_settings(&manufacturing);
        let same = settings.user_origin.is_some_and(|stored| {
            (stored.x - point.x).abs() <= SAME_POINT_UM
                && (stored.y - point.y).abs() <= SAME_POINT_UM
        });
        if same && settings.start == Start::UserOrigin {
            return false;
        }
        let id = manufacturing.id;
        self.recording_unguarded(|this| {
            let stored = [point.x, point.y];
            if typed {
                this.document.type_user_origin(id, stored)
            } else {
                this.document.set_user_origin(id, stored)
            }
            .unwrap_or(false)
        })
    }

    /// Whether the page stores Start `user_origin`, readable or not (the pick lists it then, W42).
    pub(super) fn stores_user_origin_start(&self) -> bool {
        self.active_manufacturing().is_some_and(|m| {
            m.process_values.get(&register_key(START_FIELD))
                == Some(&serde_json::Value::from(Start::UserOrigin.as_str()))
        })
    }

    /// The User origin row while User origin is picked (W43), `None` otherwise.
    pub(super) fn user_origin_view(&self) -> Option<UserOriginView> {
        let settings = read_output_settings(&self.active_manufacturing()?);
        let point = settings
            .user_origin
            .filter(|_| settings.start == Start::UserOrigin)?;
        Some(UserOriginView {
            x: mm_of_um(point.x),
            y: mm_of_um(point.y),
        })
    }

    /// The cross on the user origin and the dashed box where the job burns, in canvas CSS px
    /// (criterion 32, W46, W47); both `None` with another pick or off a laser page. The box reads
    /// the job the session holds and builds none.
    #[must_use]
    pub fn user_origin_marks(&self) -> UserOriginMarks {
        let none = UserOriginMarks {
            cross: None,
            burn_box: None,
        };
        let Some(manufacturing) = self.active_manufacturing() else {
            return none;
        };
        let settings = read_output_settings(&manufacturing);
        let (Start::UserOrigin, Some(origin)) = (settings.start, settings.user_origin) else {
            return none;
        };
        let (Some(page), Some((frame, _))) = (manufacturing.page, self.print_and_cut_frame())
        else {
            return none;
        };
        let view = self.view();
        let corner = page.origin;
        let screen = |p: PointUm| {
            view.document_to_screen(Point::new(
                corner.x + mm_of_um(p.x),
                corner.y + mm_of_um(p.y),
            ))
        };
        let shorter = page.size.width.as_mm().min(page.size.height.as_mm()) * view.scale();
        let cross =
            (shorter >= SMALLEST_PAGE_PX).then(|| screen(Frame::page_point(&frame, origin)));
        let burn_box = self
            .held_laser_job()
            .filter(|job| !self.toolpath_view_on() && job.built.job.start == Start::UserOrigin)
            .filter(|job| {
                let s = job.built.job.shift;
                s.x.abs() >= SAME_POINT_UM || s.y.abs() >= SAME_POINT_UM
            })
            .and_then(|job| placed_box(&job.built.job))
            .map(|(lo, hi)| {
                let (a, b) = (screen(lo), screen(hi));
                [a.0, a.1, b.0 - a.0, b.1 - a.1]
            });
        UserOriginMarks { cross, burn_box }
    }

    /// [`Session::user_origin_marks`] as JSON (`{"cross": [x, y] | null, "box": [x, y, w, h] |
    /// null}`).
    #[must_use]
    pub fn user_origin_marks_json(&self) -> String {
        serde_json::to_string(&self.user_origin_marks()).unwrap_or_else(|_| "null".to_owned())
    }
}

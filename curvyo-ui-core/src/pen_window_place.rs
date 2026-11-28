//! The remembered pen window and where it opens (`specs/0158-pen-display` slice 2a criterion 16,
//! 2b criteria 4 and 5, ADR 0096 amendment A4): the record `pen_window.json` (written as format 2,
//! format 1 still read) and the placement rule, pure, for the hub to apply.
//!
//! The file never holds project content. A missing, damaged, older or newer file reads as none,
//! and a read never rewrites it. Format 1 has no switches; it reads as both off.

use serde::{Deserialize, Serialize};

/// The format this build writes; it also reads [`PEN_WINDOW_FORMAT_V1`].
pub const PEN_WINDOW_FORMAT_VERSION: u64 = 2;
/// The format of slice 2a: the place without the switches.
pub const PEN_WINDOW_FORMAT_V1: u64 = 1;
/// The size of a pen window without a remembered size, in logical pixels (UX 9.6).
pub const DEFAULT_LOGICAL_SIZE: (u32, u32) = (1200, 720);
/// The longest file that is read.
const MAX_FILE_BYTES: usize = 4 * 1024;
/// Sizes outside this range (physical pixels) are damage, not a window.
const SIZE_RANGE: std::ops::RangeInclusive<u32> = 100..=32_768;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PenWindowJsonV1 {
    format_version: u64,
    width: u32,
    height: u32,
    x: Option<i32>,
    y: Option<i32>,
    maximised: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PenWindowJson {
    format_version: u64,
    width: u32,
    height: u32,
    x: Option<i32>,
    y: Option<i32>,
    maximised: bool,
    follow_view: bool,
    canvas_only: bool,
}

/// The pen window's two remembered switches (2b criteria 4, 5); both off without a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PenWindowFlags {
    /// Follow view is on.
    pub follow_view: bool,
    /// Canvas only is on.
    pub canvas_only: bool,
}

/// The pen window as it was last seen, in physical pixels of the desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PenWindowPlace {
    /// Its outer width.
    pub width: u32,
    /// Its outer height.
    pub height: u32,
    /// Its outer top-left corner, `None` where the platform reports no position (Wayland).
    pub position: Option<(i32, i32)>,
    /// Whether it was maximised.
    pub maximised: bool,
}

/// A connected screen in physical pixels of the desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRect {
    /// Its left edge.
    pub x: i32,
    /// Its top edge.
    pub y: i32,
    /// Its width.
    pub width: u32,
    /// Its height.
    pub height: u32,
}

impl ScreenRect {
    fn contains(&self, (x, y): (i64, i64)) -> bool {
        let (left, top) = (i64::from(self.x), i64::from(self.y));
        x >= left
            && y >= top
            && x < left + i64::from(self.width)
            && y < top + i64::from(self.height)
    }
}

/// The size the window opens with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpeningSize {
    /// No remembered size: [`DEFAULT_LOGICAL_SIZE`].
    Logical(u32, u32),
    /// The remembered size.
    Physical(u32, u32),
}

/// Where and how the pen window opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opening {
    /// Its size.
    pub size: OpeningSize,
    /// Its outer top-left corner in physical pixels, `None` to leave it to the compositor.
    pub position: Option<(i32, i32)>,
    /// Whether it opens maximised.
    pub maximised: bool,
}

/// The remembered place in `text`; `None` for a damaged, older or newer file (the defaults apply).
#[must_use]
pub fn read_pen_window(text: &str) -> Option<PenWindowPlace> {
    read_record(text).map(|(place, _)| place)
}

/// The remembered switches in `text`; both off for format 1 and for a file that does not read.
#[must_use]
pub fn read_pen_window_flags(text: &str) -> PenWindowFlags {
    read_record(text)
        .map(|(_, flags)| flags)
        .unwrap_or_default()
}

fn read_record(text: &str) -> Option<(PenWindowPlace, PenWindowFlags)> {
    if text.len() > MAX_FILE_BYTES {
        return None;
    }
    let json = match serde_json::from_str::<PenWindowJson>(text) {
        Ok(json) if json.format_version == PEN_WINDOW_FORMAT_VERSION => json,
        Ok(_) => return None,
        Err(_) => {
            let v1: PenWindowJsonV1 = serde_json::from_str(text).ok()?;
            if v1.format_version != PEN_WINDOW_FORMAT_V1 {
                return None;
            }
            PenWindowJson {
                format_version: v1.format_version,
                width: v1.width,
                height: v1.height,
                x: v1.x,
                y: v1.y,
                maximised: v1.maximised,
                follow_view: false,
                canvas_only: false,
            }
        }
    };
    let sane = SIZE_RANGE.contains(&json.width)
        && SIZE_RANGE.contains(&json.height)
        && json.x.is_some() == json.y.is_some();
    sane.then_some((
        PenWindowPlace {
            width: json.width,
            height: json.height,
            position: json.x.zip(json.y),
            maximised: json.maximised,
        },
        PenWindowFlags {
            follow_view: json.follow_view,
            canvas_only: json.canvas_only,
        },
    ))
}

/// The text of the file holding `place` and `flags` (format 2).
#[must_use]
#[allow(clippy::missing_panics_doc)] // see the invariant below
pub fn write_pen_window(place: &PenWindowPlace, flags: PenWindowFlags) -> String {
    let json = PenWindowJson {
        format_version: PEN_WINDOW_FORMAT_VERSION,
        width: place.width,
        height: place.height,
        x: place.position.map(|p| p.0),
        y: place.position.map(|p| p.1),
        maximised: place.maximised,
        follow_view: flags.follow_view,
        canvas_only: flags.canvas_only,
    };
    // invariant: a struct of numbers and booleans always serialises.
    #[allow(clippy::unwrap_used)]
    let mut text = serde_json::to_string_pretty(&json).unwrap();
    text.push('\n');
    text
}

/// Where the pen window opens: the remembered size and maximised state always; the remembered
/// position only when the window's centre lies on a connected screen, otherwise centred on the
/// main window's screen; no position at all where none was remembered (criterion 16).
#[must_use]
pub fn opening(
    saved: Option<&PenWindowPlace>,
    screens: &[ScreenRect],
    main_screen: Option<ScreenRect>,
) -> Opening {
    let Some(saved) = saved else {
        return Opening {
            size: OpeningSize::Logical(DEFAULT_LOGICAL_SIZE.0, DEFAULT_LOGICAL_SIZE.1),
            position: None,
            maximised: false,
        };
    };
    let position = saved.position.and_then(|(x, y)| {
        let centre = (
            i64::from(x) + i64::from(saved.width / 2),
            i64::from(y) + i64::from(saved.height / 2),
        );
        if screens.iter().any(|screen| screen.contains(centre)) {
            Some((x, y))
        } else {
            main_screen.map(|screen| centred(screen, saved.width, saved.height))
        }
    });
    Opening {
        size: OpeningSize::Physical(saved.width, saved.height),
        position,
        maximised: saved.maximised,
    }
}

fn centred(screen: ScreenRect, width: u32, height: u32) -> (i32, i32) {
    let offset = |space: u32, size: u32| {
        let free = i64::from(space) - i64::from(size);
        i32::try_from(free.max(0) / 2).unwrap_or(0)
    };
    (
        screen.x.saturating_add(offset(screen.width, width)),
        screen.y.saturating_add(offset(screen.height, height)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAPTOP: ScreenRect = ScreenRect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1600,
    };
    const KAMVAS: ScreenRect = ScreenRect {
        x: 2560,
        y: 0,
        width: 1920,
        height: 1080,
    };

    fn on_kamvas() -> PenWindowPlace {
        PenWindowPlace {
            width: 1800,
            height: 1000,
            position: Some((2600, 20)),
            maximised: false,
        }
    }

    const FOLLOW: PenWindowFlags = PenWindowFlags {
        follow_view: true,
        canvas_only: false,
    };

    #[test]
    fn a_written_place_reads_back_with_its_switches() {
        for place in [
            on_kamvas(),
            PenWindowPlace {
                position: None,
                maximised: true,
                ..on_kamvas()
            },
        ] {
            let text = write_pen_window(&place, FOLLOW);
            assert_eq!(read_pen_window(&text), Some(place));
            assert_eq!(read_pen_window_flags(&text), FOLLOW);
        }
    }

    #[test]
    fn format_1_reads_with_both_switches_off() {
        let v1 = "{\"format_version\": 1, \"width\": 1800, \"height\": 1000, \"x\": 2600, \"y\": 20, \"maximised\": false}";
        assert_eq!(read_pen_window(v1), Some(on_kamvas()));
        assert_eq!(read_pen_window_flags(v1), PenWindowFlags::default());
    }

    #[test]
    fn damaged_newer_or_unknown_fields_read_as_none() {
        let good = write_pen_window(&on_kamvas(), FOLLOW);
        assert!(read_pen_window(&good).is_some());
        for bad in [
            good.replace("\"format_version\": 2", "\"format_version\": 3"),
            good.replace("\"format_version\": 2", "\"format_version\": 1"),
            good.replace("\"maximised\"", "\"mirror\": true,\n  \"maximised\""),
            good.replace("1800", "0"),
            good.replace("\"y\": 20", "\"y\": null"),
            "{".to_owned(),
            String::new(),
            "x".repeat(5000),
        ] {
            assert_eq!(read_pen_window(&bad), None, "{bad}");
        }
    }

    #[test]
    fn without_a_file_the_defaults_apply_and_the_compositor_places_it() {
        assert_eq!(
            opening(None, &[LAPTOP, KAMVAS], Some(LAPTOP)),
            Opening {
                size: OpeningSize::Logical(1200, 720),
                position: None,
                maximised: false,
            }
        );
    }

    #[test]
    fn a_place_on_a_connected_screen_is_kept_with_size_and_maximised_state() {
        let saved = PenWindowPlace {
            maximised: true,
            ..on_kamvas()
        };
        let open = opening(Some(&saved), &[LAPTOP, KAMVAS], Some(LAPTOP));
        assert_eq!(open.size, OpeningSize::Physical(1800, 1000));
        assert_eq!(open.position, Some((2600, 20)));
        assert!(open.maximised);
    }

    #[test]
    fn a_place_off_every_screen_opens_centred_on_the_main_screen() {
        // The Kamvas is unplugged: only the laptop is left.
        let open = opening(Some(&on_kamvas()), &[LAPTOP], Some(LAPTOP));
        assert_eq!(open.position, Some((380, 300)));
        assert_eq!(open.size, OpeningSize::Physical(1800, 1000));
    }

    #[test]
    fn wayland_without_a_position_leaves_the_place_to_the_compositor() {
        let saved = PenWindowPlace {
            position: None,
            ..on_kamvas()
        };
        let open = opening(Some(&saved), &[], None);
        assert_eq!(open.position, None);
        assert_eq!(open.size, OpeningSize::Physical(1800, 1000));
    }
}

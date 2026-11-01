//! The document's snapping settings: whether snapping is on, which target kinds
//! contribute, and the two zone sizes, stored as root registers in the
//! document root (`specs/0060-snapping`, `adrs.md` decisions 1 and 2).
//!
//! Snapping is a property of the document, not of the user or the session: the
//! settings travel with the file and are not in `localStorage`. [`Document::
//! snapping`] reads the registers leniently (a merged replica never makes a
//! read panic), [`validate`] is the strict check on open, and [`Document::
//! set_snapping`] writes only the registers that changed, in one commit.

use loro::{LoroDoc, LoroMap, LoroValue, ValueOrContainer};
use serde::Serialize;

use crate::document::{Document, ROOT_MAP};
use crate::path_codec::as_f64;

/// The root register holding the on/off switch. Absent means off.
const KEY_ENABLED: &str = "snapping_enabled";
/// The root register for object boxes. Absent means on.
const KEY_OBJECTS: &str = "snapping_objects";
/// The root register for guides. Absent means on.
const KEY_GUIDES: &str = "snapping_guides";
/// The root register for path nodes. Absent means on.
const KEY_NODES: &str = "snapping_nodes";
/// The root register for the page fractions. Absent means on.
const KEY_PAGE: &str = "snapping_page";
/// The root register for the page grids (`specs/0090-grid` criterion 10). Absent means on.
const KEY_GRID: &str = "snapping_grid";
/// The root register for the grid snap point (criterion 11). Absent means centre.
const KEY_GRID_POINT: &str = "snapping_grid_point";
const KEY_X: &str = "snapping_x";
const KEY_Y: &str = "snapping_y";
/// The root register for the capture zone in screen pixels. Absent means
/// [`DocumentSnapping::DEFAULT_CAPTURE_PX`].
const KEY_CAPTURE_PX: &str = "snapping_capture_px";
/// The root register for the release zone in screen pixels. Absent means
/// [`DocumentSnapping::DEFAULT_RELEASE_PX`].
const KEY_RELEASE_PX: &str = "snapping_release_px";

/// Which part of a moved selection snaps to a grid (`specs/0090-grid` criterion 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GridSnapPoint {
    /// The centre of the moving selection's box.
    Centre,
    /// The nine reference points of the box; the nearest wins.
    Box,
    /// The moving anchors of paths; an object without anchors snaps by its box centre.
    Nodes,
}

impl GridSnapPoint {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Centre => "centre",
            Self::Box => "box",
            Self::Nodes => "nodes",
        }
    }

    /// Parses the stored and host spelling (`"centre"`, `"box"`, `"nodes"`).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "centre" => Some(Self::Centre),
            "box" => Some(Self::Box),
            "nodes" => Some(Self::Nodes),
            _ => None,
        }
    }
}

/// The document's snapping settings (`specs/0060-snapping` criterion 1).
///
/// Every field is plain data; [`Document::set_snapping`] refuses a value whose
/// zones are not finite or are negative, and raises a release zone smaller than the
/// capture zone before writing, so a file this build writes never holds the
/// pair inverted. A hand-written or older file that does is read as the
/// capture size (criterion 3).
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // Independent persisted target switches.
pub struct DocumentSnapping {
    /// The switch the Select bar shows. Off by default.
    pub enabled: bool,
    /// Snap to the boxes of other objects.
    pub objects: bool,
    /// Snap to guides.
    pub guides: bool,
    /// Snap to path nodes.
    pub nodes: bool,
    /// Snap to the six page fractions on each axis.
    pub page: bool,
    /// Snap to shown page grids (`specs/0090-grid` criterion 10).
    pub grid: bool,
    /// Which part of a moved selection snaps to a grid.
    pub grid_point: GridSnapPoint,
    /// Allow correction of the horizontal coordinate.
    pub x: bool,
    /// Allow correction of the vertical coordinate.
    pub y: bool,
    /// How close a point must come, in screen pixels, before it captures.
    pub capture_px: f64,
    /// How far it must move away, in screen pixels, before it lets go. Never
    /// smaller than `capture_px` after a write or a read.
    pub release_px: f64,
}

impl DocumentSnapping {
    /// The default capture zone, in screen pixels.
    pub const DEFAULT_CAPTURE_PX: f64 = 8.0;
    /// The default release zone, in screen pixels.
    pub const DEFAULT_RELEASE_PX: f64 = 14.0;

    /// What a new document and a file without the registers have: off, every
    /// target kind on, capture 8 px and release 14 px (criterion 1).
    pub const DEFAULT: Self = Self {
        enabled: false,
        objects: true,
        guides: true,
        nodes: true,
        page: true,
        grid: true,
        grid_point: GridSnapPoint::Centre,
        x: true,
        y: true,
        capture_px: Self::DEFAULT_CAPTURE_PX,
        release_px: Self::DEFAULT_RELEASE_PX,
    };

    /// Whether both zones are finite and inside the stored range. The flags
    /// are always valid.
    #[must_use]
    pub fn zones_are_valid(self) -> bool {
        let valid = |px: f64| px.is_finite() && px >= 0.0;
        valid(self.capture_px) && valid(self.release_px)
    }

    /// This value with a release zone smaller than the capture zone raised to
    /// it (criterion 3). The flags and the capture zone are unchanged.
    #[must_use]
    pub fn resolved(self) -> Self {
        Self {
            release_px: self.release_px.max(self.capture_px),
            ..self
        }
    }
}

impl Default for DocumentSnapping {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The `snapping` member of `document.json`.
#[derive(Serialize)]
#[allow(clippy::struct_excessive_bools)] // Mirrors the independent persisted switches.
pub(crate) struct SnappingJson {
    enabled: bool,
    objects: bool,
    guides: bool,
    nodes: bool,
    page: bool,
    /// Left out at its default (on), like a node's registers, so the export of a file
    /// without grid snapping registers is unchanged.
    #[serde(skip_serializing_if = "is_true")]
    grid: bool,
    #[serde(skip_serializing_if = "is_centre")]
    grid_point: GridSnapPoint,
    x: bool,
    y: bool,
    capture_px: f64,
    release_px: f64,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // Serde's skip predicate takes a reference.
fn is_true(value: &bool) -> bool {
    *value
}

#[allow(clippy::trivially_copy_pass_by_ref)] // Serde's skip predicate takes a reference.
fn is_centre(value: &GridSnapPoint) -> bool {
    *value == GridSnapPoint::Centre
}

impl From<DocumentSnapping> for SnappingJson {
    fn from(snapping: DocumentSnapping) -> Self {
        Self {
            enabled: snapping.enabled,
            objects: snapping.objects,
            guides: snapping.guides,
            nodes: snapping.nodes,
            page: snapping.page,
            grid: snapping.grid,
            grid_point: snapping.grid_point,
            x: snapping.x,
            y: snapping.y,
            capture_px: snapping.capture_px,
            release_px: snapping.release_px,
        }
    }
}

fn bool_of(value: &LoroValue) -> Option<bool> {
    match value {
        LoroValue::Bool(value) => Some(*value),
        _ => None,
    }
}

fn grid_point_of(value: &LoroValue) -> Option<GridSnapPoint> {
    match value {
        LoroValue::String(text) => GridSnapPoint::parse(text),
        _ => None,
    }
}

fn zone_of(value: &LoroValue) -> Option<f64> {
    let px = as_f64(value)?;
    (px.is_finite() && px >= 0.0).then_some(px)
}

/// The plain value of a root register: `Ok(None)` when absent, `Err` when the
/// key holds a Loro container, which no Curvyo writer produces and which peers
/// could edit element by element.
fn plain(root: &LoroMap, key: &str) -> Result<Option<LoroValue>, ()> {
    match root.get(key) {
        None => Ok(None),
        Some(ValueOrContainer::Value(value)) => Ok(Some(value)),
        Some(ValueOrContainer::Container(_)) => Err(()),
    }
}

/// Reads the settings from a root map. An absent or malformed register reads as
/// its default; the read writes nothing (criterion 14).
pub(crate) fn snapping_of_root(root: &LoroMap) -> DocumentSnapping {
    let boolean = |key: &str, default: bool| {
        plain(root, key)
            .ok()
            .flatten()
            .and_then(|value| bool_of(&value))
            .unwrap_or(default)
    };
    let zone = |key: &str, default: f64| {
        plain(root, key)
            .ok()
            .flatten()
            .and_then(|value| zone_of(&value))
            .unwrap_or(default)
    };
    DocumentSnapping {
        enabled: boolean(KEY_ENABLED, false),
        objects: boolean(KEY_OBJECTS, true),
        guides: boolean(KEY_GUIDES, true),
        nodes: boolean(KEY_NODES, true),
        page: boolean(KEY_PAGE, true),
        grid: boolean(KEY_GRID, true),
        grid_point: plain(root, KEY_GRID_POINT)
            .ok()
            .flatten()
            .and_then(|value| grid_point_of(&value))
            .unwrap_or(GridSnapPoint::Centre),
        x: boolean(KEY_X, true),
        y: boolean(KEY_Y, true),
        capture_px: zone(KEY_CAPTURE_PX, DocumentSnapping::DEFAULT_CAPTURE_PX),
        release_px: zone(KEY_RELEASE_PX, DocumentSnapping::DEFAULT_RELEASE_PX),
    }
    .resolved()
}

/// Strict validation on open (criterion 14): every register that is present
/// must be a well-formed plain value in range. An absent register is valid, and
/// a release zone below the capture zone is valid because it reads as the
/// capture size.
pub(crate) fn validate(loro: &LoroDoc) -> bool {
    let root = loro.get_map(ROOT_MAP);
    let ok = |key: &str, check: fn(&LoroValue) -> bool| {
        plain(&root, key).is_ok_and(|value| value.as_ref().is_none_or(check))
    };
    ok(KEY_ENABLED, |value| bool_of(value).is_some())
        && ok(KEY_OBJECTS, |value| bool_of(value).is_some())
        && ok(KEY_GUIDES, |value| bool_of(value).is_some())
        && ok(KEY_NODES, |value| bool_of(value).is_some())
        && ok(KEY_PAGE, |value| bool_of(value).is_some())
        && ok(KEY_GRID, |value| bool_of(value).is_some())
        && ok(KEY_GRID_POINT, |value| grid_point_of(value).is_some())
        && ok(KEY_X, |value| bool_of(value).is_some())
        && ok(KEY_Y, |value| bool_of(value).is_some())
        && ok(KEY_CAPTURE_PX, |value| zone_of(value).is_some())
        && ok(KEY_RELEASE_PX, |value| zone_of(value).is_some())
}

impl Document {
    /// The document's snapping settings. An absent register reads as its
    /// default; a malformed one (only a merged, never validated replica can
    /// hold one) does too, and the read writes nothing.
    #[must_use]
    pub fn snapping(&self) -> DocumentSnapping {
        snapping_of_root(&self.loro().get_map(ROOT_MAP))
    }

    /// Sets the snapping settings as one commit labelled
    /// `set_document_snapping`, writing only the registers that differ from the
    /// stored value. A release zone below the capture zone is raised to it
    /// first; a zone outside the stored range, or not finite, writes nothing.
    /// Returns `false` and writes nothing if the resolved value equals the
    /// stored one or the value is invalid.
    #[allow(clippy::float_cmp)] // Exact register equality avoids writing unchanged values.
    pub fn set_snapping(&self, value: DocumentSnapping) -> bool {
        if !value.zones_are_valid() {
            return false;
        }
        let value = value.resolved();
        let root = self.loro().get_map(ROOT_MAP);
        let current = snapping_of_root(&root);
        let mut changed = false;
        let mut set = |key: &str, new: LoroValue| {
            // invariant: attached document root accepts primitive registers.
            if root.insert(key, new).is_err() {
                return;
            }
            changed = true;
        };
        if value.enabled != current.enabled {
            set(KEY_ENABLED, LoroValue::Bool(value.enabled));
        }
        if value.objects != current.objects {
            set(KEY_OBJECTS, LoroValue::Bool(value.objects));
        }
        if value.guides != current.guides {
            set(KEY_GUIDES, LoroValue::Bool(value.guides));
        }
        if value.nodes != current.nodes {
            set(KEY_NODES, LoroValue::Bool(value.nodes));
        }
        if value.page != current.page {
            set(KEY_PAGE, LoroValue::Bool(value.page));
        }
        if value.grid != current.grid {
            set(KEY_GRID, LoroValue::Bool(value.grid));
        }
        if value.grid_point != current.grid_point {
            set(KEY_GRID_POINT, LoroValue::from(value.grid_point.as_str()));
        }
        if value.x != current.x {
            set(KEY_X, LoroValue::Bool(value.x));
        }
        if value.y != current.y {
            set(KEY_Y, LoroValue::Bool(value.y));
        }
        if value.capture_px != current.capture_px {
            set(KEY_CAPTURE_PX, LoroValue::Double(value.capture_px));
        }
        if value.release_px != current.release_px {
            set(KEY_RELEASE_PX, LoroValue::Double(value.release_px));
        }
        if changed {
            self.commit_with_label("set_document_snapping");
        }
        changed
    }
}

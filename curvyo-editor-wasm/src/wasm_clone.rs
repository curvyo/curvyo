//! Linked clones' `wasm-bindgen` surface (`specs/0049-linked-clone`): the Select-bar button, the
//! commands of the Style tab's clone block and origin line, their notices, and the tags and edge
//! chips of the canvas marks. A further `impl WasmSession` block of pass-throughs; the rules and
//! the sentences are in `Session`.

use wasm_bindgen::prelude::*;

use crate::session::CloneResult;
use crate::wasm_api::WasmSession;

/// What a clone command did: `kind` is `"ignored"`, `"done"` or `"refused"`, and `notice` the
/// sentence to show (empty for a selection command). A refusal is an alert, the rest a status.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct CloneResultView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// The one-line notice.
    #[wasm_bindgen(getter_with_clone)]
    pub notice: String,
}

impl From<CloneResult> for CloneResultView {
    fn from(result: CloneResult) -> Self {
        Self {
            kind: result.code().to_owned(),
            notice: result.notice,
        }
    }
}

/// What the Style tab shows about linked clones (criteria 22, 23, 56).
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct ClonePanelView {
    /// The selection holds only clones: the clone block is shown.
    pub clones_only: bool,
    /// How many clones are selected.
    pub count: usize,
    /// The name of the first clone's direct original.
    #[wasm_bindgen(getter_with_clone)]
    pub original: String,
    /// The depth of the deepest selected clone (the chain line and Select root original from 2).
    pub deepest: usize,
    /// The name of its root original.
    #[wasm_bindgen(getter_with_clone)]
    pub root: String,
    /// How many clones follow the selected objects directly (the origin line from 1).
    pub followers: usize,
    /// How many selected objects clones follow ("these objects" from 2).
    pub followed: usize,
    /// The Placement row: `"original"` (Follows original), `"own"` or `"mixed"`
    /// (`specs/0057-clone-overrides` criterion 1).
    #[wasm_bindgen(getter_with_clone)]
    pub placement: String,
    /// The Style row: `"original"`, `"own"` or `"mixed"` (criterion 10).
    #[wasm_bindgen(getter_with_clone)]
    pub style: String,
    /// Spread colours is enabled: two or more clones are selected (criterion 42).
    pub spread_ready: bool,
    /// The muted summary of the greyed Stroke header: "0.5 mm", or "None" when off.
    #[wasm_bindgen(getter_with_clone)]
    pub stroke_summary: String,
    /// The stroke colour as `#rrggbb`, empty when the stroke is off.
    #[wasm_bindgen(getter_with_clone)]
    pub stroke_hex: String,
    /// The fill colour as `#rrggbb`, empty when the fill is off.
    #[wasm_bindgen(getter_with_clone)]
    pub fill_hex: String,
    /// The muted summary of the greyed Markers header: "None" or the marker names; empty when the
    /// stroke is off, and the header is then not shown.
    #[wasm_bindgen(getter_with_clone)]
    pub markers_summary: String,
}

fn hex(color: curvyo_document_core::Color) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

fn markers_text(markers: curvyo_document_core::Markers) -> String {
    let name = |shape: curvyo_document_core::MarkerShape| match shape {
        curvyo_document_core::MarkerShape::None => None,
        curvyo_document_core::MarkerShape::Arrow => Some("Arrow"),
        curvyo_document_core::MarkerShape::Dot => Some("Dot"),
    };
    let names: Vec<&str> = [markers.start, markers.mid, markers.end]
        .into_iter()
        .filter_map(name)
        .collect();
    if names.is_empty() {
        "None".to_owned()
    } else {
        names.join(", ")
    }
}

/// A tag or an edge chip of the canvas marks, in canvas px: `kind` is `"tag"` or `"chip"`.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct CloneTagView {
    /// `"tag"` or `"chip"`.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// The text ("Clone", "Original", "Clone + Original", "Original (12 clones)").
    #[wasm_bindgen(getter_with_clone)]
    pub text: String,
    /// Left edge of a tag, or the centre x of a chip.
    pub x: f64,
    /// Top edge of a tag, or the centre y of a chip.
    pub y: f64,
    /// Width of a tag (0 for a chip).
    pub w: f64,
    /// For a chip, the direction to its object in radians (screen, y down); 0 for a tag.
    pub toward: f64,
}

#[wasm_bindgen]
impl WasmSession {
    /// Whether the Linked clone button is in the Select bar.
    #[must_use]
    pub fn linked_clone_shown(&self) -> bool {
        self.session.linked_clone_shown()
    }

    /// Linked clone: one clone above each selected object, in one commit.
    pub fn linked_clone(&mut self) -> CloneResultView {
        self.session.linked_clone().into()
    }

    /// Unlink clone: every selected clone becomes a path, in one commit.
    pub fn unlink_clones(&mut self) -> CloneResultView {
        self.session.unlink_clones().into()
    }

    /// Select original: the direct originals of the selected clones.
    pub fn select_original(&mut self) -> CloneResultView {
        self.session.select_original().into()
    }

    /// Select root original: the root originals of the selected clones.
    pub fn select_root_original(&mut self) -> CloneResultView {
        self.session.select_root_original().into()
    }

    /// Select clones: the clones that follow the selected objects directly.
    pub fn select_clones(&mut self) -> CloneResultView {
        self.session.select_clones().into()
    }

    /// What the Style tab shows about linked clones.
    #[must_use]
    pub fn clone_panel(&self) -> ClonePanelView {
        let panel = self.session.clone_panel();
        let block = panel.block.clone();
        ClonePanelView {
            clones_only: block.is_some(),
            count: block.as_ref().map_or(0, |b| b.count),
            original: block
                .as_ref()
                .map(|b| b.original.clone())
                .unwrap_or_default(),
            deepest: block.as_ref().map_or(0, |b| b.deepest),
            root: block.as_ref().map(|b| b.root.clone()).unwrap_or_default(),
            followers: panel.followers,
            followed: panel.followed,
            placement: block
                .as_ref()
                .map_or("original", |b| b.placement.code())
                .to_owned(),
            style: block
                .as_ref()
                .map_or("original", |b| b.style.code())
                .to_owned(),
            spread_ready: self.session.spread_colours_ready(),
            stroke_summary: block.as_ref().map_or_else(String::new, |b| {
                if b.summary.stroke.enabled {
                    format!("{} mm", b.summary.stroke.width.as_mm())
                } else {
                    "None".to_owned()
                }
            }),
            stroke_hex: block
                .as_ref()
                .filter(|b| b.summary.stroke.enabled)
                .map(|b| hex(b.summary.stroke.color))
                .unwrap_or_default(),
            fill_hex: block
                .as_ref()
                .filter(|b| b.summary.fill.enabled)
                .map(|b| hex(b.summary.fill.color))
                .unwrap_or_default(),
            // The open state shows Markers rows only with a stroke (`0018`), so the greyed header
            // goes with it (UX review of 0057, must-fix 1).
            markers_summary: block
                .filter(|b| b.summary.stroke.enabled)
                .map(|b| markers_text(b.summary.stroke.markers))
                .unwrap_or_default(),
        }
    }

    /// The Placement row of the clone block: `own` true is "Own placement".
    pub fn set_clone_placement(&mut self, own: bool) -> CloneResultView {
        self.session.set_clone_placement(own).into()
    }

    /// The Style row of the clone block: `own` true is "Own".
    pub fn set_clone_style(&mut self, own: bool) -> CloneResultView {
        self.session.set_clone_style(own).into()
    }

    /// Spread colours: evenly spaced hues over the selected clones, one commit.
    pub fn spread_colours(&mut self) -> CloneResultView {
        self.session.spread_colours().into()
    }

    /// The sentence of the last transform that held clones with own placement back, once.
    pub fn take_clone_hold_notice(&mut self) -> String {
        self.session.take_clone_hold_notice()
    }

    /// The notice of the last style or opacity edit that left clones alone, once.
    pub fn take_style_skip_notice(&mut self) -> String {
        self.session.take_style_skip_notice()
    }

    /// The notice of the last Delete key when it repaired clones, once: "Deleted 1 object. 1
    /// clone became a path."; empty otherwise.
    pub fn take_delete_notice(&mut self) -> String {
        self.session.take_delete_notice()
    }

    /// The sentence the clone repair of the last command adds to its notice, once.
    pub fn take_clone_repair_notice(&mut self) -> String {
        self.session.take_clone_repair_notice()
    }

    /// The refusal of the last command because a linked clone would pass the limit, once.
    pub fn take_clone_limit_notice(&mut self) -> String {
        self.session.take_clone_limit_notice()
    }

    /// The notice of the orphan repair after the Open, once.
    pub fn take_orphan_notice(&mut self) -> String {
        self.session.take_orphan_notice()
    }

    /// The DOM chips over the canvas as `[x, y, w, h, ...]` in canvas px: no clone tag meets one.
    pub fn set_clone_tag_obstacles(&mut self, flat: &[f64]) {
        self.session.set_clone_tag_obstacles(flat);
    }

    /// The pointer or focus is on Select original (`"original"`), Select root original
    /// (`"root"`) or neither (`""`): the targets get the hover box.
    pub fn set_clone_hover(&mut self, kind: &str) {
        self.session.set_clone_hover(kind);
    }

    /// The tags and edge chips of the canvas marks of this frame.
    #[must_use]
    pub fn clone_tags(&self) -> Vec<CloneTagView> {
        let marks = self.session.clone_marks();
        let tags = marks.tags.into_iter().map(|tag| CloneTagView {
            kind: "tag".to_owned(),
            text: tag.text,
            x: tag.rect.x,
            y: tag.rect.y,
            w: tag.rect.w,
            toward: 0.0,
        });
        let chips = marks.chips.into_iter().map(|chip| CloneTagView {
            kind: "chip".to_owned(),
            text: chip.text.to_owned(),
            x: chip.at.0,
            y: chip.at.1,
            w: 0.0,
            toward: chip.toward,
        });
        tags.chain(chips).collect()
    }
}

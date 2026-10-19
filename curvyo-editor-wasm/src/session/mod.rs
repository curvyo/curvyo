//! One open document's editing session: the plain-Rust orchestration
//! this crate's `wasm-bindgen` surface is a thin shell around.
//!
//! Deliberately free of any `wasm-bindgen`/`web-sys`/GPU type so it can
//! be exercised by ordinary `cargo test` — "no editing logic of its own"
//! (ADR 0001 §3) describes the `wasm-bindgen` boundary in `lib.rs`, not
//! this module: the logic itself is `curvyo-document-core`'s,
//! `curvyo-ui-core`'s and `curvyo-render-core`'s, and this module's own
//! job is exactly "which tool is active, and where do its inputs and the
//! view transform come from" — nothing a browser is required to answer.
//!
//! `primitive-shapes` adds three shape tools (Rectangle, Ellipse,
//! Polygon/Star) alongside Pen and Node, plus "object to path"
//! (acceptance criteria 1-22). Everything specific to those three tools
//! — tool-options, live preview/readout, decoration input, "object to
//! path" itself — lives in the `shapes` submodule (architect review:
//! this file alone grew past a size that still read as "one
//! responsibility"); it reaches this module's otherwise-private fields
//! because a child module shares its parent's privacy boundary, and its
//! methods join this type's `impl Session` the same way any other
//! `impl` block in the same crate would.

#[cfg(test)]
mod arc_tests;
mod background;
mod background_view;
mod boolean;
#[cfg(test)]
mod box_refit_tests;
mod clone_overrides;
#[cfg(test)]
mod clone_tests;
mod clone_view;
mod clones;
mod close_path;
mod colour_pick;
mod combine;
mod context;
mod corner_readout;
mod cut;
mod document;
mod document_presets;
mod draw;
mod file_chord;
mod flip;
mod formats;
mod frame;
mod group_view;
mod grouping;
#[cfg(test)]
mod grouping_tests;
mod history;
mod history_list;
mod history_preview;
mod history_rows;
mod history_version;
mod key_table;
mod keys;
mod layers;
mod layers_edit;
mod layers_panel;
#[cfg(test)]
mod layers_panel_tests;
#[cfg(test)]
mod layers_tests;
mod layers_toolbar;
mod move_entry;
mod move_indicators;
mod navigation;
mod node;
mod node_marquee;
mod nudge;
mod object_cache;
mod object_history;
mod object_history_rows;
mod offset;
mod offset_chip;
mod offset_view;
mod open_error;
mod panel_tabs;
mod pen;
mod placement;
mod placement_view;
mod project;
mod reference_view;
mod refusal;
mod ruler;
mod select;
mod select_bar;
mod select_bar_arc;
mod select_cursor;
mod select_gesture;
mod select_readout;
mod select_view;
mod shapes;
mod split;
mod style;
mod style_view;
mod tolerances;
mod transform_entry;

use curvyo_document_core::{
    Document, DocumentVersion, NodeId, ObjectSnapshot, OpenError, Point, SaveError,
};
use curvyo_ui_core::{
    AnchorIdMinter, EllipseTool, Hit, Modifiers, NodeTool, ObjectSelection, PenTool,
    PolygonStarTool, RectangleTool, SelectTool, StyleEditor, Viewport,
};

#[cfg(target_arch = "wasm32")]
pub use background_view::BackgroundView;
pub use boolean::BooleanOutcome;
#[cfg(target_arch = "wasm32")]
pub use clones::CloneResult;
pub use close_path::{ClosePathOutcome, ClosePathState};
pub use combine::CombineOutcome;
pub use cut::CutOutcome;
pub use document::{DocumentSide, FitOutcome, SizeOutcome};
pub use document_presets::DocumentPresetsRecord;
pub use formats::{FormTexts, FormatEdit};
pub use grouping::{GroupingKind, GroupingResult};
pub use history::HistoryResult;
pub use history_preview::{FAR_PREVIEW_ROWS, HistoryRowPress};
pub use history_rows::{WipeOutcome, WipeState};
pub use history_version::{CloneOutcome, GoToOutcome};
pub use keys::{EscapeStep, KeyHint, KeyInput, KeyOutcome};
pub use layers_edit::{DropPlace, RowFlag};
pub use layers_panel::{PanelReply, RowPress};
pub use move_indicators::MoveIndicators;
pub use object_history::ObjectWipeOutcome;
pub use offset::{DragStop, OFFSET_COMMIT_LABEL, OffsetOutcome};
pub use offset_view::OffsetView;
pub use panel_tabs::{PanelTabsRecord, PanelView};
pub use project::APP_VERSION;
pub use select::DoubleClickHint;
pub use split::{BreakApartOutcome, SplitOutcome};
#[cfg(target_arch = "wasm32")]
pub use style_view::StylePanelView;

#[cfg(target_arch = "wasm32")]
pub use open_error::map_open_error;

// Re-exported only for `wasm_api`'s own `LiveReadout` wrapper (its only
// consumer, and itself `wasm32`-only) — `#[cfg]`-gated the same way so
// a host build does not see an unused public re-export.
#[cfg(target_arch = "wasm32")]
pub use move_entry::MoveEntryView;
#[cfg(target_arch = "wasm32")]
pub use placement_view::{PlacementChipView, PlacementFieldView};
#[cfg(target_arch = "wasm32")]
pub use shapes::LiveReadout;
#[cfg(target_arch = "wasm32")]
pub use transform_entry::{EntryFieldView, EntryView};

/// The largest pointer coordinate (document millimetres) a tool ever sees.
/// A finite value beyond it — one past `f32` range panics the draw-list
/// tessellator — is clamped to it; real pointer events are nowhere near.
const MAX_POINTER_COORDINATE_MM: f64 = 1e9;

/// `point` with both coordinates clamped to ±[`MAX_POINTER_COORDINATE_MM`],
/// or `None` if either is NaN or infinite. Pointer events come from the
/// host; non-finite values must never reach a tool, where they could turn
/// into a NaN `rotation` or an infinite size in a saved file
/// (`specs/0005-object-transform/adrs.md`: "a drag must never make a file
/// unopenable"), and absurd finite ones must not reach the renderer.
fn sanitized_point(point: Point) -> Option<Point> {
    if !(point.x.is_finite() && point.y.is_finite()) {
        return None;
    }
    let clamp = |v: f64| v.clamp(-MAX_POINTER_COORDINATE_MM, MAX_POINTER_COORDINATE_MM);
    Some(Point::new(clamp(point.x), clamp(point.y)))
}

/// Which tool is active. Exactly one at a time — `specification.md`'s
/// tool rail has six buttons (Select, Pen, Node, Rectangle, Ellipse,
/// Polygon/Star), and switching tools is a single active-tool state,
/// not independent flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// The Select tool (acceptance criteria 12-24,
    /// `canvas-navigation-and-selection`) — first in the tool rail, and
    /// the launch default for both `New` and `Open` (acceptance
    /// criterion 13).
    Select,
    /// The pen tool (acceptance criteria 1-5, `path-node-editing`).
    Pen,
    /// The node tool (acceptance criteria 7-14, `path-node-editing`).
    Node,
    /// The rectangle tool (acceptance criteria 1-6).
    Rectangle,
    /// The ellipse/circle tool (acceptance criteria 7-9).
    Ellipse,
    /// The polygon/star tool (acceptance criteria 10-15).
    PolygonStar,
    /// The Offset tool (`specs/0038-path-offset` criteria 1, 2): handles on the selection box,
    /// its options in the Offset bar.
    Offset,
}

/// One open document's whole editing session.
pub struct Session {
    document: Document,
    /// The version of the document that the last Save, Save as, Open or New left (`project.rs`).
    saved: DocumentVersion,
    /// The version that went into the bytes of a save whose write has not been reported yet.
    packed: Option<DocumentVersion>,
    minter: AnchorIdMinter,
    pen: PenTool,
    node: NodeTool,
    rectangle: RectangleTool,
    ellipse: EllipseTool,
    poly_star: PolygonStarTool,
    select: SelectTool,
    /// Shared across the three shape tools (acceptance criterion 22:
    /// two or more primitives can be selected together) and the Select
    /// tool (`specs/0004-canvas-navigation-and-selection/adrs.md`: "one
    /// object selection, shared by the Select tool and the shape tools").
    selection: ObjectSelection,
    tool: Tool,
    /// The Style panel's drag in flight: the ephemeral override drawn in place
    /// of the stored style, committed once on release
    /// (`specs/0007-stroke-and-fill-styling` criterion 36).
    style: StyleEditor,
    /// The eyedropper (`colour_pick.rs`).
    colour_pick: colour_pick::ColourPick,
    /// The Background block's drag in flight: drawn in place of the stored background,
    /// committed once on release, never in the document (`background.rs`).
    background_preview: Option<curvyo_document_core::DocumentBackground>,
    /// The document formats: the built-in list with the maker's file over it
    /// (`formats.rs`).
    formats: curvyo_document_core::FormatLibrary,
    /// Why the maker's formats file could not be read, while it cannot.
    formats_broken: Option<String>,
    /// The Properties panel's active tab (`panel_tabs.rs`).
    panel_tabs: std::cell::Cell<curvyo_ui_core::PanelTabs>,
    /// Pan/zoom view state (ADR 0009 §2: ephemeral — never written to
    /// the document, resets on `New`/`Open`).
    viewport: Viewport,
    hovered: Option<Hit>,
    /// The object currently hovered while the Select tool is active —
    /// the Select-tool counterpart to `hovered` above (acceptance criteria
    /// 14, 15's hover box). A creation tool has no hover highlight.
    hovered_object: Option<NodeId>,
    /// The objects of the History row, or the targets of Select original, the pointer or focus
    /// rests on (`history.rs`, `clone_view.rs`): they show the hover box.
    history_hover: Vec<NodeId>,
    /// The live pointer position in document space, tracked regardless
    /// of the active tool — the pen tool's rubber-band preview
    /// (`specification.md`'s UX notes) needs it even though it keeps no
    /// hit-test hover state of its own (see [`Session::pointer_hover`]).
    /// `None` before the first move, or once [`Session::pointer_leave`]
    /// says the pointer is off the canvas.
    pointer_position: Option<Point>,
    /// The Node tool's hit test found no node, handle or segment at the pointer, so a press
    /// there would start a node marquee (`specs/0053-node-marquee-selection/` UX notes: the
    /// minus badge with Ctrl held).
    node_hover_on_nothing: bool,
    /// The polite live region after a node marquee or Ctrl+A ("5 nodes selected."), valid
    /// while the node selection is the one it described (`0053` criterion 24).
    node_announcement: Option<node_marquee::NodeAnnouncement>,
    /// Counts node announcements, so the host speaks a repeated sentence again.
    node_announcement_seq: u32,
    /// The Shift, Ctrl and Alt modifiers' live state, as of the most recent
    /// [`Session::pointer_hover`] or [`Session::modifiers_changed`] call —
    /// the Select tool's resize/rotate live preview needs them at *render*
    /// time (`specs/0005-object-transform/specification.md`'s pivot-swap
    /// modifier; acceptance criteria 5, 17), when `draw_list` has no event
    /// of its own to read them from, and the lasso, the marquee's mode
    /// inversion and the cursor read Alt the same way (`advanced-selection`).
    held: Modifiers,
    /// Every object as the document held it at its current version, read once and shared by every
    /// frame and event until the document changes (`object_cache.rs`). A Select or Node-tool drag
    /// keeps its first read for its whole life (`specs/0009-unified-object-editing` criterion 15).
    object_cache: object_cache::ObjectCache,
    /// `window.devicePixelRatio` as of the last attach or resize, so the
    /// selection box can snap to whole device pixels
    /// (`edit-interaction-polish` criterion 65). Always positive and finite.
    device_pixel_ratio: f64,
    /// Whether the pointer button is down: set by a press, cleared by a
    /// release or [`Session::pointer_cancelled`]. Only the Escape cascade
    /// reads it, to stop at the drag while the button is held
    /// (`specs/0010-edit-interaction-polish/` criterion 49); the key gate reads
    /// each tool's own drag state instead.
    button_down: bool,
    /// The "r 12.0 mm max" notice shown for 1.5 s at a knob after a typed
    /// radius was limited (`specs/0013-rectangle-corner-radii/` criterion 6): a
    /// limit is never silent. The host clears it after the delay
    /// ([`Session::clear_limit_notice`]); a press clears it too.
    limit_notice: Option<shapes::LiveReadout>,
    /// The objects a refused rail command is drawn around (red, hollow, never stored), and
    /// the selection it was refused for: the outline ends with the selection or the tool.
    command_refusal: Option<refusal::RefusalMarks>,
    /// The notice of the last resize or typed size that set an axis to 0.01 mm instead of
    /// 0 (`specs/0056-mirror-and-negative-scale/` criterion 15), until the host takes it.
    transform_notice: Option<&'static str>,
    /// The announcement of the last resize that mirrored ("Resized. Mirrored left to
    /// right."), until the host takes it (`specs/0056-mirror-and-negative-scale/`).
    transform_announcement: Option<String>,
    /// The Pen's end-node cache and the target of the press in flight (`0034`).
    pen_cue: pen::PenCueState,
    /// The maker's undo and redo stacks of this session (`history.rs`).
    history: history::HistoryState,
    /// The arrow-key nudges of the held press that last moved the selection (`nudge.rs`).
    nudge_run: Option<curvyo_ui_core::NudgeRun>,
    /// The groups the maker has entered, outermost first (`grouping.rs`). View state: not saved.
    context: Vec<NodeId>,
    /// The objects and units of the context, kept for as long as the document read they were made
    /// from (`grouping.rs`).
    context_cache: context::ContextCache,
    /// What the last Group or Ungroup said, until the host has taken it (`grouping.rs`).
    grouping_result: Option<grouping::GroupingResult>,
    /// What a screen reader hears about the context, until the host has taken it.
    context_announcement: Option<String>,
    /// The active layer and the layer notice (`layers.rs`).
    layers: layers::LayerState,
    /// The Layers list's view state (`layers_panel.rs`).
    panel: layers_panel::PanelState,
    /// The Offset tool: its bar's values (session memory), drag and preview
    /// (`specs/0038-path-offset`).
    offset: offset::OffsetTool,
    /// The notice of the orphan repair after the Open, until the host has taken it
    /// (`clones.rs`).
    orphan_notice: Option<String>,
    /// The notice of the last style or opacity edit that left clones with their original's style
    /// alone, until the host has taken it (`clone_overrides.rs`).
    skip_notice: Option<String>,
    /// How many objects the last Delete key removed, for its notice (`clones.rs`).
    deleted_objects: usize,
    /// The DOM chips over the canvas, as the host reported them, which a tag of the clone marks
    /// may not meet (`clone_view.rs`).
    dom_chips: Vec<curvyo_ui_core::MarkRect>,
    /// The click-to-create placement and its press (`placement.rs`).
    placements: placement::PlacementState,
}

impl Session {
    /// A brand-new, empty document (File → New), bound to `peer`.
    #[must_use]
    pub fn new(peer: u64) -> Self {
        let document = Document::new(peer);
        Self {
            saved: document.version(),
            packed: None,
            document,
            minter: AnchorIdMinter::new(peer),
            pen: PenTool::new(),
            node: NodeTool::new(),
            rectangle: RectangleTool::new(),
            ellipse: EllipseTool::new(),
            poly_star: PolygonStarTool::new(),
            select: SelectTool::new(),
            selection: ObjectSelection::new(),
            // Select is the launch default for New and Open alike
            // (acceptance criterion 13; `specs/0004-canvas-navigation-and-
            // selection/adrs.md`: "Select is the launch tool for New and
            // for Open" — matching Inkscape's own default and replacing
            // `path-node-editing`'s provisional Pen default, which that
            // slice's own spec explicitly flagged "revisit once a general
            // selection tool exists").
            tool: Tool::Select,
            style: StyleEditor::default(),
            formats: Self::builtin_formats(),
            formats_broken: None,
            // A new or opened project starts with an empty selection
            // (`specs/0043-properties-tabs/` criterion 5, 11).
            panel_tabs: std::cell::Cell::new(curvyo_ui_core::PanelTabs::new(false)),
            colour_pick: colour_pick::ColourPick::default(),
            background_preview: None,
            viewport: Viewport::new(),
            hovered: None,
            hovered_object: None,
            history_hover: Vec::new(),
            pointer_position: None,
            node_hover_on_nothing: false,
            node_announcement: None,
            node_announcement_seq: 0,
            held: Modifiers::NONE,
            object_cache: object_cache::ObjectCache::default(),
            device_pixel_ratio: 1.0,
            button_down: false,
            limit_notice: None,
            command_refusal: None,
            transform_notice: None,
            transform_announcement: None,
            pen_cue: pen::PenCueState::default(),
            history: history::HistoryState::new(),
            nudge_run: None,
            context: Vec::new(),
            context_cache: context::ContextCache::default(),
            grouping_result: None,
            context_announcement: None,
            layers: layers::LayerState::default(),
            panel: layers_panel::PanelState::default(),
            offset: offset::OffsetTool::default(),
            orphan_notice: None,
            skip_notice: None,
            deleted_objects: 0,
            dom_chips: Vec::new(),
            placements: placement::PlacementState::default(),
        }
    }

    /// Reopens a previously saved `.curvyo` container's bytes (File → Open),
    /// bound to `peer`.
    ///
    /// # Errors
    /// See [`curvyo_document_core::unpack`].
    pub fn open(peer: u64, bytes: &[u8]) -> Result<Self, OpenError> {
        let document = curvyo_document_core::unpack(peer, bytes)?;
        let mut session = Self {
            saved: document.version(),
            packed: None,
            document,
            minter: AnchorIdMinter::new(peer),
            pen: PenTool::new(),
            node: NodeTool::new(),
            rectangle: RectangleTool::new(),
            ellipse: EllipseTool::new(),
            poly_star: PolygonStarTool::new(),
            select: SelectTool::new(),
            selection: ObjectSelection::new(),
            tool: Tool::Select,
            style: StyleEditor::default(),
            formats: Self::builtin_formats(),
            formats_broken: None,
            // A new or opened project starts with an empty selection
            // (`specs/0043-properties-tabs/` criterion 5, 11).
            panel_tabs: std::cell::Cell::new(curvyo_ui_core::PanelTabs::new(false)),
            colour_pick: colour_pick::ColourPick::default(),
            background_preview: None,
            viewport: Viewport::new(),
            hovered: None,
            hovered_object: None,
            history_hover: Vec::new(),
            pointer_position: None,
            node_hover_on_nothing: false,
            node_announcement: None,
            node_announcement_seq: 0,
            held: Modifiers::NONE,
            object_cache: object_cache::ObjectCache::default(),
            device_pixel_ratio: 1.0,
            button_down: false,
            limit_notice: None,
            command_refusal: None,
            transform_notice: None,
            transform_announcement: None,
            pen_cue: pen::PenCueState::default(),
            history: history::HistoryState::new(),
            nudge_run: None,
            context: Vec::new(),
            context_cache: context::ContextCache::default(),
            grouping_result: None,
            context_announcement: None,
            layers: layers::LayerState::default(),
            panel: layers_panel::PanelState::default(),
            offset: offset::OffsetTool::default(),
            orphan_notice: None,
            skip_notice: None,
            deleted_objects: 0,
            dom_chips: Vec::new(),
            placements: placement::PlacementState::default(),
        };
        // A file another tool wrote can hold clones whose original is gone (`0049` criterion 35).
        session.remove_orphans_after_open();
        Ok(session)
    }

    /// Packs the current document into `.curvyo` container bytes. Not for a save: use
    /// [`Session::pack_for_save`], which records the saved version.
    ///
    /// # Errors
    /// See [`curvyo_document_core::pack`].
    pub fn pack(&self, app_version: &str) -> Result<Vec<u8>, SaveError> {
        curvyo_document_core::pack(&self.document, app_version)
    }

    /// The active tool.
    #[must_use]
    pub const fn tool(&self) -> Tool {
        self.tool
    }

    /// Switches the active tool (`B`/`N`/`R`/`E`/`*` canvas-focus
    /// shortcuts, or the tool rail). Switching away from the pen tool
    /// mid-path does **not** discard it — only Escape or finishing does
    /// (`specification.md`'s pen tool is not itself scoped to stay
    /// active just because another tool was clicked; no acceptance
    /// criterion covers this edge, so the safer, less surprising choice
    /// — not silently losing work — is kept). Likewise, switching away
    /// from a shape tool mid-drag does not discard that drag either —
    /// no acceptance criterion exercises switching tools mid-drag, so
    /// the same conservative stance applies.
    pub fn set_tool(&mut self, tool: Tool) {
        self.recording(|this| {
            // Flushes any pending ratio-slider preview against the
            // selection it was actually previewed against, before anything
            // else can change that selection (architect re-verification: a
            // slider drag released outside the slider element never fires
            // the slider's own `pointerup`/`blur`, so without this the
            // preview would otherwise sit unflushed until some later event
            // commits it against whatever is selected *then* instead).
            this.flush_select_bar_preview();
            this.flush_style_preview();
            this.end_colour_pick();
            this.select.cancel_entry();
            this.select.forget_press();
            this.select.cancel_gesture();
            // Any tool switch, by any route, cancels a placement (`0052` criterion 7).
            this.cancel_placement();
            // Leaving the Pen ends its path as drawn (a lone node is dropped): no
            // unfinished path stays behind that the Document section's resize would
            // not move (`specs/0015-document-size-and-rulers/` criterion 14a).
            if this.tool == Tool::Pen && tool != Tool::Pen {
                this.pen.finish(&this.document);
                this.pen.escape();
            }
            // A creation tool starts from an empty selection: no selection box
            // stays behind from the Select tool. Creating a shape then selects
            // the new one (`shape_pointer_up`).
            if matches!(tool, Tool::Rectangle | Tool::Ellipse | Tool::PolygonStar) {
                this.selection.clear();
                this.hovered_object = None;
            }
            if tool != this.tool {
                this.command_refusal = None;
                this.node_announcement = None;
                this.leave_offset_tool();
            }
            this.tool = tool;
        });
    }

    /// The paths the Node and Pen tools work on: those of the entered group, or of the whole
    /// document at the root. A compound path is left out: its nodes cannot be edited yet
    /// (`specs/0016-boolean-operations` criteria 38 and 38a), so it contributes no node, handle or
    /// segment.
    fn paths(&self) -> Vec<curvyo_document_core::PathSnapshot> {
        // A hidden or locked path is no target of the Node tool or the Pen
        // (`specs/0050-layers-and-objects-panel` criteria 19 and 20).
        self.pickable_objects()
            .iter()
            .filter_map(|object| match object {
                ObjectSnapshot::Path(path) if !path.is_compound() => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    /// The pointer went down at `point` (document space).
    pub fn pointer_down(&mut self, point: Point, shift: bool) {
        self.recording(|this| {
            let Some(point) = sanitized_point(point) else {
                return;
            };
            // A press during a preview only ends it (`specs/0042-history-branches` criterion 18).
            if this.history_preview_takes_press() {
                return;
            }
            // Same flush as `set_tool`'s own doc comment explains: a canvas
            // click can change the selection (e.g. selecting a different
            // star) before a pending ratio-slider preview ever gets a
            // chance to commit against the selection it was previewed
            // against, if the mouse was released outside the slider itself.
            this.flush_select_bar_preview();
            this.flush_style_preview();
            if this.colour_pick_press(point) {
                return;
            }
            // During a placement a press places or is ignored; it starts no drag (`0052`
            // criteria 4, 5, 22).
            if this.placement().is_some() {
                this.button_down = true;
                let modifiers = Modifiers { shift, ..this.held };
                this.placement_press(point, modifiers);
                return;
            }
            if this.refuse_drawing_into_layer() {
                return;
            }
            this.button_down = true;
            match this.tool {
                Tool::Select => {
                    this.select_pointer_down(point, shift);
                }
                Tool::Pen => {
                    this.pen_pointer_down(point, shift);
                }
                Tool::Node => {
                    let paths = this.paths();
                    let tolerances = this.hit_tolerances();
                    this.node_announcement = None;
                    this.node.pointer_down(&paths, point, tolerances, shift);
                }
                Tool::Rectangle | Tool::Ellipse | Tool::PolygonStar => {
                    this.note_shape_press(point);
                    this.shape_pointer_down(point);
                }
                Tool::Offset => {
                    this.offset_pointer_down(point, shift);
                }
            }
        });
    }

    /// The pointer moved to `point`. Always records `point` as the live
    /// cursor position — [`Session::draw_list`]'s pen-tool rubber-band
    /// preview needs it (`specification.md`'s UX notes) even though the
    /// pen tool keeps no hit-test hover state of its own. For the node
    /// tool, additionally updates hover state for the hover ring (same
    /// UX notes); for a creation tool, feeds the create-drag in flight
    /// for the live preview (ux-engineer review) and nothing else: no
    /// hover state (`unified-object-editing` criterion 25).
    /// `constrain` is the Ctrl modifier's current state, consulted only
    /// by the rectangle/ellipse tools' create-drag preview (acceptance
    /// criteria 2, 8; `shift` centres the box on the press point,
    /// `shape-creation-from-center`) and — since `object-transform` — by the Select
    /// tool's own live resize/rotate preview, alongside `shift`
    /// (acceptance criteria 5, 7, 16, 17); both are cached
    /// (`held`) so [`Session::draw_list`]
    /// can read their current state with no event of its own.
    pub fn pointer_hover(&mut self, point: Point, shift: bool, constrain: bool) {
        let Some(point) = sanitized_point(point) else {
            return;
        };
        self.pointer_position = Some(point);
        self.hovered = None;
        self.hovered_object = None;
        self.held.shift = shift;
        self.held.ctrl = constrain;
        if self.colour_pick_move(point) {
            return;
        }
        match self.tool {
            Tool::Select => {
                self.select_hover(point, self.held);
            }
            Tool::Node => {
                // A bend or a marquee in flight follows the pointer; no hover test runs during
                // any drag (`0031` criterion 17, `0053` criterion 10).
                let paths = self.paths();
                let threshold = self.hit_tolerances().drag_threshold;
                self.node.pointer_moved(&paths, point, shift, threshold);
                self.node_hover_on_nothing = false;
                if !self.node.drag_in_flight() {
                    self.node_hover_on_nothing = self.node_hit_is_nothing(&paths, point);
                    self.hovered = self.hovered_segment_hit(&paths, point);
                }
            }
            Tool::Rectangle | Tool::Ellipse | Tool::PolygonStar => {
                if self.placement().is_some() {
                    self.placement_follow(point, Modifiers::new(shift, constrain));
                } else {
                    self.shape_pointer_move(point, Modifiers::new(shift, constrain));
                }
            }
            Tool::Offset => {
                self.offset_hover(point);
            }
            Tool::Pen => {}
        }
    }

    /// The pointer left the canvas entirely — clears the live cursor
    /// position so the pen tool's rubber-band preview disappears rather
    /// than sticking at the last position inside the canvas.
    pub fn pointer_leave(&mut self) {
        self.pointer_position = None;
        self.hovered = None;
    }

    /// The pointer released at `point`, ending whatever gesture
    /// [`Session::pointer_down`] began. `constrain` is the Ctrl
    /// modifier's state at release — consulted by the rectangle and
    /// ellipse tools (acceptance criteria 2, 8) and, since
    /// `object-transform`, by the Select tool's own resize/rotate commit
    /// alongside `shift` (acceptance criteria 5, 7, 16, 17).
    pub fn pointer_up(&mut self, point: Point, shift: bool, constrain: bool) {
        self.recording(|this| {
            this.button_down = false;
            if this.colour_pick_release() {
                return;
            }
            let Some(point) = sanitized_point(point) else {
                // A release at a non-finite position cannot be committed to
                // anything; cancel the gesture rather than write NaN.
                this.cancel_gesture();
                return;
            };
            match this.tool {
                Tool::Select => {
                    this.select_pointer_up(point, shift, constrain);
                }
                Tool::Pen => {
                    let threshold = this.drag_threshold();
                    this.pen
                        .pointer_up(&mut this.minter, &this.document, point, threshold);
                    this.pen_cue.release();
                }
                Tool::Node => {
                    this.node_pointer_up(point, Modifiers::new(shift, constrain));
                }
                Tool::Rectangle | Tool::Ellipse | Tool::PolygonStar => {
                    this.shape_pointer_up(point, Modifiers::new(shift, constrain));
                }
                Tool::Offset => {
                    this.offset_pointer_up(point, shift, constrain);
                }
            }
        });
    }

    /// The one double-click dispatch point
    /// (`specs/0004-canvas-navigation-and-selection/adrs.md`: "the host
    /// detects a double-click... and calls `double_click(x, y)`.
    /// `Session` dispatches it"): the pen tool finishes its in-progress
    /// path (acceptance criterion 3), the node tool inserts a node on the
    /// hit segment (acceptance criterion 12, via [`Session::insert_at`]),
    /// the Select tool opens a handle's typed entry, hands a path off to
    /// the Node tool or, on a primitive, changes nothing
    /// (`specs/0009-unified-object-editing/` criteria 31 to 34), and the
    /// creation tools ignore it (the first click of a double-click was an
    /// ordinary press with no movement, which writes nothing). Returns
    /// whether the host should show the edit hint chip (criterion 32). The shorthand of the
    /// tests that predate [`Session::double_click_hint`], which the wasm facade calls.
    pub fn double_click(&mut self, point: Point, shift: bool, ctrl: bool) -> bool {
        self.double_click_hint(point, shift, ctrl) == DoubleClickHint::EditHint
    }

    /// [`Session::double_click`], telling the host which hint to show, if any.
    pub fn double_click_hint(&mut self, point: Point, shift: bool, ctrl: bool) -> DoubleClickHint {
        self.recording(|this| -> DoubleClickHint {
            let Some(point) = sanitized_point(point) else {
                return DoubleClickHint::None;
            };
            match this.tool {
                Tool::Pen => this.finish_pen(),
                Tool::Node => this.insert_at(point),
                Tool::Select => return this.select_double_click(point, shift, ctrl),
                Tool::Offset => return this.offset_double_click(point, shift, ctrl),
                Tool::Rectangle | Tool::Ellipse | Tool::PolygonStar => {}
            }
            DoubleClickHint::None
        })
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorKind, Vec2};
    use curvyo_ui_core::NodeToolbarState;

    use super::*;

    /// Acceptance criterion 13: Select, not Pen, is the launch default —
    /// `path-node-editing`'s own Pen default was provisional, explicitly
    /// flagged "revisit once a general selection tool exists".
    #[test]
    fn a_new_session_defaults_to_the_select_tool_on_an_empty_document() {
        let session = Session::new(1);
        assert_eq!(session.tool(), Tool::Select);
        assert_eq!(session.document.object_ids(), Vec::new());
    }

    #[test]
    fn drawing_an_open_path_with_the_pen_tool_then_reading_it_back() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(10.0, 0.0), false, false);
        session.finish_pen();

        let paths = session.paths();
        assert_eq!(paths.len(), 1);
        assert!(!paths[0].closed);
    }

    #[test]
    fn escape_discards_the_in_progress_pen_path_only() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.escape();
        assert_eq!(session.document.object_ids(), Vec::new());
    }

    #[test]
    fn switching_to_the_node_tool_selects_and_edits_a_finished_path() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(10.0, 0.0), false, false);
        session.finish_pen();

        session.set_tool(Tool::Node);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(5.0, 5.0), false, false);

        let paths = session.paths();
        assert_eq!(paths[0].anchors[0].point, Point::new(5.0, 5.0));
    }

    #[test]
    fn convert_and_delete_dispatch_only_when_the_node_tool_is_active() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(10.0, 0.0), false, false);
        session.finish_pen();

        // Pen tool is still active: these are no-ops.
        session.convert_selected(AnchorKind::Symmetric);
        session.delete_selected();
        assert_eq!(session.paths()[0].anchors.len(), 2);

        session.set_tool(Tool::Node);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.convert_selected(AnchorKind::Symmetric);
        assert_eq!(session.paths()[0].anchors[0].kind, AnchorKind::Symmetric);
    }

    #[test]
    fn draw_list_is_empty_for_a_brand_new_document() {
        let session = Session::new(1);
        assert_eq!(session.draw_list().triangles.len(), 0);
    }

    #[test]
    fn draw_list_includes_geometry_once_a_path_exists() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(10.0, 0.0), false, false);
        session.finish_pen();
        assert_ne!(session.draw_list().triangles.len(), 0);
    }

    /// The pen tool's in-progress preview (not yet committed) also shows
    /// up in `draw_list`, and tracks the live pointer position via
    /// `pointer_hover` for its rubber-band line.
    #[test]
    fn draw_list_includes_the_pen_tools_in_progress_preview() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        let empty = session.draw_list().triangle_count();

        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        let one_node = session.draw_list().triangle_count();
        assert!(one_node > empty, "the placed node's glyph/hover ring draw");

        session.pointer_hover(Point::new(10.0, 0.0), false, false);
        let with_rubber_band = session.draw_list().triangle_count();
        assert!(
            with_rubber_band > one_node,
            "the rubber-band line to the cursor adds geometry"
        );

        session.pointer_leave();
        let after_leave = session.draw_list().triangle_count();
        assert_eq!(after_leave, one_node, "no cursor, no rubber-band line");
    }

    /// The bug this run fixes: acceptance criterion 2's live
    /// drag-to-curve preview — dragging while placing a new node (mouse
    /// held down, not yet released) must render strictly more than the
    /// plain rubber-band line a hover alone draws, through the actual
    /// `Session::draw_list` path the host calls every frame (not just
    /// `curvyo-render-core`'s own unit test of `build_pen_preview`
    /// directly).
    #[test]
    fn draw_list_shows_the_live_curve_preview_during_a_pen_drag() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);

        // Press down at C and move the cursor without releasing — a drag
        // in flight, same as `pointer_up`'s own AC2 test fixture.
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_hover(Point::new(10.0, 0.0), false, false);
        let press_with_no_movement_yet = session.draw_list().triangle_count();

        session.pointer_hover(Point::new(13.0, 4.0), false, false);
        let mid_drag = session.draw_list().triangle_count();
        assert!(
            mid_drag > press_with_no_movement_yet,
            "the live curve segment and C's growing handle lines/endpoints must add geometry \
             as the drag moves, not just a static rubber-band line"
        );
    }

    /// The bug this run fixes: acceptance criteria 8/10's "the two
    /// adjoining segments update live during the drag" — dragging a
    /// selected node with the node tool (mouse held down, not yet
    /// released) must already draw the node at its live position,
    /// through the actual `Session::draw_list` path, not only after
    /// `pointer_up` commits it.
    #[test]
    fn draw_list_shows_the_live_node_position_during_a_node_drag() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(10.0, 0.0), false, false);
        session.finish_pen();

        session.set_tool(Tool::Node);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_hover(Point::new(0.0, 0.0), false, false);
        let press_with_no_movement_yet = session.draw_list();

        session.pointer_hover(Point::new(40.0, 40.0), false, false);
        let mid_drag = session.draw_list();

        // The committed document must not have moved yet — this is a
        // rendering-only preview (ADR 0009 §2: ephemeral, not written).
        assert_eq!(session.paths()[0].anchors[0].point, Point::new(0.0, 0.0));
        assert_ne!(
            mid_drag, press_with_no_movement_yet,
            "the dragged node (and the segment reshaping with it) must draw at its live \
             position mid-drag, not the stale committed one"
        );

        // On release, the commit matches what was just being previewed.
        session.pointer_up(Point::new(40.0, 40.0), false, false);
        assert_eq!(session.paths()[0].anchors[0].point, Point::new(40.0, 40.0));
    }

    /// Same bug, the handle-drag half (acceptance criterion 9): dragging
    /// a selected node's handle must show it (and, for a smooth node, its
    /// mirrored opposite) at the live value mid-drag.
    #[test]
    fn draw_list_shows_the_live_handle_value_during_a_handle_drag() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        // AC2: a click-drag places a smooth node with symmetric handles.
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(13.0, 4.0), false, false);
        session.finish_pen();

        session.set_tool(Tool::Node);
        // Select the node first — handles are only hittable once selected.
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(10.0, 0.0), false, false);
        let selected_not_dragging = session.draw_list();

        // Press on the handle endpoint (anchor + handle_out, (13, 4)) and
        // drag it without releasing.
        session.pointer_down(Point::new(13.0, 4.0), false);
        session.pointer_hover(Point::new(20.0, 8.0), false, false);
        let mid_drag = session.draw_list();

        assert_eq!(
            session.paths()[0].anchors[1].handle_out,
            Vec2::new(3.0, 4.0),
            "not committed yet"
        );
        assert_ne!(
            mid_drag, selected_not_dragging,
            "the dragged handle (and its mirrored opposite) must draw at its live value \
             mid-drag"
        );
    }

    /// Resets `session`'s viewport to an identity-equivalent view (1
    /// screen px per document mm, origin at the document origin) — these
    /// tests were written and pinned against `ViewTransform::
    /// identity()`, back when `Session`'s only view was a bare,
    /// never-defaulted-to-100%-zoom `ViewTransform`. `canvas-navigation-
    /// and-selection` gives `Session` a real `Viewport` defaulting to
    /// 100% zoom (`Zoom::default()`, acceptance criterion 7's `96/25.4`
    /// px/mm) instead, so the exact-pixel-distance reasoning these two
    /// tests pin needs an explicit identity view rather than relying on
    /// the session's own default.
    fn reset_to_identity_view(session: &mut Session) {
        session.viewport = Viewport::new();
        session
            .viewport
            .zoom_about(0.0, 0.0, 1.0 / curvyo_ui_core::PX_PER_MM_AT_100);
    }

    /// Tester verification (PR #20, handles-doubled fix, 2026-10-05):
    /// `HANDLE_TOLERANCE_PX` is 16.0 (was 8.0 before that round; by this
    /// round `POINT_TOLERANCE_PX` is 16.0 too, but was still 8.0 when
    /// this test was written); at the identity view (1 screen px per
    /// document mm, `ViewTransform::identity`) a click 13px from a
    /// selected smooth node's handle endpoint — outside the old 8px
    /// radius, inside the new 16px one — must register as a handle hit.
    /// The click point is also kept far from the anchor itself (~23.8px,
    /// still outside even the now-doubled 16px node tolerance) and from
    /// the segment, so this cannot pass by accidentally hitting
    /// something else.
    #[test]
    fn a_click_13px_from_a_handle_hits_under_the_doubled_tolerance() {
        let mut session = Session::new(1);
        reset_to_identity_view(&mut session);
        session.set_tool(Tool::Pen);
        // A at (0, 0), a plain corner click.
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        // B at (50, 0), dragged so its handle_out lands at (50, 20) —
        // a handle endpoint 20px straight up from B.
        session.pointer_down(Point::new(50.0, 0.0), false);
        session.pointer_up(Point::new(50.0, 20.0), false, false);
        session.finish_pen();

        session.set_tool(Tool::Node);
        // Select B first: handles are only hittable on a selected node.
        session.pointer_down(Point::new(50.0, 0.0), false);
        session.pointer_up(Point::new(50.0, 0.0), false, false);

        let b = session.paths()[0].anchors[1].id;
        assert_eq!(
            session.paths()[0].anchors[1].handle_out,
            Vec2::new(0.0, 20.0),
            "handle endpoint is at document (50, 20)"
        );

        // 13px from the handle endpoint (50, 20); ~23.8px from B itself
        // and far from the A-B segment, so only the handle tolerance can
        // explain a hit here.
        session.pointer_hover(Point::new(63.0, 20.0), false, false);
        assert_eq!(
            session.hovered,
            Some(Hit::Handle {
                path: session.paths()[0].id,
                anchor: b,
                slot: curvyo_document_core::HandleSlot::Out,
            }),
            "13px is outside the old 8px handle radius but inside the new 16px one"
        );
    }

    /// Pins the node-size round's own hit-test doubling, the same way
    /// `a_click_13px_from_a_handle_hits_under_the_doubled_tolerance`
    /// pins the earlier handle one: `POINT_TOLERANCE_PX` is 16.0 now
    /// (was 8.0). At the identity view, a click 13px from node A (a
    /// classic 5-12-13 offset) — outside the old 8px radius, inside the
    /// new 16px one — must register as a node hit. B sits far away
    /// (200, 0) so neither it nor the long A-B segment can explain a hit
    /// here; only the node tolerance can.
    #[test]
    fn a_click_13px_from_a_node_hits_under_the_doubled_tolerance() {
        let mut session = Session::new(1);
        reset_to_identity_view(&mut session);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(200.0, 0.0), false);
        session.pointer_up(Point::new(200.0, 0.0), false, false);
        session.finish_pen();

        session.set_tool(Tool::Node);
        let a = session.paths()[0].anchors[0].id;

        // 13px from A (0, 0) — a 5-12-13 offset, well clear of the
        // 200px-long A-B segment and of B itself.
        session.pointer_hover(Point::new(5.0, 12.0), false, false);
        assert_eq!(
            session.hovered,
            Some(Hit::Node {
                path: session.paths()[0].id,
                anchor: a,
            }),
            "13px is outside the old 8px node radius but inside the new 16px one"
        );
    }

    #[test]
    fn pack_then_open_round_trips_a_drawn_path() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(10.0, 0.0), false, false);
        session.finish_pen();

        let bytes = session.pack("0.1.0").expect("pack");
        let reopened = Session::open(2, &bytes).expect("open");
        assert_eq!(reopened.paths().len(), 1);
    }

    /// Nothing selected, pen tool active: every toolbar action disabled.
    #[test]
    fn node_toolbar_state_is_all_false_outside_the_node_tool() {
        let session = Session::new(1);
        assert_eq!(session.node_toolbar_state(), NodeToolbarState::default());
    }

    /// A selected node enables delete/convert but not insert/make-line/
    /// make-curve (those need a segment selection).
    #[test]
    fn node_toolbar_state_for_a_selected_node() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(10.0, 0.0), false, false);
        session.finish_pen();

        session.set_tool(Tool::Node);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);

        let state = session.node_toolbar_state();
        assert!(state.can_delete);
        assert!(state.can_convert_to_corner);
        assert!(state.can_convert_to_symmetric);
        assert!(!state.can_insert);
        assert!(!state.can_make_line);
        assert!(!state.can_make_curve);
    }

    /// A selected line segment enables insert and make-curve, not
    /// make-line; `insert_selected` then splits it and clears the
    /// selection.
    #[test]
    fn node_toolbar_state_and_insert_selected_for_a_line_segment() {
        // A long segment: its midpoint sits well outside the 16px/mm
        // point-hit tolerance around either endpoint (2026-10-05: was
        // 8px/mm, doubled alongside the node glyph), so the click below
        // lands on the segment itself rather than being read as a node
        // hit of whichever endpoint happens to be nearest.
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(100.0, 0.0), false);
        session.pointer_up(Point::new(100.0, 0.0), false, false);
        session.finish_pen();

        session.set_tool(Tool::Node);
        session.pointer_down(Point::new(50.0, 0.0), false);
        session.pointer_up(Point::new(50.0, 0.0), false, false);

        let state = session.node_toolbar_state();
        assert!(state.can_insert);
        assert!(!state.can_make_line, "already a line");
        assert!(state.can_make_curve);
        assert!(!state.can_delete, "a segment, not a node, is selected");

        session.insert_selected();
        assert_eq!(session.paths()[0].anchors.len(), 3);
        // Nothing is selected after the insert; only the counts of `0053` criterion 23 remain.
        assert_eq!(
            session.node_toolbar_state(),
            NodeToolbarState {
                join_block: Some(curvyo_ui_core::JoinBlock::Count(0)),
                total_nodes: 3,
                ..NodeToolbarState::default()
            }
        );
    }

    /// Acceptance criteria 8-11: selecting the two ends of one open path
    /// and triggering Join through the whole `Session` surface closes
    /// it, as one merged node.
    #[test]
    fn join_selected_closes_an_open_path_through_the_session() {
        // Far enough apart that the third click does not land inside the
        // (doubled, 16px/mm at this identity view) close-path tolerance
        // around the first node.
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(50.0, 0.0), false);
        session.pointer_up(Point::new(50.0, 0.0), false, false);
        session.pointer_down(Point::new(25.0, 50.0), false);
        session.pointer_up(Point::new(25.0, 50.0), false, false);
        session.finish_pen();

        session.set_tool(Tool::Node);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(25.0, 50.0), true);
        session.pointer_up(Point::new(25.0, 50.0), false, false);
        assert!(session.node_toolbar_state().can_join);

        session.join_selected();

        let paths = session.paths();
        assert_eq!(paths.len(), 1, "still one object, now closed");
        assert!(paths[0].closed);
        assert_eq!(paths[0].anchors.len(), 2);
    }

    /// Acceptance criteria 12-15: splitting an interior node through the
    /// whole `Session` surface produces two objects.
    #[test]
    fn split_selected_on_an_interior_node_through_the_session() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(10.0, 0.0), false, false);
        session.pointer_down(Point::new(20.0, 0.0), false);
        session.pointer_up(Point::new(20.0, 0.0), false, false);
        session.finish_pen();

        session.set_tool(Tool::Node);
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(10.0, 0.0), false, false);
        assert!(session.node_toolbar_state().can_split);

        session.split_selected();

        assert_eq!(session.paths().len(), 2, "two separate objects now");
        // `edit-interaction-polish` criteria 50 and 52 (superseding `0006`
        // criterion 15): exactly one of the two coincident nodes is selected,
        // so Join (needs two) and Split (needs an interior node) are off.
        let state = session.node_toolbar_state();
        assert!(!state.can_join && !state.can_split);
        assert!(state.can_delete, "one node is selected");
        // A drag from the shared point moves one end only.
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(10.0, 8.0), false, false);
        let ends: Vec<Point> = session
            .paths()
            .iter()
            .flat_map(|path| {
                [
                    path.anchors[0].point,
                    path.anchors[path.anchors.len() - 1].point,
                ]
            })
            .filter(|p| (p.x - 10.0).abs() < 1e-9)
            .collect();
        assert_eq!(ends.len(), 2, "both pieces still end at x = 10");
        assert!(ends.contains(&Point::new(10.0, 0.0)));
        assert!(ends.contains(&Point::new(10.0, 8.0)));
    }

    /// `specs/0006-path-merge-split-and-node-types/specification.md`
    /// acceptance criteria 6, 7, 9: the real UI flow, through the whole
    /// `Session` surface — select two *pre-existing*, unrelated path
    /// objects with the Select tool (shift-click, `canvas-navigation-
    /// and-selection`), switch to the Node tool (a rail click / `N`, not
    /// the double-click handoff), click one endpoint and shift-click an
    /// endpoint on the *other* visible path, then Join — merging two
    /// objects that were never touched by Split at all, unlike every
    /// other Join test in this file.
    #[test]
    fn select_two_objects_then_join_their_endpoints_across_paths() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(50.0, 0.0), false);
        session.pointer_up(Point::new(50.0, 0.0), false, false);
        session.finish_pen();

        session.pointer_down(Point::new(0.0, 100.0), false);
        session.pointer_up(Point::new(0.0, 100.0), false, false);
        session.pointer_down(Point::new(50.0, 100.0), false);
        session.pointer_up(Point::new(50.0, 100.0), false, false);
        session.finish_pen();

        assert_eq!(session.paths().len(), 2, "two separate, unrelated objects");

        // Select tool: shift-click selects both objects together
        // (acceptance criterion 17).
        session.set_tool(Tool::Select);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(0.0, 100.0), true);
        session.pointer_up(Point::new(0.0, 100.0), false, false);

        // Switch to the Node tool via the rail/shortcut, not a double-
        // click — acceptance criterion 6: every selected path object's
        // nodes are visible and editable in this one Node-tool session.
        session.set_tool(Tool::Node);
        let paths = session.paths();
        assert_eq!(paths.len(), 2, "both objects still exist, unmerged so far");

        // Click one endpoint, then shift-click an endpoint on the
        // *other* visible path (acceptance criterion 7).
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(0.0, 100.0), true);
        session.pointer_up(Point::new(0.0, 100.0), false, false);

        assert!(
            session.node_toolbar_state().can_join,
            "two endpoint nodes of two different open path objects: joinable (AC 8, 9)"
        );

        session.join_selected();

        let paths = session.paths();
        assert_eq!(paths.len(), 1, "the two objects merged into one (AC 9)");
        assert_eq!(
            paths[0].anchors.len(),
            3,
            "2 + 2 anchors, minus the merged pair"
        );
        assert!(!paths[0].closed);
    }

    /// `docs/design-system.md`: path nodes are drawn by the Node tool
    /// only — not in Select (or any other) tool, where they used to
    /// appear as grey squares on every path.
    #[test]
    fn path_nodes_are_drawn_only_while_the_node_tool_is_active() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(50.0, 0.0), false);
        session.pointer_up(Point::new(50.0, 0.0), false, false);
        session.finish_pen();

        session.set_tool(Tool::Select);
        assert!(!session.decoration_input().show_nodes);
        let select_triangles = session.draw_list().triangle_count();

        session.set_tool(Tool::Node);
        assert!(session.decoration_input().show_nodes);
        // Two nodes, each outline + fill quad (4 triangles per node).
        assert_eq!(session.draw_list().triangle_count(), select_triangles + 8);
    }

    /// The bug this round fixes: `decoration_input()` used to zip
    /// `selection.nodes()` against `selection.path()`, which is `None`
    /// for a genuine cross-path selection — so neither of two selected
    /// nodes on two different paths ever reached `DecorationInput`, and
    /// neither drew as selected. Confirms both land in `selected_nodes`
    /// now, directly, without rendering a frame.
    #[test]
    fn decoration_input_includes_every_selected_node_across_two_different_paths() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(50.0, 0.0), false);
        session.pointer_up(Point::new(50.0, 0.0), false, false);
        session.finish_pen();
        session.pointer_down(Point::new(0.0, 100.0), false);
        session.pointer_up(Point::new(0.0, 100.0), false, false);
        session.pointer_down(Point::new(50.0, 100.0), false);
        session.pointer_up(Point::new(50.0, 100.0), false, false);
        session.finish_pen();
        let (path_a, path_b) = {
            let paths = session.paths();
            (paths[0].id, paths[1].id)
        };

        session.set_tool(Tool::Node);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        session.pointer_down(Point::new(0.0, 100.0), true);
        session.pointer_up(Point::new(0.0, 100.0), false, false);

        let input = session.decoration_input();
        assert_eq!(input.selected_nodes.len(), 2);
        assert!(input.selected_nodes.iter().any(|&(p, _)| p == path_a));
        assert!(input.selected_nodes.iter().any(|&(p, _)| p == path_b));
    }
}

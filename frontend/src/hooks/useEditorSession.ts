import { createSketchPresentation, type SketchPresentation } from "@/lib/sketchPresentation";
import { type SketchView } from "@/lib/sketchView";
import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from "react";
import { flushSync } from "react-dom";

import { type CanvasNotice, type HistoryNames, MAC, useHistory } from "@/hooks/useHistory";
import { type ClonesApi, useClones } from "@/hooks/useClones";
import { type StrokeToPathApi, useStrokeToPath } from "@/hooks/useStrokeToPath";
import { type RecogniseApi, useRecognise } from "@/hooks/useRecognise";
import { type TraceApi, useTrace } from "@/hooks/useTrace";
import { ViewRest } from "@/lib/viewRest";
import { isStrokeToPathChord } from "@/lib/strokeToPathText";
import { type GroupingApi, useGrouping } from "@/hooks/useGrouping";
import { type OffsetApi, readOffsetView, useOffset } from "@/hooks/useOffset";
import { type TextApi, fontListingReady, sendFontLists, useText } from "@/hooks/useText";
import { type PenEntryApi, usePenEntry } from "@/hooks/usePenEntry";
import { type PlacementApi, usePlacement } from "@/hooks/usePlacement";
import { offsetKeyOutcome, offsetReadout } from "@/lib/offsetText";
import type { HistoryDirection } from "@/lib/historyText";
import { type NudgeFeedback, useNudgeFeedback } from "@/hooks/useNudgeFeedback";
import { type BackingBuffer, backingBufferFor, sameBackingBuffer } from "@/lib/canvasSize";
import { ModifierTracker } from "@/lib/keyModifiers";
import { parseLayerNotice } from "@/lib/layerRows";
import { FLIP_NOTICE_MS } from "@/lib/flipText";
import { canvasLeavesKey } from "@/lib/fontPreviewKeys";
import { type ArcBarApi, useArcBar } from "@/hooks/useArcBar";
import { type FlipApi, useFlip } from "@/hooks/useFlip";
import { TOO_FAR_HINT } from "@/lib/nudgeText";
import { createSession, openFileSession, openSession } from "@/lib/editorSession";
import type { WasmSession } from "@/lib/editorSession";
import { freeCanvasArea, type GuideState, readGuideState } from "@/lib/guideText";
import type { EditHint } from "@/components/EditHintChip";
import type { BooleanOp, BooleanResult } from "@/lib/booleanText";
import type { BreakApartResult, CombineResult, CutResult, SplitResult } from "@/lib/pathText";
import type { StackCommand, StackResult } from "@/lib/stackText";
import type { WeldResult } from "@/lib/weldText";
import { joinNoticeText } from "@/lib/nodeText";
import type { ClosePathResult } from "@/lib/penText";
import { type LineCode, type SideCode, type TextPathEntry, parseTextPathEntry } from "@/lib/textPathText";

/** The canvas's backing-buffer size for a CSS size at the window's current
 * `devicePixelRatio` (`backingBufferFor`). */
function backingBufferSize(cssWidth: number, cssHeight: number): BackingBuffer {
  return backingBufferFor(cssWidth, cssHeight, window.devicePixelRatio);
}

/** Which tool is active (`specification.md`'s tool rail: Select is first
 * — `canvas-navigation-and-selection`'s launch default, acceptance
 * criterion 13 — then Pen, Node, Rectangle, Ellipse, Polygon/Star). */
export type Tool =
  | "select"
  | "pen"
  | "node"
  | "rectangle"
  | "ellipse"
  | "polygon-star"
  | "offset"
  | "text"
  | "trace";

/** The polygon/star tool-options bar's mode toggle (acceptance criteria
 * 11 vs. 12). */
export type PolyStarMode = "polygon" | "star";

/** How bytes came into an import: they pick the History name and the name of a file that has
 * none (`specs/0087-image-clipboard-and-drop` `adrs.md` decision 3). */
export type ImportOrigin = "picker" | "drop" | "paste" | "paste-files";

/** A plain-JS copy of the Rust `NodeToolbarState` — read out of the
 * wasm-bindgen struct instance once, immediately, so the instance itself
 * can be `free()`d rather than held onto (see `readToolbarState` below). */
export interface NodeToolbarState {
  canInsert: boolean;
  canDelete: boolean;
  canConvertToCorner: boolean;
  canConvertToSymmetric: boolean;
  canConvertToAsymmetric: boolean;
  canMakeLine: boolean;
  canMakeCurve: boolean;
  /** Join (acceptance criterion 8). */
  canJoin: boolean;
  /** Split (acceptance criterion 12). */
  canSplit: boolean;
  cloneOnly: boolean;
  /** Why Join is disabled (`0053` criterion 19): 0 enabled, 1 count, 2 not end nodes, 3 other. */
  joinBlock: number;
  /** With `joinBlock` 2: how many selected nodes are not an end node. */
  joinNotEnd: number;
  /** The count of `0053` criterion 23. */
  selectedNodes: number;
  totalNodes: number;
  segmentSelected: boolean;
}

const EMPTY_TOOLBAR_STATE: NodeToolbarState = {
  canInsert: false,
  canDelete: false,
  canConvertToCorner: false,
  canConvertToSymmetric: false,
  canConvertToAsymmetric: false,
  canMakeLine: false,
  canMakeCurve: false,
  canJoin: false,
  canSplit: false,
  cloneOnly: false,
  joinBlock: 0,
  joinNotEnd: 0,
  selectedNodes: 0,
  totalNodes: 0,
  segmentSelected: false,
};

/** A plain-JS copy of the Rust `SelectBarView` (`specs/0009-unified-object-editing/`
 * criteria 21, 21a, 22): which kind controls the Select bar shows and their
 * values. A `*Mixed` flag means the selected objects differ (the field is
 * empty with the placeholder "Mixed"). */
export interface SelectBarState {
  radiusShown: boolean;
  radiusMixed: boolean;
  /** The effective radius, millimetres. */
  radius: number;
  /** The stored radius exceeds what the rectangle allows ("limited"). */
  radiusLimited: boolean;
  /** The stored radius, millimetres, for the tooltip of "limited". */
  radiusStored: number;
  /** One rectangle with unequal corners: its four effective radii in its own
   * frame, millimetres (top-left, top-right, bottom-right, bottom-left) for the
   * "Mixed" tooltip (`specs/0013-rectangle-corner-radii/` criterion 22); `null`
   * otherwise. */
  radiusCorners: [number, number, number, number] | null;
  removeRoundingShown: boolean;
  removeRoundingEnabled: boolean;
  pointsShown: boolean;
  pointsMixed: boolean;
  points: number;
  ratioShown: boolean;
  ratioMixed: boolean;
  ratio: number;
  objectToPathShown: boolean;
  scaleSwitchesShown: boolean;
  /** Stroke to path is enabled (`specs/0085-stroke-to-path` criterion 1). */
  strokeToPathEnabled: boolean;
}

const EMPTY_SELECT_BAR_STATE: SelectBarState = {
  radiusShown: false,
  radiusMixed: false,
  radius: 0,
  radiusLimited: false,
  radiusStored: 0,
  radiusCorners: null,
  removeRoundingShown: false,
  removeRoundingEnabled: false,
  pointsShown: false,
  pointsMixed: false,
  points: 0,
  ratioShown: false,
  ratioMixed: false,
  ratio: 0,
  objectToPathShown: false,
  scaleSwitchesShown: true,
  strokeToPathEnabled: false,
};

function readSelectBar(raw: {
  radius_shown: boolean;
  radius_mixed: boolean;
  radius: number;
  radius_limited: boolean;
  radius_stored: number;
  radius_corners_shown: boolean;
  radius_tl: number;
  radius_tr: number;
  radius_br: number;
  radius_bl: number;
  remove_rounding_shown: boolean;
  remove_rounding_enabled: boolean;
  points_shown: boolean;
  points_mixed: boolean;
  points: number;
  ratio_shown: boolean;
  ratio_mixed: boolean;
  ratio: number;
  object_to_path_shown: boolean;
  scale_switches_shown: boolean;
  stroke_to_path_enabled: boolean;
  free(): void;
}): SelectBarState {
  const state: SelectBarState = {
    radiusShown: raw.radius_shown,
    radiusMixed: raw.radius_mixed,
    radius: raw.radius,
    radiusLimited: raw.radius_limited,
    radiusStored: raw.radius_stored,
    radiusCorners: raw.radius_corners_shown
      ? [raw.radius_tl, raw.radius_tr, raw.radius_br, raw.radius_bl]
      : null,
    removeRoundingShown: raw.remove_rounding_shown,
    removeRoundingEnabled: raw.remove_rounding_enabled,
    pointsShown: raw.points_shown,
    pointsMixed: raw.points_mixed,
    points: raw.points,
    ratioShown: raw.ratio_shown,
    ratioMixed: raw.ratio_mixed,
    ratio: raw.ratio,
    objectToPathShown: raw.object_to_path_shown,
    scaleSwitchesShown: raw.scale_switches_shown,
    strokeToPathEnabled: raw.stroke_to_path_enabled,
  };
  raw.free();
  return state;
}

function sameSelectBar(a: SelectBarState, b: SelectBarState): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** A hand-rolled double-click detector (specs/0002-path-node-editing/
 * adrs.md's `PenTool` doc comment: "double-click detection itself is the
 * frontend's job"). A real DOM `dblclick` event fires only after BOTH
 * clicks' own `pointerdown`/`pointerup` pairs have already run — relaying
 * both to the session would place an unwanted extra node (acceptance
 * criterion 3: a finishing double-click is "instead of placing another
 * node", not in addition to it). This tracks press timing/position
 * itself instead, so the second press of a double-click can be withheld
 * from the session entirely and read as one `double_click(x, y)` call
 * once its matching release arrives
 * (`specs/0004-canvas-navigation-and-selection/adrs.md`: "the host
 * detects a double-click... and calls `double_click(x, y)`. `Session`
 * dispatches it" — no more per-tool branching here). The first press
 * still committed normally — exactly the node the maker expects to end
 * up with (matching Inkscape).
 */
/** How long the "max" notice of a limited typed corner radius stays at its knob
 * (`specs/0013-rectangle-corner-radii/` criterion 6). */
const LIMIT_NOTICE_MS = 1500;

const DOUBLE_CLICK_MS = 400;
/** Canvas-relative CSS pixels now (`canvas-navigation-and-selection`: all
 * screen↔document conversion moved into Rust), not document millimetres
 * — comparing in screen space is also exactly right for "did the two
 * presses land close enough to read as one double-click", independent
 * of zoom. */
const DOUBLE_CLICK_PX = 5;
/** A press on a mark of the typed move is a click if the pointer moves less than this before the
 * release, px (`specs/0047-transform-polish/` criterion 40); 8 px for a touch or a pen. */
const MARK_SLOP_PX = 3;
const MARK_TOUCH_SLOP_PX = 8;

/** How many wheel-delta pixels correspond to one print-sized "line" of
 * scroll (`deltaMode === 1`) or one "page" (`deltaMode === 2`) — most
 * browsers never actually send these modes for a mouse wheel/trackpad,
 * but normalizing them here means `Session::wheel` only ever has to
 * reason about pixels (`adrs.md`'s frontend requirement: "normalize
 * `deltaMode` to pixels"). */
const WHEEL_LINE_HEIGHT_PX = 16;
const WHEEL_PAGE_HEIGHT_PX = 800;

function normalizedWheelDelta(event: WheelEvent): { deltaX: number; deltaY: number } {
  let factor = 1;
  if (event.deltaMode === 1) {
    factor = WHEEL_LINE_HEIGHT_PX;
  } else if (event.deltaMode === 2) {
    factor = WHEEL_PAGE_HEIGHT_PX;
  }
  return { deltaX: event.deltaX * factor, deltaY: event.deltaY * factor };
}

function readToolbarState(raw: {
  can_insert: boolean;
  can_delete: boolean;
  can_convert_to_corner: boolean;
  can_convert_to_symmetric: boolean;
  can_convert_to_asymmetric: boolean;
  can_make_line: boolean;
  can_make_curve: boolean;
  can_join: boolean;
  can_split: boolean;
  clone_only: boolean;
  join_block: number;
  join_not_end: number;
  selected_nodes: number;
  total_nodes: number;
  segment_selected: boolean;
  free(): void;
}): NodeToolbarState {
  const state: NodeToolbarState = {
    canInsert: raw.can_insert,
    canDelete: raw.can_delete,
    canConvertToCorner: raw.can_convert_to_corner,
    canConvertToSymmetric: raw.can_convert_to_symmetric,
    canConvertToAsymmetric: raw.can_convert_to_asymmetric,
    canMakeLine: raw.can_make_line,
    canMakeCurve: raw.can_make_curve,
    canJoin: raw.can_join,
    canSplit: raw.can_split,
    cloneOnly: raw.clone_only,
    joinBlock: raw.join_block,
    joinNotEnd: raw.join_not_end,
    selectedNodes: raw.selected_nodes,
    totalNodes: raw.total_nodes,
    segmentSelected: raw.segment_selected,
  };
  raw.free();
  return state;
}

/** Whether a keyboard event's target is an interactive control (a switch,
 * button, input, slider, radio, select) rather than the canvas itself —
 * canvas shortcuts must ignore those. The Style panel sits outside the canvas
 * container, so its keys never get here; the roles are a second guard
 * (`specs/0007-stroke-and-fill-styling` criterion 38). */
/**
 * A wheel step or a pan moves the document under a still pointer: a running placement is told
 * the pointer's screen point again, so the preview and the chip follow its new document position
 * (`specs/0052-click-to-create-shapes/` criterion 9). Nothing for any other tool.
 */
function followPlacementAfterView(
  session: WasmSession,
  x: number,
  y: number,
  modifiers: { shiftKey: boolean; ctrlKey: boolean; metaKey: boolean; altKey: boolean },
): boolean {
  const chip = session.placement_chip();
  if (!chip) {
    return false;
  }
  chip.free();
  session.pointer_hover(
    x,
    y,
    modifiers.shiftKey,
    modifiers.ctrlKey || modifiers.metaKey,
    modifiers.altKey,
  );
  return true;
}

function isFormControl(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    target.closest(
      'input, textarea, select, button, [role="switch"], [role="slider"], [role="spinbutton"], [role="radio"], [role="combobox"], [role="option"], [role="dialog"], [contenteditable]:not([contenteditable="false"])',
    ) !== null
  );
}

/** Reads the wasm-bindgen `LiveReadout` instance once, immediately, so
 * it can be `free()`d rather than held onto — same reasoning as
 * `readToolbarState`. */
function readLiveReadout(
  raw: { text: string; anchor_x: number; anchor_y: number; free(): void } | undefined,
): LiveReadout | null {
  if (!raw) {
    return null;
  }
  // A resize through zero adds the line that names the mirror (`0056` criterion 10).
  const [text = "", second, third] = raw.text.split("\n");
  const readout: LiveReadout = { text, second, third, x: raw.anchor_x, y: raw.anchor_y };
  raw.free();
  return readout;
}

/** The Offset tool's readout chip during a drag (`specs/0038-path-offset` criteria 12, 15, 18),
 * at the pointer; `null` without a drag. */
function offsetLiveReadout(session: WasmSession, x: number, y: number): LiveReadout | null {
  const view = readOffsetView(session);
  if (!view.dragging) {
    return null;
  }
  const { lines, invalid } = offsetReadout(view);
  const [text = "", second, third] = lines;
  return { text, second, third, thirdInvalid: invalid, x, y };
}

/** Reads a wasm-bindgen `DocumentPoint` instance once, immediately, so
 * it can be `free()`d rather than held onto — same reasoning as
 * `readToolbarState`. */
function readDocumentPoint(raw: { x: number; y: number; free(): void }): {
  x: number;
  y: number;
} {
  const point = { x: raw.x, y: raw.y };
  raw.free();
  return point;
}

/** The on-canvas numeric readout shown during a shape tool's
 * create-drag (`specs/0003-primitive-shapes/specification.md`'s "Live
 * creation feedback"). `x`/`y` are already canvas-relative CSS pixels
 * (`specs/0004-canvas-navigation-and-selection/adrs.md`: "Anything the
 * DOM positions... is returned already converted") — position the
 * overlay directly from these, no conversion math here. */
export interface LiveReadout {
  text: string;
  x: number;
  y: number;
  /** A muted second line (the Offset tool's mode). */
  second?: string;
  /** A third line (the Offset tool's message). */
  third?: string;
  /** The third line is in the invalid colour (the Offset tool's stop). */
  thirdInvalid?: boolean;
}

/** One field of the typed numeric entry chip (`object-transform-
 * refinements` criteria 18-32), as `curvyo-editor-wasm` describes it. */
export interface TransformEntryField {
  /** Visible label ("W", "H", "r"; empty for the angle). */
  label: string;
  /** Accessible name ("Width", "Height", "Outer radius", "Angle",
   * "Corner radius", "Inner ratio"). */
  name: string;
  /** The text the field opens with. */
  prefill: string;
  editable: boolean;
}

/** The open numeric entry: what to show and where. Positions are canvas-
 * relative CSS pixels, already converted by Rust. */
export interface TransformEntryState {
  kind:
    | "angle"
    | "size"
    | "radius"
    | "corner-radius"
    | "inner-ratio"
    | "skew"
    | "arc-angle"
    | "curve";
  fields: TransformEntryField[];
  /** The muted second row of a corner radius entry ("All four corners" or "This
   * corner only", `specs/0013-rectangle-corner-radii/` criterion 6); empty for every
   * other entry. */
  scope: string;
  /** Whether the two fields are linked (Ctrl at the second press). */
  linked: boolean;
  handle: { x: number; y: number };
  center: { x: number; y: number };
  /** How far past the handle's position its outermost glyph reaches, in
   * CSS pixels: 6 normally, 22 for an edge resize handle with a skew arrow
   * on the same side (the chip must clear the arrow). */
  glyphReach: number;
  /** The chip goes 16 px right of and below the box centre, as the move chip
   * does, instead of outward from the handle: the key S, whose fixed point is
   * the centre (`edit-interaction-polish` criterion 59). */
  atCentre: boolean;
  /** The chip edits a multi-selection (`specs/0019-multi-object-transform/`):
   * the size chip is named "Resize selection". */
  selection: boolean;
}

/** Reads the wasm-bindgen `TransformEntryView` once, immediately, so it can
 * be `free()`d rather than held onto — same reasoning as `readToolbarState`. */
function readTransformEntry(
  raw:
    | {
        kind: string;
        field_count: number;
        field_label(index: number): string;
        field_name(index: number): string;
        field_prefill(index: number): string;
        field_editable(index: number): boolean;
        linked: boolean;
        scope: string;
        handle_x: number;
        handle_y: number;
        center_x: number;
        center_y: number;
        glyph_reach: number;
        at_centre: boolean;
        selection: boolean;
        free(): void;
      }
    | undefined,
): TransformEntryState | null {
  if (!raw) {
    return null;
  }
  const fields: TransformEntryField[] = [];
  for (let index = 0; index < raw.field_count; index += 1) {
    fields.push({
      label: raw.field_label(index),
      name: raw.field_name(index),
      prefill: raw.field_prefill(index),
      editable: raw.field_editable(index),
    });
  }
  const entry: TransformEntryState = {
    kind: raw.kind as TransformEntryState["kind"],
    fields,
    linked: raw.linked,
    scope: raw.scope,
    handle: { x: raw.handle_x, y: raw.handle_y },
    center: { x: raw.center_x, y: raw.center_y },
    glyphReach: raw.glyph_reach,
    atCentre: raw.at_centre,
    selection: raw.selection,
  };
  raw.free();
  return entry;
}

function sameEntry<T>(a: T | null, b: T | null): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** The open typed move (`specs/0010-edit-interaction-polish/` criteria 15 to 25):
 * where to put the chip and what an untouched field shows in each mode.
 * Positions are canvas-relative CSS pixels, already converted by Rust. */
export interface MoveEntryState {
  /** The box centre: the centre handle is there, or would be. */
  center: { x: number; y: number };
  /** The lower edge of the true bounding box: the chip sits 16 px below it
   * (`specs/0047-transform-polish/` UX notes, "Chip"). */
  boxBottomY: number;
  /** What an untouched X and Y show in Relative mode. */
  relative: [string, string];
  /** What an untouched X and Y show in Absolute mode for each of the nine reference points, in
   * the order of the radios: top left, top centre, top right, left middle, centre, right
   * middle, bottom left, bottom centre, bottom right. */
  absolute: Array<[string, string]>;
  /** The chosen reference point, 0 to 8: 4, the centre, on every open. */
  reference: number;
  /** The accessible name of each reference point's radio ("Top left", ...). */
  referenceNames: string[];
  /** The first line of the hint chip of each reference point's mark ("Top left corner", ...). */
  referenceHints: string[];
  /** Whether the Copy check opens on (Ctrl at the second press). */
  copyPreset: boolean;
}

/** The nine reference points, as the index of [`MoveEntryState.absolute`]. */
export const REFERENCE_COUNT = 9;

/** Reads the wasm-bindgen `MoveEntryChip` once, immediately, so it can be
 * `free()`d rather than held onto. */
function readMoveEntry(
  raw:
    | {
        center_x: number;
        center_y: number;
        box_bottom_y: number;
        relative_prefill(axis: number): string;
        absolute_prefill(reference: number, axis: number): string;
        reference(): number;
        reference_name(index: number): string;
        reference_hint_name(index: number): string;
        copy_preset(): boolean;
        free(): void;
      }
    | undefined,
): MoveEntryState | null {
  if (!raw) {
    return null;
  }
  const points = Array.from({ length: REFERENCE_COUNT }, (_, index) => index);
  const entry: MoveEntryState = {
    center: { x: raw.center_x, y: raw.center_y },
    boxBottomY: raw.box_bottom_y,
    relative: [raw.relative_prefill(0), raw.relative_prefill(1)],
    absolute: points.map((index) => [raw.absolute_prefill(index, 0), raw.absolute_prefill(index, 1)]),
    reference: raw.reference(),
    referenceNames: points.map((index) => raw.reference_name(index)),
    referenceHints: points.map((index) => raw.reference_hint_name(index)),
    copyPreset: raw.copy_preset(),
  };
  raw.free();
  return entry;
}

/** The modifier badges of a move (`specs/0010-edit-interaction-polish/` criteria
 * 26, 27, 33): which show, and where the pointer is, canvas-relative CSS
 * pixels. Rust decides everything but the placement
 * (`Session::move_indicators`). */
export interface MoveBadgeState {
  /** The plus badge: a copy drag runs, or a press with Ctrl would start a move. */
  copy: boolean;
  /** The minus badge: a Ctrl marquee or lasso runs, or a press with Ctrl would
   * arm one (`specs/0014-advanced-selection/`): it removes from the selection. */
  remove: boolean;
  /** The lock badge's axis: `""` (none), `"x"` or `"y"`, or `"angle"` for a Sketch direction. */
  lock: "" | "x" | "y" | "angle";
  /** The pointer hotspot. */
  x: number;
  y: number;
}

const NO_BADGES: MoveBadgeState = { copy: false, remove: false, lock: "", x: 0, y: 0 };

/** A key that could not act, shown for 2 s next to the pointer
 * (`specs/0010-edit-interaction-polish/` criterion 59). */
export interface KeyHint {
  text: string;
  /** Changes on every message, so the clock restarts. */
  id: number;
}

/** What `WasmSession.key_down` returns for a refused key. */
const KEY_HINT_TEXT: Record<string, string> = {
  "hint-select-first": "Select an object first",
  "hint-too-far": TOO_FAR_HINT,
};

/** How long a key hint stays. */
const KEY_HINT_MS = 2000;

export interface EditorSession {
  sketchView: SketchView;
  sketchPresentation: SketchPresentation;
  /** Attach to the `<canvas>` element the host renders. */
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  /** Attach to the canvas's focusable container (canvas-focus shortcut
   * scope, docs/design-system.md's keyboard shortcut table). */
  containerRef: React.RefObject<HTMLDivElement | null>;
  tool: Tool;
  nodeToolbarState: NodeToolbarState;
  /** The polygon/star tool-options bar's current mode. */
  polyStarMode: PolyStarMode;
  /** The polygon/star tool-options bar's current point count. */
  polyStarPointCount: number;
  /** The polygon/star tool-options bar's current ratio. */
  polyStarRatio: number;
  /** The live numeric readout for an in-progress create-drag, or `null`
   * outside one (ux-engineer review item 2). */
  liveReadout: LiveReadout | null;
  /** The edit hint after a double-click on a primitive, or `null`. */
  editHint: EditHint | null;
  /** Closes the edit hint (3 s, a press, a key, the pointer leaving). */
  dismissEditHint: () => void;
  /** `curvyo-editor-wasm`'s `cursor_hint()` — `"default"`, `"rotate"`
   * or `"resize:<degrees>"` (`object-transform`'s transform-handle
   * cursors); `Canvas` turns it into a CSS cursor via `lib/cursors`. */
  cursorHint: string;
  /** Ends pick mode of text on path without a change (a right click). */
  cancelTextPathPick: () => void;
  /** Which hint the handle under the pointer earns (`""`, `"resize-edge"`,
   * `"resize-corner"`, `"resize-corner-uniform"`, `"rotate-corner"`,
   * `"rotate-side"`, `"skew"` (top and bottom handle), `"skew-y"` (left and
   * right), `"move"`, `"param-radius"`, `"param-inner"`): the hover chip's content
   * (`object-transform-refinements` criterion 54). */
  handleHint: string;
  /** The lines of the hint chip of a corner radius knob under the pointer
   * (they depend on the "Link corners" switch and on a limited corner, see
   * `Session::corner_hint_lines`); empty on any other handle. */
  cornerHintLines: string[];
  /** The typed numeric entry to show, or `null`. */
  transformEntry: TransformEntryState | null;
  /** The open typed move, if any. */
  moveEntry: MoveEntryState | null;
  /** The open entry box of a text on path's position handle, if any (`specs/0080-text-on-path`,
   * customer 2026-10-24). */
  textPathEntry: TextPathEntry | null;
  /** Enter in that box: `"committed"`, `"unchanged"` or `"invalid:<field>"`. */
  commitTextPathEntry: (position: string, distance: string) => string;
  /** Escape in that box, or leaving it: closed with no change. */
  cancelTextPathEntry: () => void;
  /** The box's Side, Line on path and Mirror buttons: previewed until Enter. */
  textPathEntrySide: (side: SideCode) => void;
  textPathEntryLine: (line: LineCode) => void;
  textPathEntryMirror: () => void;
  /** The plus and lock badges by the pointer. */
  moveBadges: MoveBadgeState;
  /** Enter in the move chip: `"committed"`, `"unchanged"` or
   * `"invalid:<field>:number"` (the chip stays open). `copy` is the state of
   * the chip's Copy check. */
  commitMoveEntry: (
    first: string,
    second: string,
    absolute: boolean,
    copy: boolean,
  ) => string;
  /** The typed move's Reference control and marks (`specs/0047-transform-polish/` Part C): the
   * chip reports its mode, the point its control chose and the radio under the pointer to Rust,
   * which draws the marks and reads the point at Enter. */
  setMoveChipAbsolute: (absolute: boolean) => void;
  setMoveReference: (reference: number) => void;
  /** The radio with the pointer or the focus (0 to 8), or -1. */
  setMoveRadioHover: (reference: number) => void;
  /** A click on a mark: the move executes with that mark's point as the reference, with the
   * texts of the fields and the Copy check; the codes of `commitMoveEntry` (criterion 40). */
  commitMoveEntryAt: (first: string, second: string, copy: boolean, reference: number) => string;
  /** The mark (0 to 8) under the pointer, or -1 (criteria 43 and 44). */
  moveMarkHover: number;
  /** The chip registers what a click on a mark does while it is open; `null` when it closes. */
  setMarkClickHandler: (handler: ((mark: number) => void) | null) => void;
  /** Enter in the entry chip: `"committed"`, `"unchanged"` (both close it)
   * or `"invalid:<field>:number|negative|ratio-range|skew-range|too-large"`
   * (it stays
   * open). */
  commitTransformEntry: (first: string, second: string, lastEdited: number) => string;
  /** Closes the entry without writing (Escape, blur). Idempotent. */
  cancelTransformEntry: () => void;
  /** For linked fields: the text the other field takes. */
  transformEntryLinked: (field: number, text: string) => string | undefined;
  /** The current zoom level's integer percentage read-out (acceptance
   * criterion 9) — `StatusBar`'s new center segment. */
  zoomPercent: number;
  /** Whether a drag-pan gesture (middle-mouse or Space+primary,
   * acceptance criteria 3, 4) is currently in flight — `Canvas`'s own
   * grabbing-cursor convention. */
  isPanning: boolean;
  /** Whether Space is currently held — `Canvas`'s own open-hand cursor
   * convention, the moment Space is held but before any drag motion
   * (`docs/design-system.md`'s "Pan cursor"). */
  isSpaceHeld: boolean;
  setTool: (tool: Tool) => void;
  deleteSelected: () => void;
  /** How many objects the session has selected: the rail's tooltips name
   * "Esc, R" while the Select tool has one (criterion 62). */
  selectionCount: number;
  /** What a screen reader says about a multi-selection: the count and the group
   * box size, empty for fewer than two objects. */
  selectionAnnouncement: string;
  /** Changes with every node announcement, so a repeated sentence is spoken again. */
  selectionAnnouncementSeq: number;
  /** The one-line message of a key that could not act ("Select one object to
   * type a value"), or `null`; it clears itself after 2 s. */
  keyHint: KeyHint | null;
  /** The notice of the last undo or redo, in the canvas notice slot. */
  canvasNotice: CanvasNotice | null;
  /** Groups (`specs/0023-groups`): the Grouping card, the entered group's depth and its
   * announcements. */
  grouping: GroupingApi;
  /** The Offset tool (`specs/0038-path-offset`): its view, the entry chip's values and Enter,
   * and the notice after a release. */
  offset: OffsetApi;
  /** The Text tool (`specs/0074-text-objects`): the input sink, the Text bar, the Fonts tab and
   * the fonts the host hands in. */
  text: TextApi;
  /** Linked clones: the button, the panel block, the canvas tags (`specs/0049-linked-clone`). */
  clones: ClonesApi;
  /** Stroke to path: the Select bar button and Ctrl+Alt+C (`specs/0085-stroke-to-path`). */
  strokeToPath: StrokeToPathApi;
  /** The Trace tool (`specs/0025-raster-trace`). */
  trace: TraceApi;
  /** Recognise text: the Select bar button, its bar and the run (`specs/0088-trace-recognition`). */
  recognise: RecogniseApi;
  /** The flip buttons and the keys H and V (`specs/0056-mirror-and-negative-scale/`). */
  flip: FlipApi;
  /** The flip buttons act: something is selected in the Select tool. */
  flipEnabled: boolean;
  /** The Guide button of the Select bar (`specs/0046-document-shapes` criteria 9 and 10). */
  guideState: GuideState;
  /** A press of the Guide button: one step "Guide". */
  toggleGuide: () => void;
  /** The File tab's Template button, once the file is read: the session's outcome string
   * (`"loaded:<n>"` or `"error:<message>"`). */
  loadTemplate: (bytes: Uint8Array) => string;
  /** The Select bar's ellipse group and Curve field (`specs/0021-ellipse-arcs-and-shaping/`). */
  arcBar: ArcBarApi;
  /** The size chip's note line for the typed texts ("Mirrored left to right"), or "". */
  transformEntryMirrorLine: (first: string, second: string, lastEdited: number) => string;
  /** The click-to-create placement and its chip (`specs/0052-click-to-create-shapes`). */
  placement: PlacementApi;
  /** The Pen's typed Length and Angle entry (`specs/0072-pen-angle-snap-and-typed-values`). */
  penEntry: PenEntryApi;
  /** A large undo or redo is running. */
  historyBusy: boolean;
  /** The names of the next undo and redo; `null` for an empty stack. */
  historyNames: HistoryNames;
  /** Undo or redo from a button. */
  pressHistory: (direction: HistoryDirection, source?: "canvas" | "panel") => void;
  /** Shows a line in the notice slot of the canvas. */
  showHistoryNotice: (
    text: string,
    ms?: number,
    source?: "canvas" | "panel",
    kind?: "status" | "alert",
  ) => void;
  /** Wipes the history; the steps removed, -1 when ignored, -2 for a shared document. */
  wipeHistory: () => number;
  /** Hides the selected object's earlier history; the rows hidden, -1 when ignored. */
  wipeObjectHistory: () => number;
  /** Runs a History-list action that may change the document (`useHistory`'s `runAction`). */
  runHistoryAction: (action: (session: WasmSession) => string) => void;
  /** The move readout and hidden announcements of a nudge and of select all. */
  nudgeFeedback: NudgeFeedback;
  convertSelected: (kind: "corner" | "symmetric" | "asymmetric") => void;
  makeLine: () => void;
  makeCurve: () => void;
  /** Join: merges the two selected endpoint nodes into one (acceptance
   * criteria 8-11). No keyboard shortcut (specification.md's "Out of
   * scope"). Returns how many outlines moved into the surviving object
   * besides the joined one (`0055` criterion 5 c), for the notice. */
  joinSelected: () => number;
  /** Split: breaks a path apart at the one selected node (acceptance
   * criteria 12-15). No keyboard shortcut, same reasoning as Join. Returns
   * `""`, `"path"`, `"outline_in_two"` or `"outline_opened"` (`0055`
   * criterion 6), for the notice. */
  splitSelected: () => string;
  insertSelected: () => void;
  /** Acceptance criterion 6's "remove rounding" action. */
  removeCornerRounding: () => void;
  /** The Select tool's "Scale stroke width" switch
   * (`object-transform` criteria 8, 26-31): whether a resize scales the
   * stroke. Session state, off in every new session, never saved. */
  scaleStrokeWidth: boolean;
  setScaleStrokeWidth: (on: boolean) => void;
  /** The Select tool's "Scale corner radius" switch
   * (`unified-object-editing` criterion 23): whether a resize scales a
   * rectangle's corner radius. Session state, off in every new session. */
  scaleCornerRadius: boolean;
  setScaleCornerRadius: (on: boolean) => void;
  /** The "Link corners" switch (`specs/0013-rectangle-corner-radii/` criterion 2):
   * on, a corner handle sets all four radii. On in every new or opened project. */
  linkCorners: boolean;
  setLinkCorners: (on: boolean) => void;
  /** What the Select bar shows for the current selection (criteria 21-22). */
  selectBar: SelectBarState;
  /** Counts every re-read of the session's UI state: the Style panel reads
   * its own state from the session whenever this changes. */
  syncRevision: number;
  /** Re-reads every bit of session-owned UI state: after a panel call that changed the
   * selection or the document (the Layers tab). */
  sync: () => void;
  /** The live wasm session, or `null` before the first attach. */
  getSession: () => WasmSession | null;
  /** Registers a function that runs right after every canvas resize has
   * reached the session (window resize, Properties panel toggle), in the
   * same task as the canvas's own redraw: whatever is drawn beside the canvas
   * from the view repaints there instead of one frame late. Returns the
   * function that removes it. */
  addViewResizedListener: (listener: () => void) => () => void;
  /** A side pane now lies over the canvas's right `px` (0 for none): the canvas keeps its size,
   * the session's fit, centring and visibility rules use the visible part. */
  coverCanvasRight: (px: number) => void;
  /** Re-reads the cursor readout at the last pointer position: a resize or a
   * fit moves the document under a still pointer. */
  refreshCursor: () => void;
  /** Enter in the bar's "Radius" field: `"committed"`, `"unchanged"`,
   * `"invalid:number"` or `"invalid:negative"`. */
  setSelectedRadius: (text: string) => string;
  /** The bar's "Points" field and stepper: one commit. */
  setSelectedPointCount: (count: number) => void;
  /** The bar's "Ratio" slider: a live preview on every tick (nothing is
   * written), one commit on release. */
  previewSelectedRatio: (ratio: number) => void;
  commitSelectedRatio: () => void;
  /** A click on the slider or a typed ratio: one commit for every selected star. */
  setSelectedRatio: (ratio: number) => void;
  /** Escape in the middle of a slider drag: the preview is dropped. */
  cancelSelectedRatio: () => void;
  /** The mode toggle (acceptance criteria 11 vs. 12). */
  setPolyStarMode: (mode: PolyStarMode) => void;
  /** The point-count setting for the next shape (acceptance criterion 10). */
  setPolyStarPointCount: (count: number) => void;
  /** The ratio setting for the next star (it never changes a selected
   * star: that is the Select bar's Ratio). */
  setPolyStarRatio: (ratio: number) => void;
  /** "Object to path" (acceptance criteria 17, 21, 22). */
  convertSelectedToPaths: () => void;
  /** A boolean operation on the selection, in one commit (`0016-boolean-operations`);
   * `"ignored"` when no session is live or the Select tool is not active. */
  applyBoolean: (op: BooleanOp) => BooleanResult;
  /** Combine on the selection, in one commit (`0035-combine-and-break-apart`). */
  applyCombine: () => CombineResult;
  /** Split on the selection, in one commit (`0048-split-compound-path`). */
  applySplit: () => SplitResult;
  /** Break apart on the selection, in one commit (`0048-split-compound-path`). */
  applyBreakApart: () => BreakApartResult;
  /** Cut on the selection, in one commit (`0036-cut-at-crossings`). */
  applyCut: () => CutResult;
  /** Fracture or Flatten on the selection, in one commit (`0037-fracture-and-flatten`). */
  applyStack: (command: StackCommand) => StackResult;
  /** Weld on the selection, in one commit (`0068-laserable-lettering`). */
  applyWeld: () => WeldResult;
  /** The Node bar's Close path buttons (`0034-pen-path-extension`): closes the open paths of
   * the editing set with `join` (`"sharp"` or `"smooth"`) in one commit. */
  closePath: (join: "sharp" | "smooth") => ClosePathResult;
  onPointerDown: (event: React.PointerEvent<HTMLCanvasElement>) => void;
  onPointerMove: (event: React.PointerEvent<HTMLCanvasElement>) => void;
  onPointerUp: (event: React.PointerEvent<HTMLCanvasElement>) => void;
  onPointerLeave: () => void;
  /** The browser took the pointer away (a system gesture, an alert):
   * cancel any in-flight drag instead of leaving it half-done. */
  onPointerCancel: () => void;
  /** The canvas container lost focus: clears the Shift/Ctrl reveal state. */
  onContainerBlur: (event: React.FocusEvent<HTMLDivElement>) => void;
  onKeyDown: (event: React.KeyboardEvent<HTMLDivElement>) => void;
  onKeyUp: (event: React.KeyboardEvent<HTMLDivElement>) => void;
  /** New and Close: swaps in a brand-new, empty session. Resolves once it is attached. */
  newProject: () => Promise<void>;
  /** File → Open / the OS file association: parses `bytes` as a `.curvyo`
   * and swaps it in as the live session.
   * @throws the exact user-facing sentence naming why `bytes` could not
   * be opened (see `@/lib/editorSession`'s `openSession`) — the caller
   * shows it, this hook does not. */
  openProject: (bytes: Uint8Array) => Promise<void>;
  /** Open… of any file (`specs/0024-svg-import-export` criterion 1): swaps in the session the
   * bytes give, an SVG drawing or a project. Resolves with whether it was an SVG (an untitled,
   * dirty project) and the notice to show (empty for none).
   * @throws the error dialog's sentence; the open project is untouched. */
  openFile: (bytes: Uint8Array, fileName: string) => Promise<{ svg: boolean; notice: string }>;
  /** The Import button (`specs/0058-bitmap-import` criteria 1 to 5) and Ctrl+V
   * (`specs/0087-image-clipboard-and-drop`): starts an import from `origin` centred on `at`
   * (canvas CSS pixels: the drop point, the pointer of a paste) or, with `null`, on the view
   * centre. */
  importBegin: (origin: ImportOrigin, at: { x: number; y: number } | null) => void;
  /** The pointer's last position over the canvas (canvas CSS pixels), `null` once it left the
   * canvas or before it was ever over it (`specs/0087-image-clipboard-and-drop` criterion 2). */
  lastCanvasPointer: () => { x: number; y: number } | null;
  /** Reads one file of the import (an image is decoded, an SVG parsed). */
  importAdd: (bytes: Uint8Array, fileName: string) => void;
  /** Refuses a file of the import the host did not read because it is `bytes` long, over the
   * import's file limit. */
  importAddTooLarge: (fileName: string, bytes: number) => void;
  /** Commits the import as one step and selects what came in, once the faces of an SVG's texts
   * are loaded. */
  importFinish: () => Promise<{ imported: number; notice: string; alert: boolean }>;
  /** Save / Save as: packs the live session's document into `.curvyo` container bytes for the
   * host to write, recording the application version. Call `markSaved` once they are written.
   * @throws a plain `Error` if there is no live session yet, or the
   * `JsValue` `curvyo-editor-wasm`'s `pack` throws if the document could
   * not be serialized — the caller shows it, this hook does not. */
  packProject: () => Uint8Array;
  /** The bytes of the last `packProject` are written: the project is clean unless it changed
   * since (`specs/0051-sidebar-and-no-menu/` criterion 22). */
  markSaved: () => void;
}

/** Owns one `WasmSession` for the app's lifetime: creates it, attaches
 * it to the host's `<canvas>`, keeps its `wgpu` surface sized to the
 * canvas (specs/0002-path-node-editing/adrs.md's PASS note, requirement 2),
 * runs its per-frame render loop, and forwards pointer/keyboard/wheel
 * input — "the frontend renders state and forwards input events into it;
 * it holds no editing logic of its own" (same file, ADR 0001 §1/§2).
 * `canvas-navigation-and-selection` moves every screen↔document
 * conversion into Rust (`Session::screen_to_document`) — this hook now
 * only ever passes canvas-relative CSS pixels across the wasm boundary,
 * never a document-space point it computed itself.
 *
 * The session itself is swappable: `newProject`/`openProject` free
 * whatever is currently attached and attach a different one in its
 * place (`specs/0002-path-node-editing/adrs.md`'s PR review: "the host does
 * byte I/O only" — Open/New used to recreate the native `Document`;
 * now they recreate this hook's `WasmSession`), without tearing down
 * the mount effect's resize observer or render loop, which both always
 * read `sessionRef.current` fresh rather than closing over one instance. */
export function useEditorSession(
  onCursorMove: (point: { x: number; y: number }) => void,
): EditorSession {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const sessionRef = useRef<WasmSession | null>(null);
  /** Functions to run after each canvas resize reached the session. */
  const viewResizedListenersRef = useRef(new Set<() => void>());
  /** The tail of the session queue (`enqueueSessionTask`). */
  const sessionQueueRef = useRef<Promise<void>>(Promise.resolve());
  const lastPressRef = useRef<{ time: number; x: number; y: number } | null>(
    null,
  );
  const clickContextRef = useRef("");
  const suppressedPressRef = useRef(false);
  /** A press that landed on a mark of the open typed move: consumed, and a click at the release. */
  const markPressRef = useRef<{ mark: number; x: number; y: number; touch: boolean } | null>(null);
  const markClickRef = useRef<((mark: number) => void) | null>(null);
  const [moveMarkHover, setMoveMarkHover] = useState(-1);
  /** The second press of a double-click: its position and modifiers, the
   * ones `double_click` is called with when its release arrives (a handle
   * is picked at the second press, `object-transform-refinements`). */
  const doubleClickPressRef = useRef<{
    x: number;
    y: number;
    shift: boolean;
    ctrl: boolean;
  } | null>(null);
  /** Whether a drag-pan gesture is in flight — a ref (not just the
   * mirrored `isPanning` state below) so `onPointerMove`/`onPointerUp`
   * read the current value synchronously within the same event, not a
   * stale one from before the latest `setState` re-render. */
  /** A heavy release is waiting for its `wait` cursor to paint: pointer events
   * are ignored until the commit has run (`specs/0019-multi-object-transform/`
   * criteria 28, 46, 49). */
  const releasePendingRef = useRef(false);
  const panningRef = useRef(false);

  const [sketchPresentation] = useState(createSketchPresentation);
  const sketchView = useSyncExternalStore(sketchPresentation.subscribePanel, sketchPresentation.panel);
  const setSketchView = sketchPresentation.publish;
  const sketchWorkbenchRef = useRef(false);
  const refreshSketch = useCallback(() => {
    if (sketchWorkbenchRef.current && sessionRef.current) setSketchView(JSON.parse(sessionRef.current.sketch_view_json()) as SketchView);
  }, [setSketchView]);
  const [tool, setToolState] = useState<Tool>("select");
  const [nodeToolbarState, setNodeToolbarState] = useState<NodeToolbarState>(
    EMPTY_TOOLBAR_STATE,
  );
  const [polyStarMode, setPolyStarModeState] = useState<PolyStarMode>("polygon");
  const [polyStarPointCount, setPolyStarPointCountState] = useState(6);
  const [polyStarRatio, setPolyStarRatioState] = useState(0.5);
  const [liveReadout, setLiveReadout] = useState<LiveReadout | null>(null);
  const [scaleStrokeWidth, setScaleStrokeWidthState] = useState(false);
  const [scaleCornerRadius, setScaleCornerRadiusState] = useState(false);
  const [linkCorners, setLinkCornersState] = useState(true);
  const [selectBar, setSelectBar] = useState<SelectBarState>(EMPTY_SELECT_BAR_STATE);
  const [syncRevision, setSyncRevision] = useState(0);
  const [cursorHint, setCursorHint] = useState("default");
  const [editHint, setEditHint] = useState<EditHint | null>(null);
  const editHintCounter = useRef(0);
  const [handleHint, setHandleHint] = useState("");
  const [cornerHintLines, setCornerHintLines] = useState<string[]>([]);
  const limitNoticeTimer = useRef<number | undefined>(undefined);
  const [transformEntry, setTransformEntry] = useState<TransformEntryState | null>(null);
  const [moveEntry, setMoveEntry] = useState<MoveEntryState | null>(null);
  const [textPathEntry, setTextPathEntry] = useState<TextPathEntry | null>(null);
  const [moveBadges, setMoveBadges] = useState<MoveBadgeState>(NO_BADGES);
  const entryOpenRef = useRef(false);
  /** The last pointer position over the canvas (CSS px) — lets a
   * modifier key press/release re-run the hover, so the Select tool's
   * pivot marker and live preview react the instant Shift/Ctrl change,
   * with no pointer motion needed. */
  const lastPointerRef = useRef<{ x: number; y: number } | null>(null);
  /** The Shift, Ctrl and Alt keys that are down (`lib/keyModifiers.ts`). */
  const modifierTrackerRef = useRef(new ModifierTracker());
  /** A pointer event's flags are right: they correct the tracked keys, so a
   * key-up that never arrived is forgotten. */
  const syncTrackedModifiers = useCallback(
    (event: { shiftKey: boolean; ctrlKey: boolean; metaKey: boolean; altKey: boolean }) => {
      modifierTrackerRef.current.pointerFlags({
        shift: event.shiftKey,
        ctrl: event.ctrlKey || event.metaKey,
        alt: event.altKey,
      });
    },
    [],
  );
  const [zoomPercent, setZoomPercent] = useState(100);
  const [isPanning, setIsPanning] = useState(false);
  const [isSpaceHeld, setIsSpaceHeld] = useState(false);
  const [selectionCount, setSelectionCount] = useState(0);
  const [selectionAnnouncement, setSelectionAnnouncement] = useState("");
  const [selectionAnnouncementSeq, setSelectionAnnouncementSeq] = useState(0);
  const [keyHint, setKeyHint] = useState<KeyHint | null>(null);
  const [flipEnabled, setFlipEnabled] = useState(false);
  const [guideState, setGuideState] = useState<GuideState>("disabled");
  const keyHintCounter = useRef(0);
  const nudgeFeedback = useNudgeFeedback();
  const {
    onNudge,
    onSelectAll,
    onEscape,
    onHistory: onNudgeHistory,
    onKeyUp: onNudgeKeyUp,
  } = nudgeFeedback;

  /** Re-reads the typed entry (position follows zoom, pan and resize; it
   * closes on a tool switch or a selection change). */
  const syncEntry = useCallback((session: WasmSession) => {
    const next = readTransformEntry(session.transform_entry());
    const nextMove = readMoveEntry(session.move_entry());
    const nextPath = parseTextPathEntry(session.text_path_entry_json());
    entryOpenRef.current = next !== null || nextMove !== null || nextPath !== null;
    setTextPathEntry((previous) => (sameEntry(previous, nextPath) ? previous : nextPath));
    setTransformEntry((previous) => (sameEntry(previous, next) ? previous : next));
    setMoveEntry((previous) => (sameEntry(previous, nextMove) ? previous : nextMove));
    if (nextMove === null) {
      // No chip, no mark under the pointer.
      setMoveMarkHover(-1);
    }
  }, []);

  /** Re-reads the modifier badges: a pure read of Rust's state, called after
   * every pointer event and every modifier change so a badge follows its key
   * in the same frame, with the pointer at rest. */
  const syncBadges = useCallback((session: WasmSession) => {
    const last = lastPointerRef.current;
    const raw = session.move_indicators();
    // Ctrl+Z and the like are a key chord, not a Ctrl click in waiting: no plus or minus.
    const chord = modifierTrackerRef.current.chord;
    const next: MoveBadgeState = {
      copy: last !== null && !chord && raw.copy_badge,
      remove: last !== null && !chord && raw.remove_badge,
      lock: last !== null && (raw.lock === "x" || raw.lock === "y" || raw.lock === "angle") ? raw.lock : "",
      x: last?.x ?? 0,
      y: last?.y ?? 0,
    };
    raw.free();
    setMoveBadges((previous) =>
      previous.copy === next.copy &&
      previous.remove === next.remove &&
      previous.lock === next.lock &&
      (!next.copy && !next.remove && next.lock === ""
        ? true
        : previous.x === next.x && previous.y === next.y)
        ? previous
        : next,
    );
  }, []);

  /** Re-reads every bit of session-owned UI state after any call that
   * might have changed it — cheap, and simpler than having every call
   * site know which ones to refresh. */
  const refreshHistoryRef = useRef<() => void>(() => {});
  const refreshGroupingRef = useRef<() => boolean>(() => false);
  const refreshClonesRef = useRef<() => void>(() => {});
  const refreshPlacementRef = useRef<() => void>(() => {});
  const refreshPenEntryRef = useRef<() => void>(() => {});
  const placementKeyDownRef = useRef<PlacementApi["keyDown"]>(() => {});
  const syncFromSession = useCallback(() => {
    const session = sessionRef.current;
    if (!session) {
      return;
    }
    refreshHistoryRef.current();
    const contextAnnounced = refreshGroupingRef.current();
    refreshClonesRef.current();
    setToolState(session.tool() as Tool);
    const nextSketch = JSON.parse(session.sketch_view_json()) as SketchView;
    // A completed pick in the previous tool must not consume the first pick of
    // a newly chosen tool as a double-click (e.g. Line endpoint -> Arc start).
    const clickContext = JSON.stringify([
      session.tool(), nextSketch.workbench, nextSketch.tool,
      nextSketch.page, nextSketch.active, nextSketch.mode,
    ]);
    if (clickContextRef.current !== clickContext) {
      lastPressRef.current = null;
      clickContextRef.current = clickContext;
    }
    sketchWorkbenchRef.current = nextSketch.workbench === "sketch";
    setSketchView(nextSketch);
    setNodeToolbarState(readToolbarState(session.node_toolbar_state()));
    setPolyStarModeState(session.poly_star_mode() as PolyStarMode);
    setPolyStarPointCountState(session.poly_star_point_count());
    setPolyStarRatioState(session.poly_star_ratio());
    // A new or opened project's session starts with the switch off
    // (criterion 27); reading it back here is what resets the UI.
    setScaleStrokeWidthState(session.scale_stroke_width());
    setScaleCornerRadiusState(session.scale_corner_radius());
    setLinkCornersState(session.link_corners());
    setFlipEnabled(session.flip_enabled());
    setGuideState(readGuideState(session.guide_state()));
    const nextBar = readSelectBar(session.select_bar_state());
    setSelectBar((previous) => (sameSelectBar(previous, nextBar) ? previous : nextBar));
    setZoomPercent(session.zoom_percent());
    setSelectionCount(session.selection_count());
    // One polite message at a time: the context announcement wins over the selection one.
    setSelectionAnnouncement(contextAnnounced ? "" : session.selection_announcement());
    setSelectionAnnouncementSeq(session.selection_announcement_seq());
    setSyncRevision((revision) => revision + 1);
    syncEntry(session);
    syncBadges(session);
    refreshPlacementRef.current();
    refreshPenEntryRef.current();
  }, [syncEntry, syncBadges, setSketchView]);

  const getSession = useCallback(() => sessionRef.current, []);
  // A right click in pick mode of text on path ends it (`specs/0080-text-on-path` UX notes §2).
  const cancelTextPathPick = useCallback(() => {
    const session = sessionRef.current;
    if (session?.cancel_text_path_pick()) {
      setCursorHint(session.cursor_hint());
      syncFromSession();
    }
  }, [syncFromSession]);
  const history = useHistory(getSession, syncFromSession, containerRef, onNudgeHistory);
  // `showNotice` is stable (a callback without dependencies), so the render loop may keep it.
  const showNoticeRef = useRef(history.showNotice);
  const { refresh: refreshHistory, handleKeyOutcome } = history;
  useEffect(() => {
    refreshHistoryRef.current = refreshHistory;
  }, [refreshHistory]);
  const grouping = useGrouping(getSession, syncFromSession, history.showNotice);
  const offset = useOffset(getSession, syncFromSession, syncRevision, history.showNotice);
  const text = useText(getSession, syncFromSession, syncRevision, history.showNotice, containerRef);
  const { focusSinkNow } = text;
  const { showReleaseOutcome: showOffsetOutcome } = offset;
  const { refresh: refreshGrouping, handleKeyOutcome: handleGroupingOutcome } = grouping;
  const placement = usePlacement(getSession, syncFromSession, containerRef);
  const { refresh: refreshPlacement, notePointer, keyDown: placementKeyDown } = placement;
  useEffect(() => {
    placementKeyDownRef.current = placementKeyDown;
  }, [placementKeyDown]);
  useEffect(() => {
    refreshPlacementRef.current = refreshPlacement;
  }, [refreshPlacement]);
  const showPenEntryNotice = useCallback(
    (text: string) => history.showNotice(text, undefined, "canvas", "status"),
    [history],
  );
  const penEntry = usePenEntry(getSession, syncFromSession, showPenEntryNotice, containerRef);
  const { refresh: refreshPenEntry } = penEntry;
  useEffect(() => {
    refreshPenEntryRef.current = refreshPenEntry;
  }, [refreshPenEntry]);
  useEffect(() => {
    refreshGroupingRef.current = refreshGrouping;
  }, [refreshGrouping]);
  const clones = useClones(getSession, syncFromSession, history.showNotice, containerRef);
  const strokeToPath = useStrokeToPath(getSession, syncFromSession);
  const strokeToPathReady = sketchView.workbench === "vector" && tool === "select" && selectBar.strokeToPathEnabled;
  const focusCanvas = useCallback(() => containerRef.current?.focus(), []);
  const vectorTool = sketchView.workbench === "vector" ? tool : "select";
  const trace = useTrace(getSession, syncFromSession, syncRevision, vectorTool, focusCanvas);
  const recognise = useRecognise(getSession, syncFromSession, {
    tool: vectorTool,
    selectionCount,
    syncRevision,
    faces: text.fonts.faces,
    containerRef,
  });
  // Enter and Escape of the Trace tool, and a double-click that creates, reach it through refs so
  // the canvas handlers keep their dependencies (`specs/0025-raster-trace` criterion 12).
  const traceKeyRef = useRef(trace.keyDown);
  const traceCreateRef = useRef(trace.create);
  useEffect(() => {
    traceKeyRef.current = trace.keyDown;
    traceCreateRef.current = trace.create;
  }, [trace.keyDown, trace.create]);
  const { refresh: refreshClones, handleKeyOutcome: handleCloneOutcome } = clones;
  const flip = useFlip(getSession, syncFromSession, history.showNotice);
  const { handleKeyOutcome: handleFlipOutcome } = flip;
  // A release or an Enter that set a size to 0.01 mm says so in the canvas notice slot
  // (`0056` criterion 15).
  const { announceText } = flip;
  const arcBar = useArcBar(getSession, syncFromSession, announceText, syncRevision);
  const showTransformNotice = useCallback(
    (session: WasmSession) => {
      const notice = session.take_transform_notice();
      if (notice !== "") {
        history.showNotice(notice, FLIP_NOTICE_MS, "canvas");
      }
      // A resize that mirrored says so politely: "Resized. Mirrored left to right."
      const announcement = session.take_transform_announcement();
      if (announcement !== "") {
        announceText(announcement);
      }
    },
    [history, announceText],
  );
  useEffect(() => {
    refreshClonesRef.current = refreshClones;
  }, [refreshClones]);

  const refreshCursor = useCallback(() => {
    const session = sessionRef.current;
    const at = lastPointerRef.current;
    if (session && at) {
      onCursorMove(readDocumentPoint(session.screen_to_document(at.x, at.y)));
    }
  }, [onCursorMove]);

  const addViewResizedListener = useCallback((listener: () => void) => {
    viewResizedListenersRef.current.add(listener);
    return () => {
      viewResizedListenersRef.current.delete(listener);
    };
  }, []);

  /** The canvas's right px a side pane lies over (`specs/0077-font-preview` criterion 2). */
  const coveredRightRef = useRef(0);
  const coverCanvasRight = useCallback((px: number) => {
    coveredRightRef.current = px;
    sessionRef.current?.cover_canvas_right(px);
  }, []);

  /** The backing buffer the session last drew into; a resize to the same buffer is skipped, as
   * setting a canvas's size clears it. `fitCanvas` measures the canvas and, if its buffer no
   * longer matches, resizes and redraws it in the same call. */
  const appliedBufferRef = useRef<BackingBuffer | null>(null);
  const fitCanvas = useCallback(() => {
    const canvas = canvasRef.current;
    const session = sessionRef.current;
    if (!canvas || !session) {
      return;
    }
    const next = backingBufferSize(canvas.clientWidth, canvas.clientHeight);
    const applied = appliedBufferRef.current;
    if (applied && sameBackingBuffer(applied, next)) {
      return;
    }
    appliedBufferRef.current = next;
    canvas.width = next.width;
    canvas.height = next.height;
    try {
      // Reconfigures the wgpu surface, updates the viewport's own CSS size (acceptance
      // criterion 10) and renders the next frame, all in this one call (see
      // `WasmSession::resize`'s own doc comment): a resize never leaves a stale or empty frame
      // on screen waiting for the render loop's next animation-frame tick.
      session.resize(next.width, next.height, next.devicePixelRatio);
      if (entryOpenRef.current) {
        syncEntry(session);
      }
    } catch {
      // A dropped frame on resize (e.g. a momentarily lost surface) is not fatal: the render
      // loop's own next tick recovers it.
    }
    for (const listener of viewResizedListenersRef.current) {
      listener();
    }
  }, [syncEntry]);

  /** Runs `task` once every earlier queued task has settled. Everything
   * that frees, replaces or publishes the live session goes through this
   * one queue, so none of it can overlap an in-flight `attach_canvas`:
   * that call is async (it negotiates a `wgpu` adapter/device) and holds
   * the session's `Gpu` slot while it runs, so freeing the session
   * underneath it is a use-after-free (`memory access out of bounds` in
   * `drop_glue<Option<Gpu>>`). A task that throws does not stop later
   * ones; its error goes to the caller of `enqueueSessionTask` only. */
  const enqueueSessionTask = useCallback((task: () => Promise<void> | void) => {
    const run = sessionQueueRef.current.then(task);
    sessionQueueRef.current = run.catch(() => {});
    return run;
  }, []);

  /** Frees whatever session is currently live, then makes `session` the
   * live one and attaches it to the host's `<canvas>` — the mount
   * effect's own first attach and every later New/Open swap (including a
   * file passed at launch, which arrives while the first attach may still
   * be in flight) all go through this one path.
   *
   * Runs on the session queue: it starts only after any earlier attach
   * has finished, so the session it frees is never mid-`attach_canvas`.
   * `sessionRef.current` holds a session only while it is safe to use
   * (null during the attach itself), so the render loop and input
   * handlers never call into a session that has an `attach_canvas` in
   * flight. The returned promise rejects if `attach_canvas` does; the
   * session is still published then, as a session without a canvas. */
  const attachSession = useCallback(
    (session: WasmSession) =>
      enqueueSessionTask(async () => {
        sessionRef.current?.free();
        sessionRef.current = null;
        // A new or opened project keeps the pane over the canvas (`coverCanvasRight`).
        session.cover_canvas_right(coveredRightRef.current);

        const canvas = canvasRef.current;
        if (!canvas) {
          sessionRef.current = session;
          return;
        }
        const { width, height, devicePixelRatio } = backingBufferSize(
          canvas.clientWidth,
          canvas.clientHeight,
        );
        canvas.width = width;
        canvas.height = height;
        appliedBufferRef.current = { width, height, devicePixelRatio };
        try {
          // `attach_canvas` also records the canvas's CSS size on the new
          // session's viewport (acceptance criterion 10's "resize keeps
          // the center" starts from a correctly-sized viewport on attach,
          // not just on the first later resize).
          await session.attach_canvas(canvas, width, height, devicePixelRatio);
        } finally {
          sessionRef.current = session;
        }
        // `attach_canvas` is async, and the ResizeObserver below skips every
        // resize while the session is not yet published, so a layout change
        // during the attach (the window shown or maximised, fonts or panels
        // settling) never reached the session. It would keep the size measured
        // before the attach and centre the document in that one. Measure again
        // and hand over the real size.
        const current = backingBufferSize(canvas.clientWidth, canvas.clientHeight);
        if (!sameBackingBuffer(current, { width, height, devicePixelRatio })) {
          appliedBufferRef.current = current;
          canvas.width = current.width;
          canvas.height = current.height;
          try {
            session.resize(current.width, current.height, current.devicePixelRatio);
          } catch {
            // As in the ResizeObserver: the render loop recovers a lost frame.
          }
        }
        syncFromSession();
        // Reads the file's history after the first paint, so that the first undo is as fast as
        // later ones (`specs/0020-undo-redo` criterion 30c).
        const idle = window.requestIdleCallback ?? ((run: () => void) => window.setTimeout(run, 200));
        idle(() => {
          if (sessionRef.current === session) {
            session.warm_history();
          }
        });
      }),
    [enqueueSessionTask, syncFromSession],
  );

  useEffect(() => {
    let cancelled = false;

    createSession().then((session) => {
      if (cancelled) {
        session.free();
        return;
      }
      void attachSession(session);
    });

    const canvas = canvasRef.current;
    let resizeObserver: ResizeObserver | null = null;
    if (canvas) {
      // A window resize: resized and redrawn inside the observer, before the frame is shown.
      resizeObserver = new ResizeObserver(() => fitCanvas());
      resizeObserver.observe(canvas);
    }

    // React's `onWheel` is passive and cannot `preventDefault`, so
    // Ctrl+wheel would zoom the whole webview page itself rather than
    // just the canvas (`specs/0004-canvas-navigation-and-selection/
    // adrs.md`'s frontend requirement) — a native, non-passive listener
    // is the only way to stop that.
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const session = sessionRef.current;
      if (!session || !canvas) {
        return;
      }
      const bounds = canvas.getBoundingClientRect();
      const x = event.clientX - bounds.left;
      const y = event.clientY - bounds.top;
      const { deltaX, deltaY } = normalizedWheelDelta(event);
      session.wheel(deltaX, deltaY, x, y, event.shiftKey, event.ctrlKey || event.metaKey);
      setZoomPercent(session.zoom_percent());
      // A pan or zoom moves the document under a still pointer.
      onCursorMove(readDocumentPoint(session.screen_to_document(x, y)));
      if (entryOpenRef.current) {
        syncEntry(session);
      }
      if (followPlacementAfterView(session, x, y, event)) {
        refreshPlacementRef.current();
      }
    };
    canvas?.addEventListener("wheel", onWheel, { passive: false });

    // Reads `sessionRef.current` fresh every frame (rather than closing
    // over one session instance) so a New/Open swap underneath it is
    // picked up on the very next frame, with no need to restart the
    // loop itself.
    // The view the clone tags were last placed for: they follow every pan and zoom, whatever
    // moved the view (`0049` UX review must-fix 7).
    let tagView = "";
    // The Trace tool draws its kept meshes while the view moves and tessellates them again for
    // the new zoom once it rests (`specs/0025-raster-trace` criterion 56).
    const viewRest = new ViewRest();
    let frame = requestAnimationFrame(function renderLoop() {
      const session = sessionRef.current;
      if (session) {
        const moving = viewRest.update(`${session.view_origin_x()},${session.view_origin_y()},${session.view_scale()}`, performance.now());
        if (moving !== null) {
          session.set_trace_view_moving(moving);
        }
      }
      session?.render();
      // An image of an opened project whose picture cannot be read, once per open
      // (`specs/0058-bitmap-import` criterion 30).
      const badAsset = session?.take_bad_asset_notice();
      if (badAsset) {
        showNoticeRef.current(badAsset, 5000, "canvas", "alert");
      }
      if (session) {
        const view = `${session.view_origin_x()},${session.view_origin_y()},${session.view_scale()}`;
        if (view !== tagView) {
          tagView = view;
          refreshClonesRef.current();
          refreshSketch();
        }
      }
      // An open typed entry follows every pan, zoom and resize, whatever moved the view: the
      // chip stays where its placement rule puts it (`0047` UX review N1). A re-read that
      // changed nothing sets no state.
      if (session && entryOpenRef.current) {
        syncEntry(session);
      }
      frame = requestAnimationFrame(renderLoop);
    });

    return () => {
      cancelled = true;
      cancelAnimationFrame(frame);
      resizeObserver?.disconnect();
      canvas?.removeEventListener("wheel", onWheel);
      // Queued, not immediate: an `attach_canvas` still in flight must
      // finish before its session is freed. StrictMode's remount queues
      // its own attach behind this, so it never sees the freed session.
      void enqueueSessionTask(() => {
        sessionRef.current?.free();
        sessionRef.current = null;
      });
    };
  }, [attachSession, enqueueSessionTask, fitCanvas, onCursorMove, syncEntry, refreshSketch]);

  const newProject = useCallback(
    () => createSession().then(attachSession),
    [attachSession],
  );

  const openProject = useCallback(
    async (bytes: Uint8Array) => {
      // The texts' faces are matched against the full listing, never a partial one at start.
      await fontListingReady();
      const session = await openSession(bytes);
      sendFontLists(session);
      await attachSession(session);
    },
    [attachSession],
  );

  const openFile = useCallback(
    async (bytes: Uint8Array, fileName: string) => {
      // Phase 1 matches the texts' faces against the full listing: a face in the Curvyo folder or
      // among the system fonts is found on the first Open after start too.
      await fontListingReady();
      const session = await openFileSession(bytes, fileName);
      sendFontLists(session);
      await attachSession(session);
      // An SVG with texts waits for their faces: the text hook loads what the session asks for
      // on every sync (`specs/0024-svg-import-export` criterion 106).
      await untilFacesLoaded((force) => session.continue_open_svg(force), syncFromSession);
      const result = { svg: session.opened_svg(), notice: session.take_open_notice() };
      syncFromSession();
      return result;
    },
    [attachSession, syncFromSession],
  );

  const importBegin = useCallback((origin: ImportOrigin, at: { x: number; y: number } | null) => {
    sessionRef.current?.import_begin(origin, at !== null, at?.x ?? 0, at?.y ?? 0);
  }, []);

  const lastCanvasPointer = useCallback(() => lastPointerRef.current, []);

  const importAdd = useCallback((bytes: Uint8Array, fileName: string) => {
    sessionRef.current?.import_add(bytes, fileName);
  }, []);

  const importAddTooLarge = useCallback((fileName: string, bytes: number) => {
    sessionRef.current?.import_add_too_large(fileName, bytes);
  }, []);

  const importFinish = useCallback(async () => {
    // As for Open: the faces are matched once every font block is listed.
    await fontListingReady();
    const session = sessionRef.current;
    if (!session) {
      return { imported: 0, notice: "", alert: false };
    }
    sendFontLists(session);
    let result = { imported: 0, notice: "", alert: false };
    await untilFacesLoaded((force) => {
      const outcome = session.import_finish(force);
      result = { imported: outcome.imported, notice: outcome.notice, alert: outcome.alert };
      const waiting = outcome.waiting;
      outcome.free();
      return waiting;
    }, syncFromSession);
    syncFromSession();
    return result;
  }, [syncFromSession]);

  const packProject = useCallback((): Uint8Array => {
    const session = sessionRef.current;
    if (!session) {
      throw new Error("no editing session is attached yet");
    }
    // A save in text edit mode writes the session as its step and edit mode goes on, so the file
    // holds what is on screen (`specs/0074-text-objects`, UX notes §2).
    session.checkpoint_text_edit();
    return session.pack();
  }, []);

  const toggleGuide = useCallback(() => {
    sessionRef.current?.toggle_guide();
    syncFromSession();
  }, [syncFromSession]);

  const loadTemplate = useCallback((bytes: Uint8Array): string => {
    const session = sessionRef.current;
    if (!session) {
      return "error:";
    }
    const outcome = session.load_template(bytes);
    syncFromSession();
    if (outcome.startsWith("loaded:") && outcome !== "loaded:0") {
      // The first view waits until the bars are laid out for the new selection, so the page and
      // the guides are fitted into the area they leave free (`specs/0046-document-shapes`
      // criterion 13).
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          const canvas = containerRef.current;
          if (!canvas || sessionRef.current !== session) {
            return;
          }
          const bar = canvas.parentElement?.querySelector("[data-context-bar]") ?? null;
          const free = freeCanvasArea(
            canvas.getBoundingClientRect(),
            bar?.getBoundingClientRect() ?? null,
          );
          session.show_page_and_guides(free.left, free.top);
          syncFromSession();
        }),
      );
    }
    return outcome;
  }, [syncFromSession]);

  const markSaved = useCallback(() => {
    sessionRef.current?.mark_saved();
  }, []);

  const setTool = useCallback(
    (next: Tool) => {
      sessionRef.current?.set_tool(next);
      syncFromSession();
    },
    [syncFromSession],
  );

  const deleteSelected = useCallback(() => {
    sessionRef.current?.delete_selected();
    syncFromSession();
  }, [syncFromSession]);

  const convertSelected = useCallback(
    (kind: "corner" | "symmetric" | "asymmetric") => {
      sessionRef.current?.convert_selected(kind);
      syncFromSession();
    },
    [syncFromSession],
  );

  const makeLine = useCallback(() => {
    sessionRef.current?.make_line();
    syncFromSession();
  }, [syncFromSession]);

  const makeCurve = useCallback(() => {
    sessionRef.current?.make_curve();
    syncFromSession();
  }, [syncFromSession]);

  const joinSelected = useCallback((): number => {
    const moved = sessionRef.current?.join_selected() ?? 0;
    syncFromSession();
    return moved;
  }, [syncFromSession]);

  const splitSelected = useCallback((): string => {
    const code = sessionRef.current?.split_selected() ?? "";
    syncFromSession();
    return code;
  }, [syncFromSession]);

  const insertSelected = useCallback(() => {
    sessionRef.current?.insert_selected();
    syncFromSession();
  }, [syncFromSession]);

  const removeCornerRounding = useCallback(() => {
    sessionRef.current?.remove_corner_rounding();
    syncFromSession();
  }, [syncFromSession]);

  const setScaleStrokeWidth = useCallback(
    (on: boolean) => {
      sessionRef.current?.set_scale_stroke_width(on);
      syncFromSession();
    },
    [syncFromSession],
  );

  const setScaleCornerRadius = useCallback(
    (on: boolean) => {
      sessionRef.current?.set_scale_corner_radius(on);
      syncFromSession();
    },
    [syncFromSession],
  );

  const setLinkCorners = useCallback(
    (on: boolean) => {
      sessionRef.current?.set_link_corners(on);
      syncFromSession();
    },
    [syncFromSession],
  );

  const setSelectedRadius = useCallback(
    (text: string): string => {
      const session = sessionRef.current;
      if (!session) {
        return "unchanged";
      }
      const outcome = session.set_selected_radius(text);
      syncFromSession();
      return outcome;
    },
    [syncFromSession],
  );

  const setSelectedPointCount = useCallback(
    (count: number) => {
      sessionRef.current?.set_selected_point_count(count);
      syncFromSession();
    },
    [syncFromSession],
  );

  const previewSelectedRatio = useCallback(
    (ratio: number) => {
      sessionRef.current?.preview_selected_ratio(ratio);
      syncFromSession();
    },
    [syncFromSession],
  );

  const commitSelectedRatio = useCallback(() => {
    sessionRef.current?.commit_selected_ratio();
    syncFromSession();
  }, [syncFromSession]);

  const setSelectedRatio = useCallback(
    (ratio: number) => {
      sessionRef.current?.set_selected_ratio(ratio);
      syncFromSession();
    },
    [syncFromSession],
  );

  const cancelSelectedRatio = useCallback(() => {
    sessionRef.current?.cancel_selected_ratio();
    syncFromSession();
  }, [syncFromSession]);

  const setPolyStarMode = useCallback(
    (mode: PolyStarMode) => {
      sessionRef.current?.set_poly_star_mode(mode);
      syncFromSession();
    },
    [syncFromSession],
  );

  const setPolyStarPointCount = useCallback(
    (count: number) => {
      sessionRef.current?.set_poly_star_point_count(count);
      syncFromSession();
    },
    [syncFromSession],
  );

  const setPolyStarRatio = useCallback(
    (ratio: number) => {
      sessionRef.current?.set_poly_star_ratio(ratio);
      syncFromSession();
    },
    [syncFromSession],
  );

  const convertSelectedToPaths = useCallback(() => {
    sessionRef.current?.convert_selected_to_paths();
    syncFromSession();
  }, [syncFromSession]);

  const closePath = useCallback(
    (join: "sharp" | "smooth"): ClosePathResult => {
      const raw = sessionRef.current?.close_path(join);
      syncFromSession();
      if (!raw) {
        return { closed: 0, skipped: 0, sameEnds: 0 };
      }
      const result = {
        closed: raw.closed,
        skipped: raw.skipped,
        sameEnds: raw.same_ends,
        closedOutlines: raw.closed_outlines,
        skippedOutlines: raw.skipped_outlines,
        sameEndsOutlines: raw.same_ends_outlines,
      };
      raw.free();
      return result;
    },
    [syncFromSession],
  );

  /** The one missing face a command's refusal of texts names (`specs/0074-text-objects`). */
  const refusedFace = useCallback(
    (kind: string) =>
      kind === "missing_fonts" ? (sessionRef.current?.text_refusal_face() ?? "") : "",
    [],
  );

  const applyBoolean = useCallback(
    (op: BooleanOp): BooleanResult => {
      const raw = sessionRef.current?.apply_boolean(op);
      syncFromSession();
      if (!raw) {
        return {
          kind: "ignored",
          count: 0,
          of: 0,
          compound: false,
          closedOpen: 0,
          removedOpen: 0,
        };
      }
      const result = {
        kind: raw.kind,
        count: raw.count,
        of: raw.of,
        compound: raw.compound,
        closedOpen: raw.closed_open,
        removedOpen: raw.removed_open,
        face: refusedFace(raw.kind),
      };
      raw.free();
      return result;
    },
    [syncFromSession, refusedFace],
  );

  const applyCombine = useCallback((): CombineResult => {
    const raw = sessionRef.current?.apply_combine();
    syncFromSession();
    if (!raw) {
      return {
        kind: "ignored",
        count: 0,
        of: 0,
        holes: 0,
        open: 0,
        touching: 0,
        touchingHoles: 0,
        singleNodesRemoved: 0,
        markers: false,
        stylesDiffer: false,
      };
    }
    const result = {
      kind: raw.kind,
      count: raw.count,
      of: raw.of,
      holes: raw.holes,
      open: raw.open,
      touching: raw.touching,
      touchingHoles: raw.touching_holes,
      singleNodesRemoved: raw.single_nodes_removed,
      markers: raw.markers,
      stylesDiffer: raw.styles_differ,
      face: refusedFace(raw.kind),
    };
    raw.free();
    return result;
  }, [syncFromSession, refusedFace]);

  const applySplit = useCallback((): SplitResult => {
    const raw = sessionRef.current?.apply_split();
    syncFromSession();
    if (!raw) {
      return { kind: "ignored", compounds: 0, pieces: 0, withHoles: false, leftAlone: 0 };
    }
    const result = {
      kind: raw.kind,
      compounds: raw.compounds,
      pieces: raw.pieces,
      withHoles: raw.with_holes,
      leftAlone: raw.left_alone,
      of: raw.of,
      face: refusedFace(raw.kind),
    };
    raw.free();
    return result;
  }, [syncFromSession, refusedFace]);

  const applyBreakApart = useCallback((): BreakApartResult => {
    const raw = sessionRef.current?.apply_break_apart();
    syncFromSession();
    if (!raw) {
      return { kind: "ignored", compounds: 0, pieces: 0, withHoles: false, singleNodesRemoved: 0 };
    }
    const result = {
      kind: raw.kind,
      compounds: raw.compounds,
      pieces: raw.pieces,
      withHoles: raw.with_holes,
      singleNodesRemoved: raw.single_nodes_removed,
      face: refusedFace(raw.kind),
    };
    raw.free();
    return result;
  }, [syncFromSession, refusedFace]);

  const applyCut = useCallback((): CutResult => {
    const raw = sessionRef.current?.apply_cut();
    syncFromSession();
    if (!raw) {
      return { kind: "ignored", count: 0, of: 0, pieces: 0 };
    }
    const result = {
      kind: raw.kind,
      count: raw.count,
      of: raw.of,
      pieces: raw.pieces,
      face: refusedFace(raw.kind),
    };
    raw.free();
    return result;
  }, [syncFromSession, refusedFace]);

  const applyStack = useCallback(
    (command: StackCommand): StackResult => {
      const session = sessionRef.current;
      const raw = command === "fracture" ? session?.apply_fracture() : session?.apply_flatten();
      syncFromSession();
      if (!raw) {
        return {
          kind: "ignored",
          count: 0,
          of: 0,
          removed: 0,
          pieces: 0,
          hidden: 0,
          strokesFollow: false,
          closedOpen: 0,
          removedOpen: 0,
        };
      }
      const result = {
        kind: raw.kind,
        count: raw.count,
        of: raw.of,
        removed: raw.removed,
        pieces: raw.pieces,
        hidden: raw.hidden,
        strokesFollow: raw.strokes_follow,
        closedOpen: raw.closed_open,
        removedOpen: raw.removed_open,
        face: refusedFace(raw.kind),
      };
      raw.free();
      return result;
    },
    [syncFromSession, refusedFace],
  );

  const applyWeld = useCallback((): WeldResult => {
    const raw = sessionRef.current?.apply_weld();
    syncFromSession();
    if (!raw) {
      return {
        kind: "ignored",
        count: 0,
        of: 0,
        textsOnly: false,
        compound: false,
        lengthText: "",
        hidden: 0,
        closedOpen: 0,
        removedOpen: 0,
        mixedStyles: false,
        fillSwitchedOff: false,
        strokeSwitchedOn: false,
      };
    }
    const result = {
      kind: raw.kind,
      count: raw.count,
      of: raw.of,
      textsOnly: raw.texts_only,
      compound: raw.compound,
      lengthText: raw.length_text,
      hidden: raw.hidden,
      closedOpen: raw.closed_open,
      removedOpen: raw.removed_open,
      mixedStyles: raw.mixed_styles,
      fillSwitchedOff: raw.fill_switched_off,
      strokeSwitchedOn: raw.stroke_switched_on,
      face: refusedFace(raw.kind),
    };
    raw.free();
    return result;
  }, [syncFromSession, refusedFace]);

  /** The event's canvas-relative CSS pixel position — pure DOM geometry,
   * no wasm call and no document-space conversion (that happens inside
   * `Session` now, `specs/0004-canvas-navigation-and-selection/adrs.md`).
   */
  const canvasPoint = useCallback((event: { clientX: number; clientY: number }) => {
    const canvas = canvasRef.current;
    if (!canvas) {
      return { x: 0, y: 0 };
    }
    const bounds = canvas.getBoundingClientRect();
    return {
      x: event.clientX - bounds.left,
      y: event.clientY - bounds.top,
    };
  }, []);

  const onPointerDown = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>) => {
      if (releasePendingRef.current) {
        return;
      }
      syncTrackedModifiers(event);
      modifierTrackerRef.current.pointerPressed();
      // A press on the canvas takes the focus from a field of the panel, and a
      // field that commits when it is left (the Document size) must do so
      // before the tool sees the press: a resize moves every object and the
      // view, so a press handled first would be stored in the old coordinates.
      const focused = document.activeElement;
      if (
        focused instanceof HTMLInputElement &&
        !containerRef.current?.contains(focused)
      ) {
        focused.blur();
      }
      const session = sessionRef.current;
      if (!session) {
        return;
      }
      const { x, y } = canvasPoint(event);

      // Middle-mouse or Space+primary: a drag-pan gesture (acceptance
      // criteria 3, 4), never forwarded to the active tool — captured so
      // the gesture continues even if the cursor leaves the canvas.
      const isMiddleButton = event.button === 1;
      const isSpacePrimary = event.button === 0 && isSpaceHeld;
      if (isMiddleButton || isSpacePrimary) {
        event.preventDefault();
        event.currentTarget.setPointerCapture(event.pointerId);
        panningRef.current = true;
        setIsPanning(true);
        session.begin_pan(x, y);
        return;
      }

      // Sketch secondary events cancel a pending step and never enter primary creation.
      if (event.button === 2 && sketchWorkbenchRef.current) {
        if (session.sketch_secondary_press()) event.preventDefault();
        syncFromSession();
        return;
      }

      // The other button during an Offset drag cancels it (`0038` criterion 17).
      if (event.button !== 0 && session.cancel_offset_drag()) {
        event.preventDefault();
        setLiveReadout(null);
        syncFromSession();
        return;
      }

      // A right press while the eyedropper is picking ends picking and writes
      // nothing; no tool sees it and no context menu opens
      // (`specs/0017-style-panel-rework` criterion 26).
      if (event.button === 2 && session.cursor_hint() === "eyedropper") {
        event.preventDefault();
        session.end_colour_pick();
        setCursorHint(session.cursor_hint());
        syncFromSession();
        return;
      }

      // A right press in pick mode of text on path ends it and puts nothing
      // (`specs/0080-text-on-path` UX notes §2).
      if (event.button === 2 && session.text_path_picking()) {
        event.preventDefault();
        session.cancel_text_path_pick();
        setCursorHint(session.cursor_hint());
        syncFromSession();
        return;
      }

      // A secondary press with the Pen finishes the path in progress as Enter does and places
      // nothing (`0072` criterion 19a, customer 2026-10-19).
      if (event.button === 2 && session.pen_secondary_press()) {
        event.preventDefault();
        syncFromSession();
        return;
      }

      // A secondary press in a creation tool creates nothing and starts no placement; during a
      // placement the window listener of `usePlacement` has cancelled it (`0052` criterion 7).
      const pressedTool = session.tool();
      if (
        event.button === 2 &&
        (pressedTool === "rectangle" || pressedTool === "ellipse" || pressedTool === "polygon-star")
      ) {
        return;
      }

      // A press on a mark of the open typed move belongs to its chip: it does not cancel the
      // chip, select, move or start a marquee, and acts at the release
      // (`specs/0047-transform-polish/` criterion 40). Cancelling the default keeps the focus
      // in the chip.
      if (event.button === 0) {
        const touch = event.pointerType !== "mouse";
        const mark = session.move_mark_at(x, y, touch);
        if (mark >= 0) {
          event.preventDefault();
          try {
            event.currentTarget.setPointerCapture(event.pointerId);
          } catch {
            // Not capturable (e.g. a synthetic event): the click still works over the canvas.
          }
          markPressRef.current = { mark, x, y, touch };
          lastPressRef.current = null;
          return;
        }
      }

      const now = performance.now();
      const last = lastPressRef.current;
      // Alt-clicks step through overlapping objects (`specs/0014-advanced-selection/`
      // criterion 5), quickly and at one point: never a double-click.
      const isDoubleClick =
        !event.altKey &&
        last !== null &&
        now - last.time < DOUBLE_CLICK_MS &&
        Math.hypot(x - last.x, y - last.y) < DOUBLE_CLICK_PX;
      lastPressRef.current = { time: now, x, y };
      suppressedPressRef.current = isDoubleClick;
      if (isDoubleClick) {
        doubleClickPressRef.current = {
          x,
          y,
          shift: event.shiftKey,
          ctrl: event.ctrlKey || event.metaKey,
        };
        return;
      }
      setHandleHint("");
      // Capture the pointer for every tool, not only for panning: a
      // transform drag (a rotate swings the pointer in a wide arc) must
      // keep receiving moves and the release when the pointer leaves the
      // canvas, instead of losing the release and clearing its readout.
      if (event.button === 0) {
        try {
          event.currentTarget.setPointerCapture(event.pointerId);
        } catch {
          // Not capturable (e.g. a synthetic event) — the gesture still
          // works while the pointer stays over the canvas.
        }
      }
      // A tap is 8 px, a click 3 px (`0052` criteria 1 and 31).
      notePointer(event.pointerType);
      session.pointer_down(
        x,
        y,
        event.shiftKey,
        event.ctrlKey || event.metaKey,
        event.altKey,
      );
      // Drawing into a hidden or locked layer is refused with a notice (`0050` criterion 23).
      const layerNotice = parseLayerNotice(session.take_layer_notice());
      if (layerNotice) {
        history.showNotice(layerNotice.text, undefined, "canvas", layerNotice.kind);
      }
      setCursorHint(session.cursor_hint());
      syncFromSession();
    },
    [canvasPoint, history, isSpaceHeld, notePointer, syncFromSession, syncTrackedModifiers],
  );

  const onPointerMove = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>) => {
      if (releasePendingRef.current) {
        return;
      }
      syncTrackedModifiers(event);
      const session = sessionRef.current;
      const { x, y } = canvasPoint(event);
      lastPointerRef.current = { x, y };
      if (session) {
        onCursorMove(readDocumentPoint(session.screen_to_document(x, y)));
      }
      if (panningRef.current) {
        session?.pan_to(x, y);
        if (session) {
          // The pan moved the document under the pointer: read it again.
          onCursorMove(readDocumentPoint(session.screen_to_document(x, y)));
        }
        if (entryOpenRef.current && session) {
          syncEntry(session);
        }
        if (session && followPlacementAfterView(session, x, y, event)) {
          refreshPlacement();
        }
        return;
      }
      session?.pointer_hover(
        x,
        y,
        event.shiftKey,
        event.ctrlKey || event.metaKey,
        event.altKey,
      );
      if (sketchWorkbenchRef.current) {
        // Publish annotations before the canvas paints the same Rust preview.
        // Vector tool readouts have no owner in the Sketch workbench.
        flushSync(refreshSketch);
        // The Lock badge follows the pointer while Ctrl holds a direction (`0066` UX notes
        // revision 2, section 5).
        if (session) syncBadges(session);
        return;
      }
      const offsetReadout = session?.tool() === "offset" ? offsetLiveReadout(session, x, y) : null;
      setLiveReadout(offsetReadout ?? readLiveReadout(session?.live_readout()));
      if (offsetReadout) {
        // The handles and the preview follow an Offset drag live (`0038` criterion 15).
        syncFromSession();
      }
      setCursorHint(session?.cursor_hint() ?? "default");
      setHandleHint(session?.handle_hint() ?? "");
      setCornerHintLines(session?.corner_hint_lines() ?? []);
      setMoveMarkHover(session?.move_mark_hover() ?? -1);
      if (session) {
        syncBadges(session);
      }
      // The placement chip follows the pointer until the freeze (`0052` criterion 11).
      refreshPlacement();
      // The Pen entry chip follows the pointer until the freeze (`0072` criterion 15).
      refreshPenEntry();
    },
    [
      canvasPoint,
      onCursorMove,
      syncEntry,
      syncBadges,
      syncTrackedModifiers,
      refreshSketch,
      syncFromSession,
      refreshPlacement,
      refreshPenEntry,
    ],
  );

  const onPointerUp = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>) => {
      if (releasePendingRef.current) {
        return;
      }
      syncTrackedModifiers(event);
      const session = sessionRef.current;
      if (!session) {
        return;
      }
      if (event.button === 2 && sketchWorkbenchRef.current) return;
      const { x, y } = canvasPoint(event);

      if (panningRef.current) {
        panningRef.current = false;
        setIsPanning(false);
        session.end_pan();
        if (followPlacementAfterView(session, x, y, event)) {
          refreshPlacementRef.current();
        }
        try {
          event.currentTarget.releasePointerCapture(event.pointerId);
        } catch {
          // Already released (e.g. the pointer was already up) — not an
          // error worth surfacing.
        }
        return;
      }

      try {
        event.currentTarget.releasePointerCapture(event.pointerId);
      } catch {
        // Not captured (e.g. a non-primary button) — nothing to release.
      }

      // The release of a press on a mark: a click if the pointer stayed within the slop, else
      // nothing happens and the chip stays open.
      const markPress = markPressRef.current;
      if (markPress) {
        markPressRef.current = null;
        const slop = markPress.touch ? MARK_TOUCH_SLOP_PX : MARK_SLOP_PX;
        if (Math.hypot(x - markPress.x, y - markPress.y) < slop) {
          markClickRef.current?.(markPress.mark);
        }
        return;
      }

      if (suppressedPressRef.current) {
        suppressedPressRef.current = false;
        // One dispatch point for every tool (acceptance criteria 3, 12,
        // 22, 23) — `Session` itself decides what a double-click does. A
        // transform handle is picked at the *second press*: its position
        // and modifiers, not the release's.
        const press = doubleClickPressRef.current ?? {
          x,
          y,
          shift: event.shiftKey,
          ctrl: event.ctrlKey || event.metaKey,
        };
        doubleClickPressRef.current = null;
        const hintCode = session.double_click(press.x, press.y, press.shift, press.ctrl);
        if (hintCode === "offset_chip") {
          // The chip mounts with the next sync and takes the focus (`0038` criterion 53).
        } else if (hintCode === "create_trace") {
          traceCreateRef.current();
        } else if (hintCode) {
          // A double-click on a primitive changes nothing; the hint says how
          // to edit it (criterion 32), on every such double-click. On a
          // linked clone it says it has no nodes (`0049` criterion 26).
          editHintCounter.current += 1;
          setEditHint({
            x: press.x,
            y: press.y,
            id: editHintCounter.current,
            clone: hintCode === "clone",
          });
        }
        // Re-run the hover so the cursor describes the handle under the
        // pointer right away (a skew double-click changes nothing else).
        session.pointer_hover(
          x,
          y,
          event.shiftKey,
          event.ctrlKey || event.metaKey,
          event.altKey,
        );
      } else {
        const [shift, ctrl, alt] = [event.shiftKey, event.ctrlKey || event.metaKey, event.altKey];
        if (session.release_is_slow()) {
          // A move or transform of many objects: the commit can take seconds.
          // Show the `wait` cursor with the preview unchanged, let it paint,
          // and only then release (`specs/0019-multi-object-transform/`
          // criteria 28, 46, 49). Pointer events are ignored meanwhile.
          releasePendingRef.current = true;
          setCursorHint("wait");
          window.requestAnimationFrame(() => {
            window.setTimeout(() => {
              try {
                session.pointer_up(x, y, shift, ctrl, alt);
                showOffsetOutcome();
                showTransformNotice(session);
                setLiveReadout(null);
                setCursorHint(session.cursor_hint());
                syncFromSession();
              } finally {
                releasePendingRef.current = false;
              }
            }, 0);
          });
          return;
        }
        const joinSeq = session.pen_join_notice_seq();
        session.pointer_up(x, y, shift, ctrl, alt);
        showOffsetOutcome();
        showTransformNotice(session);
        // A Pen join that moved outlines into the survivor says so at the joined node, in the
        // edit hint chip's surface (`0055` criterion 9): the Pen has no button for a notice.
        const joinNotice =
          session.pen_join_notice_seq() !== joinSeq
            ? joinNoticeText(session.pen_join_moved_outlines())
            : null;
        if (joinNotice !== null) {
          editHintCounter.current += 1;
          setEditHint({ x, y, id: editHintCounter.current, lines: [joinNotice] });
        }
      }
      setLiveReadout(null);
      setCursorHint(session.cursor_hint());
      syncFromSession();
      // A press in the Text tool starts edit mode: the keys typed right after the release must
      // reach the sink, not the canvas, where they are tool letters (`0074` UX review).
      focusSinkNow();
    },
    [
      canvasPoint,
      syncFromSession,
      syncTrackedModifiers,
      showOffsetOutcome,
      showTransformNotice,
      focusSinkNow,
    ],
  );

  const onPointerCancel = useCallback(() => {
    markPressRef.current = null;
    sessionRef.current?.pointer_cancelled();
    setLiveReadout(null);
    setCursorHint("default");
    syncFromSession();
  }, [syncFromSession]);

  const dismissEditHint = useCallback(() => setEditHint(null), []);

  const onPointerLeave = useCallback(() => {
    // A double-click is two presses with the pointer on the canvas between them: a press after
    // the pointer went elsewhere (a rail tool, the entry box) is a new first press (0066 AC33).
    lastPressRef.current = null;
    sessionRef.current?.pointer_leave();
    setLiveReadout(null);
    setCursorHint("default");
    setHandleHint("");
    setMoveMarkHover(-1);
    lastPointerRef.current = null;
    setMoveBadges(NO_BADGES);
  }, []);

  /** Tells the session the modifier state changed — from window-level key
   * events, so Shift reveals the side rotate handles with no pointer
   * movement and while the canvas is not focused — and re-runs the hover
   * at the last pointer position, so the pivot marker, live preview, cursor
   * and hint follow the key in the same frame
   * (`object-transform-refinements` criteria 6, 14). */
  const applyModifiers = useCallback((shift: boolean, ctrl: boolean, alt: boolean) => {
    const session = sessionRef.current;
    if (!session) {
      return;
    }
    session.modifiers_changed(shift, ctrl, alt);
    refreshSketch();
    const last = lastPointerRef.current;
    if (last) {
      session.pointer_hover(last.x, last.y, shift, ctrl, alt);
      // An Offset drag keeps its own readout through a modifier change (`0038` criteria 15, 19).
      const offsetReadout =
        session.tool() === "offset" ? offsetLiveReadout(session, last.x, last.y) : null;
      setLiveReadout(offsetReadout ?? readLiveReadout(session?.live_readout()));
      setHandleHint("");
    }
    setCursorHint(session.cursor_hint());
    syncBadges(session);
    // An unfrozen placement follows the keys with the pointer still (`0052` criterion 3).
    refreshPlacementRef.current();
    // An unfrozen Pen entry chip follows Ctrl too (`0072` criterion 15).
    refreshPenEntryRef.current();
  }, [syncBadges, refreshSketch]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      // A lone Alt press or release would focus the menu bar in some browsers
      // and on Windows. With the pointer on the canvas, Select uses Alt for
      // its selection gestures; Vector drawing inverts snapping and Sketch drawing cycles modes.
      // Only the Alt key itself is intercepted; text-entry chords stay native.
      if (
        event.key === "Alt" && !event.ctrlKey && !event.metaKey && !event.getModifierState("AltGraph") &&
        (!sketchWorkbenchRef.current || !isFormControl(event.target)) &&
        lastPointerRef.current !== null &&
        ["select", "pen", "rectangle", "ellipse", "polygon-star"].includes(
          sessionRef.current?.tool() ?? "",
        )
      ) {
        event.preventDefault();
      }
      // Tracked by key, not read from the event's flags (`lib/keyModifiers.ts`).
      const held = modifierTrackerRef.current.keyEvent(event);
      // In a placement Shift and Ctrl are taps, not holds (`0052` criterion 32): a key event
      // reports them up; the pointer events carry the keys that are really down.
      if (sessionRef.current?.placement_running()) {
        if (event.type === "keydown") {
          placementKeyDownRef.current(event);
        }
        applyModifiers(false, false, held.alt);
        return;
      }
      applyModifiers(held.shift, held.ctrl, held.alt);
    };
    // A key released outside the window must not leave Shift stuck: the
    // side rotate handles and the pivot follow `modifiers_changed`.
    const onWindowBlur = () => {
      sessionRef.current?.pointer_cancelled();
      modifierTrackerRef.current.reset();
      applyModifiers(false, false, false);
    };
    // A hidden page gets no key-up either.
    const onVisibility = () => {
      if (document.visibilityState === "hidden") {
        modifierTrackerRef.current.reset();
        applyModifiers(false, false, false);
      }
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("keyup", onKey);
    window.addEventListener("blur", onWindowBlur);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("keyup", onKey);
      window.removeEventListener("blur", onWindowBlur);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [applyModifiers]);

  /** The canvas container lost focus to something outside it. */
  const onContainerBlur = useCallback(
    (event: React.FocusEvent<HTMLDivElement>) => {
      const next = event.relatedTarget;
      if (next instanceof Node && event.currentTarget.contains(next)) {
        return;
      }
      modifierTrackerRef.current.reset();
      applyModifiers(false, false, false);
    },
    [applyModifiers],
  );

  /** A key pressed on the canvas. Every rule (the gate of criterion 55, the
   * key table of criterion 54, the Escape cascade of criterion 42) is
   * `Session::key_down`'s: this forwards the key, the modifiers, the key
   * repeat and the one fact only the DOM has (an IME composition is running
   * or Space is held), and prevents the page's default for everything the
   * session handled, so Ctrl+R and Ctrl+S keep their page and menu behaviour.
   * Keys typed into a control are that control's, not the canvas's. */
  const onKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      if (isFormControl(event.target)) {
        return;
      }
      if (event.key === " ") {
        // Space+drag pans (acceptance criterion 4) — `preventDefault`
        // so it never activates a focused button
        // (`specs/0004-canvas-navigation-and-selection/adrs.md`'s
        // frontend requirement); the open-hand cursor shows from this
        // moment, before any drag motion (`docs/design-system.md`'s
        // "Pan cursor").
        event.preventDefault();
        setIsSpaceHeld(true);
        return;
      }
      const session = sessionRef.current;
      if (!session) {
        return;
      }
      // Tab opens the on-canvas entry box when Rust offers one (`0066` AC33); otherwise nothing.
      if (sketchWorkbenchRef.current && event.key === "Tab" && !event.ctrlKey && !event.metaKey && !event.altKey) {
        // A press-drag in progress continues as the follow state while the box is open (AC39).
        if (sketchPresentation.canvas().entry) { event.preventDefault(); session.sketch_open_entry(); sketchPresentation.setEntryOpen(true); return; }
      }
      // Ctrl+Alt+C: Stroke to path with the button's rule; with a dimmed button it does nothing
      // and shows no notice (`specs/0085-stroke-to-path` criterion 3, UX notes "Keyboard").
      if (sketchView.workbench === "vector" && isStrokeToPathChord(event.nativeEvent) && !isSpaceHeld) {
        event.preventDefault();
        if (strokeToPathReady && !event.repeat) {
          strokeToPath.apply();
        }
        return;
      }
      if (sketchView.workbench === "vector" && traceKeyRef.current(event.nativeEvent)) {
        event.preventDefault();
        return;
      }
      const outcome = session.key_down(
        event.key,
        event.shiftKey,
        event.ctrlKey || event.metaKey,
        event.altKey,
        event.repeat,
        event.nativeEvent.isComposing || isSpaceHeld,
        MAC,
        event.timeStamp,
      );
      const offsetKey = offsetKeyOutcome(outcome);
      if (offsetKey.ignored) {
        return;
      }
      // Escape with nothing to cancel stays unhandled: the open font preview closes on it.
      if (!canvasLeavesKey(outcome)) {
        event.preventDefault();
      }
      if (offsetKey.cornerStyle) {
        // Alt stepped the corner style of an Offset drag: the readout and the preview show it
        // now (`0038` criterion 13).
        const at = lastPointerRef.current;
        if (at) {
          setLiveReadout(offsetLiveReadout(session, at.x, at.y));
        }
        syncFromSession();
        return;
      }
      if (handleKeyOutcome(outcome)) {
        setLiveReadout(readLiveReadout(session.live_readout()));
        setCursorHint(session.cursor_hint());
        syncFromSession();
        return;
      }
      // Group and Ungroup say what they did in the canvas notice slot, as the buttons do.
      handleGroupingOutcome(outcome);
      if (outcome === "reordered") {
        // A refused or unchanged stacking-order key says why (`0050` criterion 33).
        const layerNotice = parseLayerNotice(session.take_layer_notice());
        if (layerNotice) {
          history.showNotice(layerNotice.text, undefined, "canvas", layerNotice.kind);
        }
      }
      handleCloneOutcome(outcome);
      handleFlipOutcome(outcome);
      if (outcome === "select-all") {
        onSelectAll(session.selection_count());
      } else if (outcome === "nudge" || outcome === "nudge-new") {
        onNudge(session, outcome === "nudge-new");
      } else if (outcome.startsWith("escape")) {
        onEscape();
      }
      const hint = KEY_HINT_TEXT[outcome];
      if (hint !== undefined) {
        keyHintCounter.current += 1;
        setKeyHint({ text: hint, id: keyHintCounter.current });
      }
      setLiveReadout(readLiveReadout(session?.live_readout()));
      setCursorHint(session.cursor_hint());
      syncFromSession();
    },
    [
      handleKeyOutcome,
      handleGroupingOutcome,
      history,
      handleCloneOutcome,
      handleFlipOutcome,
      isSpaceHeld,
      syncFromSession,
      onNudge,
      onSelectAll,
      onEscape,
      strokeToPath,
      strokeToPathReady,
      sketchView.workbench,
      sketchPresentation,
    ],
  );

  // A key hint clears itself after 2 s; a newer message restarts the clock.
  useEffect(() => {
    if (!keyHint) {
      return undefined;
    }
    const timer = window.setTimeout(() => setKeyHint(null), KEY_HINT_MS);
    return () => window.clearTimeout(timer);
  }, [keyHint]);

  const onKeyUp = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      if (isFormControl(event.target)) {
        return;
      }
      if (event.key === " ") {
        setIsSpaceHeld(false);
      }
      onNudgeKeyUp(event.key);
    },
    [onNudgeKeyUp],
  );

  const commitTransformEntry = useCallback(
    (first: string, second: string, lastEdited: number): string => {
      const session = sessionRef.current;
      if (!session) {
        return "unchanged";
      }
      const outcome = session.commit_transform_entry(first, second, lastEdited);
      showTransformNotice(session);
      syncFromSession();
      // A typed corner radius past its limit is committed limited and says
      // "r 12.0 mm max" at the knob for 1.5 s: a limit is never silent.
      const notice = readLiveReadout(session?.live_readout());
      if (outcome === "committed" && notice) {
        setLiveReadout(notice);
        window.clearTimeout(limitNoticeTimer.current);
        limitNoticeTimer.current = window.setTimeout(() => {
          sessionRef.current?.clear_limit_notice();
          setLiveReadout(null);
        }, LIMIT_NOTICE_MS);
      }
      return outcome;
    },
    [syncFromSession, showTransformNotice],
  );

  const transformEntryMirrorLine = useCallback(
    (first: string, second: string, lastEdited: number): string =>
      sessionRef.current?.transform_entry_mirror_line(first, second, lastEdited) ?? "",
    [],
  );

  const commitMoveEntry = useCallback(
    (first: string, second: string, absolute: boolean, copy: boolean): string => {
      const session = sessionRef.current;
      if (!session) {
        return "unchanged";
      }
      const outcome = session.commit_move_entry(first, second, absolute, copy);
      syncFromSession();
      return outcome;
    },
    [syncFromSession],
  );

  const setMoveChipAbsolute = useCallback(
    (absolute: boolean) => {
      sessionRef.current?.set_move_chip_absolute(absolute);
      syncFromSession();
    },
    [syncFromSession],
  );

  const setMoveReference = useCallback(
    (reference: number) => {
      sessionRef.current?.set_move_reference(reference);
      syncFromSession();
    },
    [syncFromSession],
  );

  const setMoveRadioHover = useCallback(
    (reference: number) => {
      sessionRef.current?.set_move_radio_hover(reference);
      syncFromSession();
    },
    [syncFromSession],
  );

  const commitMoveEntryAt = useCallback(
    (first: string, second: string, copy: boolean, reference: number): string => {
      const session = sessionRef.current;
      if (!session) {
        return "unchanged";
      }
      const outcome = session.commit_move_entry_at(first, second, copy, reference);
      syncFromSession();
      return outcome;
    },
    [syncFromSession],
  );

  const setMarkClickHandler = useCallback((handler: ((mark: number) => void) | null) => {
    markClickRef.current = handler;
  }, []);

  const cancelTransformEntry = useCallback(() => {
    const session = sessionRef.current;
    if (!session || !entryOpenRef.current) {
      return;
    }
    session.cancel_transform_entry();
    syncFromSession();
  }, [syncFromSession]);

  const commitTextPathEntry = useCallback(
    (position: string, distance: string): string => {
      const session = sessionRef.current;
      if (!session) {
        return "unchanged";
      }
      const outcome = session.commit_text_path_entry(position, distance);
      syncFromSession();
      return outcome;
    },
    [syncFromSession],
  );

  /** Runs `change` on the open entry box of a text on path, then re-reads the session. */
  const textPathEntryCommand = useCallback(
    (change: (session: WasmSession) => void) => {
      const session = sessionRef.current;
      if (!session) {
        return;
      }
      change(session);
      syncFromSession();
    },
    [syncFromSession],
  );
  const cancelTextPathEntry = useCallback(
    () => textPathEntryCommand((session) => session.cancel_text_path_entry()),
    [textPathEntryCommand],
  );
  const textPathEntrySide = useCallback(
    (side: SideCode) => textPathEntryCommand((session) => session.text_path_entry_side(side)),
    [textPathEntryCommand],
  );
  const textPathEntryLine = useCallback(
    (line: LineCode) => textPathEntryCommand((session) => session.text_path_entry_line(line)),
    [textPathEntryCommand],
  );
  const textPathEntryMirror = useCallback(
    () => textPathEntryCommand((session) => session.text_path_entry_mirror()),
    [textPathEntryCommand],
  );

  const transformEntryLinked = useCallback(
    (field: number, text: string): string | undefined =>
      sessionRef.current?.transform_entry_linked(field, text) ?? undefined,
    [],
  );

  return {
    sketchView,
    sketchPresentation,
    canvasRef,
    containerRef,
    tool,
    nodeToolbarState,
    polyStarMode,
    polyStarPointCount,
    polyStarRatio,
    liveReadout,
    cursorHint,
    cancelTextPathPick,
    editHint,
    dismissEditHint,
    handleHint,
    cornerHintLines,
    transformEntry,
    moveEntry,
    textPathEntry,
    commitTextPathEntry,
    cancelTextPathEntry,
    textPathEntrySide,
    textPathEntryLine,
    textPathEntryMirror,
    moveBadges,
    commitMoveEntry,
    setMoveChipAbsolute,
    setMoveReference,
    setMoveRadioHover,
    commitMoveEntryAt,
    moveMarkHover,
    setMarkClickHandler,
    commitTransformEntry,
    cancelTransformEntry,
    transformEntryLinked,
    transformEntryMirrorLine,
    onContainerBlur,
    zoomPercent,
    isPanning,
    isSpaceHeld,
    setTool,
    deleteSelected,
    selectionCount,
    selectionAnnouncement,
    selectionAnnouncementSeq,
    keyHint,
    canvasNotice: history.notice,
    grouping,
    offset,
    text,
    clones,
    strokeToPath,
    recognise,
    trace,
    flip,
    flipEnabled,
    guideState,
    toggleGuide,
    loadTemplate,
    arcBar,
    placement,
    penEntry,
    historyBusy: history.busy,
    historyNames: history.names,
    pressHistory: history.press,
    wipeHistory: history.wipe,
    wipeObjectHistory: history.wipeObject,
    runHistoryAction: history.runAction,
    showHistoryNotice: history.showNotice,
    nudgeFeedback,
    convertSelected,
    makeLine,
    makeCurve,
    joinSelected,
    splitSelected,
    insertSelected,
    removeCornerRounding,
    scaleStrokeWidth,
    setScaleStrokeWidth,
    scaleCornerRadius,
    setScaleCornerRadius,
    linkCorners,
    setLinkCorners,
    selectBar,
    syncRevision,
    sync: syncFromSession,
    getSession,
    addViewResizedListener,
    coverCanvasRight,
    refreshCursor,
    setSelectedRadius,
    setSelectedPointCount,
    previewSelectedRatio,
    commitSelectedRatio,
    setSelectedRatio,
    cancelSelectedRatio,
    setPolyStarMode,
    setPolyStarPointCount,
    setPolyStarRatio,
    convertSelectedToPaths,
    applyBoolean,
    applyCombine,
    applySplit,
    applyBreakApart,
    applyCut,
    applyStack,
    applyWeld,
    closePath,
    onPointerDown,
    onPointerMove,
    onPointerUp,
    onPointerLeave,
    onPointerCancel,
    onKeyDown,
    onKeyUp,
    newProject,
    openProject,
    openFile,
    importBegin,
    lastCanvasPointer,
    importAdd,
    importAddTooLarge,
    importFinish,
    packProject,
    markSaved,
  };
}

/** How long an SVG import waits for the faces of its texts before it takes what is loaded. */
const FACE_WAIT_MS = 10_000;

/** Calls `step` until it no longer waits, syncing between calls so the text hook reads and hands
 * in the faces the session asked for; after `FACE_WAIT_MS` the step is forced. */
async function untilFacesLoaded(step: (force: boolean) => boolean, sync: () => void): Promise<void> {
  const started = performance.now();
  while (step(performance.now() - started > FACE_WAIT_MS)) {
    sync();
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
}

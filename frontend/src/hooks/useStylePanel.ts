import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { usePreviewGesture } from "@/hooks/usePreviewGesture";
import { CLONE_STATUS_MS } from "@/lib/cloneText";
import type { WasmSession } from "@/lib/editorSession";

/** A property edited with typed text or a colour area; the names
 * `WasmSession.set_style_text` and the preview calls take. */
export type StyleFieldName =
  | "stroke-width"
  | "stroke-color"
  | "stroke-opacity"
  | "fill-color"
  | "fill-opacity"
  | "marker-count";

/** A property edited by a value field (`curvyo-ui-core::ValueField`). */
export type ValueFieldName =
  | "stroke-width"
  | "stroke-opacity"
  | "fill-opacity"
  | "marker-count";
/** A marker slot and the shape it holds (`curvyo-ui-core::MarkerSlot`). */
export type MarkerSlotName = "start" | "mid" | "end";
export type MarkerShapeName = "none" | "arrow" | "dot";
export type MarkerPlaceName = "spaced" | "nodes";
/** The rounding grid of a value field: Shift is coarse, Ctrl (Cmd) is fine. */
export type GridName = "normal" | "coarse" | "fine";

export type DashName = "solid" | "dash" | "dot" | "dash-dot";
export type JoinName = "miter" | "round" | "bevel";
export type CapName = "butt" | "round" | "square";

/** A plain-JS copy of the Rust `StylePanelView` (`specs/0017-style-panel-
 * rework`): what the Style panel shows. A `*Mixed` flag means the edited
 * objects differ (the field is empty with the placeholder "Mixed"). Colours
 * are packed `0xRRGGBB`; opacities are percents, shown rounded; the hex fields
 * are `#RRGGBBAA`. `*Rows` says whether the rows under a Paint switch exist at
 * all (they are removed while every edited paint is off). */
export interface StyleView {
  subject: string;
  /** The Markers group is shown (stroke on, a path in the selection). */
  markersShown: boolean;
  markerStart: MarkerShapeName | "mixed";
  markerMid: MarkerShapeName | "mixed";
  markerEnd: MarkerShapeName | "mixed";
  markerPlaceShown: boolean;
  markerPlace: MarkerPlaceName | "mixed";
  markerCountShown: boolean;
  markerCountText: string;
  markerCount: number;
  markerCountMixed: boolean;
  markerCountBar: number;
  /** Where a count drag starts from, on the scale continued beyond the track. */
  markerCountDragBase: number;
  /** The largest position of a count drag. */
  countDragLimit: number;
  markerCountResettable: boolean;
  /** Start or End is set and every selected path is closed. */
  markerClosedNote: boolean;
  /** The paint the eyedropper is picking for, or empty. */
  pickTarget: "stroke" | "fill" | "background" | "";
  /** Changes when the edited objects change. */
  scopeKey: string;
  strokePaint: "on" | "off" | "mixed";
  strokeRows: boolean;
  strokeColorMixed: boolean;
  strokeColor: number;
  strokeHex: string;
  strokeHexMixed: boolean;
  strokeOpacityMixed: boolean;
  strokeOpacity: number;
  strokeWidthMixed: boolean;
  strokeWidth: number;
  /** The width as the field shows it. */
  strokeWidthText: string;
  /** The share of the field's width its bar fills, 0 to 1. */
  strokeWidthBar: number;
  /** Where a width drag starts from, on the scale continued beyond the track. */
  strokeWidthDragBase: number;
  /** The largest position of a width drag. */
  widthDragLimit: number;
  /** The value is mixed or not the default: the reset icon shows. */
  strokeWidthResettable: boolean;
  strokeOpacityText: string;
  strokeOpacityBar: number;
  strokeOpacityResettable: boolean;
  /** The pressed preset button, `"none"` for a list that is no preset. */
  strokeDash: DashName | "none" | "mixed";
  /** The numbers of the pattern line, one space apart. */
  strokeDashText: string;
  strokeJoin: JoinName | "mixed";
  strokeCap: CapName | "mixed";
  fillPaint: "on" | "off" | "mixed";
  fillRows: boolean;
  /** The Fill section is in the tree: false while every edited object is a single-line text
   * (`specs/0076-single-line-fonts` criterion 8). */
  fillShown: boolean;
  /** Stroke and Fill are shown: false while only dimensions are selected (`0070` UI review). */
  sectionsShown: boolean;
  fillColorMixed: boolean;
  fillColor: number;
  fillHex: string;
  fillHexMixed: boolean;
  fillOpacityMixed: boolean;
  fillOpacity: number;
  fillOpacityText: string;
  fillOpacityBar: number;
  fillOpacityResettable: boolean;
  /** Limits and reset words of the value fields, from the Rust scales. */
  widthTypedMax: number;
  widthDefaultText: string;
  opacityTypedMax: number;
  opacityDefaultText: string;
  countTypedMax: number;
  countDefaultText: string;
}

const DISABLED_VIEW: StyleView = {
  subject: "",
  pickTarget: "",
  markersShown: false,
  markerStart: "none",
  markerMid: "none",
  markerEnd: "none",
  markerPlaceShown: false,
  markerPlace: "spaced",
  markerCountShown: false,
  markerCountText: "1",
  markerCount: 1,
  markerCountMixed: false,
  markerCountBar: 0,
  markerCountDragBase: 0,
  countDragLimit: 1,
  markerCountResettable: false,
  markerClosedNote: false,
  scopeKey: "",
  strokePaint: "on",
  strokeRows: true,
  strokeColorMixed: false,
  strokeColor: 0,
  strokeHex: "#000000FF",
  strokeHexMixed: false,
  strokeOpacityMixed: false,
  strokeOpacity: 100,
  strokeWidthMixed: false,
  strokeWidth: 0.25,
  strokeWidthText: "0.25",
  strokeWidthBar: 0.18,
  strokeWidthDragBase: 0.18,
  widthDragLimit: 1,
  strokeWidthResettable: false,
  strokeOpacityText: "100",
  strokeOpacityBar: 1,
  strokeOpacityResettable: false,
  strokeDash: "solid",
  strokeDashText: "",
  strokeJoin: "miter",
  strokeCap: "butt",
  fillPaint: "off",
  fillRows: false,
  fillShown: true,
  sectionsShown: true,
  fillColorMixed: false,
  fillColor: 0,
  fillHex: "#000000FF",
  fillHexMixed: false,
  fillOpacityMixed: false,
  fillOpacity: 100,
  fillOpacityText: "100",
  fillOpacityBar: 1,
  fillOpacityResettable: false,
  widthTypedMax: 0,
  widthDefaultText: "",
  opacityTypedMax: 0,
  opacityDefaultText: "",
  countTypedMax: 0,
  countDefaultText: "",
};

/** Reads the wasm-bindgen `StylePanelView` once, immediately, so the instance
 * can be `free()`d rather than held onto. */
function readView(session: WasmSession | null): StyleView {
  if (!session) {
    return DISABLED_VIEW;
  }
  const raw = session.style_panel_view();
  const view: StyleView = {
    subject: raw.subject,
    pickTarget: raw.pick_target as StyleView["pickTarget"],
    markersShown: raw.markers_shown,
    markerStart: raw.marker_start as StyleView["markerStart"],
    markerMid: raw.marker_mid as StyleView["markerMid"],
    markerEnd: raw.marker_end as StyleView["markerEnd"],
    markerPlaceShown: raw.marker_place_shown,
    markerPlace: raw.marker_place as StyleView["markerPlace"],
    markerCountShown: raw.marker_count_shown,
    markerCountText: raw.marker_count_text,
    markerCount: raw.marker_count,
    markerCountMixed: raw.marker_count_mixed,
    markerCountBar: raw.marker_count_bar,
    markerCountDragBase: raw.marker_count_drag_base,
    countDragLimit: raw.count_drag_limit,
    markerCountResettable: raw.marker_count_resettable,
    markerClosedNote: raw.marker_closed_note,
    scopeKey: raw.scope_key,
    strokePaint: raw.stroke_paint as StyleView["strokePaint"],
    strokeRows: raw.stroke_rows,
    strokeColorMixed: raw.stroke_color_mixed,
    strokeColor: raw.stroke_color,
    strokeHex: raw.stroke_hex,
    strokeHexMixed: raw.stroke_hex_mixed,
    strokeOpacityMixed: raw.stroke_opacity_mixed,
    strokeOpacity: raw.stroke_opacity,
    strokeWidthMixed: raw.stroke_width_mixed,
    strokeWidth: raw.stroke_width,
    strokeWidthText: raw.stroke_width_text,
    strokeWidthBar: raw.stroke_width_bar,
    strokeWidthDragBase: raw.stroke_width_drag_base,
    widthDragLimit: raw.width_drag_limit,
    strokeWidthResettable: raw.stroke_width_resettable,
    strokeOpacityText: raw.stroke_opacity_text,
    strokeOpacityBar: raw.stroke_opacity_bar,
    strokeOpacityResettable: raw.stroke_opacity_resettable,
    strokeDash: raw.stroke_dash as StyleView["strokeDash"],
    strokeDashText: raw.stroke_dash_text,
    strokeJoin: raw.stroke_join as StyleView["strokeJoin"],
    strokeCap: raw.stroke_cap as StyleView["strokeCap"],
    fillPaint: raw.fill_paint as StyleView["fillPaint"],
    fillRows: raw.fill_rows,
    fillShown: raw.fill_shown,
    sectionsShown: raw.sections_shown,
    fillColorMixed: raw.fill_color_mixed,
    fillColor: raw.fill_color,
    fillHex: raw.fill_hex,
    fillHexMixed: raw.fill_hex_mixed,
    fillOpacityMixed: raw.fill_opacity_mixed,
    fillOpacity: raw.fill_opacity,
    fillOpacityText: raw.fill_opacity_text,
    fillOpacityBar: raw.fill_opacity_bar,
    fillOpacityResettable: raw.fill_opacity_resettable,
    widthTypedMax: raw.width_typed_max,
    widthDefaultText: raw.width_default_text,
    opacityTypedMax: raw.opacity_typed_max,
    opacityDefaultText: raw.opacity_default_text,
    countTypedMax: raw.count_typed_max,
    countDefaultText: raw.count_default_text,
  };
  raw.free();
  return view;
}

/** What `setText` answers: `"committed"`, `"unchanged"`, or
 * `"invalid:<code>"` with the code `hex`, `percent`, `width` or `dash`. */
export type TextOutcome = string;

/** The hue (degrees, `-1` for none), saturation and value (0 to 1) of a colour. */
export type HsvTriple = readonly [hue: number, saturation: number, value: number];

/** The notice of `specs/0057-clone-overrides` criterion 18 after a style edit of the panel. */
export interface StyleSkipNotice {
  id: number;
  text: string;
}

export interface StylePanelApi {
  view: StyleView;
  /** "2 clones show their original's style and were not changed." for 3 s after an edit of the
   * panel (typed, pressed or dragged) that left clones alone; null otherwise. */
  skipNotice: StyleSkipNotice | null;
  /** A panel drag is running. */
  previewing: boolean;
  /** Escapes that dropped a preview; a picker that kept following the pointer
   * starts over from the committed colour on each. */
  cancels: number;
  /** Enter or Tab in a typed field. */
  setText: (field: StyleFieldName, text: string) => TextOutcome;
  /** A tick of a drag in the colour area or the hue slider: shown on the
   * canvas, nothing written. The gesture ends, with one commit, at the next
   * pointer release or arrow key-up anywhere. */
  previewHsv: (field: StyleFieldName, hue: number, saturation: number, value: number) => void;
  /** Commits what `previewHsv` showed, once: the end of a click on the picker. */
  settle: () => void;
  /** The hue, saturation and value of a packed colour (Rust's rounding). */
  hsvOf: (rgb: number) => HsvTriple;
  /** The packed colour of a hue, saturation and value (Rust's rounding). */
  rgbOf: (hue: number, saturation: number, value: number) => number;
  /** A tick of a value field drag: position `p` (0 to 1) on the field's scale,
   * rounded to `grid`. Same gesture rules: one commit at the release. */
  previewValue: (field: ValueFieldName, p: number, grid: GridName) => void;
  /** An arrow key: `steps` steps from the shown value; the key-up commits. */
  stepValue: (field: ValueFieldName, steps: number, grid: GridName) => void;
  /** The reset icon or Ctrl+Backspace: the default, one commit. */
  resetValue: (field: ValueFieldName) => void;
  setStrokePaint: (on: boolean) => void;
  setFillPaint: (on: boolean) => void;
  setStrokeDash: (name: DashName) => void;
  /** Enter or Tab in the pattern line. */
  setStrokeDashText: (text: string) => TextOutcome;
  /** The eyedropper button: starts picking for a paint, or ends it when that
   * paint is already being picked. */
  beginPick: (target: "stroke" | "fill") => void;
  /** Ends picking and writes nothing. */
  endPick: () => void;
  /** A marker slot choice; one commit to the paths of the selection. */
  setMarkerShape: (slot: MarkerSlotName, shape: MarkerShapeName) => void;
  setMarkerPlace: (place: MarkerPlaceName) => void;
  setStrokeJoin: (name: JoinName) => void;
  setStrokeCap: (name: CapName) => void;
}

interface EditorHandle {
  getSession: () => WasmSession | null;
  syncRevision: number;
}

/**
 * The Style panel's state and commands. Everything with a rule is the
 * session's (`curvyo-ui-core::style_panel`, `style_edit`); this holds the
 * snapshot the panel renders and forwards each control's command. A drag's
 * gesture bookkeeping is `usePreviewGesture`.
 */
export function useStylePanel(editor: EditorHandle): StylePanelApi {
  const { getSession, syncRevision } = editor;
  const [local, setLocal] = useState(0);
  const refresh = useCallback(() => setLocal((n) => n + 1), []);
  // The snapshot is re-read whenever the session says its UI state changed
  // (`syncRevision`) or this panel changed something (`local`).
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const view = useMemo(() => readView(getSession()), [getSession, syncRevision, local]);
  // Every commit of the panel reads the notice of criterion 18 right after it, before a sync
  // could hand it to the canvas slot (UX review of 0057, must-fix 4).
  const [skipNotice, setSkipNotice] = useState<StyleSkipNotice | null>(null);
  const skipCount = useRef(0);
  const drainSkip = useCallback((session: WasmSession) => {
    const text = session.take_style_skip_notice();
    if (text !== "") {
      skipCount.current += 1;
      setSkipNotice({ id: skipCount.current, text });
    }
  }, []);
  useEffect(() => {
    if (!skipNotice) {
      return undefined;
    }
    const timer = window.setTimeout(() => setSkipNotice(null), CLONE_STATUS_MS);
    return () => window.clearTimeout(timer);
  }, [skipNotice]);
  const ends = useMemo(
    () => ({
      commit: (session: WasmSession) => {
        session.commit_style_preview();
        drainSkip(session);
      },
      cancel: (session: WasmSession) => session.cancel_style_preview(),
    }),
    [drainSkip],
  );
  const { queue: preview, settle, previewing, cancels } = usePreviewGesture(getSession, refresh, ends);

  const act = useCallback(
    <T,>(run: (session: WasmSession) => T, none: T): T => {
      const session = getSession();
      if (!session) {
        return none;
      }
      const result = run(session);
      drainSkip(session);
      refresh();
      return result;
    },
    [getSession, refresh, drainSkip],
  );
  const setText = useCallback(
    (field: StyleFieldName, text: string): TextOutcome =>
      act((session) => session.set_style_text(field, text), "unchanged"),
    [act],
  );
  const run = useCallback((call: (session: WasmSession) => void) => act(call, undefined), [act]);

  const hsvOf = useCallback(
    (rgb: number): HsvTriple => {
      const [hue, saturation, value] = getSession()?.colour_hsv(rgb) ?? [-1, 0, 0];
      return [hue, saturation, value];
    },
    [getSession],
  );
  const rgbOf = useCallback(
    (hue: number, saturation: number, value: number) =>
      getSession()?.hsv_colour(hue, saturation, value) ?? 0,
    [getSession],
  );

  return {
    view,
    skipNotice,
    previewing,
    cancels,
    setText,
    previewHsv: (field, hue, saturation, value) =>
      preview((s) => s.preview_style_hsv(field, hue, saturation, value)),
    settle: settle,
    hsvOf,
    rgbOf,
    previewValue: (field, p, grid) => preview((s) => s.preview_value_field(field, p, grid)),
    stepValue: (field, steps, grid) => preview((s) => s.step_value_field(field, steps, grid)),
    resetValue: (field) => run((s) => void s.reset_value_field(field)),
    setStrokePaint: (on) => run((s) => s.set_stroke_paint(on)),
    setFillPaint: (on) => run((s) => s.set_fill_paint(on)),
    setStrokeDash: (name) => run((s) => s.set_stroke_dash(name)),
    setStrokeDashText: (text) => act((s) => s.set_stroke_dash_text(text), "unchanged"),
    beginPick: (target) => run((s) => s.begin_colour_pick(target)),
    endPick: () => run((s) => s.end_colour_pick()),
    setMarkerShape: (slot, shape) => run((s) => s.set_marker_shape(slot, shape)),
    setMarkerPlace: (place) => run((s) => s.set_marker_place(place)),
    setStrokeJoin: (name) => run((s) => s.set_stroke_join(name)),
    setStrokeCap: (name) => run((s) => s.set_stroke_cap(name)),
  };
}

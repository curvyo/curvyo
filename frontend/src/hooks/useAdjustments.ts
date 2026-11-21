import { useCallback, useMemo, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { type AdjustmentsView, decodeAdjustmentsText, type PresetKey, type StepKindKey } from "@/lib/adjustmentsView";

/** How long the notice of Make editable stays (`ux-notes.md` §5; a preset pick says nothing, §4). */
const NOTICE_MS = 3000;

interface EditorHandle {
  getSession: () => WasmSession | null;
  syncRevision: number;
  sync: () => void;
  showHistoryNotice: (text: string, ms?: number, source?: "canvas" | "panel", kind?: "status" | "alert") => void;
}

/** The Adjustments group of the selected picture and its commands (`specs/0135-image-optimisation`
 * criteria 1, 2, 9, 25 to 30, 59). Every rule and every sentence is the session's. */
export interface AdjustmentsApi {
  /** `null` while not exactly one image or image clone is selected. */
  view: AdjustmentsView | null;
  pickPreset: (key: PresetKey) => boolean;
  /** A press in the picture's Dither strip: one step, Preset and Strength kept. */
  setDither: (mode: string) => boolean;
  /** A field commit of the picture's dither; the refusal text, empty when taken. */
  editDitherField: (field: string, value: number) => string;
  /** A typed or clicked Strength; the refusal text, empty when taken. */
  setStrength: (value: number) => string;
  /** The ticks of a drag or a key run: shown, not written. */
  holdStrength: (value: number) => void;
  /** The release of a drag or a key run: one step. */
  releaseStrength: () => boolean;
  cancelStrength: () => void;
  addStep: (kind: StepKindKey) => boolean;
  /** A field commit of row `index`; the refusal text, empty when taken. */
  editField: (index: number, key: string, value: number) => string;
  setEnabled: (index: number, on: boolean) => boolean;
  move: (index: number, up: boolean) => boolean;
  /** The Stretch button of Levels row `index`: one step. */
  stretch: (index: number) => boolean;
  /** A drag of row `from` landing at index `to`: one step. */
  moveTo: (from: number, to: number) => boolean;
  remove: (index: number) => boolean;
  /** A click on Curves row `index`'s curve at Input and Output %: a point there, one step. */
  addCurvePoint: (index: number, input: number, output: number) => boolean;
  /** A drag or arrow key of a Curves point to Input and Output %: one step. */
  moveCurvePoint: (index: number, point: number, input: number, output: number) => boolean;
  /** Delete on a Curves point (not an end point). */
  removeCurvePoint: (index: number, point: number) => boolean;
  /** The Straight curve button. */
  straightenCurve: (index: number) => boolean;
  /** Channel preset cell `preset` (0 Luminance to 3 Blue) of Channel mix row `index`. */
  setChannelPreset: (index: number, preset: number) => boolean;
  /** Crop on the canvas of Crop row `index`: the Crop bar opens. */
  openCrop: (index: number) => boolean;
  /** Copy adjustments: held for the session, no step. */
  copyAdjustments: () => boolean;
  /** Paste adjustments: one step "Paste adjustments". */
  pasteAdjustments: () => boolean;
  makeEditable: () => boolean;
  restoreOriginal: () => boolean;
  /** Own on a clone's override row: a full copy of the original's adjustments, one step. */
  ownAdjustments: () => boolean;
  followOriginal: () => boolean;
}

/**
 * The Style tab's Adjustments group: its view (`adjustments_view_json`, re-read after every
 * sync) and its commands. It is the only caller of these bindings and does not grow
 * `useEditorSession`; the one notice of an edit goes to the canvas notice slot.
 */
export function useAdjustments(editor: EditorHandle): AdjustmentsApi {
  const { getSession, syncRevision, sync, showHistoryNotice } = editor;
  const [local, setLocal] = useState(0);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const view = useMemo(() => decodeAdjustmentsText(getSession()?.adjustments_view_json() ?? ""), [getSession, syncRevision, local]);

  const act = useCallback(
    <T,>(none: T, run: (session: WasmSession) => T): T => {
      const session = getSession();
      if (!session) return none;
      const result = run(session);
      const notice = session.take_adjust_notice();
      if (notice !== "") showHistoryNotice(notice, NOTICE_MS, "canvas", "status");
      setLocal((n) => n + 1);
      sync();
      return result;
    },
    [getSession, showHistoryNotice, sync],
  );

  return useMemo(
    () => ({
      view,
      pickPreset: (key) => act(false, (s) => s.pick_adjust_preset(key)),
      setDither: (mode) => act(false, (s) => s.set_picture_dither(mode)),
      editDitherField: (field, value) => act("", (s) => s.edit_picture_dither_field(field, value)),
      setStrength: (value) => act("", (s) => s.set_adjust_strength(value)),
      holdStrength: (value) => act(undefined, (s) => s.hold_adjust_strength(value)),
      releaseStrength: () => act(false, (s) => s.release_adjust_strength()),
      cancelStrength: () => act(undefined, (s) => s.cancel_adjust_strength()),
      addStep: (kind) => act(false, (s) => s.add_adjust_step(kind)),
      editField: (index, key, value) => act("", (s) => s.edit_adjust_field(index, key, value)),
      setEnabled: (index, on) => act(false, (s) => s.set_adjust_step_enabled(index, on)),
      move: (index, up) => act(false, (s) => s.move_adjust_step(index, up)),
      moveTo: (from, to) => act(false, (s) => s.move_adjust_step_to(from, to)),
      stretch: (index) => act(false, (s) => s.stretch_adjust_levels(index)),
      remove: (index) => act(false, (s) => s.remove_adjust_step(index)),
      addCurvePoint: (index, input, output) => act(false, (s) => s.add_adjust_curve_point(index, input, output)),
      moveCurvePoint: (index, point, input, output) =>
        act(false, (s) => s.move_adjust_curve_point(index, point, input, output)),
      removeCurvePoint: (index, point) => act(false, (s) => s.remove_adjust_curve_point(index, point)),
      straightenCurve: (index) => act(false, (s) => s.straighten_adjust_curve(index)),
      setChannelPreset: (index, preset) => act(false, (s) => s.set_adjust_channel_preset(index, preset)),
      openCrop: (index) => act(false, (s) => s.open_crop_bar(index)),
      copyAdjustments: () => act(false, (s) => s.copy_adjustments()),
      pasteAdjustments: () => act(false, (s) => s.paste_adjustments()),
      makeEditable: () => act(false, (s) => s.make_adjust_editable()),
      restoreOriginal: () => act(false, (s) => s.restore_original_adjustments()),
      ownAdjustments: () => act(false, (s) => s.own_adjustments()),
      followOriginal: () => act(false, (s) => s.follow_original_adjustments()),
    }),
    [view, act],
  );
}

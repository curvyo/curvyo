import { useCallback, useEffect, useMemo, useState } from "react";

import { usePreviewGesture, type GestureEnds, type PreviewGesture } from "@/hooks/usePreviewGesture";
import type { HsvTriple, TextOutcome } from "@/hooks/useStylePanel";
import type { WasmSession } from "@/lib/editorSession";
import {
  decodeGradientHandles,
  decodeGradientView,
  decodeText,
  type GradientHandles,
  type GradientView,
} from "@/lib/gradientView";

const NO_VIEW: GradientView = {
  offered: false,
  shown: false,
  kind: "",
  angle: "",
  angleShown: false,
  radius: "",
  radiusShown: false,
  stops: [],
  mixedGradients: false,
  activeStop: 0,
  skip: "",
};

/** The entry box's picker: a drag only previews; the box's Enter writes it (criterion 7). */
const ENTRY_ENDS: GestureEnds = {
  commit: () => undefined,
  cancel: (session) => session.cancel_style_preview(),
};

interface EditorHandle {
  getSession: () => WasmSession | null;
  syncRevision: number;
  sync: () => void;
}

/** What the Gradient block, the canvas handles and the entry box need (`specs/0124-gradient-fill`). */
export interface GradientApi {
  view: GradientView;
  /** The handles to draw, `null` while none are shown. */
  handles: GradientHandles | null;
  setKind: (kind: "linear" | "radial") => void;
  setAngleText: (text: string) => TextOutcome;
  setRadiusText: (text: string) => TextOutcome;
  setStopHex: (stop: number, text: string) => TextOutcome;
  setStopPosition: (stop: number, text: string) => TextOutcome;
  addStop: () => void;
  removeStop: (stop: number) => void;
  reverse: () => void;
  /** Focus or a press entered the row of `stop`. */
  activate: (stop: number) => void;
  /** The panel picker's drag on `stop`: previews per frame, commits "Edit gradient" at the release. */
  panelPicker: { preview: (stop: number, h: number, s: number, v: number) => void; settle: PreviewGesture["settle"] };
  /** The entry box's picker: previews only. */
  entryPicker: { preview: (stop: number, h: number, s: number, v: number) => void; settle: PreviewGesture["settle"] };
  commitEntry: (fieldText: string, hex: string) => TextOutcome;
  closeEntry: () => void;
  hsvOf: (rgb: number) => HsvTriple;
  rgbOf: (hue: number, saturation: number, value: number) => number;
}

/**
 * The gradient fill's host state: the Gradient block (`gradient_view_json`), the canvas handles
 * with their readout and entry box (`gradient_handles_json`, polled each frame while shown) and
 * the commands of both. Every rule is the session's; the handles' presses, drags and keys are the
 * canvas's own pointer calls.
 */
export function useGradient(editor: EditorHandle): GradientApi {
  const { getSession, syncRevision, sync } = editor;
  const [local, setLocal] = useState(0);
  const refresh = useCallback(() => setLocal((n) => n + 1), []);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const view = useMemo(() => decodeText(getSession()?.gradient_view_json() ?? "", decodeGradientView) ?? NO_VIEW, [getSession, syncRevision, local]);

  const [handles, setHandles] = useState<GradientHandles | null>(null);
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const text = getSession()?.gradient_handles_json() ?? "";
      if (text !== previous) {
        previous = text;
        const next = decodeText(text, decodeGradientHandles);
        setHandles(next && next.shown ? next : null);
        // A press, drag or Escape on the canvas changed the gradient: the block follows.
        setLocal((n) => n + 1);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);

  const done = useCallback(() => {
    refresh();
    sync();
  }, [refresh, sync]);
  const act = useCallback(
    <T,>(run: (session: WasmSession) => T, none: T): T => {
      const session = getSession();
      if (!session) {
        return none;
      }
      const result = run(session);
      done();
      return result;
    },
    [getSession, done],
  );
  const panel = usePreviewGesture(getSession, refresh);
  const entry = usePreviewGesture(getSession, refresh, ENTRY_ENDS);

  const hsvOf = useCallback(
    (rgb: number): HsvTriple => {
      const [hue, saturation, value] = getSession()?.colour_hsv(rgb) ?? [-1, 0, 0];
      return [hue, saturation, value];
    },
    [getSession],
  );
  const rgbOf = useCallback(
    (hue: number, saturation: number, value: number) => getSession()?.hsv_colour(hue, saturation, value) ?? 0,
    [getSession],
  );

  return {
    view,
    handles,
    setKind: (kind) => act((s) => s.set_gradient_kind(kind), false),
    setAngleText: (text) => act((s) => s.set_gradient_angle_text(text), "unchanged"),
    setRadiusText: (text) => act((s) => s.set_gradient_radius_text(text), "unchanged"),
    setStopHex: (stop, text) => act((s) => s.set_gradient_stop_hex(stop, text), "unchanged"),
    setStopPosition: (stop, text) => act((s) => s.set_gradient_stop_position_text(stop, text), "unchanged"),
    addStop: () => void act((s) => s.add_gradient_stop(), false),
    removeStop: (stop) => void act((s) => s.remove_gradient_stop(stop), false),
    reverse: () => void act((s) => s.reverse_gradient(), false),
    activate: (stop) => act((s) => s.set_gradient_active_stop(stop), undefined),
    panelPicker: {
      preview: (stop, h, s, v) => panel.queue((session) => session.preview_gradient_stop_hsv(stop, h, s, v)),
      settle: panel.settle,
    },
    entryPicker: {
      preview: (stop, h, s, v) => entry.queue((session) => session.preview_gradient_stop_hsv(stop, h, s, v)),
      settle: entry.settle,
    },
    commitEntry: (fieldText, hex) => act((s) => s.commit_gradient_entry(fieldText, hex), "unchanged"),
    closeEntry: () => act((s) => {
      s.cancel_style_preview();
      s.close_gradient_entry();
    }, undefined),
    hsvOf,
    rgbOf,
  };
}

/** The notice of an SVG Open or Import joined with the gradient features it lost (criterion 27). */
export function withGradientNotice(session: WasmSession | null, notice: string): string {
  const lost = session?.take_gradient_import_notice() ?? "";
  return [notice, lost].filter((text) => text !== "").join("\n");
}

import { useCallback, useEffect, useState } from "react";

import { type CropBarView, decodeCropBarText } from "@/lib/cropBar";
import type { WasmSession } from "@/lib/editorSession";

interface EditorHandle {
  getSession: () => WasmSession | null;
  sync: () => void;
}

/** The Crop bar of the selected picture and its commands (`specs/0135-image-optimisation`
 * criterion 5, `ux-notes.md` §3.9). Every rule is the session's. */
export interface CropBarApi {
  /** `null` while the bar is closed (it follows the selection and the tool by itself). */
  view: CropBarView | null;
  /** The release of a drag of the frame or a handle: one step. */
  setRect: (pct: [number, number, number, number]) => boolean;
  /** A bar field commit of the bar's row; the refusal text, empty when taken. */
  editField: (key: string, value: number) => string;
  /** The working image of the picture, RGBA8 (`before_after_pixels("before")`). */
  pixels: () => Uint8Array | null;
  /** Done, Enter, Escape. */
  close: () => void;
}

/** The bar's view, polled each frame (the canvas moves under it), and its commands. */
export function useCropBar(editor: EditorHandle): CropBarApi {
  const { getSession, sync } = editor;
  const [view, setView] = useState<CropBarView | null>(null);
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const text = getSession()?.crop_bar_json() ?? "null";
      if (text !== previous) {
        previous = text;
        setView(decodeCropBarText(text));
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);

  const setRect = useCallback(
    ([l, t, w, h]: [number, number, number, number]) => {
      const done = getSession()?.set_crop_rect(l, t, w, h) ?? false;
      if (done) sync();
      return done;
    },
    [getSession, sync],
  );
  const editField = useCallback(
    (key: string, value: number) => {
      const session = getSession();
      if (!session || !view) return "";
      const refusal = session.edit_adjust_field(view.row, key, value);
      sync();
      return refusal;
    },
    [getSession, sync, view],
  );
  const pixels = useCallback(() => {
    const bytes = getSession()?.before_after_pixels("before");
    return bytes && bytes.length > 0 ? bytes : null;
  }, [getSession]);
  const close = useCallback(() => {
    getSession()?.close_crop_bar();
    sync();
  }, [getSession, sync]);
  return { view, setRect, editField, pixels, close };
}

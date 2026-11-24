import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { type FrameShape, type PageFrame } from "@/lib/frameControls";

/** What the Frame row needs of the active laser page (`specs/0118` 2b): its Frame button's view,
 * the frame text, the pick and the head ring. */
export interface FramePage {
  view: PageFrame;
  /** Head position runs relative: the ring shows the offset from where the frame began. */
  relative: boolean;
  shape: FrameShape;
  setShape: (shape: FrameShape) => void;
  /** The frame text of the page now, in the shape picked; `undefined` while it cannot be framed. */
  bytes: () => Uint8Array | undefined;
  /** The head ring at the controller's position (work coordinates, or the offset when relative). */
  head: (x: number, y: number) => void;
  clearHead: () => void;
  /** Set origin here (`specs/0118` 29): stores the work position `(x, y)` mm as the page's user
   * origin, one step; whether it changed. */
  setOrigin: (x: number, y: number) => boolean;
}

/** The shape pick of each laser page this run: session state, never saved (W30). */
const picks = new Map<string, FrameShape>();

/** The Frame row's page API: the pick is per page and lost on reload, the rest asks the session. */
export function useFramePage(
  getSession: () => WasmSession | null,
  sync: () => void,
  pageId: string,
  view: PageFrame,
  relative: boolean,
): FramePage {
  const [, rerender] = useState(0);
  const shape = picks.get(pageId) ?? "rectangle";
  const latest = useRef({ shape, relative });
  useEffect(() => {
    latest.current = { shape, relative };
  }, [shape, relative]);
  const setShape = useCallback(
    (next: FrameShape) => {
      picks.set(pageId, next);
      rerender((n) => n + 1);
    },
    [pageId],
  );
  const bytes = useCallback(() => getSession()?.frame_bytes(latest.current.shape) ?? undefined, [getSession]);
  const head = useCallback(
    (x: number, y: number) => {
      if (getSession()?.set_frame_head(x, y, latest.current.relative)) sync();
    },
    [getSession, sync],
  );
  const clearHead = useCallback(() => {
    if (getSession()?.clear_frame_head()) sync();
  }, [getSession, sync]);
  const setOrigin = useCallback(
    (x: number, y: number) => {
      const changed = getSession()?.set_user_origin(x, y) ?? false;
      if (changed) sync();
      return changed;
    },
    [getSession, sync],
  );
  return useMemo(
    () => ({ view, relative, shape, setShape, bytes, head, clearHead, setOrigin }),
    [view, relative, shape, setShape, bytes, head, clearHead, setOrigin],
  );
}

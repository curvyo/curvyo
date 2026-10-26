import { useCallback, useEffect, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { type StrokeToPathNotice, strokeToPathNotice } from "@/lib/strokeToPathText";

/** How long a notice stays (row "Action notice": 3 s, 5 s past 70 characters, 8 s past 140 or
 * for a refusal). */
function noticeMs(notice: StrokeToPathNotice): number {
  if (notice.alert || notice.text.length > 140) {
    return 8000;
  }
  return notice.text.length > 70 ? 5000 : 3000;
}

/** The notice of the last press, keyed so a repeated text is announced again. */
export interface StrokeToPathBarNotice extends StrokeToPathNotice {
  id: number;
}

export interface StrokeToPathApi {
  /** Runs Stroke to path on the selection (the button and Ctrl+Alt+C). */
  apply: () => void;
  /** The notice under the button; `null` once it ended. */
  notice: StrokeToPathBarNotice | null;
}

/**
 * Stroke to path in the frontend (`specs/0085-stroke-to-path`): forwards the press to the
 * session and shows the notice it reports. Every decision is the session's.
 */
export function useStrokeToPath(
  getSession: () => WasmSession | null,
  onChange: () => void,
): StrokeToPathApi {
  const [notice, setNotice] = useState<StrokeToPathBarNotice | null>(null);
  const counter = useRef(0);
  useEffect(() => {
    if (!notice) {
      return undefined;
    }
    const timer = window.setTimeout(() => setNotice(null), noticeMs(notice));
    return () => window.clearTimeout(timer);
  }, [notice]);
  // "Until the next action": a press anywhere or a key ends the notice.
  useEffect(() => {
    if (!notice) {
      return undefined;
    }
    const end = () => setNotice(null);
    const timer = window.setTimeout(() => {
      window.addEventListener("pointerdown", end, true);
      window.addEventListener("keydown", end, true);
    }, 0);
    return () => {
      window.clearTimeout(timer);
      window.removeEventListener("pointerdown", end, true);
      window.removeEventListener("keydown", end, true);
    };
  }, [notice]);

  const apply = useCallback(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    const raw = session.apply_stroke_to_path();
    const next = strokeToPathNotice({
      kind: raw.kind,
      count: raw.count,
      of: raw.of,
      strokes: raw.strokes,
      unstroked: raw.unstroked,
      texts: raw.texts,
      unlinked: raw.unlinked,
      face: raw.face || undefined,
    });
    raw.free();
    if (next) {
      counter.current += 1;
      setNotice({ ...next, id: counter.current });
    }
    onChange();
  }, [getSession, onChange]);

  return { apply, notice };
}

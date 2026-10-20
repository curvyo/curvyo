import { useCallback, useRef, useState } from "react";

import { type ActionNotice, useActionNotice } from "@/hooks/useActionNotice";
import type { EditorSession } from "@/hooks/useEditorSession";
import { afterFrames, useBusyWindow } from "@/hooks/useRailCommands";
import { weldGapInRange } from "@/lib/editorSession";
import { noticeMs } from "@/lib/pathText";
import {
  type WeldResult,
  gapClampNotice,
  readStoredGap,
  storeGap,
  weldRefusalOutlines,
  weldRefusalText,
  weldSuccessText,
} from "@/lib/weldText";

/** What the Weld group of the Select bar shows and does. */
export interface WeldApi {
  /** The Gap in mm, as stored. */
  gapMm: number;
  /** Sets a typed or stepped Gap; returns the clamp notice under the field, if any. */
  setGap: (mm: number) => string | null;
  /** A press on the Weld button. */
  weld: () => void;
  /** The call is running: the button keeps its pressed ground, the group is `aria-busy`. */
  busy: boolean;
  /** The notice of the last Weld, under the button. */
  notice: ActionNotice | null;
}

function storage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/**
 * Weld in the frontend (`specs/0068-laserable-lettering/` criteria 1 to 3, UX notes §2 to §5):
 * the per-user Gap, the busy state around the call and the notice under the button. What Weld
 * does and why it refuses is the session's.
 */
export function useWeld(editor: EditorSession): WeldApi {
  const { getSession, sync, tool, selectionCount } = editor;
  const [gapMm, setGapMm] = useState(() => readStoredGap(storage()));
  const gapRef = useRef(gapMm);
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  const outlinedRef = useRef(false);
  useBusyWindow(busy);

  const clearOutline = useCallback(() => {
    if (outlinedRef.current) {
      outlinedRef.current = false;
      getSession()?.clear_boolean_refusal();
    }
  }, [getSession]);
  const { notice, show } = useActionNotice(tool, selectionCount, clearOutline);

  const setGap = useCallback((mm: number): string | null => {
    // Stored as the field shows it, to 0.01 mm ("0.004 in" is 0.1, not 0.1016).
    const { mm: clamped, clamp } = weldGapInRange(Math.round(mm * 100) / 100);
    gapRef.current = clamped;
    setGapMm(clamped);
    storeGap(storage(), clamped);
    return gapClampNotice(clamp);
  }, []);

  const weld = useCallback(() => {
    const session = getSession();
    if (!session || busyRef.current) {
      return;
    }
    busyRef.current = true;
    setBusy(true);
    void (async () => {
      try {
        // Let the busy state reach the screen before the call takes the thread.
        await afterFrames(2);
        const raw = session.apply_weld(gapRef.current);
        const result: WeldResult = {
          kind: raw.kind,
          count: raw.count,
          of: raw.of,
          textsOnly: raw.texts_only,
          compound: raw.compound,
          pieces: raw.pieces,
          dashed: raw.dashed,
          closedOpen: raw.closed_open,
          removedOpen: raw.removed_open,
          mixedStyles: raw.mixed_styles,
          face: raw.face || undefined,
        };
        raw.free();
        sync();
        outlinedRef.current = weldRefusalOutlines(result);
        const applied = result.kind === "applied";
        const text = applied ? weldSuccessText(result) : weldRefusalText(result);
        if (text !== null) {
          show(applied ? "success" : "refusal", text, applied ? noticeMs(text) : undefined);
        }
      } finally {
        busyRef.current = false;
        setBusy(false);
      }
    })();
  }, [getSession, show, sync]);

  return { gapMm, setGap, weld, busy, notice };
}

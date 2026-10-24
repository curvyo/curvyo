// The text layout block's state and commands (`specs/0079-text-layout-controls` criteria 6, 7,
// 12 to 14): the four fields as the session shows them, their drag preview, arrow steps, reset
// and typed values. Every rule is the session's; this forwards each control's command.
import { useCallback, useEffect, useMemo, useState } from "react";

import type { GridName } from "@/hooks/useStylePanel";
import { type GestureEnds, usePreviewGesture } from "@/hooks/usePreviewGesture";
import type { WasmSession } from "@/lib/editorSession";
import {
  type LayoutFieldCode,
  type LayoutFieldState,
  entryAnswer,
  layoutNotice,
  parseLayoutBlock,
  parseLayoutOutcome,
} from "@/lib/textLayoutText";

/** How long a notice stays under its field (Action notice, status). */
const NOTICE_MS = 3000;

export interface TextLayoutApi {
  /** The four fields, or `null` when the block is not shown. */
  fields: LayoutFieldState[] | null;
  /** The notice under a field after a typed value was held to a limit. */
  notice: { field: LayoutFieldCode; text: string } | null;
  preview: (field: LayoutFieldCode, p: number, grid: GridName) => void;
  step: (field: LayoutFieldCode, steps: number, grid: GridName) => void;
  settle: () => void;
  reset: (field: LayoutFieldCode) => void;
  /** A typed value: the shared entry field's answer. */
  submit: (field: LayoutFieldCode, text: string) => string;
}

export function useTextLayout(
  getSession: () => WasmSession | null,
  syncFromSession: () => void,
  syncRevision: number,
): TextLayoutApi {
  const [local, setLocal] = useState(0);
  const [notice, setNotice] = useState<TextLayoutApi["notice"]>(null);
  const refresh = useCallback(() => setLocal((n) => n + 1), []);
  const fields = useMemo(
    () => parseLayoutBlock(getSession()?.text_layout_block_json() ?? "null"),
    // `syncRevision` and `local` say the session changed.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [getSession, syncRevision, local],
  );
  // The release writes one step; the bars and the history read it at once.
  const ends: GestureEnds = useMemo(
    () => ({
      commit: (session) => {
        session.commit_text_layout_preview();
        syncFromSession();
      },
      cancel: (session) => session.cancel_text_layout_preview(),
    }),
    [syncFromSession],
  );
  const { queue, settle } = usePreviewGesture(getSession, refresh, ends);

  useEffect(() => {
    if (!notice) {
      return undefined;
    }
    const timer = window.setTimeout(() => setNotice(null), NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notice]);

  const act = useCallback(
    (field: LayoutFieldCode, run: (session: WasmSession) => string): string => {
      const session = getSession();
      if (!session) {
        return "unchanged";
      }
      const outcome = parseLayoutOutcome(run(session));
      const text = layoutNotice(field, outcome);
      setNotice(text ? { field, text } : null);
      syncFromSession();
      refresh();
      return entryAnswer(outcome);
    },
    [getSession, syncFromSession, refresh],
  );

  return {
    fields,
    notice,
    preview: (field, p, grid) => queue((s) => s.preview_text_layout(field, p, grid)),
    step: (field, steps, grid) => queue((s) => s.step_text_layout(field, steps, grid)),
    settle,
    reset: (field) => {
      act(field, (s) => s.reset_text_layout(field));
    },
    submit: (field, text) => act(field, (s) => s.set_text_layout_text(field, text)),
  };
}

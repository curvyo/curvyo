import { useEffect, useRef } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import {
  flushMachineWrites,
  reloadMachineRecords,
  setMachineRecordNoticeHandler,
  watchMachineRecords,
} from "@/lib/machineProfilesFile";

/**
 * The host's part of the machine profile records (`specs/0113-fixture-sets-and-pins` criteria 44,
 * 45): every writes the session queued (a set edit, an undo, a drag at its release) is performed
 * after each re-read, a failed one shows its notice, and the records are read again when the
 * window gets the focus or another browser window writes one.
 */
export function useMachineRecords(editor: Pick<EditorSession, "getSession" | "sync" | "syncRevision" | "showHistoryNotice">): void {
  const { getSession, sync, syncRevision, showHistoryNotice } = editor;
  useEffect(() => {
    const session = getSession();
    if (session) void flushMachineWrites(session, sync);
  }, [getSession, sync, syncRevision]);
  const notice = useRef(showHistoryNotice);
  notice.current = showHistoryNotice;
  useEffect(() => {
    setMachineRecordNoticeHandler((text) => notice.current(text, 8000, "canvas", "alert"));
    return () => setMachineRecordNoticeHandler(null);
  }, []);
  useEffect(
    () =>
      watchMachineRecords(() => {
        const session = getSession();
        if (!session) return;
        void reloadMachineRecords(session).then((changed) => {
          if (changed) sync();
        });
      }),
    [getSession, sync],
  );
}

import { useCallback, useRef } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { flushMaterialWrites } from "@/lib/materialsFile";
import { EXPORT_FAILED_TITLE, parseExportNotice } from "@/lib/outputText";
import { downloadJobFile, invoke, isTauri } from "@/platform/host";

/** What Export job file reads from the editor: the session, its re-read signal and the notice slot. */
export interface JobExportEditor {
  getSession: () => WasmSession | null;
  sync: () => void;
  showHistoryNotice: (text: string, ms?: number) => void;
}

/**
 * Export job file (`specs/0029-laser-job-preview-and-output` criteria 27, 28; UX notes 7): the
 * session builds the file, the host saves it (the desktop's dialog, a browser download), and only
 * a written file is reported back as one step. Cancel writes nothing and says nothing; a failed
 * write is the error dialog with the system's reason. Returns the press handler.
 */
export function useJobExport(
  editor: JobExportEditor,
  projectFile: () => string | null,
  onError: (message: string, title: string) => void,
): () => Promise<void> {
  const { getSession, sync, showHistoryNotice } = editor;
  const running = useRef(false);
  return useCallback(async () => {
    const session = getSession();
    if (!session || running.current) return;
    const job = session.export_job(projectFile() ?? "");
    if (!job) return;
    running.current = true;
    try {
      const { bytes, name, file_type: fileType, extension, token } = job;
      let written = name;
      if (isTauri) {
        const picked = await invoke<string | null>("pick_job_export_path", {
          suggestedName: name,
          fileType,
          extension,
        });
        if (!picked) return;
        try {
          written = await invoke<string>("write_job_export", {}, bytes);
        } catch (error) {
          onError(String(error), EXPORT_FAILED_TITLE);
          return;
        }
      } else {
        downloadJobFile(name, bytes);
      }
      const notice = parseExportNotice(session.export_job_done(token, written, !isTauri, Date.now()));
      void flushMaterialWrites(session, sync);
      sync();
      if (notice) showHistoryNotice(notice.text, notice.ms);
    } finally {
      running.current = false;
    }
  }, [getSession, sync, showHistoryNotice, projectFile, onError]);
}

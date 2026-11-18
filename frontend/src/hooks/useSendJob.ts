import { useCallback, useRef } from "react";

import type { JobExportEditor } from "@/hooks/useJobExport";
import { linkStore } from "@/hooks/useLinkEntry";
import { flushMaterialWrites } from "@/lib/materialsFile";
import { parseExportNotice } from "@/lib/outputText";

/** What the Output group uses of Send to machine. */
export interface SendApi {
  /** Send to machine: builds the job from the page as it is now and sends it. */
  press: () => Promise<void>;
  /** Tells the session how this app stands to the page's connector (`set_send_link`). */
  applyLink: (profileId: string, spelling: string) => void;
}

/**
 * Send to machine (`specs/0125-send-to-machine`): the session builds the job and decides whether
 * Send is enabled; the host sends it and follows it by its id; only when the connector has
 * accepted it is the page committed as lasered, once. The Output group reads the connector from
 * the page's link store while it is shown and gives its link to the session through `applyLink`.
 */
export function useSendJob(editor: JobExportEditor): SendApi {
  const { getSession, sync, showHistoryNotice } = editor;
  const told = useRef("");
  const applyLink = useCallback(
    (profileId: string, spelling: string) => {
      const wanted = `${profileId}|${spelling}`;
      const session = getSession();
      if (!session || told.current === wanted) return;
      told.current = wanted;
      session.set_send_link(spelling);
      sync();
    },
    [getSession, sync],
  );

  const press = useCallback(async () => {
    const session = getSession();
    if (!session) return;
    const job = session.send_job();
    if (!job) return;
    const { bytes, name, profile_id: profile, address: to, token } = job;
    sync();
    await linkStore.sendPage(profile, to, { token, name, bytes }, (accepted) => {
      const now = getSession();
      if (!now) return;
      if (accepted) {
        const notice = parseExportNotice(now.send_job_done(token, Date.now()));
        void flushMaterialWrites(now, sync);
        if (notice) showHistoryNotice(notice.text, notice.ms);
      } else {
        now.send_job_failed(token);
      }
      sync();
    }, () => {
      getSession()?.send_job_timed_out(token);
      sync();
    });
  }, [getSession, sync, showHistoryNotice]);
  return { press, applyLink };
}

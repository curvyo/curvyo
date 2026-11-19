import { useCallback, useRef } from "react";

import type { JobExportEditor } from "@/hooks/useJobExport";
import { linkStore } from "@/hooks/useLinkEntry";
import { type LinkEntry } from "@/lib/connectorLink";
import { controllerWord } from "@/lib/connectorText";
import { flushMaterialWrites } from "@/lib/materialsFile";
import { parseExportNotice } from "@/lib/outputText";

/** What the Output group uses of Send to machine. */
export interface SendApi {
  /** Send to machine: builds the job from the page as it is now and sends it; with `start` the
   * host also starts it on a free machine (`specs/0134`). */
  press: (start: boolean) => Promise<void>;
  /** Tells the session how this app stands to the page's connector (`set_send_link`). */
  applyLink: (profileId: string, spelling: string) => void;
  /** Tells the session the connector's last status (`set_send_machine`). */
  applyMachine: (entry: LinkEntry) => void;
  /** Whether a press now also starts: the session's rule over the link store's latest status of
   * `profileId`, read at the press (`0134` UX P4). */
  willStart: (profileId: string) => boolean;
}

/** The state spelling and controller word `set_send_machine` takes; no status while the
 * connection is lost (UX notes section 3: "Connection lost" only sends). */
function machineOf(entry: LinkEntry): [string, string] {
  const status = entry.lost ? null : entry.status;
  return [status?.state.name ?? "", controllerWord(status) ?? ""];
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

  const toldMachine = useRef("");
  const applyMachine = useCallback(
    (entry: LinkEntry) => {
      const [state, word] = machineOf(entry);
      const session = getSession();
      if (!session || toldMachine.current === `${state}|${word}`) return;
      toldMachine.current = `${state}|${word}`;
      session.set_send_machine(state, word);
      sync();
    },
    [getSession, sync],
  );
  const willStart = useCallback(
    (profileId: string) => {
      const session = getSession();
      if (!session) return false;
      const [state, word] = machineOf(linkStore.get(profileId));
      toldMachine.current = `${state}|${word}`;
      session.set_send_machine(state, word);
      const starts = session.send_will_start();
      sync();
      return starts;
    },
    [getSession, sync],
  );

  const press = useCallback(async (start: boolean) => {
    const session = getSession();
    if (!session) return;
    const job = session.send_job();
    if (!job) return;
    const { bytes, name, profile_id: profile, address: to, token } = job;
    sync();
    await linkStore.sendPage(profile, to, { token, name, bytes, start }, (accepted) => {
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
  return { press, applyLink, applyMachine, willStart };
}

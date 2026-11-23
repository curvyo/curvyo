// The Machine controls footer of the Operations tab (`specs/0118-machine-control` UX notes
// sections 3 to 8; architect addendum M11): Send job..., the job commands and Home, the failure
// line of its own presses and their settle rules, on the page's one link store. The host holds the
// pin and the token; this hook only sees answers and statuses.
import { useEffect, useRef, useState } from "react";

import { linkStore, useLinkEntry } from "@/hooks/useLinkEntry";
import { type Busy, type LinkEntry } from "@/lib/connectorLink";
import {
  type CommandAnswer,
  type Line,
  type LinkError,
  type SendAnswer,
  type StatusMessage,
  NO_JOB_TO_PAUSE_TEXT,
  UNKNOWN_JOB_TEXT,
  UNREADABLE_TEXT,
  errorLine,
  refusalText,
} from "@/lib/connectorText";
import { type JobListMessage, type ListCall } from "@/lib/jobList";
import { type OwnPress, SETTLES, heldAfterSend, refusalEnds } from "@/lib/machineControls";
import { invoke } from "@/platform/host";

/** A command of the toolbar or Unlock. */
export type Command = keyof typeof SETTLES;

export interface MachineControlsModel {
  entry: LinkEntry;
  own: OwnPress;
  send: () => void;
  command: (kind: Command) => void;
  /** A press in the job list (`specs/0132`); resolves when the connector answered. */
  listPress: (call: ListCall) => Promise<void>;
  /** The last list press found the list changed meanwhile: "The list changed" until the next
   * press in the list or push (UX notes section 4). */
  listChanged: boolean;
}

/** "Connector busy": the connector refused a command that no longer fits (UX notes section 5). */
const BUSY_TEXT = "Connector busy";

/** Resolves once the entry's status satisfies `done`, or after `ms`, or when the link drops. */
function settle(profileId: string, done: (s: StatusMessage | null) => boolean, ms: number): Promise<void> {
  return new Promise<void>((resolve) => {
    let off = () => {};
    let timer: ReturnType<typeof setTimeout> | undefined;
    const finish = () => {
      off();
      if (timer !== undefined) clearTimeout(timer);
      resolve();
    };
    if (ms === 0 || done(linkStore.get(profileId).status) || linkStore.get(profileId).lost) {
      resolve();
      return;
    }
    timer = setTimeout(finish, ms);
    off = linkStore.subscribe(() => {
      const entry = linkStore.get(profileId);
      if (done(entry.status) || entry.lost) finish();
    });
  });
}

/** The footer's model for one saved profile with a connector address; `enabled` is false in the
 * browser build. This reader bootstraps the entry (pairing and one status), because the Machine
 * block need not be mounted. The caller keys the component on the profile and address. */
export function useMachineControls(profileId: string, address: string, enabled: boolean): MachineControlsModel {
  const [failure, setFailure] = useState<Line | null>(null);
  // The list the line "The list changed" was shown against: a newer push ends it.
  const [changedAt, setChangedAt] = useState<JobListMessage | null | undefined>(undefined);
  const entry = useLinkEntry(profileId, address, enabled, true);
  const lastStatus = useRef<StatusMessage | null>(null);
  // The state "No job to pause" was given in; it ends with any other (`specs/0133` H8).
  const refusedAt = useRef<string | null>(null);

  // A refusal or failure ends when the job moves into another state that has a line count.
  useEffect(() => {
    const next = entry.status;
    if (next !== null && refusalEnds(lastStatus.current, next, refusedAt.current)) {
      setFailure(null);
      refusedAt.current = null;
    }
    lastStatus.current = next;
  }, [entry.status]);

  const latestList = useRef(entry.jobList);
  useEffect(() => {
    latestList.current = entry.jobList;
  }, [entry.jobList]);
  const listChanged = changedAt !== undefined && changedAt === entry.jobList;

  const listPress = async (call: ListCall) => {
    if (!enabled) return;
    setChangedAt(undefined);
    const answer = await invoke<{ kind: string; error?: LinkError }>("connector_list_command", {
      profileId,
      address,
      call,
    }).catch(() => ({ kind: "failed", error: { kind: "other", message: "Connector error" } as LinkError }));
    if (answer.kind === "stale" || answer.kind === "ownerRequired") {
      setChangedAt(latestList.current);
    } else if (answer.kind === "failed" && answer.error) {
      fail(answer.error);
    }
  };

  const show = (line: Line | null) => {
    refusedAt.current = null;
    setFailure(line);
  };

  const fail = (error: LinkError) => {
    // A key change or a lost pairing shows in the Machine block too.
    linkStore.linkFailure(profileId, error);
    show(errorLine(error));
  };

  const guard = (kind: Busy, run: () => Promise<void>) => {
    if (!enabled || !linkStore.claim(profileId, kind)) return;
    void run().finally(() => linkStore.release(profileId));
  };

  /** A fresh status; `false` when the connector could not be read. */
  const refresh = async (): Promise<boolean> => {
    const answer = await linkStore.read(profileId, address);
    if (answer.kind === "failed") fail(answer.error);
    return answer.kind === "connected";
  };

  const send = () =>
    guard("send", async () => {
      show(null);
      // A job sent behind a held one waits in the list: it does not take the held name.
      const before = linkStore.get(profileId);
      const keep = (name: string, lines: number, bytes: number) => heldAfterSend(before, { name, lines, bytes, jobId: null });
      const answer = await invoke<SendAnswer>("connector_send_job", { profileId, address }).catch(
        (): SendAnswer => ({ kind: "failed", error: { kind: "other", message: "Connector error" } }),
      );
      linkStore.patch(profileId, { sendingName: null });
      if (answer.kind === "ready") {
        linkStore.patch(profileId, { held: keep(answer.name, answer.lines, answer.bytes) ?? before.held });
        await refresh();
      } else if (answer.kind === "rejected") {
        linkStore.patch(profileId, { held: keep(answer.name, 0, 0) ?? before.held });
        show({ text: refusalText(answer.reason, entry.status?.alarm ?? null), failure: true });
      } else if (answer.kind === "unreadable") {
        // The counts line names the chosen file, as for a refused job (`0064` UX notes §8).
        linkStore.patch(profileId, { held: keep(answer.name, 0, 0) ?? before.held });
        show({ text: UNREADABLE_TEXT, failure: true });
      } else if (answer.kind === "failed") {
        fail(answer.error);
      }
    });

  const command = (kind: Command) =>
    guard(kind, async () => {
      show(null);
      // A Start of the footer ends the page's "Not started" line (`0134` UX notes section 6).
      if (kind === "start") linkStore.patch(profileId, { notStarted: null });
      const answer = await invoke<CommandAnswer>("connector_command", { profileId, address, command: kind }).catch(
        (): CommandAnswer => ({ kind: "failed", error: { kind: "other", message: "Connector error" } }),
      );
      if (answer.kind === "failed") {
        fail(answer.error);
      } else if (answer.kind === "unknownJob") {
        show({ text: UNKNOWN_JOB_TEXT, failure: true });
      } else if (answer.kind === "wrongState" && kind === "home") {
        // A job started meanwhile (UX notes section 5); the fresh status shows the state.
        show({ text: BUSY_TEXT, failure: true });
        await refresh();
      } else if (answer.kind === "wrongState" && kind === "pause") {
        // The Home button's homing (or no job at all) has nothing to pause (`specs/0133` H8).
        show({ text: NO_JOB_TO_PAUSE_TEXT, failure: true });
        if (await refresh()) refusedAt.current = linkStore.get(profileId).status?.state.name ?? null;
      } else {
        // The connector's status says what changed; a fresh read also covers a lost connection.
        await refresh();
        // A real machine confirms later: the button stays busy until it does (`0127` G10 to G12).
        await settle(profileId, SETTLES[kind].done, SETTLES[kind].ms);
      }
    });

  return {
    entry,
    own: { sending: entry.busy === "send", failure },
    send,
    command,
    listPress,
    listChanged,
  };
}

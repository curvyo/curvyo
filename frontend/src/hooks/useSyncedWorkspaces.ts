// The synced workspaces of this device (`specs/0096-own-devices-sync` criteria 8 to 14, 18 to 27,
// ADR 0040 decisions 5 to 8): one background session per switched-on workspace that is not open,
// each reconciled with the device group where this device is admin; arrival of workspaces this
// device does not hold, the decision for a copy that is here already, choosing workspaces after
// Pair (also ones that are not open, shared in the background), and the facts the My devices rows
// show. Every rule is Rust's; this starts, ticks and stops sessions and calls the host.

import { useCallback, useEffect, useRef, useState } from "react";

import {
  type BackgroundHost,
  type BackgroundWorkspace,
  adoptBackground,
  arriveBackground,
  backgroundWorkspaces,
  openRoomNow,
  resumeBackground,
} from "@/lib/backgroundWorkspace";
import { mintPeerId, wasmReady } from "@/lib/editorSession";
import { RETRY_MS, type SwitchOnResult, UnreachableWorkspaces } from "@/lib/unreachableWorkspaces";
import { commitShare, prepareShare, toHex, type ShareEntry } from "@/lib/workspaceShare";
import type { RecentWorkspaceData } from "@/lib/workspaceRows";
import { invoke, isTauri } from "@/platform/host";
import {
  WasmSession,
  type WasmDeviceGroup,
  devices_arrived_notice,
  devices_connect_copy,
  devices_could_not_sync,
  devices_syncing_workspaces,
  keyring_is_admin,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** How long Sync these waits for a room's first answer (ms). */
const CONNECT_MS = 15_000;

/** The group's server, Synced workspaces folder and where each synced workspace is here. */
export interface GroupRecord {
  server: string;
  syncedFolder: string;
  folders: Record<string, string>;
}

/** A workspace offered after Pair (ux-notes 3.1). */
export interface OfferedWorkspace {
  /** Its folder; "" for the open workspace when it has none known. */
  path: string;
  name: string;
  documents: number;
  /** The open workspace. */
  open: boolean;
  /** Its id (hex) as its marker names it. */
  id: string;
  shared: boolean;
  /** Shared, and this device is only a member there: it cannot sync it (criterion 17). */
  memberOnly: boolean;
}

/** A copy of a synced workspace found here, waiting for Connect or a second copy (2.3). */
export interface CopyDecision {
  id: string;
  name: string;
  path: string;
  text: string;
}

interface Summary {
  id: string;
  documents: number;
  shared: boolean;
  /** The marker's `keyring.log` as hex, empty when not shared. */
  keyring: string;
}

interface GroupWorkspace {
  id: string;
  name: string;
  on: boolean;
  /** Left by the maker from every device (`specs/0097` criterion 16). */
  left?: boolean;
}

export interface SyncedWorkspacesDeps {
  group: () => WasmDeviceGroup | null;
  record: { current: GroupRecord };
  /** The record's folders changed: the group's files are written again. */
  recordChanged: () => void;
  label: () => string;
  keys: BackgroundHost["keys"];
  notice: (text: string, alert: boolean) => void;
  /** The open workspace's shared id (hex), "" when none. */
  openId: () => string;
  /** Whether a workspace is open (the first recent one). */
  workspaceOpen: () => boolean;
  /** The editor's session, for the open workspace's facts. */
  openSession: () => WasmSession | null;
  /** This device's member id (hex), "" before the group names it. */
  ownMember: () => string;
  /** A synced workspace's folder was created here: Recent workspaces lists it (ux-notes 4.4). */
  recentsChanged: () => void;
  /** The Share tab's server address as typed, for an alert about another server. */
  tabServer: () => string;
}

export interface SyncedWorkspacesApi {
  /** Every 50 ms: the background sessions' sockets and notices. */
  tick: (now: number) => void;
  /** Every second: start, reconcile, arrive and stop. */
  slow: () => void;
  /** The facts of every synced workspace as one JSON array for the group's view. */
  facts: () => string;
  decisions: CopyDecision[];
  connectCopy: (id: string) => void;
  secondCopy: (id: string) => void;
  /** Loads the workspaces offered after Pair. */
  loadOffered: () => Promise<OfferedWorkspace[]>;
  /** Sync these, for the workspaces that are not open (the open one switches on as before). */
  syncChosen: (chosen: OfferedWorkspace[]) => void;
  progress: string;
  alerts: string[];
  /** Leave my devices in every background workspace, then the workspaces `local` names (ids)
   * become plain local files; whether every removal went out. */
  leaveAll: (local: string[]) => Promise<boolean>;
  /** Stops every background session (this device left or was removed). */
  stopAll: () => Promise<void>;
  /** The network is back. */
  networkBack: () => void;
}

function listed(group: WasmDeviceGroup): GroupWorkspace[] {
  try {
    return (JSON.parse(group.view_json(0, "[]")) as { workspaces: GroupWorkspace[] }).workspaces;
  } catch {
    return [];
  }
}

function random(length: number): Uint8Array {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  return bytes;
}

function fromHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = Number.parseInt(hex.slice(2 * i, 2 * i + 2), 16);
  }
  return bytes;
}

/** Resolves once `runner`'s room answered: whether it was admitted. */
function admitted(runner: BackgroundWorkspace): Promise<"connected" | "refused" | "not-connected"> {
  return new Promise((resolve) => {
    const started = Date.now();
    const timer = window.setInterval(() => {
      const { state, ended } = runner.connection;
      // A relay that refused the device answered: no 15 s wait, and the alert says so.
      const refused = ended === "refused" || ended === "removed";
      if (state === "connected" || state === "not-connected" || refused || Date.now() - started > CONNECT_MS) {
        window.clearInterval(timer);
        resolve(state === "connected" ? "connected" : refused ? "refused" : "not-connected");
      }
    }, 100);
  });
}

/** What a room that did not admit a switch-on tells the alert. */
function notAdmitted(answer: "refused" | "not-connected"): SwitchOnResult {
  return answer === "refused" ? "server-refused" : "unreachable";
}

export function useSyncedWorkspaces(deps: SyncedWorkspacesDeps): SyncedWorkspacesApi {
  const depsRef = useRef(deps);
  useEffect(() => {
    depsRef.current = deps;
  });
  const [decisions, setDecisions] = useState<CopyDecision[]>([]);
  const decisionsRef = useRef<CopyDecision[]>([]);
  const [progress, setProgress] = useState("");
  /** Workspaces Sync these could not switch on for want of the server; a change re-renders. */
  const unreachableRef = useRef(new UnreachableWorkspaces<OfferedWorkspace>());
  const [, setUnreachableSeen] = useState(0);
  const unreachableChanged = useCallback(() => setUnreachableSeen((n) => n + 1), []);
  /** Workspaces whose start is on its way or that wait for a retry, by id. */
  const busyRef = useRef(new Set<string>());
  const retryAtRef = useRef(new Map<string, number>());
  /** Workspaces arriving now, for the arrival notice. */
  const arrivingRef = useRef(new Set<string>());
  /** Workspaces Sync these is switching on: nothing else starts them meanwhile. */
  const switchingRef = useRef(new Set<string>());

  const host = useCallback(
    (): BackgroundHost => ({
      keys: (room) => depsRef.current.keys(room),
      notice: (text, alert) => depsRef.current.notice(text, alert),
      ended: (id, removed) => {
        if (removed) {
          // The copy stays a local workspace; switched on again, it is offered to connect.
          const { record, recordChanged } = depsRef.current;
          const folders = { ...record.current.folders };
          delete folders[id];
          record.current.folders = folders;
          recordChanged();
        }
      },
    }),
    [],
  );

  const setDecisionList = useCallback((next: CopyDecision[]) => {
    decisionsRef.current = next;
    setDecisions(next);
  }, []);

  /** The Synced workspaces folder, made the default one on first need (the opener never chose
   * one at pairing). */
  const syncedFolder = useCallback(async (): Promise<string> => {
    const { record, recordChanged } = depsRef.current;
    if (record.current.syncedFolder !== "") {
      return record.current.syncedFolder;
    }
    const answer = await invoke<{ kind: string; path?: string }>("prepare_synced_folder", { path: "" }).catch(
      (): { kind: string; path?: string } => ({ kind: "refused" }),
    );
    if (answer.kind !== "picked" || !answer.path) {
      return "";
    }
    record.current.syncedFolder = answer.path;
    recordChanged();
    return answer.path;
  }, []);

  /** Starts the background session of `workspace`, whose folder here is known. */
  const start = useCallback(
    async (workspace: GroupWorkspace, path: string) => {
      const { record, label } = depsRef.current;
      const registered = await invoke<string>("register_synced_workspace", { id: workspace.id, path }).catch(() => "");
      if (registered === "") {
        return;
      }
      const marker = await invoke<unknown>("workspace_marker_share", { workspace: workspace.id }).catch(() => null);
      if (marker) {
        await resumeBackground(workspace.id, workspace.name, label(), host());
        return;
      }
      // Not joined yet (a try that did not get in, or a copy to connect): arrive.
      arrivingRef.current.add(workspace.id);
      await arriveBackground(workspace.id, workspace.name, record.current.server, label(), host());
    },
    [host],
  );

  /** A switched-on workspace this device does not hold (criteria 10, 11, 14). */
  const arrive = useCallback(
    async (workspace: GroupWorkspace) => {
      const { record, recordChanged } = depsRef.current;
      const synced = await syncedFolder();
      if (synced === "") {
        return;
      }
      const copy = await invoke<string | null>("find_workspace_copy", { id: workspace.id, synced }).catch(() => null);
      if (copy) {
        if (!decisionsRef.current.some((d) => d.id === workspace.id)) {
          setDecisionList([
            ...decisionsRef.current,
            { id: workspace.id, name: workspace.name, path: copy, text: devices_connect_copy(workspace.name, copy) },
          ]);
        }
        return;
      }
      const answer = await invoke<{ kind: string; path?: string }>("create_synced_workspace", {
        id: workspace.id,
        synced,
        name: workspace.name,
      }).catch((): { kind: string; path?: string } => ({ kind: "refused" }));
      if (answer.kind !== "picked" || !answer.path) {
        return;
      }
      record.current.folders = { ...record.current.folders, [workspace.id]: answer.path };
      recordChanged();
      depsRef.current.recentsChanged();
      await start(workspace, answer.path);
    },
    [setDecisionList, start, syncedFolder],
  );

  const run = useCallback((id: string, work: () => Promise<void>) => {
    if (busyRef.current.has(id)) {
      return;
    }
    busyRef.current.add(id);
    void work().finally(() => busyRef.current.delete(id));
  }, []);

  /** Switches on one workspace that is not open (Sync these): Share it in the background if it is
   * not shared, admit every group device, then the list entry (ADR 0040 decision 6). */
  const switchOnClosed = useCallback(
    async (chosen: OfferedWorkspace, report: (done: number, total: number) => void): Promise<SwitchOnResult> => {
      const { group: getGroup, record, recordChanged, label, keys } = depsRef.current;
      const group = getGroup();
      if (record.current.server === "") {
        return "no-server";
      }
      const identity = await keys("");
      if (!group || !identity) {
        return "refused";
      }
      if (!group.is_online()) {
        // The group's own connection is down: no room would answer either, so no 15 s wait.
        return "offline";
      }
      const registered = await invoke<string>("register_synced_workspace", { id: chosen.id, path: chosen.path }).catch(() => "");
      if (registered === "") {
        return "refused";
      }
      let id = chosen.id;
      let runner: BackgroundWorkspace | null;
      if (chosen.shared) {
        runner = await resumeBackground(id, chosen.name, label(), host());
      } else {
        const entries = await invoke<ShareEntry[]>("scan_open_workspace", { workspace: chosen.id }).catch(() => null);
        if (!entries) {
          return "refused";
        }
        const prepared = await prepareShare(entries, { progress: report }, chosen.id);
        try {
          id = toHex(prepared.id);
          switchingRef.current.add(id);
          await wasmReady();
          const session = new WasmSession(mintPeerId());
          const error = session.share_workspace(
            record.current.server,
            label(),
            chosen.name,
            prepared.builder,
            fromHex(identity.identity),
            random(32),
            random(32),
            Date.now(),
          );
          if (error) {
            // Failed here before anything was sent: not the server's doing.
            session.free();
            return "share-failed";
          }
          runner = adoptBackground(id, chosen.name, session, host());
          const first = await admitted(runner);
          if (first !== "connected") {
            await runner.stop();
            switchingRef.current.delete(id);
            return notAdmitted(first);
          }
          await commitShare(prepared, chosen.id);
          await invoke<string>("register_synced_workspace", { id, path: chosen.path }).catch(() => "");
          runner.held = false;
        } finally {
          prepared.builder.free();
        }
      }
      if (!runner) {
        switchingRef.current.delete(id);
        return "refused";
      }
      const admission = await admitted(runner);
      if (admission !== "connected") {
        // Not running in the background without a list entry: a retry starts it afresh.
        await runner.stop();
        switchingRef.current.delete(id);
        return notAdmitted(admission);
      }
      const answer = runner.session.reconcile_workspace(group, true);
      runner.drive();
      switchingRef.current.delete(id);
      if (answer === "" || (JSON.parse(answer) as { needs_admin?: boolean }).needs_admin) {
        // Only a member here: nothing is switched on (criterion 17).
        return "member-only";
      }
      // The devices are members now, then the list entry (criterion 11).
      group.switch_workspace(id, chosen.name, true);
      record.current.folders = { ...record.current.folders, [id]: chosen.path };
      recordChanged();
      return "ok";
    },
    [host],
  );

  /** Tries the workspaces that waited for the server again, without progress lines. */
  const retryUnreachable = useCallback(
    (due: OfferedWorkspace[]) => {
      void (async () => {
        for (const workspace of due) {
          const result = await switchOnClosed(workspace, () => undefined);
          // A shared one resumes from its marker; an unshared one would be read and uploaded again.
          unreachableRef.current.record(workspace.id, workspace.name, workspace, result, workspace.shared);
        }
        unreachableRef.current.settled(Date.now());
        unreachableChanged();
      })();
    },
    [switchOnClosed, unreachableChanged],
  );

  const slow = useCallback(() => {
    const { group: getGroup, record, openId } = depsRef.current;
    const group = getGroup();
    const unreachable = unreachableRef.current;
    if (!group || group.removed()) {
      if (unreachable.clear()) {
        unreachableChanged();
      }
      return;
    }
    const open = openId() || openRoomNow();
    const now = Date.now();
    const runners = backgroundWorkspaces();
    const list = listed(group);
    if (unreachable.forget(list.map((w) => w.id))) {
      unreachableChanged();
    }
    const due = unreachable.due(group.is_online(), now);
    if (due.length > 0) {
      retryUnreachable(due);
    }
    for (const workspace of list) {
      const runner = runners.get(workspace.id);
      if (workspace.id === open || switchingRef.current.has(workspace.id)) {
        continue;
      }
      if (runner) {
        if (!runner.live) {
          // Refused (not admitted yet) or ended: stop, and try again later.
          retryAtRef.current.set(workspace.id, now + RETRY_MS);
          void runner.stop();
          continue;
        }
        if (arrivingRef.current.has(workspace.id) && runner.session.workspace_id_hex() !== "" && runner.state === "connected") {
          arrivingRef.current.delete(workspace.id);
          depsRef.current.notice(devices_arrived_notice(workspace.name), false);
        }
        if (group.is_online() && runner.state === "connected" && workspace.left) {
          // The maker left it from every device (`specs/0097` criterion 16): this device runs the
          // left plan; the runner stops once its own removal ended the share.
          runner.session.reconcile_left_workspace(group);
          runner.drive();
          continue;
        }
        if (group.is_online() && runner.state === "connected") {
          runner.session.reconcile_workspace(group, workspace.on);
          runner.drive();
          if (!workspace.on) {
            const facts = runner.session.device_facts_json(group, false);
            const pending = facts ? (JSON.parse(facts) as { pending_removes: string[] }).pending_removes : [];
            if (pending.length === 0) {
              // Switched off and nothing left to remove: it stays a local, shared workspace.
              void runner.stop();
            }
          }
        }
        continue;
      }
      if (!workspace.on || (retryAtRef.current.get(workspace.id) ?? 0) > now) {
        continue;
      }
      const path = record.current.folders[workspace.id];
      if (path === undefined) {
        if (!decisionsRef.current.some((d) => d.id === workspace.id)) {
          run(workspace.id, () => arrive(workspace));
        }
      } else if (path !== "") {
        run(workspace.id, () => start(workspace, path));
      }
    }
  }, [arrive, retryUnreachable, run, start, unreachableChanged]);

  const tick = useCallback((now: number) => {
    for (const runner of backgroundWorkspaces().values()) {
      runner.tick(now);
    }
  }, []);

  const facts = useCallback((): string => {
    const { group: getGroup, openId, openSession } = depsRef.current;
    const group = getGroup();
    if (!group) {
      return "[]";
    }
    const on = new Map(listed(group).map((w) => [w.id, w.on]));
    const all: string[] = [];
    for (const runner of backgroundWorkspaces().values()) {
      if (on.get(runner.id)) {
        const one = runner.session.device_facts_json(group, true);
        if (one !== "") {
          all.push(one);
        }
      }
    }
    const id = openId();
    const session = openSession();
    if (id !== "" && session && on.get(id)) {
      const one = session.device_facts_json(group, true);
      if (one !== "") {
        all.push(one);
      }
    }
    for (const decision of decisionsRef.current) {
      all.push(JSON.stringify({ id: decision.id, name: decision.name, state: { kind: "answer" } }));
    }
    return `[${all.join(",")}]`;
  }, []);

  const connectCopy = useCallback(
    (id: string) => {
      const decision = decisionsRef.current.find((d) => d.id === id);
      const group = depsRef.current.group();
      if (!decision || !group) {
        return;
      }
      setDecisionList(decisionsRef.current.filter((d) => d.id !== id));
      const { record, recordChanged } = depsRef.current;
      record.current.folders = { ...record.current.folders, [id]: decision.path };
      recordChanged();
      // Joining into the copy: each document that is there meets the room's in its file.
      run(id, () => start({ id, name: decision.name, on: true }, decision.path));
    },
    [run, setDecisionList, start],
  );

  const secondCopy = useCallback(
    (id: string) => {
      const decision = decisionsRef.current.find((d) => d.id === id);
      if (!decision) {
        return;
      }
      setDecisionList(decisionsRef.current.filter((d) => d.id !== id));
      run(id, async () => {
        const { record, recordChanged } = depsRef.current;
        const synced = await syncedFolder();
        const answer = await invoke<{ kind: string; path?: string }>("create_synced_workspace", {
          id,
          synced,
          name: decision.name,
        }).catch((): { kind: string; path?: string } => ({ kind: "refused" }));
        if (answer.kind !== "picked" || !answer.path) {
          return;
        }
        record.current.folders = { ...record.current.folders, [id]: answer.path };
        recordChanged();
        depsRef.current.recentsChanged();
        await start({ id, name: decision.name, on: true }, answer.path);
      });
    },
    [run, setDecisionList, start, syncedFolder],
  );

  const loadOffered = useCallback(async (): Promise<OfferedWorkspace[]> => {
    const recents = await invoke<RecentWorkspaceData[]>("recent_workspaces").catch((): RecentWorkspaceData[] => []);
    const out: OfferedWorkspace[] = [];
    for (const [index, recent] of recents.entries()) {
      if (!recent.found) {
        continue;
      }
      const summary = await invoke<Summary | null>("workspace_summary", { path: recent.path }).catch(() => null);
      if (!summary) {
        continue;
      }
      // Opening a workspace puts it first among the recent ones.
      const open = index === 0 && depsRef.current.workspaceOpen();
      const me = depsRef.current.ownMember();
      const memberOnly = summary.shared && me !== "" && !keyring_is_admin(fromHex(summary.keyring), me);
      out.push({
        path: recent.path,
        name: recent.name,
        documents: summary.documents,
        open,
        id: summary.id,
        shared: summary.shared,
        memberOnly,
      });
    }
    return out;
  }, []);

  const syncChosen = useCallback(
    (chosen: OfferedWorkspace[]) => {
      const closed = chosen.filter((c) => !c.open);
      if (closed.length === 0) {
        return;
      }
      void (async () => {
        const total = closed.reduce((sum, c) => sum + c.documents, 0);
        let before = 0;
        for (const workspace of closed) {
          const report = (done: number) => setProgress(devices_syncing_workspaces(closed.length, before + done, total));
          report(0);
          const result = await switchOnClosed(workspace, report);
          // A shared one resumes from its marker; an unshared one would be read and uploaded again.
          unreachableRef.current.record(workspace.id, workspace.name, workspace, result, workspace.shared);
          before += workspace.documents;
        }
        unreachableRef.current.settled(Date.now());
        setProgress("");
        unreachableChanged();
      })();
    },
    [switchOnClosed, unreachableChanged],
  );

  const leaveAll = useCallback(async (local: string[]) => {
    let all = true;
    for (const runner of [...backgroundWorkspaces().values()]) {
      const id = runner.id;
      all = (await runner.leave()) && all;
      // Criterion 27: the epoch keys of the workspaces it left are forgotten.
      await invoke("write_document_keys", { room: id, keys: [] }).catch(() => undefined);
    }
    // A runner whose removal could not go out (not yet connected) stops all the same.
    for (const runner of [...backgroundWorkspaces().values()]) {
      await runner.stop();
    }
    // The group ends here: the workspaces `local` names stay as plain local files (customer
    // 2026-11-09), also one whose removal could not go out.
    for (const id of local) {
      await invoke("clear_workspace_marker_share", { workspace: id }).catch(() => undefined);
    }
    return all;
  }, []);


  const stopAll = useCallback(async () => {
    for (const runner of [...backgroundWorkspaces().values()]) {
      await runner.stop();
    }
    setDecisionList([]);
    unreachableRef.current.clear();
    unreachableChanged();
  }, [setDecisionList, unreachableChanged]);

  const networkBack = useCallback(() => {
    for (const runner of backgroundWorkspaces().values()) {
      runner.networkBack();
    }
    // Workspaces that waited for a retry try again on the next look, not 30 s later.
    retryAtRef.current.clear();
    unreachableRef.current.wake();
  }, []);

  const server = deps.record.current.server;
  // A browser on an https page cannot open a ws:// address: that is why, not the server.
  const blocked = !isTauri && window.location.protocol === "https:" && server.startsWith("ws://");
  const alerts = unreachableRef.current
    .alerts(server)
    .map(({ problem, names }) =>
      devices_could_not_sync(
        JSON.stringify(names),
        problem === "unreachable" && blocked ? "needs-tls" : problem,
        server,
        deps.tabServer(),
      ),
    );

  return {
    tick,
    slow,
    facts,
    decisions,
    connectCopy,
    secondCopy,
    loadOffered,
    syncChosen,
    progress,
    alerts,
    leaveAll,
    stopAll,
    networkBack,
  };
}

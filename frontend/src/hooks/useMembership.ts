// The membership lifecycle in the page (`specs/0097-membership-lifecycle`): role changes, Leave,
// Change keys now, Add back, the return check with Sync again and Keep as it is, the notices
// with the state kept per room, and the Share tab mark. Every rule and word is Rust's; this hook
// carries commands, keys, entropy and the kept state across.

import { useCallback, useEffect, useRef, useState } from "react";

import { type CollaborationApi, keptLabelsJson } from "@/hooks/useCollaboration";
import type { EditorSession } from "@/hooks/useEditorSession";
import type { ProjectApi } from "@/hooks/useProject";
import type { WorkspaceApi } from "@/hooks/useWorkspace";
import type { WorkspaceSyncApi } from "@/hooks/useWorkspaceSync";
import type { DevicesApi } from "@/lib/devicesApi";
import { readWorkspaceEnded } from "@/lib/endedMarker";
import { type EndedMarker, parseEnded } from "@/lib/endedRecord";
import {
  type AdminChangeLine,
  EMPTY_ROOM,
  type MembershipNotice,
  type RoomMembership,
  acknowledge as acknowledgeIn,
  applyNotices,
  dismissTier as dismissIn,
  type FrozenRoom,
  frozenRoom,
  loadMembership,
  parseNotices,
  saveMembership,
  shownNotices,
} from "@/lib/membershipState";
import { invoke } from "@/platform/host";
import { membership_clock_time, membership_word } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** A removed or left file or workspace as the Removed block shows it (ux-notes 7). */
export interface EndedShown {
  workspace: boolean;
  left: boolean;
  server: string;
  /** "" before Check, else the session's check state. */
  check: string;
  addedBy: string;
}

export interface MembershipApi {
  /** The shared thing's name as the texts say it: the workspace's, else the document's. */
  name: string;
  /** The notices block and the second-admin line, as shown (dismissed and acknowledged gone). */
  notices: MembershipNotice[];
  /** The frozen room behind Make admin's warning (criterion 28), `null` when not frozen. */
  frozen: FrozenRoom | null;
  /** The admin-change line with its time, `null` when none. */
  line: (AdminChangeLine & { at: string }) | null;
  /** The Share tab mark (criterion 30). */
  unread: boolean;
  markRead: () => void;
  dismissTier: (tier: string) => void;
  acknowledge: (notice: MembershipNotice) => void;
  /** Make admin, Make member, Step down: "" when written, else why not. */
  setRole: (member: string, admin: boolean) => string;
  /** Change keys now. */
  rotate: () => string;
  /** Add back with every key (`full`) or from now on; `label` names it in the canvas notice. */
  addBack: (member: string, full: boolean, label: string) => string;
  /** Remove with a key rotation. */
  remove: (member: string) => string;
  /** Leave: the person leave for a workspace synced to my devices (criterion 16). */
  leave: () => string;
  /** Whether Leave is the person leave. */
  personLeave: boolean;
  ended: EndedShown | null;
  /** Check whether I was added back: one connection, nothing else (criterion 33). */
  check: () => void;
  /** Sync again: "" when it started, else why not (the prompt stays). */
  syncAgain: () => Promise<string>;
  /** Keep as it is: one own leave entry. */
  /** Keep as it is: "" when the leave's connection started, else the refusal's words. */
  keepAsItIs: () => Promise<string>;
  /** The switch turned on for a removed or left workspace: clears the `left` mark (ux-notes
   * 5.4) and never shares it anew, which would drop its way back; Check stays the way back. */
  clearLeftMark: () => void;
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

/** The open document's name as the texts say it: its file name without `.curvyo`. */
function documentName(project: ProjectApi): string {
  return (project.info.fileName ?? "").replace(/\.curvyo$/i, "");
}

function storage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

const CHANGE_TO_ME: Record<string, string> = { nowAdmin: "you-are-admin", noLongerAdmin: "you-are-no-longer-admin" };

/** How long a document that synced again waits for its room before it is saved anyway (ms). */
const SAVE_AFTER_SYNC_MS = 10_000;

/** Saves once `session` is online again (or after [`SAVE_AFTER_SYNC_MS`]), unless another
 * document replaced it meanwhile. */
function saveWhenOnline(session: { sync_online(): boolean }, current: () => unknown, save: () => void): void {
  const started = Date.now();
  const timer = window.setInterval(() => {
    if (current() !== session) {
      window.clearInterval(timer);
    } else if (session.sync_online() || Date.now() - started > SAVE_AFTER_SYNC_MS) {
      window.clearInterval(timer);
      save();
    }
  }, 100);
}

export function useMembership(
  editor: EditorSession,
  project: ProjectApi,
  collab: CollaborationApi,
  workspace: WorkspaceApi,
  workspaceSync: WorkspaceSyncApi,
  devices: DevicesApi,
): MembershipApi {
  const { getSession } = editor;
  const showNotice = editor.showHistoryNotice;
  const { view } = collab;
  const refs = useRef({ project, collab, workspace, workspaceSync, devices });
  useEffect(() => {
    refs.current = { project, collab, workspace, workspaceSync, devices };
  });
  const [kept, setKept] = useState(() => loadMembership(storage()));
  const [line, setLine] = useState<(AdminChangeLine & { at: string }) | null>(null);
  const [notices, setNotices] = useState<MembershipNotice[]>([]);
  const [endedMarker, setEndedMarker] = useState<EndedMarker | null>(null);
  /** This device's own act: the next notices never set the mark (ux-notes 2.1). */
  const ownActRef = useRef(false);
  /** Change keys now was pressed here: its end is a canvas notice. */
  const rotatingRef = useRef(false);
  // This device's own leave of a document waits for the server; once it is confirmed the file
  // holds the ended record "left" and is saved again.
  const leavingRef = useRef(false);
  useEffect(() => {
    if (leavingRef.current && view.ended === "left") {
      leavingRef.current = false;
      refs.current.project.save();
    }
  }, [view.ended]);
  const roomRef = useRef("");

  const live = view.live && (view.shared || view.workspace != null);
  const room = live ? (view.workspace?.id ?? getSession()?.shared_room_hex() ?? "") : "";
  useEffect(() => {
    roomRef.current = room;
  }, [room]);
  const roomState: RoomMembership = (room !== "" && kept[room]) || EMPTY_ROOM;

  const keep = useCallback((id: string, state: RoomMembership) => {
    setKept((all) => saveMembership(storage(), all, id, state));
  }, []);

  // The notices on every change of the share: against the admin set last shown here.
  const ownGroup = JSON.stringify(devices.ownMembers);
  useEffect(() => {
    const session = getSession();
    if (!session || room === "") {
      setNotices([]);
      return;
    }
    const all = loadMembership(storage());
    const state = all[room] ?? EMPTY_ROOM;
    const previous = state.admins ? JSON.stringify(state.admins) : "";
    const found = parseNotices(session.membership_notices_json(previous, ownGroup));
    const current = JSON.parse(session.share_admin_set_json() || "[]") as string[];
    const own = ownActRef.current;
    ownActRef.current = false;
    const { room: next, line: changed } = applyNotices(state, found, current, view.admin === true, own);
    if (JSON.stringify(next) !== JSON.stringify(state)) {
      keep(room, next);
    }
    setNotices(found);
    if (rotatingRef.current && !found.some((n) => n.kind === "rotationPending")) {
      // Change keys now is done: the notice left for everybody (ux-notes 4).
      rotatingRef.current = false;
      showNotice(membership_word("keys-changed"), 5000, "canvas", "status");
    }
    if (changed) {
      const now = new Date();
      setLine({ ...changed, at: membership_clock_time(now.getHours(), now.getMinutes()) });
    }
    // Changes to me are also a canvas notice (ux-notes 2.4), never for my own act.
    const me = view.participants.find((p) => p.self)?.member;
    for (const notice of found) {
      const key = CHANGE_TO_ME[notice.kind];
      if (!own && key && notice.member === me) {
        showNotice(membership_word(key), 5000, "canvas", "status");
      }
    }
  }, [view, room, ownGroup, getSession, keep, showNotice]);

  // A new room starts without a line.
  useEffect(() => setLine(null), [room]);

  // A removed or left workspace's record, read from its marker while it is open and not shared.
  const workspaceOpen = workspace.open;
  const sharedOpen = workspaceSync.sharedOpen;
  useEffect(() => {
    if (!workspaceOpen || sharedOpen) {
      setEndedMarker(null);
      return undefined;
    }
    let cancelled = false;
    const read = () =>
      void readWorkspaceEnded().then((found) => {
        if (!cancelled) {
          setEndedMarker((old) => (JSON.stringify(old) === JSON.stringify(found) ? old : found));
        }
      });
    read();
    // The removal writes the record a moment after the session ended.
    const timer = window.setInterval(read, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [workspaceOpen, sharedOpen]);

  const endedFacts = endedMarker ? parseEnded(endedMarker.ended_json) : null;
  const documentEnded = !view.live && (view.ended === "removed" || view.ended === "left") && !view.checkWorkspace;
  let ended: EndedShown | null = null;
  if (view.check !== undefined && view.checkWorkspace !== undefined) {
    ended = {
      workspace: view.checkWorkspace,
      left: view.ended === "left",
      server: view.server ?? "",
      check: view.check,
      addedBy: view.addedBy ?? "",
    };
  } else if (endedFacts && workspaceOpen && !sharedOpen) {
    ended = { workspace: true, left: endedFacts.left, server: endedFacts.server, check: "", addedBy: "" };
  } else if (documentEnded && view.endedRoom) {
    ended = { workspace: false, left: view.ended === "left", server: view.server ?? "", check: "", addedBy: "" };
  }

  const drive = useCallback((session: NonNullable<ReturnType<typeof getSession>>) => refs.current.collab.drive(session), []);

  const act = useCallback(
    (run: (session: NonNullable<ReturnType<typeof getSession>>) => string): string => {
      const session = getSession();
      if (!session) {
        return "";
      }
      const refusal = run(session);
      if (refusal === "") {
        ownActRef.current = true;
      }
      drive(session);
      return refusal;
    },
    [getSession, drive],
  );

  const setRole = useCallback((member: string, admin: boolean) => act((s) => s.set_participant_role(member, admin)), [act]);
  const rotate = useCallback(() => {
    const refusal = act((s) => s.rotate_share_keys());
    rotatingRef.current = refusal === "";
    return refusal;
  }, [act]);
  // "Ben was added back." comes from the session once the server holds the add (ADR 0041
  // decision 11), as a sync notice.
  const addBack = useCallback(
    (member: string, full: boolean, _label: string) => act((s) => s.add_back_participant(member, full)),
    [act],
  );
  const remove = useCallback((member: string) => act((s) => s.remove_participant(member)), [act]);

  const personLeave = devices.openSynced && view.workspace != null;
  const leave = useCallback((): string => {
    if (personLeave) {
      const refusal = refs.current.devices.leavePerson();
      if (refusal === "") {
        ownActRef.current = true;
      }
      return refusal;
    }
    const refusal = act((s) => s.leave_share(refs.current.workspace.open ? "" : documentName(refs.current.project)));
    if (refusal === "" && view.workspace == null) {
      // The keyring log with the pending leave is saved now: a leave not yet confirmed goes out
      // on the next open (ADR 0041 decision 11); the ended record is saved once it is.
      leavingRef.current = true;
      refs.current.project.save();
    }
    return refusal;
  }, [act, personLeave, view.workspace]);

  const withKeys = useCallback(
    async (slot: string, run: (keys: { identity: string; keys: [number, string][] }) => void) => {
      const keys = await refs.current.collab.deviceKeys(slot, () => undefined);
      if (keys) {
        run(keys);
      }
    },
    [],
  );

  const check = useCallback(() => {
    const session = getSession();
    if (!session || !ended) {
      return;
    }
    const marker = endedMarker;
    if (ended.workspace && marker && endedFacts) {
      void withKeys(endedFacts.room, (keys) => {
        session.check_workspace_added_back(
          Uint8Array.from(marker.ended_json),
          JSON.stringify(keys.keys),
          fromHex(keys.identity),
          random(32),
          random(32),
          Date.now(),
        );
        session.remember_share_labels(keptLabelsJson());
        drive(session);
      });
      return;
    }
    const slot = view.endedRoom ?? "";
    void withKeys(slot, (keys) => {
      session.check_added_back(JSON.stringify(keys.keys), fromHex(keys.identity), random(32), random(32), Date.now());
      session.remember_share_labels(keptLabelsJson());
      drive(session);
    });
  }, [getSession, ended, endedMarker, endedFacts, view.endedRoom, withKeys, drive]);

  const syncAgain = useCallback(async (): Promise<string> => {
    const session = getSession();
    if (!session || !ended) {
      return "";
    }
    const { collab: c, workspace: ws, devices: d, project: p } = refs.current;
    const keys = await c.deviceKeys("", () => undefined);
    if (!keys) {
      return "";
    }
    if (ended.workspace) {
      const marker = endedMarker;
      const room = marker ? (parseEnded(marker.ended_json)?.room ?? "") : "";
      // A workspace the maker left: the mark goes first, acknowledged before connecting (5.4).
      if (room !== "" && d.leftIds.includes(room) && !(await d.clearLeft(room, ws.name))) {
        return membership_word("not-connected");
      }
      const started = session.workspace_sync_again(
        Uint8Array.from(marker?.list ?? []),
        ws.name,
        c.name,
        fromHex(keys.identity),
        random(32),
        random(32),
        Date.now(),
      );
      drive(session);
      return started ? "" : membership_word("sync-again-failed");
    }
    const started = session.sync_again(c.name, fromHex(keys.identity), random(32), random(32), Date.now());
    drive(session);
    if (started) {
      // The file gets its share record once the room admitted the document: the first save
      // builds the thumbnail renderer, seconds of work that would hold back the connection.
      saveWhenOnline(session, getSession, p.save);
    }
    return started ? "" : membership_word("sync-again-failed");
  }, [getSession, ended, endedMarker, drive]);

  const keepAsItIs = useCallback(async (): Promise<string> => {
    const session = getSession();
    if (!session) {
      return membership_word("could-not");
    }
    // Refused before anything is sent while offline; the prompt stays (ux-notes 7.3).
    if (!navigator.onLine) {
      return membership_word("not-connected");
    }
    const keys = await refs.current.collab.deviceKeys("", () => undefined);
    if (!keys) {
      return membership_word("could-not");
    }
    const started = session.keep_as_it_is(fromHex(keys.identity), random(32), random(32), Date.now());
    drive(session);
    return started ? "" : membership_word("could-not");
  }, [getSession, drive]);

  const leftRoom = endedFacts?.room ?? "";
  const clearLeftMark = useCallback(() => {
    const { devices: d, workspace: ws } = refs.current;
    if (leftRoom !== "" && d.leftIds.includes(leftRoom)) {
      // Proposal: written switched off, as Sync again does, so no sibling acts on it.
      void d.clearLeft(leftRoom, ws.name);
    }
  }, [leftRoom]);

  // Keep as it is answered: the record says "left" in the file or the marker.
  const answered = view.check;
  useEffect(() => {
    if (answered !== "leftAgain") {
      return;
    }
    const session = getSession();
    if (!session) {
      return;
    }
    const record = Array.from(session.workspace_ended_json());
    if (view.checkWorkspace && record.length > 0) {
      void invoke("end_workspace_marker_share", { ended_json: record }).catch(() => undefined);
    } else if (!view.checkWorkspace) {
      refs.current.project.save();
    }
  }, [answered, view.checkWorkspace, getSession]);

  const markRead = useCallback(() => {
    const id = roomRef.current;
    if (id === "") {
      return;
    }
    const state = loadMembership(storage())[id] ?? EMPTY_ROOM;
    if (state.unread) {
      keep(id, { ...state, unread: false });
    }
  }, [keep]);

  const dismissTier = useCallback((tier: string) => keep(room, dismissIn(roomState, tier)), [keep, room, roomState]);
  const acknowledge = useCallback(
    (notice: MembershipNotice) => keep(room, acknowledgeIn(roomState, notice)),
    [keep, room, roomState],
  );

  const name = workspace.open ? workspace.name : documentName(project);
  return {
    name,
    notices: view.admin ? shownNotices(roomState, notices) : shownNotices(roomState, notices).filter((n) => n.kind !== "oneAdmin" && n.kind !== "onlyOwnAdmins"),
    frozen: frozenRoom(notices),
    line,
    unread: roomState.unread,
    markRead,
    dismissTier,
    acknowledge,
    setRole,
    rotate,
    addBack,
    remove,
    leave,
    personLeave,
    ended,
    check,
    syncAgain,
    keepAsItIs,
    clearLeftMark,
  };
}

// What this data directory keeps per room for the membership notices (`specs/0097`
// criteria 7, 8, 27 and 30, ADR 0041 decision 9): the admin set last shown, the unread mark,
// the dismissed second-admin tiers and the acknowledged mutual removals. Kept in local storage
// beside the participant labels (adrs.md decision 5); never in the file, never sent.

/** One notice of `membership_notices_json`. */
export interface MembershipNotice {
  kind:
    | "nowAdmin"
    | "noLongerAdmin"
    | "adminLeft"
    | "adminRemoved"
    | "rotationPending"
    | "mutualRemoval"
    | "notApplied"
    | "oneAdmin"
    | "onlyOwnAdmins";
  member?: string;
  label?: string;
  left?: boolean;
  admins?: { member: string; label: string }[];
  entries?: string[];
  author?: string;
  subject?: string;
  removal?: boolean;
}

/** The kept state of one room. */
export interface RoomMembership {
  /** The admin set last shown (hex ids), absent before the first showing. */
  admins?: string[];
  unread: boolean;
  /** Dismissed second-admin tiers (`oneAdmin`, `onlyOwnAdmins`). */
  dismissed: string[];
  /** Notices that already set the mark once (a rotation pending, a mutual removal). */
  marked: string[];
  /** Acknowledged mutual removals, by their first entry hash. */
  acked: string[];
}

/** The admin-change line (ux-notes 2.3). */
export interface AdminChangeLine {
  kind: MembershipNotice["kind"];
  label: string;
  more: number;
}

const CHANGES = new Set(["nowAdmin", "noLongerAdmin", "adminLeft", "adminRemoved"]);

export const EMPTY_ROOM: RoomMembership = { unread: false, dismissed: [], marked: [], acked: [] };

/** The key under which a notice sets the mark once. */
function markKey(notice: MembershipNotice): string {
  if (notice.kind === "rotationPending") {
    return `rot:${notice.member ?? ""}`;
  }
  if (notice.kind === "mutualRemoval") {
    return `mut:${notice.entries?.[0] ?? ""}`;
  }
  return "";
}

/** The notices as the session answered them, `[]` when unreadable. */
export function parseNotices(json: string): MembershipNotice[] {
  try {
    const parsed: unknown = JSON.parse(json);
    return Array.isArray(parsed) ? (parsed as MembershipNotice[]) : [];
  } catch {
    return [];
  }
}

/**
 * The room's state after the session answered `notices` against `room.admins`, with
 * `current` the admin set now. `own` says the change came from this device's own act, which
 * never sets the mark (ux-notes 2.1). The admin-change line is the newest change, `null` when
 * the admin set did not change.
 */
export function applyNotices(
  room: RoomMembership,
  notices: MembershipNotice[],
  current: string[],
  admin: boolean,
  own: boolean,
): { room: RoomMembership; line: AdminChangeLine | null } {
  const changes = notices.filter((n) => CHANGES.has(n.kind));
  let unread = room.unread;
  const marked = room.marked.filter((key) => notices.some((n) => markKey(n) === key));
  if (changes.length > 0 && !own) {
    unread = true;
  }
  for (const notice of notices) {
    const key = markKey(notice);
    const marks = notice.kind === "mutualRemoval" ? !room.acked.includes(notice.entries?.[0] ?? "") : notice.kind === "rotationPending" && admin;
    if (key !== "" && marks && !marked.includes(key)) {
      marked.push(key);
      unread ||= !own;
    }
  }
  const newest = changes[changes.length - 1];
  return {
    room: { ...room, admins: current, unread, marked },
    line: newest ? { kind: newest.kind, label: newest.label ?? "", more: changes.length - 1 } : null,
  };
}

/** The notices the Share tab shows: second-admin tiers not dismissed, mutual removals not
 * acknowledged (with their held-back lines), the rest as they come. */
export function shownNotices(room: RoomMembership, notices: MembershipNotice[]): MembershipNotice[] {
  return notices.filter((n) => {
    if (CHANGES.has(n.kind)) {
      return false;
    }
    if (n.kind === "oneAdmin" || n.kind === "onlyOwnAdmins") {
      return !room.dismissed.includes(n.kind);
    }
    if (n.kind === "mutualRemoval") {
      return !room.acked.includes(n.entries?.[0] ?? "");
    }
    return true;
  });
}

/** The room after Dismiss on a second-admin tier (MEM-NUDGE-DISMISS: per tier). */
export function dismissTier(room: RoomMembership, tier: string): RoomMembership {
  return room.dismissed.includes(tier) ? room : { ...room, dismissed: [...room.dismissed, tier] };
}

/** The room after OK on a mutual-removal notice. */
export function acknowledge(room: RoomMembership, notice: MembershipNotice): RoomMembership {
  const first = notice.entries?.[0] ?? "";
  return first === "" || room.acked.includes(first) ? room : { ...room, acked: [...room.acked, first] };
}

/** The labels behind the frozen-room warning (criterion 28, from the keyring whether or not
 * the notice was acknowledged): the mutual removal's admins, else the admin whose own act was
 * held back. */
export function frozenRoom(notices: MembershipNotice[]): { mutual: string[]; refused: string } | null {
  const mutual = notices.find((n) => n.kind === "mutualRemoval");
  if (mutual) {
    return { mutual: (mutual.admins ?? []).map((a) => a.label), refused: "" };
  }
  const refused = notices.find((n) => n.kind === "notApplied" && n.author === n.subject);
  return refused ? { mutual: [], refused: refused.label ?? "" } : null;
}

/** Storage as the page has it (`window.localStorage`), or a stand-in in tests. */
export interface KeyValueStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

const KEY = "curvyo.share-membership";

/** Every kept room, `{}` when none or unreadable. */
export function loadMembership(store: KeyValueStore | null): Record<string, RoomMembership> {
  try {
    const parsed: unknown = JSON.parse(store?.getItem(KEY) ?? "{}");
    return parsed && typeof parsed === "object" ? (parsed as Record<string, RoomMembership>) : {};
  } catch {
    return {};
  }
}

/** Keeps `room`'s state; a refused write keeps it for this run only. */
export function saveMembership(store: KeyValueStore | null, all: Record<string, RoomMembership>, room: string, state: RoomMembership): Record<string, RoomMembership> {
  const next = { ...all, [room]: state };
  try {
    store?.setItem(KEY, JSON.stringify(next));
  } catch {
    // Kept in memory only.
  }
  return next;
}

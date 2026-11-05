import assert from "node:assert/strict";
import { test } from "node:test";

import {
  EMPTY_ROOM,
  acknowledge,
  applyNotices,
  dismissTier,
  frozenRoom,
  loadMembership,
  parseNotices,
  saveMembership,
  shownNotices,
  type MembershipNotice,
} from "../src/lib/membershipState.ts";

const promoted: MembershipNotice = { kind: "nowAdmin", member: "bb", label: "Ben" };
const left: MembershipNotice = { kind: "adminLeft", member: "cc", label: "Cy" };
const rotation: MembershipNotice = { kind: "rotationPending", member: "cc", label: "Cy", left: true };
const mutual: MembershipNotice = {
  kind: "mutualRemoval",
  admins: [
    { member: "aa", label: "Ana" },
    { member: "cc", label: "Chris" },
  ],
  entries: ["e1", "e2"],
};

test("criterion 30: someone else's admin change sets the mark and the newest line", () => {
  const { room, line } = applyNotices(EMPTY_ROOM, [promoted, left], ["aa", "bb"], true, false);
  assert.equal(room.unread, true);
  assert.deepEqual(room.admins, ["aa", "bb"]);
  assert.deepEqual(line, { kind: "adminLeft", label: "Cy", more: 1 });
});

test("ux-notes 2.1: this device's own act never sets the mark", () => {
  const { room, line } = applyNotices(EMPTY_ROOM, [promoted], ["aa", "bb"], true, true);
  assert.equal(room.unread, false);
  assert.equal(line?.kind, "nowAdmin");
});

test("criterion 11: a rotation pending marks an admin once, a member never", () => {
  const first = applyNotices(EMPTY_ROOM, [rotation], ["aa"], true, false).room;
  assert.equal(first.unread, true);
  const read = { ...first, unread: false };
  assert.equal(applyNotices(read, [rotation], ["aa"], true, false).room.unread, false, "no second mark");
  assert.equal(applyNotices(EMPTY_ROOM, [rotation], ["aa"], false, false).room.unread, false);
  const gone = applyNotices(read, [], ["aa"], true, false).room;
  assert.deepEqual(gone.marked, [], "a cleared notice can mark again later");
});

test("criteria 7, 8 and 27: tiers are dismissed one by one, mutual removals until OK", () => {
  const notices: MembershipNotice[] = [{ kind: "oneAdmin" }, { kind: "onlyOwnAdmins" }, mutual, promoted];
  let room = dismissTier(EMPTY_ROOM, "oneAdmin");
  assert.deepEqual(
    shownNotices(room, notices).map((n) => n.kind),
    ["onlyOwnAdmins", "mutualRemoval"],
  );
  room = acknowledge(room, mutual);
  assert.deepEqual(
    shownNotices(room, notices).map((n) => n.kind),
    ["onlyOwnAdmins"],
  );
  assert.equal(applyNotices(room, [mutual], [], false, false).room.unread, false, "acknowledged: no mark");
});

test("criterion 28: the frozen room comes from the notices, acknowledged or not", () => {
  assert.deepEqual(frozenRoom([mutual]), { mutual: ["Ana", "Chris"], refused: "" });
  const own: MembershipNotice = { kind: "notApplied", author: "aa", subject: "aa", label: "Ana", removal: true };
  assert.deepEqual(frozenRoom([own]), { mutual: [], refused: "Ana" });
  assert.equal(frozenRoom([promoted]), null);
});

test("the state survives a reload and an unreadable store", () => {
  const map = new Map<string, string>();
  const store = { getItem: (k: string) => map.get(k) ?? null, setItem: (k: string, v: string) => void map.set(k, v) };
  const all = saveMembership(store, {}, "r1", { ...EMPTY_ROOM, unread: true });
  assert.equal(loadMembership(store).r1?.unread, true);
  assert.deepEqual(all, loadMembership(store));
  map.set("curvyo.share-membership", "{oops");
  assert.deepEqual(loadMembership(store), {});
  assert.deepEqual(parseNotices("nope"), []);
});

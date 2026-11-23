// `specs/0132-connector-job-queue` UX notes: the pure rules of the list (rights, selection, moves,
// drops, texts, Start's name, "List full"), with the connector's `JobList` as the only input.
import assert from "node:assert/strict";
import { test } from "node:test";

import { createLinkStore } from "../src/lib/connectorLink.ts";
import { controlsLine, countsName, heldAfterSend, ownFailureBelow } from "../src/lib/machineControls.ts";
import { jobButtons, refusalText } from "../src/lib/connectorText.ts";
import {
  type JobListMessage,
  type ListEntry,
  LIST_LIMIT,
  addedAnnouncement,
  countsLine,
  dragDecision,
  dropGap,
  armHolds,
  dropPlace,
  firstWaiting,
  focusAfterRemoval,
  movedAnnouncement,
  showsSendingRow,
  interruptedLine,
  keepSelection,
  listFull,
  moveCause,
  moveTo,
  ordered,
  rights,
  rowText,
  senderText,
  startName,
} from "../src/lib/jobList.ts";

const entry = (n: number, over: Partial<ListEntry> = {}): ListEntry => ({
  job_id: `id${n}`,
  name: `job${n}.gcode`,
  lines: 412,
  bytes: 4301,
  sent_by: "Phone",
  position: n,
  ...over,
});
const list = (waiting: ListEntry[], note: string | null = null): JobListMessage => ({
  type: "JobList",
  current: null,
  waiting,
  note,
});

test("rows say lines, size and sender, with 'this app' for the app's own job", () => {
  assert.equal(countsLine(entry(1), "Laptop"), "412 lines, 4.2 KiB, Phone");
  assert.equal(countsLine(entry(1), "Phone"), "412 lines, 4.2 KiB, this app");
  assert.equal(senderText("Laptop", "Laptop"), "this app");
  assert.equal(interruptedLine(entry(1, { interrupted_at: 1234 }), "Laptop"), "Interrupted at line 1,234, Phone");
  assert.equal(rowText(entry(2), 3), "job2.gcode, 412 lines, 4.2 KiB, from Phone, 2 of 3");
  assert.equal(rowText(entry(2, { run_next: true }), 3), "job2.gcode, 412 lines, 4.2 KiB, from Phone, 2 of 3, runs next automatically");
  assert.equal(rowText(entry(1, { interrupted_at: 1234, position: 0 }), 3), "job1.gcode, 412 lines, 4.2 KiB, from Phone, interrupted at line 1,234");
});

test("rights: an owner everything, a member the flag and Remove of its own waiting job, nothing on another's", () => {
  const own = entry(1);
  const other = entry(2, { sent_by: "Tablet" });
  const interrupted = entry(3, { interrupted_at: 5, position: 0 });
  assert.deepEqual(rights(true, "Laptop", other), { move: true, flag: true, remove: true });
  assert.deepEqual(rights(true, "Laptop", interrupted), { move: false, flag: false, remove: true });
  assert.deepEqual(rights(false, "Phone", own), { move: false, flag: true, remove: true });
  assert.deepEqual(rights(false, "Phone", other), { move: false, flag: false, remove: false });
  assert.deepEqual(rights(false, "Phone", entry(4, { interrupted_at: 5, position: 0 })), { move: false, flag: false, remove: false });
});

test("the order leaves the interrupted job out; Start takes the first waiting job and names it", () => {
  const l = list([entry(0, { interrupted_at: 9, position: 0 }), entry(1), entry(2)]);
  assert.deepEqual(ordered(l).map((e) => e.job_id), ["id1", "id2"]);
  assert.equal(firstWaiting(l)?.job_id, "id1");
  assert.equal(firstWaiting(list([entry(0, { interrupted_at: 9, position: 0 })])), null);
  assert.equal(startName(entry(1)), "Start job1.gcode");
  assert.equal(startName(entry(1, { name: `${"x".repeat(60)}.gcode` })).length, "Start ".length + 40);
});

test("a move key moves one place or to the ends; the ends give their cause", () => {
  const order = [entry(1), entry(2), entry(3)];
  assert.equal(moveTo(order, "id2", "up"), 1);
  assert.equal(moveTo(order, "id2", "down"), 3);
  assert.equal(moveTo(order, "id2", "first"), 1);
  assert.equal(moveTo(order, "id2", "last"), 3);
  assert.equal(moveTo(order, "id1", "up"), null);
  assert.equal(moveTo(order, "id3", "last"), null);
  assert.equal(moveCause(order, "id1", "up"), "First in the list");
  assert.equal(moveCause(order, "id3", "down"), "Last in the list");
  assert.equal(moveCause(order, "id2", "down"), null);
  assert.equal(moveCause(order, null, "up"), "Interrupted");
});

test("a drop lands in the gap nearest the pointer and a drop in place moves nothing", () => {
  const middles = [20, 60, 100];
  assert.equal(dropGap(middles, 5), 0);
  assert.equal(dropGap(middles, 70), 2);
  assert.equal(dropGap(middles, 500), 3);
  // Row at place 3 dropped before the first row: place 1; into its own gap: nothing.
  assert.equal(dropPlace(3, 0), 1);
  assert.equal(dropPlace(3, 2), null);
  assert.equal(dropPlace(3, 3), null);
  // Row at place 1 dropped at the end: place 3; just below itself: nothing.
  assert.equal(dropPlace(1, 3), 3);
  assert.equal(dropPlace(1, 1), null);
  assert.equal(dropPlace(1, 0), null);
});

test("the selection follows its job and, when the job leaves, the row now at its place", () => {
  const before = list([entry(1), entry(2), entry(3)]);
  assert.equal(keepSelection(before, list([entry(3), entry(1), entry(2)]), "id2"), "id2");
  assert.equal(keepSelection(before, list([entry(1), entry(3)]), "id2"), "id3");
  assert.equal(keepSelection(before, list([entry(1), entry(2)]), "id3"), "id2");
  assert.equal(keepSelection(before, list([]), "id1"), null);
  assert.equal(keepSelection(null, list([entry(1)]), null), "id1");
});

test("the list is full at 20 jobs and Send greys with its cause", () => {
  const twenty = list(Array.from({ length: LIST_LIMIT }, (_, n) => entry(n + 1)));
  assert.equal(listFull(twenty), true);
  assert.equal(listFull(list([entry(1)])), false);
  assert.equal(listFull(null), false);
});

test("Send job... is on in every held state, as UX notes section 5 lists", () => {
  for (const name of ["Receiving", "Ready", "AwaitingLocalStart", "Running", "Paused", "Idle", "Done"]) {
    const state = name === "Done" ? { name: "Done" as const, result: "Completed" as const } : ({ name } as never);
    assert.equal(jobButtons(state).send, true, name);
  }
  for (const name of ["Homing", "Alarm", "NoController"]) {
    assert.equal(jobButtons({ name } as never).send, false, name);
  }
});

test("the page's link store keeps the connector's list and drops it with the pairing", async () => {
  const handlers: ((event: { payload: unknown }) => void)[] = [];
  const store = createLinkStore({
    invoke: async () => ({ kind: "none" }) as never,
    listen: async (_event, handler) => {
      handlers.push(handler as never);
      return () => {};
    },
  });
  store.acquire("p", "addr", false);
  await Promise.resolve();
  const l = list([entry(1)], "Chain stopped: Failed");
  for (const h of handlers) h({ payload: { profileId: "p", kind: "jobList", list: l } });
  assert.deepEqual(store.get("p").jobList, l);
  for (const h of handlers) h({ payload: { profileId: "p", kind: "failed", error: { kind: "noLongerPaired" } } });
  assert.equal(store.get("p").jobList, null);
});

test("the app's own add is announced with its place among the waiting jobs, others' are not", () => {
  const before = list([entry(1), entry(2)]);
  const after = list([entry(1), entry(2), entry(3, { sent_by: "Laptop" })]);
  assert.equal(addedAnnouncement(before, after, "Laptop", true), "Added to the job list: job3.gcode, 3 of 3");
  assert.equal(addedAnnouncement(before, after, "Phone", true), null);
  assert.equal(addedAnnouncement(before, list([entry(1), entry(2), entry(3)]), "Laptop", true), null);
  // A first list read is not an add; a send into a list not yet read is.
  assert.equal(addedAnnouncement(null, after, "Laptop", false), null);
  assert.equal(addedAnnouncement(null, list([entry(1, { sent_by: "Laptop" })]), "Laptop", true), "Added to the job list: job1.gcode, 1 of 1");
  // Own jobs that were already there stay silent.
  assert.equal(addedAnnouncement(after, after, "Laptop", false), null);
});

test("a touch drag starts after a 400 ms hold, a mouse after 4 px; moving early on touch cancels", () => {
  assert.equal(dragDecision("mouse", 0, 3), "wait");
  assert.equal(dragDecision("mouse", 0, 4), "drag");
  assert.equal(dragDecision("touch", 100, 0), "wait");
  assert.equal(dragDecision("touch", 100, 10), "cancel");
  assert.equal(dragDecision("touch", 400, 0), "drag");
  assert.equal(dragDecision("touch", 400, 30), "drag");
});

test("a refusal by the list's limit reads \"List full\" (UX notes section 5)", () => {
  assert.equal(refusalText("ListFull", null), "List full");
});

test("F1: after a removal the focus goes to the row now at its place, the last when it was last", () => {
  const before = [entry(1), entry(2), entry(3)];
  assert.equal(focusAfterRemoval(before, [entry(1), entry(3)], "id2"), "id3");
  assert.equal(focusAfterRemoval(before, [entry(1), entry(2)], "id3"), "id2");
  assert.equal(focusAfterRemoval(before, [entry(2), entry(3)], "id1"), "id2");
  assert.equal(focusAfterRemoval([entry(1)], [], "id1"), null);
  assert.equal(focusAfterRemoval(before, before, "id2"), null);
});

test("F3: an armed Remove holds while its job keeps its place, not when it moves or leaves", () => {
  const a = list([entry(1), entry(2), entry(3)]);
  assert.equal(armHolds(a, list([entry(1), entry(2), entry(3)]), "id2"), true);
  assert.equal(armHolds(a, list([entry(2, { position: 1 }), entry(1, { position: 2 }), entry(3)]), "id2"), false);
  assert.equal(armHolds(a, list([entry(1), entry(3, { position: 2 })]), "id2"), false);
  assert.equal(armHolds(a, list([entry(1), entry(3, { position: 2 })]), "id3"), false);
});

test("F4: a move is announced with the job's new place", () => {
  const after = list([entry(2, { position: 1 }), entry(1, { position: 2 }), entry(3)]);
  assert.equal(movedAnnouncement(after, "id1"), "job1.gcode, 2 of 3");
  assert.equal(movedAnnouncement(after, "gone"), null);
});

const state = (name: string, job: boolean, extra: object = {}) =>
  ({ type: "Status", state: { name }, job: job ? { job_id: "run", line: 1, lines: 9, elapsed_ms: 0 } : null, position: null, alarm: null, ...extra }) as never;
const link = (status: unknown, held: unknown = null) => ({ ...createLinkStore({ invoke: () => Promise.resolve(undefined as never), listen: () => Promise.resolve(() => {}) }).get("p"), status, held }) as never;

test("F5: a footer send behind a held job stores no name; with nothing held it does", () => {
  const held = { name: "box.gcode", lines: 3, bytes: 9, jobId: null };
  assert.equal(heldAfterSend(link(state("Running", true)), held), null);
  assert.equal(heldAfterSend(link(state("Idle", false)), held), held);
});

test("F6: another app's job shows its name and sender from the status; own held name still wins", () => {
  const running = state("Running", true, { job_name: "sign.gcode", sent_by: "Phone" });
  assert.equal(countsName(link(running)), "sign.gcode");
  const own = { name: "box.gcode", lines: 3, bytes: 9, jobId: "run" };
  assert.equal(countsName(link(running, own)), "box.gcode");
  assert.equal(countsName(link(running, { ...own, jobId: "queued" })), "sign.gcode");
  assert.equal(countsName(link(state("Idle", false))), null);
});

test("F7: List full while a job is held reads under the state line, not instead of it", () => {
  const own = { sending: false, failure: { text: "List full", failure: true } };
  const running = link(state("Running", true));
  assert.equal(ownFailureBelow(running, own), "List full");
  assert.equal(controlsLine(running, own)?.text, "Running");
  assert.equal(ownFailureBelow(link(state("Idle", false)), own), null);
  assert.equal(ownFailureBelow(running, { sending: false, failure: { text: "Connector busy", failure: true } }), null);
});

test("F8: the Sending row shows only while another job is held", () => {
  assert.equal(showsSendingRow(state("Running", true)), true);
  assert.equal(showsSendingRow(state("Idle", false)), false);
  assert.equal(showsSendingRow(state("Receiving", true)), false);
  assert.equal(showsSendingRow(null), false);
});

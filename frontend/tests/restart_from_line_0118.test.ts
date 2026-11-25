// `specs/0118` slice 2e (ux-notes W65 to W78; `specs/0132` criterion 12; Fragen 390 C, 391 C):
// the interrupted row's lines, Restart's names and causes, the confirm's question and when it
// closes, and the state lines of a restart.
import assert from "node:assert/strict";
import { test } from "node:test";

import { createLinkStore } from "../src/lib/connectorLink.ts";
import { jobStateLine } from "../src/lib/connectorText.ts";
import {
  type ListEntry,
  confirmHolds,
  countsLine,
  interruptedLine,
  restartAt,
  restartLabel,
  restartName,
  restartQuestion,
  rights,
  rowText,
  tabStop,
} from "../src/lib/jobList.ts";
import { controlsLine, restartCause } from "../src/lib/machineControls.ts";

const row = (over: Partial<ListEntry> = {}): ListEntry => ({
  job_id: "sign",
  name: "sign.gcode",
  lines: 4000,
  bytes: 4301,
  sent_by: "Phone",
  position: 0,
  interrupted_at: 1249,
  restart_line: 1234,
  ...over,
});

const status = (name: string, job: object | null = null, extra: object = {}) =>
  ({ type: "Status", state: { name }, job, position: null, alarm: null, ...extra }) as never;
const link = (s: unknown, extra: object = {}) =>
  ({
    ...createLinkStore({ invoke: () => Promise.resolve(undefined as never), listen: () => Promise.resolve(() => {}) }).get("p"),
    status: s,
    ...extra,
  }) as never;

test("W65, W66: the row's three lines name the restart line, near after a crash", () => {
  assert.equal(restartAt(row()), 1234);
  assert.equal(interruptedLine(row()), "Interrupted at line 1,234");
  assert.equal(interruptedLine(row({ line_approximate: true })), "Interrupted near line 1,234");
  assert.equal(countsLine(row(), "Laptop"), "4,000 lines, 4.2 KiB, Phone");
  assert.equal(rowText(row(), 2), "sign.gcode, 4,000 lines, 4.2 KiB, from Phone, interrupted at line 1,234 of 4,000");
  assert.equal(
    rowText(row({ line_approximate: true }), 2),
    "sign.gcode, 4,000 lines, 4.2 KiB, from Phone, interrupted near line 1,234 of 4,000",
  );
  // A row that offers no line shows the stored one and has no Restart.
  assert.equal(restartAt(row({ restart_line: null })), 1249);
});

test("W67, W75: Restart is an owner's, named with the job and the line; a member has none", () => {
  assert.equal(rights(true, "Laptop", row()).restart, true);
  assert.equal(rights(false, "Phone", row()).restart, false);
  assert.equal(rights(true, "Laptop", row({ restart_line: null })).restart, false);
  assert.equal(rights(true, "Laptop", row({ interrupted_at: null, position: 1 })).restart, false);
  assert.equal(restartLabel(row()), "Restart from line 1,234");
  assert.equal(restartName(row()), "Restart sign.gcode from line 1,234");
});

test("W68: the app's own causes first, then the connector's; a wait reads in the app's words", () => {
  const idle = status("Idle");
  assert.equal(restartCause(link(idle, { lost: true }), row()), "Connection lost");
  assert.equal(restartCause(link(status("NoController", null, { detail: "Port /dev/ttyUSB0 not found" })), row()), "Port /dev/ttyUSB0 not found");
  assert.equal(restartCause(link(idle), row()), null);
  assert.equal(restartCause(link(idle), row({ restart_cause: "Not homed" })), "Not homed");
  assert.equal(restartCause(link(idle), row({ restart_cause: "Work offset changed" })), "Work offset changed");
  const waiting = status("AwaitingLocalStart", { job_id: "x", line: 0, lines: 9, elapsed_ms: 0 });
  assert.equal(restartCause(link(waiting), row({ restart_cause: "Waiting for the start at the machine" })), "Waiting for the start at the machine");
  assert.equal(
    restartCause(link(waiting), row({ restart_cause: "Starts at the head position" })),
    "Starts at the head position",
    "the job text comes before the state",
  );
});

test("W69, Frage 391 C: the question without the material sentence; it closes when the facts change", () => {
  assert.equal(restartQuestion(row()), "Restart sign.gcode from line 1,234 of 4,000?");
  const long = row({ name: `${"a".repeat(45)}.gcode` });
  assert.equal(restartQuestion(long), `Restart ${"a".repeat(37)}... from line 1,234 of 4,000?`);
  assert.equal(confirmHolds(row(), 1234, null), true);
  assert.equal(confirmHolds(undefined, 1234, null), false, "the row left");
  assert.equal(confirmHolds(row({ restart_line: 1200 }), 1234, null), false, "its line changed");
  assert.equal(confirmHolds(row(), 1234, "Not homed"), false, "a cause applies");
});

test("W73: moving to the line, running from it, completed from it", () => {
  const job = (extra: object) => ({ job_id: "sign", line: 1233, lines: 4000, elapsed_ms: 0, from_line: 1234, ...extra });
  assert.equal(jobStateLine(status("Running", job({ entering: true })), null)?.text, "Moving to line 1,234");
  assert.equal(jobStateLine(status("Running", job({})), null)?.text, "Running from line 1,234");
  assert.equal(jobStateLine(status("Paused", job({})), null)?.text, "Paused");
  assert.equal(
    jobStateLine(status("Done", job({ line: 4000 }), { state: { name: "Done", result: "Completed" } }), null)?.text,
    "Completed, lines 1,234 to 4,000",
  );
  assert.equal(jobStateLine(status("Running", { job_id: "j", line: 3, lines: 9, elapsed_ms: 0 }), null)?.text, "Running");
});

test("W74: another app reads a restart that ended before its line muted, not as a failure", () => {
  const line = controlsLine(link(status("Idle", null, { detail: "Not started: Restart from line 1,234" })));
  assert.equal(line?.text, "Not started: Restart from line 1,234");
  assert.equal(line?.failure, false);
  assert.equal(line?.muted, true);
});

test("W74: the app that pressed Stop does not show the muted line, another app does", () => {
  const store = createLinkStore({ invoke: () => Promise.resolve(undefined as never), listen: () => Promise.resolve(() => {}) });
  const ended = status("Idle", null, { detail: "Not started: Restart from line 1,234" }) as never;
  assert.equal(store.claim("a", "stop"), true);
  store.setStatus("a", ended);
  store.release("a");
  assert.equal(controlsLine(store.get("a")), null);
  store.setStatus("b", ended);
  assert.equal(controlsLine(store.get("b"))?.muted, true);
});

test("the job grid has one Tab stop: the focused row, else the selected one", () => {
  const rows = [row({ job_id: "sign", interrupted_at: 1249 }), row({ job_id: "a", interrupted_at: null }), row({ job_id: "b", interrupted_at: null })];
  assert.equal(tabStop(rows, "b", "a", "sign"), "sign");
  assert.equal(tabStop(rows, "b", "a", null), "b");
  assert.equal(tabStop(rows, null, "a", "gone"), "a");
  assert.equal(tabStop([], null, null, null), null);
});

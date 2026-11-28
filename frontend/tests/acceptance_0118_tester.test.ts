// Independent acceptance tests for `specs/0118-machine-control` first slice, criteria 1, 2, 3, 4, 5
// (the browser-free part): written from `specification.md` and `ux-notes.md` (sections 3 to 9, W1
// to W14) before reading the implementation. The suite runs under `node --test` without a DOM, so
// what is rule-shaped is tested through the pure modules, and what is layout-shaped (order of the
// toolbar, the footer outside the scroll, nothing job-like left in the Machine block or the Output
// group) is checked on the component sources. The rendered result is checked separately in the
// real desktop app.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import type { LinkEntry } from "../src/lib/connectorLink.ts";
import { SETTLES, afterHoming, controlsLine, footerNotice, homeButton } from "../src/lib/machineControls.ts";
import {
  type JobState,
  type StatusMessage,
  jobButtons,
  jobLineAlerts,
  jobStateLine,
} from "../src/lib/connectorText.ts";

const JOB = "11111111-1111-4111-8111-111111111111";

function entry(over: Partial<LinkEntry> = {}): LinkEntry {
  return {
    link: "paired",
    paired: { connectorName: "navi", fingerprint: "AA" },
    noStore: false,
    keyChanged: null,
    status: null,
    controller: null,
    lost: false,
    held: null,
    pageJobId: null,
    sendingName: null,
    pageSend: null,
    sendFailure: null,
    busy: null,
    reading: false,
    readError: null,
    ...over,
  } as LinkEntry;
}

const st = (
  state: JobState,
  extra: { job?: boolean; alarm?: string | null; detail?: string | null } = {},
): StatusMessage => ({
  type: "Status",
  state,
  job: extra.job ? { job_id: JOB, line: 0, lines: 6, elapsed_ms: 0 } : null,
  position: null,
  alarm: extra.alarm ?? null,
  detail: extra.detail ?? null,
});

const src = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");
const TOOLBAR = src("../src/components/panel/MachineControls.tsx");
const MACHINE_BLOCK = [
  src("../src/components/workbench/ConnectorGroup.tsx"),
  // ADR 0098: the laser's Connection group lives in the Machines tab.
  src("../src/components/workbench/MachinesTab.tsx"),
].join("\n");
const OUTPUT_GROUP = src("../src/components/panel/OutputGroup.tsx");
const PANEL = src("../src/components/panel/PropertiesPanel.tsx");

// ---------------------------------------------------------------------------------------------
// Criterion 3 and 5: when Home is pressable, and its cause
// ---------------------------------------------------------------------------------------------

test("W13 / c3: Home is enabled in Idle, Ready, Done (every result) and Alarm, with no cause", () => {
  const states: JobState[] = [
    { name: "Idle" },
    { name: "Ready" },
    { name: "Done", result: "Completed" },
    { name: "Done", result: "Stopped" },
    { name: "Done", result: "Failed" },
    { name: "Alarm" },
  ];
  for (const state of states) {
    const home = homeButton(entry({ status: st(state, { alarm: state.name === "Alarm" ? "Locked, not homed" : null }) }));
    assert.deepEqual(home, { enabled: true, busy: false, cause: null }, JSON.stringify(state));
  }
});

test("c5: Home is disabled with the words of the notes in every other state", () => {
  const cases: [JobState, string][] = [
    [{ name: "Running" }, "A job is running"],
    [{ name: "Paused" }, "A job is running"],
    [{ name: "AwaitingLocalStart" }, "Waiting for the start at the machine"],
    [{ name: "Receiving" }, "Sending"],
    [{ name: "Homing" }, "Homing"],
  ];
  for (const [state, cause] of cases) {
    const home = homeButton(entry({ status: st(state, { job: true }) }));
    assert.equal(home.enabled, false, state.name);
    assert.equal(home.cause, cause, state.name);
  }
  // Homing is the busy look, not just off
  assert.equal(homeButton(entry({ status: st({ name: "Homing" }) })).busy, true);
});

test("section 4 table: NoController names its own text, lost, unread and the read's failure", () => {
  assert.equal(homeButton(entry({ status: st({ name: "NoController" }) })).cause, "No controller");
  assert.equal(
    homeButton(entry({ status: st({ name: "NoController" }, { detail: "Port /dev/ttyUSB0 not found" }) })).cause,
    "Port /dev/ttyUSB0 not found",
  );
  assert.equal(homeButton(entry({ status: st({ name: "Idle" }), lost: true })).cause, "Connection lost");
  assert.equal(homeButton(entry({ status: null, reading: true })).cause, "Testing");
  const none = homeButton(entry({ status: null, readError: { kind: "timedOut" } as never }));
  assert.equal(none.enabled, false);
  assert.ok(none.cause !== null && none.cause.length > 0);
});

test("section 4: the first cause wins and a waiting sibling button has no cause", () => {
  // Running AND another button waiting: the state's own cause wins
  assert.equal(homeButton(entry({ status: st({ name: "Running" }, { job: true }), busy: "stop" })).cause, "A job is running");
  // Idle, a sibling waits: off without a cause
  assert.deepEqual(homeButton(entry({ status: st({ name: "Idle" }), busy: "unlock" })), {
    enabled: false,
    busy: false,
    cause: null,
  });
  // the press itself: busy
  assert.equal(homeButton(entry({ status: st({ name: "Idle" }), busy: "home" })).busy, true);
});

test("c3: Home never enabled while the connector reports Door, Check or Sleep (refused by the connector)", () => {
  // The connector answers WrongState there ("Controller state: Door."): a button that invites a press
  // it will refuse is a finding, not a pass.
  for (const word of ["Door", "Check", "Sleep"]) {
    const home = homeButton(entry({ status: st({ name: "Idle" }, { detail: `Controller state: ${word}` }) }));
    assert.equal(home.enabled, false, `Home enabled in ${word}`);
  }
});

// ---------------------------------------------------------------------------------------------
// Criterion 5 / 1: the other buttons keep their rules; Stop stays enabled while homing
// ---------------------------------------------------------------------------------------------

test("c5: during Homing Send, Start and Pause are off and Stop is on; Running/Paused keep Stop", () => {
  const homing = jobButtons({ name: "Homing" });
  assert.deepEqual(
    { send: homing.send, start: homing.start, pause: homing.pause, stop: homing.stop },
    { send: false, start: false, pause: false, stop: true },
  );
  assert.equal(jobButtons({ name: "Running" }).stop, true);
  assert.equal(jobButtons({ name: "Paused" }).stop, true);
  assert.equal(jobButtons({ name: "Paused" }).startName, "Resume");
  assert.equal(jobButtons({ name: "Ready" }).start, true);
  assert.equal(jobButtons({ name: "Ready" }).stop, true);
  assert.equal(jobButtons({ name: "AwaitingLocalStart" }).stop, true);
  // after a Ready job's Home, Start is available again once the status is Ready
  assert.equal(jobButtons({ name: "Idle" }).start, false);
});

test("c5: Stop never settles early while homing (it waits for the end of Homing)", () => {
  assert.equal(SETTLES.stop.done(st({ name: "Homing" })), false);
  assert.equal(SETTLES.stop.done(st({ name: "Running" })), false);
  assert.equal(SETTLES.stop.done(st({ name: "Alarm" })), true);
  assert.equal(SETTLES.stop.done(st({ name: "Idle" })), true);
});

test("W12: Home has no time limit for homing itself (the press settles with the answer only)", () => {
  // The answer of Home comes at once: the busy look then follows the status (Homing), not a timer.
  assert.equal(SETTLES.home.ms, 0);
});

// ---------------------------------------------------------------------------------------------
// Criteria 3, 4: the state line and its words
// ---------------------------------------------------------------------------------------------

test("c3: the state line reads Homing, with or without a held job, neutral", () => {
  const plain = controlsLine(entry({ status: st({ name: "Homing" }) }));
  assert.equal(plain?.text, "Homing");
  assert.equal(plain?.failure, false);
  assert.equal(plain?.alarm, false);
  const held = controlsLine(
    entry({
      status: st({ name: "Homing" }, { job: true }),
      held: { name: "cut.gcode", lines: 6, bytes: 120, jobId: JOB },
    }),
  );
  assert.equal(held?.text, "Homing");
  assert.equal(held?.failure, false);
});

test("c3: after homing the line returns to the Ready line (job kept) or leaves (Idle)", () => {
  const held = { name: "cut.gcode", lines: 6, bytes: 120, jobId: JOB };
  const ready = controlsLine(entry({ status: st({ name: "Ready" }, { job: true }), held }));
  assert.match(ready?.text ?? "", /^Ready: cut\.gcode, 6 lines, /);
  assert.equal(controlsLine(entry({ status: st({ name: "Idle" }) })), null);
});

test("c4: the failure words of homing, as built texts, in alarm or failure looks", () => {
  const a9 = controlsLine(entry({ status: st({ name: "Alarm" }, { alarm: "Homing did not find the switch (alarm 9)" }) }));
  assert.equal(a9?.text, "Alarm: Homing did not find the switch (alarm 9)");
  assert.equal(a9?.alarm, true);
  assert.equal(a9?.alerts, true);
  const a8 = controlsLine(entry({ status: st({ name: "Alarm" }, { alarm: "Homing pull-off did not clear the switch (alarm 8)" }) }));
  assert.equal(a8?.text, "Alarm: Homing pull-off did not clear the switch (alarm 8)");
  const a6 = controlsLine(entry({ status: st({ name: "Alarm" }, { alarm: "Reset during homing (alarm 6)" }) }));
  assert.equal(a6?.text, "Alarm: Reset during homing (alarm 6)");
  // $22=0: error 5, Idle (and Ready with a kept job), failure look, an alert (answers the press)
  for (const state of [{ name: "Idle" }, { name: "Ready" }] as JobState[]) {
    const e5 = controlsLine(entry({ status: st(state, { detail: "Homing is not enabled (error 5)", job: state.name === "Ready" }) }));
    assert.equal(e5?.text, "Homing is not enabled (error 5)", state.name);
    assert.equal(e5?.failure, true);
    assert.equal(e5?.alarm, false);
    assert.equal(e5?.alerts, true);
  }
  assert.equal(jobLineAlerts(st({ name: "Idle" }, { detail: "Homing is not enabled (error 5)" })), true);
  // the controller's own state word is a state, never a failure line
  assert.equal(controlsLine(entry({ status: st({ name: "Idle" }, { detail: "Controller state: Door" }) })), null);
});

test("c4: every homing failure text is at most 60 characters (UX section 5)", () => {
  for (const t of [
    "Alarm: Homing did not find the switch (alarm 9)",
    "Alarm: Homing pull-off did not clear the switch (alarm 8)",
    "Alarm: Door opened during homing (alarm 7)",
    "Alarm: Reset during homing (alarm 6)",
    "Homing is not enabled (error 5)",
  ]) {
    assert.ok(t.length <= 60, t);
  }
});

test("W7: after homing the hidden status says the next state once, and nothing mid-homing", () => {
  const held = { name: "cut.gcode", lines: 6, bytes: 120, jobId: JOB };
  assert.equal(afterHoming(st({ name: "Homing" }), st({ name: "Idle" }), null), "Idle");
  assert.match(afterHoming(st({ name: "Homing" }, { job: true }), st({ name: "Ready" }, { job: true }), held) ?? "", /^Ready: cut\.gcode/);
  assert.equal(afterHoming(st({ name: "Homing" }), st({ name: "Homing" }), null), null);
  assert.equal(afterHoming(st({ name: "Idle" }), st({ name: "Idle" }), null), null);
  assert.equal(afterHoming(null, st({ name: "Idle" }), null), null);
});

// ---------------------------------------------------------------------------------------------
// Frage 262 A: the one line carries every result of a Send to machine press
// ---------------------------------------------------------------------------------------------

test("262 A: Sending, the failures of a send and the Ready line read in the footer's line", () => {
  assert.equal(controlsLine(entry({ pageSend: { token: 1, name: "plate.gcode" } }))?.text.startsWith("Sending "), true);
  assert.equal(
    controlsLine(entry({ sendFailure: { text: "Laser mode is off ($32=0)", atState: "Idle" } }))?.text,
    "Laser mode is off ($32=0)",
  );
  for (const t of ["Connector busy", "Nothing answers at 127.0.0.1:7447", "Timed out", "Not received"]) {
    const line = controlsLine(entry({ sendFailure: { text: t, atState: "Idle" } }));
    assert.equal(line?.text, t);
    assert.equal(line?.failure, true);
    assert.equal(line?.alerts, true);
  }
  assert.match(
    controlsLine(entry({ status: st({ name: "Ready" }, { job: true }), held: { name: "plate.gcode", lines: 6, bytes: 1200, jobId: JOB } }))?.text ?? "",
    /^Ready: plate\.gcode, 6 lines, /,
  );
});

test("c1: the footer's job line has the 0064 words for the old states", () => {
  const line = (state: JobState, extra = {}) => jobStateLine(st(state, extra), null)?.text;
  assert.equal(line({ name: "Running" }), "Running");
  assert.equal(line({ name: "Paused" }), "Paused");
  assert.equal(line({ name: "AwaitingLocalStart" }), "Waiting for the start at the machine");
  assert.equal(line({ name: "NoController" }), "No controller");
  assert.equal(line({ name: "Alarm" }, { alarm: "Locked, not homed" }), "Alarm: Locked, not homed");
});

// ---------------------------------------------------------------------------------------------
// Criterion 1: unpaired and unavailable states of the footer (section 8)
// ---------------------------------------------------------------------------------------------

test("section 8: browser, not paired, no longer paired, key changed, no store, paired", () => {
  assert.deepEqual(footerNotice(entry(), false), { text: "Needs the desktop app", failure: false });
  assert.deepEqual(footerNotice(entry({ link: "not_paired", paired: null }), true), { text: "Not paired", failure: false });
  assert.deepEqual(footerNotice(entry({ link: "no_longer_paired", paired: null }), true), { text: "No longer paired", failure: false });
  assert.deepEqual(footerNotice(entry({ link: "key_changed" }), true), { text: "This connector's key changed", failure: true });
  assert.deepEqual(footerNotice(entry({ link: "not_paired", noStore: true, paired: null }), true), {
    text: "No credential store on this computer",
    failure: true,
  });
  assert.equal(footerNotice(entry(), true), null);
});

// ---------------------------------------------------------------------------------------------
// Criterion 1 and W1 to W5, W11, section 10, 11: layout facts on the component sources
// ---------------------------------------------------------------------------------------------

// Amended in place by slice 2a (`specs/0118` criterion 15, UX notes W3): Jog joins Home's group.
test("W3 / c1: the toolbar order is Home, Jog, Send job..., Start / Resume, Pause, Stop with the notes' glyphs", () => {
  const keys = [...TOOLBAR.matchAll(/key: "(\w+)"/g)].map((m) => m[1]);
  assert.deepEqual(keys.slice(0, 6), ["home", "jog", "send", "start", "pause", "stop"]);
  assert.match(TOOLBAR, /name: home\.name \?\? "Home"[^\n]*<House /);
  assert.match(TOOLBAR, /name: "Send job\.\.\."[^\n]*<FileUp /);
  assert.match(TOOLBAR, /<Play /);
  assert.match(TOOLBAR, /name: "Pause"[^\n]*<Pause /);
  assert.match(TOOLBAR, /name: "Stop"[^\n]*<Square /);
  assert.match(TOOLBAR, /LockOpen/);
  // W5: Send job... is no longer the Upload glyph of Send to machine
  assert.doesNotMatch(TOOLBAR, /\bUpload\b/);
});

test("section 11: the footer is a section named by its heading, the toolbar is named Machine", () => {
  assert.match(TOOLBAR, /<section aria-labelledby=/);
  assert.match(TOOLBAR, />\s*Machine controls\s*</);
  assert.match(TOOLBAR, /role="toolbar" aria-label="Machine"/);
  assert.match(TOOLBAR, /Unlock/);
});

test("W1: the footer sits in the panel after, and outside, the scrolling column and cannot shrink", () => {
  const footer = PANEL.indexOf("<MachineControls");
  const aside = PANEL.lastIndexOf("</TabsContent>", footer);
  assert.ok(footer > 0 && aside > 0 && aside < footer, "MachineControls must come after the tab content");
  assert.match(TOOLBAR, /data-machine-controls[^>]*shrink-0/);
  // only on the Operations tab, only while the profile names a connector (absent otherwise)
  const block = PANEL.slice(PANEL.lastIndexOf("{!aboutActive", footer), footer);
  assert.match(block, /doc\.view\.content === "operations"/);
  assert.match(block, /output\.view\?\.send/);
});

test("W2: what comes and goes sits above the toolbar, the toolbar is the last row of the footer", () => {
  const body = TOOLBAR.slice(TOOLBAR.indexOf("data-machine-controls"));
  const order = ["<Warnings", "<JobBlock", "<Toolbar"].map((tag) => body.indexOf(tag));
  assert.ok(order.every((i) => i > 0), `missing parts: ${order}`);
  assert.deepEqual([...order].sort((a, b) => a - b), order, "heading, warning, job block, toolbar in this order");
});

test("section 10: no global chord, Escape never sends Stop or Pause, no hint text", () => {
  const hook = src("../src/hooks/useMachineControls.ts");
  for (const text of [TOOLBAR, hook]) {
    assert.doesNotMatch(text, /key === "Escape"|"Escape"/);
    assert.doesNotMatch(text, /addEventListener\("keydown"/);
  }
  // no operating hints: no "Press", "Tip", "Hint" strings in the footer
  assert.doesNotMatch(TOOLBAR, />\s*(Press|Tip|Hint|Click)\b/);
});

test("c2: the Machine block holds no job button, state line, progress bar or Unlock", () => {
  for (const word of ["jobButtons", "jobStateLine", "controlsLine", 'role="toolbar"', "Job progress", "aria-valuenow", "Send job", "LockOpen"]) {
    assert.ok(!MACHINE_BLOCK.includes(word), `Machine block still contains ${word}`);
  }
  // what stays
  for (const word of ["Test connection", "Forget pairing", "Pair"]) {
    assert.ok(MACHINE_BLOCK.includes(word), `Machine block lost ${word}`);
  }
});

test("262 A: the Output group has no state line, no progress bar and no connector status", () => {
  for (const word of ["Job progress", "aria-valuenow", "useConnector", "controlsLine", "jobStateLine", "sendLine", "stateLine"]) {
    assert.ok(!OUTPUT_GROUP.includes(word), `Output group still contains ${word}`);
  }
  // the Output group keeps Send to machine itself
  assert.match(OUTPUT_GROUP, /<SendToMachine/);
});

test("W10 / W11: the Machine block's Controller block names a NoController cause, focus after pairing", () => {
  const group = src("../src/components/workbench/ConnectorGroup.tsx");
  assert.match(group, /NoController/);
  assert.doesNotMatch(group, /Send job/);
});

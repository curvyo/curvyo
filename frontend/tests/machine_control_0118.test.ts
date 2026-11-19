// `specs/0118-machine-control`, first slice: Home and the Machine controls footer of the Operations
// tab (criteria 1 to 5, 7; UX notes sections 3 to 9; architect addendum M9, M11): the Homing state
// in the job texts, Home's causes, the one state line, the footer's unpaired lines, the shared busy
// press of the link store, and where the controls live.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { type LinkEntry, type LinkHost, createLinkStore } from "../src/lib/connectorLink.ts";
import {
  type JobState,
  type StatusMessage,
  jobButtons,
  jobIsActive,
  jobLineAlerts,
  jobStateLine,
} from "../src/lib/connectorText.ts";
import { SETTLES, afterHoming, controlsLine, countsName, footerNotice, homeButton } from "../src/lib/machineControls.ts";

const JOB = "11111111-1111-4111-8111-111111111111";

const status = (state: JobState, extra: Partial<StatusMessage> = {}): StatusMessage => ({
  type: "Status",
  state,
  job: null,
  position: null,
  alarm: null,
  ...extra,
});

const host = (answers: Record<string, unknown> = {}): LinkHost => ({
  invoke: (command) => Promise.resolve(answers[command] as never),
  listen: () => Promise.resolve(() => undefined),
});

const base = (): LinkEntry => createLinkStore(host()).get("x");
const entry = (s: StatusMessage | null, extra: Partial<LinkEntry> = {}): LinkEntry => ({ ...base(), link: "paired", status: s, ...extra });

const source = (file: string) =>
  readFileSync(new URL(`../src/${file}`, import.meta.url), "utf8")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/\/\/.*$/gm, "");

test("criterion 3, M9: Homing is a state of its own: only Stop applies, it counts as active", () => {
  assert.deepEqual(jobButtons({ name: "Homing" }), { send: false, start: false, startName: "Start", pause: false, stop: true });
  assert.equal(jobIsActive({ name: "Homing" }), true, "a lost connection while homing reads Connection lost");
  assert.deepEqual(jobStateLine(status({ name: "Homing" }), null), { text: "Homing", failure: false });
  const held = { name: "coaster.gcode", lines: 6, bytes: 52 };
  const homing = status({ name: "Homing" }, { job: { job_id: JOB, line: 0, lines: 6, elapsed_ms: 0 } });
  const line = controlsLine(entry(homing, { held: { ...held, jobId: JOB } }));
  assert.equal(line?.text, "Homing");
  assert.equal(line?.bar, null, "no bar while homing");
  assert.equal(line?.alerts, false, "polite");
});

test("criterion 4: a refused homing names its cause and is an alert; the controller's word is not one", () => {
  const text = "Homing is not enabled (error 5)";
  for (const name of ["Idle", "Ready"] as const) {
    const s = status({ name }, { detail: text });
    assert.deepEqual(jobStateLine(s, null), { text, failure: true }, name);
    assert.equal(jobLineAlerts(s), true, name);
  }
  const door = status({ name: "Idle" }, { detail: "Controller state: Door" });
  assert.equal(jobStateLine(door, null), null);
  assert.equal(jobLineAlerts(door), false);
  const alarm = status({ name: "Alarm" }, { alarm: "Homing did not find the switch (alarm 9)" });
  assert.equal(controlsLine(entry(alarm))?.text, "Alarm: Homing did not find the switch (alarm 9)");
  assert.equal(controlsLine(entry(alarm))?.alarm, true);
  assert.equal(
    controlsLine(entry(status({ name: "Alarm" }, { alarm: "Reset during homing (alarm 6)" })))?.text,
    "Alarm: Reset during homing (alarm 6)",
  );
});

test("criteria 3, 5, UX §4: Home is enabled in Idle, Ready, Done and Alarm, otherwise greyed with its cause", () => {
  for (const state of [{ name: "Idle" }, { name: "Ready" }, { name: "Alarm" }, { name: "Done", result: "Stopped" }] as JobState[]) {
    assert.deepEqual(homeButton(entry(status(state))), { enabled: true, busy: false, cause: null }, state.name);
  }
  const off = (e: LinkEntry) => {
    const home = homeButton(e);
    assert.equal(home.enabled, false);
    return home.cause;
  };
  assert.deepEqual(homeButton(entry(status({ name: "Homing" }))), { enabled: false, busy: true, cause: "Homing" });
  assert.deepEqual(homeButton(entry(status({ name: "Idle" }), { busy: "home" })), { enabled: false, busy: true, cause: "Homing" });
  assert.equal(off(entry(status({ name: "Running" }))), "A job is running");
  assert.equal(off(entry(status({ name: "Paused" }))), "A job is running");
  assert.equal(off(entry(status({ name: "AwaitingLocalStart" }))), "Waiting for the start at the machine");
  assert.equal(off(entry(status({ name: "Receiving" }))), "Sending");
  assert.equal(off(entry(status({ name: "NoController" }))), "No controller");
  assert.equal(off(entry(status({ name: "NoController" }, { detail: "Port /dev/ttyUSB0 not found" }))), "Port /dev/ttyUSB0 not found");
  assert.equal(off(entry(status({ name: "Ready" }), { lost: true })), "Connection lost");
  assert.equal(off(entry(null, { reading: true })), "Testing");
  assert.equal(off(entry(null, { readError: { kind: "unreachable", address: "10.0.0.5:7447" } })), "Nothing answers at 10.0.0.5:7447");
  assert.equal(off(entry(null, { readError: { kind: "timedOut" } })), "Timed out");
  assert.equal(off(entry(status({ name: "Idle" }), { busy: "start" })), null, "another press waits: greyed, no cause");
});

test("M11: Home's busy ends with the connector's answer, so Stop stays pressable while homing", () => {
  assert.equal(SETTLES.home.ms, 0);
  assert.equal(SETTLES.home.done(null), true);
  assert.equal(jobButtons({ name: "Homing" }).stop, true);
  // A Stop during homing waits until the homing ended (fake: Idle or Ready; GRBL: alarm 6).
  assert.equal(SETTLES.stop.done(status({ name: "Homing" })), false);
  assert.equal(SETTLES.stop.done(status({ name: "Alarm" })), true);
});

test("M11: one busy press per profile, shared by the Machine block and the footer", () => {
  const store = createLinkStore(host());
  assert.equal(store.claim("p", "home"), true);
  assert.equal(store.get("p").busy, "home");
  assert.equal(store.claim("p", "test"), false, "Test connection waits while Home waits");
  assert.equal(store.get("p").busy, "home");
  assert.equal(store.claim("q", "stop"), true, "another profile is free");
  store.release("p");
  assert.equal(store.get("p").busy, null);
  assert.equal(store.claim("p", "test"), true);
});

test("M11: a status read is marked while it runs and keeps why it failed", async () => {
  const unreachable = { kind: "failed", error: { kind: "unreachable", address: "a:1" } };
  const store = createLinkStore(host({ connector_test: unreachable }));
  const pending = store.read("p", "a");
  assert.equal(store.get("p").reading, true);
  await pending;
  assert.equal(store.get("p").reading, false);
  assert.deepEqual(store.get("p").readError, { kind: "unreachable", address: "a:1" });
});

test("Frage 262 A: the footer's line carries a Send to machine press, then the connector's job", () => {
  const sending = entry(status({ name: "Idle" }), { pageSend: { token: 1, name: "Coaster" } });
  assert.equal(controlsLine(sending)?.text, "Sending Coaster");
  const refused = entry(status({ name: "Idle" }), { sendFailure: { text: "Laser mode is off ($32=0)", atState: "Idle" } });
  assert.deepEqual(controlsLine(refused), { text: "Laser mode is off ($32=0)", failure: true, alarm: false, alerts: true, bar: null });
  const held = { name: "Coaster", lines: 412, bytes: 9421, jobId: JOB };
  const ready = status({ name: "Ready" }, { job: { job_id: JOB, line: 0, lines: 412, elapsed_ms: 0 } });
  assert.equal(controlsLine(entry(ready, { held }))?.text, "Ready: Coaster, 412 lines, 9.2 KiB");
  const own = { sending: false, failure: { text: "Connector busy", failure: true } };
  assert.equal(controlsLine(entry(ready, { held }), own)?.text, "Connector busy", "the footer's own refusal first");
  assert.equal(controlsLine(entry(ready, { held }), own)?.alerts, true);
});

test("W7: after homing the next state word is said once", () => {
  const homing = status({ name: "Homing" });
  assert.equal(afterHoming(homing, status({ name: "Idle" }), null), "Idle");
  const held = { name: "Coaster", lines: 412, bytes: 9421, jobId: JOB };
  assert.equal(afterHoming(homing, status({ name: "Ready" }), held), "Ready: Coaster, 412 lines, 9.2 KiB");
  assert.equal(afterHoming(homing, homing, null), null);
  assert.equal(afterHoming(status({ name: "Idle" }), status({ name: "Idle" }), null), null);
  assert.equal(afterHoming(homing, status({ name: "Alarm" }), null), null, "the alarm line is its own alert");
});

test("criterion 1, UX §8: the footer's line while the toolbar cannot apply", () => {
  assert.deepEqual(footerNotice(base(), false), { text: "Needs the desktop app", failure: false });
  assert.deepEqual(footerNotice({ ...base(), link: "not_paired" }, true), { text: "Not paired", failure: false });
  assert.deepEqual(footerNotice({ ...base(), link: "no_longer_paired" }, true), { text: "No longer paired", failure: false });
  assert.deepEqual(footerNotice({ ...base(), link: "key_changed" }, true), { text: "This connector's key changed", failure: true });
  assert.deepEqual(footerNotice({ ...base(), noStore: true }, true), { text: "No credential store on this computer", failure: true });
  assert.equal(footerNotice({ ...base(), link: "paired" }, true), null, "paired: the toolbar");
});

test("criteria 1, 2: the job buttons, state line and bar live in the Operations tab's footer only", () => {
  const group = source("components/workbench/ConnectorGroup.tsx");
  for (const gone of ["Send job...", 'role="toolbar"', "progressbar", '"Pause"', '"Stop"', "Unlock", "controller_warnings"]) {
    assert.ok(!group.includes(gone), `the Machine block holds no ${gone}`);
  }
  for (const kept of ["Test connection", "Forget pairing", '"Pair"', "Paired with"]) {
    assert.ok(group.includes(kept), `the Machine block keeps ${kept}`);
  }
  const output = source("components/panel/OutputGroup.tsx");
  assert.ok(!output.includes("progressbar") && !output.includes("data-send-line"), "the Output group's line and bar are gone");
  const footer = source("components/panel/MachineControls.tsx");
  assert.ok(footer.includes('aria-label="Machine"') && footer.includes("Machine controls"));
  assert.ok(footer.includes("<House") && footer.includes("<FileUp"), "Home is House, Send job... is FileUp (W5)");
  const panel = source("components/panel/PropertiesPanel.tsx");
  assert.equal(panel.split("<MachineControls").length - 1, 1, "rendered once");
  assert.ok(/content === "operations" && output\.view\?\.send/.test(panel), "in the Operations tab, with a connector address");
});

test("UX §4 row 7 (W15): Home is greyed with the controller's state while GRBL reports Door, Check or Sleep", () => {
  for (const name of ["Idle", "Ready", "Done"] as const) {
    for (const word of ["Door", "Check", "Sleep"]) {
      const state = (name === "Done" ? { name, result: "Completed" } : { name }) as JobState;
      const home = homeButton(entry(status(state, { detail: `Controller state: ${word}` })));
      assert.deepEqual(home, { enabled: false, busy: false, cause: `Controller state: ${word}` }, `${name} ${word}`);
    }
  }
  // The connector homes from Alarm whatever GRBL's word, and Jog or Home words refuse nothing.
  assert.equal(homeButton(entry(status({ name: "Alarm" }, { alarm: "Locked, not homed", detail: "Controller state: Door" }))).enabled, true);
  assert.equal(homeButton(entry(status({ name: "Idle" }, { detail: "Controller state: Jog" }))).enabled, true);
  // A job's own cause still wins.
  assert.equal(homeButton(entry(status({ name: "Running" }, { detail: "Controller state: Door" }))).cause, "A job is running");
});

test("F2: the counts line names a job only while the connector holds it or a refusal names the file", () => {
  const held = { name: "coaster.gcode", lines: 6, bytes: 52, jobId: JOB };
  const job = { job_id: JOB, line: 0, lines: 6, elapsed_ms: 0 };
  // Held and carried: Ready, Homing with the job, Done.
  for (const state of [{ name: "Ready" }, { name: "Homing" }, { name: "Done", result: "Completed" }] as JobState[]) {
    assert.equal(countsName(entry(status(state, { job }), { held })), "coaster.gcode", state.name);
  }
  // Alarm 6 dropped the job; homing from Idle with no job; Idle after the job.
  assert.equal(countsName(entry(status({ name: "Alarm" }, { alarm: "Reset during homing (alarm 6)" }), { held })), null);
  assert.equal(countsName(entry(status({ name: "Homing" }), { held })), null);
  assert.equal(countsName(entry(status({ name: "Idle" }), { held })), null);
  // Another app's job: no name of ours.
  assert.equal(countsName(entry(status({ name: "Ready" }, { job: { ...job, job_id: "2" } }), { held })), null);
  // A refused file keeps its name next to its refusal, and loses it with the refusal.
  const refused = { name: "big.gcode", lines: 0, bytes: 0, jobId: null };
  assert.equal(countsName(entry(status({ name: "Idle" }), { held: refused }), { sending: false, failure: { text: "Too large", failure: true } }), "big.gcode");
  assert.equal(countsName(entry(status({ name: "Idle" }), { held: refused })), null);
  // Sending: the chosen file or the page's job.
  assert.equal(countsName(entry(status({ name: "Idle" }), { sendingName: "a.nc" }), { sending: true, failure: null }), "a.nc");
  assert.equal(countsName(entry(status({ name: "Idle" }), { pageSend: { token: 1, name: "Laser 1" } })), "Laser 1");
});

test("a Home waiting for the local press carries no job: no bar and no counts", () => {
  const waiting = controlsLine(entry(status({ name: "AwaitingLocalStart" })));
  assert.equal(waiting?.text, "Waiting for the start at the machine");
  assert.equal(waiting?.bar, null);
  const job = { job_id: JOB, line: 0, lines: 6, elapsed_ms: 0 };
  assert.notEqual(controlsLine(entry(status({ name: "AwaitingLocalStart" }, { job })))?.bar, null);
});

test("UX §11 (review 2026-11-19): the line is read through two hidden regions that stay while paired", () => {
  const footer = source("components/panel/MachineControls.tsx");
  assert.doesNotMatch(footer, /role=\{line\.alerts/, "the visible line carries no live role");
  // Amended by `0134` P8: the polite line also stays quiet during a send's start step, and a "Not
  // started" is said with its Ready line as the alert; P17: a refused send under a held job is
  // said once as the alert, the polite line stays quiet meanwhile.
  assert.match(footer, /const polite = line !== null && !line\.alerts && notStarted === null && refused === null && !quiet \? line\.text : spoken;/);
  assert.match(footer, /const alert =\s*notStarted !== null \? `\$\{line\?\.text \?\? "Ready"\}\. \$\{notStarted\}` : \(refused \?\? \(line\?\.alerts \? line\.text : ""\)\);/);
  assert.match(footer, /<p role="status" className="sr-only">\s*\{polite\}/);
  assert.match(footer, /<p role="alert" className="sr-only">\s*\{alert\}/);
  assert.doesNotMatch(footer, /empty:hidden/, "the warning's region stays in the accessibility tree");
});

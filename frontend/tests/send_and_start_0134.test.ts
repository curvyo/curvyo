// `specs/0134-send-and-start`: the button's names per state (SS9, SS13), the armed press rules
// (P4, P12 to P15) on the timings of `deleteArm.ts`, the held key (P14), the decoded `starts`, the
// start flag on the host call, and "Not started: <reason>" only while the page's job is `Ready`
// (SS6, architect S3). Time is passed in, never slept.
import assert from "node:assert/strict";
import { test } from "node:test";

import { type LinkHost, createLinkStore, notStartedLine, notStartedOf } from "../src/lib/connectorLink.ts";
import type { JobState, StatusMessage } from "../src/lib/connectorText.ts";
import { ARM_GUARD_MS, ARM_TIMEOUT_MS, IDLE, pressTrash } from "../src/lib/deleteArm.ts";
import { controlsLine, sendFailureBelow } from "../src/lib/machineControls.ts";
import { decodeOutputGroup } from "../src/lib/outputView.ts";
import { armEnds, ignoredKey, sendPress } from "../src/lib/sendArm.ts";
import { SEND_ARMED_NAME, SEND_NAME, SEND_START_NAME, notStartedText, sendName } from "../src/lib/sendText.ts";

const JOB = "11111111-1111-4111-8111-111111111111";
const OTHER = "22222222-2222-4222-8222-222222222222";

const status = (state: JobState, id: string | null = null): StatusMessage => ({
  type: "Status",
  state,
  job: id === null ? null : { job_id: id, line: 0, lines: 3, elapsed_ms: 0 },
  position: null,
  alarm: null,
});

function fakeHost(answer: unknown): LinkHost & { calls: { command: string; args?: Record<string, unknown> }[] } {
  const calls: { command: string; args?: Record<string, unknown> }[] = [];
  return {
    calls,
    invoke: (command, args) => {
      calls.push({ command, args });
      return Promise.resolve((command === "connector_send_page_job" ? answer : undefined) as never);
    },
    listen: () => Promise.resolve(() => {}),
  };
}

test("SS9, SS13: the names per state", () => {
  assert.equal(sendName(true, false), "Send to machine and start");
  assert.equal(sendName(false, false), "Send to machine");
  assert.equal(sendName(true, true), "Press again to send to machine and start");
  assert.equal(SEND_START_NAME.length, 25);
  assert.equal(SEND_ARMED_NAME.length, 40);
  assert.equal(SEND_NAME, "Send to machine");
});

test("SS9: the group view decodes starts and the job number", () => {
  const view = {
    machine: "K40 · GRBL",
    start: { value: "machine_origin", damaged: false, options: [{ value: "machine_origin", label: "Machine origin" }] },
    checks: [],
    total: null,
    export: { enabled: true, tooltip: null, file_type: "GRBL G-code", extension: "gcode" },
    toolpath_view: false,
    send: { enabled: true, profile_id: "p", address: "a", starts: true, job: 4 },
  };
  assert.equal(decodeOutputGroup(view)?.send?.starts, true);
  assert.equal(decodeOutputGroup(view)?.send?.job, 4);
  assert.equal(decodeOutputGroup({ ...view, send: { ...view.send, starts: "yes" } }), null);
});

test("SS2, SS9: a press whose name only sends only sends, never arms", () => {
  let asked = 0;
  const fresh = () => {
    asked += 1;
    return true;
  };
  assert.equal(sendPress({ pressable: true, starts: false, armed: false, fresh }), "send");
  assert.equal(sendPress({ pressable: true, starts: false, armed: true, fresh }), "send");
  assert.equal(asked, 0, "P4: a fresh free status never turns only send into a start");
  assert.equal(sendPress({ pressable: false, starts: true, armed: true, fresh }), "ignore", "disabled or busy");
});

test("SS13, P4, P12, P13: both presses read a fresh status; a lost start arms nothing, a lost confirm only sends", () => {
  assert.equal(sendPress({ pressable: true, starts: true, armed: false, fresh: () => true }), "arm");
  assert.equal(sendPress({ pressable: true, starts: true, armed: false, fresh: () => false }), "ignore", "P12");
  assert.equal(sendPress({ pressable: true, starts: true, armed: true, fresh: () => true }), "sendAndStart");
  assert.equal(sendPress({ pressable: true, starts: true, armed: true, fresh: () => false }), "send", "P13");
});

test("SS13: the first press arms and sends nothing, a press in the guard is ignored, the second sends and starts", () => {
  const id = "send-to-machine";
  const first = pressTrash(IDLE, id, 1_000);
  assert.deepEqual(first, { state: { armed: id, at: 1_000 }, remove: false }, "armed, nothing performed");
  const double = pressTrash(first.state, id, 1_000 + ARM_GUARD_MS - 1);
  assert.equal(double.remove, false, "a double-click never starts");
  assert.equal(double.state, first.state);
  const second = pressTrash(first.state, id, 1_000 + ARM_GUARD_MS);
  assert.equal(second.remove, true);
  assert.equal(second.state.armed, null, "the confirm disarms");
  const late = pressTrash(first.state, id, 1_000 + ARM_TIMEOUT_MS);
  assert.equal(late.remove, false, "after 4 s a press arms again instead of starting");
  assert.equal(ARM_TIMEOUT_MS, 4_000);
  assert.equal(ARM_GUARD_MS, 400);
});

test("SS13, P15: the arm ends when the name is no longer start, the button turns busy, or the job changes", () => {
  assert.equal(armEnds(true, true, true, 3, 3), false);
  assert.equal(armEnds(true, false, true, 3, 3), true, "another app sent, an alarm, a door");
  assert.equal(armEnds(true, true, false, 3, 3), true, "a check blocks, the pairing went");
  assert.equal(armEnds(true, true, true, 4, 3), true, "undo, a peer's edit, the Start pick");
  assert.equal(armEnds(false, false, false, 4, 3), false, "nothing armed");
});

test("P14: a held Enter or Space never arms or confirms", () => {
  assert.equal(ignoredKey("Enter", true), true);
  assert.equal(ignoredKey(" ", true), true);
  assert.equal(ignoredKey("Enter", false), false, "the first keydown is a press");
  assert.equal(ignoredKey(" ", false), false);
  assert.equal(ignoredKey("a", true), false);
  // A held Enter: one press, then only repeats; it arms once and never reaches the confirm.
  const keys = [false, ...Array.from({ length: 40 }, () => true)];
  let state = IDLE;
  let performed = false;
  keys.forEach((repeat, at) => {
    if (ignoredKey("Enter", repeat)) return;
    const out = pressTrash(state, "send-to-machine", at * 50);
    state = out.state;
    performed ||= out.remove;
  });
  assert.equal(state.armed, "send-to-machine");
  assert.equal(performed, false, "a held Enter can never start the laser");
});

test("SS6: the reasons read in the words of the notes", () => {
  assert.equal(notStartedText("WrongState"), "Not started: Connector busy");
  assert.equal(notStartedText("ConnectionLost"), "Not started: Connection lost");
  assert.equal(notStartedText("Maintenance"), "Not started: Maintenance");
});

test("SS6, S3: Not started only for a press that asked, and only while that job is Ready", () => {
  const refused = { kind: "resolved", jobId: JOB, resolution: { kind: "accepted" }, start: { kind: "notStarted", reason: "WrongState" } } as const;
  assert.equal(notStartedOf(false, refused, JOB), null, "a send-only press never says Not started");
  assert.deepEqual(notStartedOf(true, refused, JOB), { jobId: JOB, text: "Not started: Connector busy" });
  assert.equal(notStartedOf(true, { ...refused, start: { kind: "started" } }, JOB), null);
  assert.equal(notStartedOf(true, { ...refused, start: { kind: "noAnswer" } }, JOB), null, "the watcher's status decides");
  assert.deepEqual(notStartedOf(true, { kind: "unresolved", error: { kind: "other", message: "x" } }, JOB)?.text, "Not started: Connection lost");
  const store = createLinkStore(fakeHost(null));
  const entry = { ...store.get("p"), notStarted: { jobId: JOB, text: "Not started: Connector busy" } };
  assert.equal(notStartedLine({ ...entry, status: status({ name: "Ready" }, JOB) }), "Not started: Connector busy");
  assert.equal(notStartedLine({ ...entry, status: status({ name: "Running" }, JOB) }), null, "never over a running job");
  assert.equal(notStartedLine({ ...entry, status: status({ name: "Ready" }, OTHER) }), null, "another app's Ready job");
});

test("SS1, SS6: the store passes the start flag, shows Starting, and keeps Not started until the state changes", async () => {
  const answer = { kind: "resolved", jobId: JOB, resolution: { kind: "accepted" }, start: { kind: "notStarted", reason: "WrongState" } };
  const host = fakeHost(answer);
  const store = createLinkStore(host);
  store.setStatus("p", status({ name: "Idle" }));
  let settled: boolean | null = null;
  const sent = store.sendPage("p", "a", { token: 1, name: "page", bytes: new Uint8Array([71]), start: true }, (ok) => (settled = ok));
  assert.equal(store.get("p").startStep, true, "busy through the start step (P5)");
  await sent;
  assert.equal(host.calls.find((c) => c.command === "connector_send_page_job")?.args?.start, true);
  assert.equal(settled, true, "the commit does not depend on the start (SS7)");
  assert.equal(store.get("p").startStep, false);
  store.setStatus("p", status({ name: "Ready" }, JOB));
  assert.equal(notStartedLine(store.get("p")), "Not started: Connector busy");
  store.setStatus("p", status({ name: "Running" }, JOB));
  assert.equal(store.get("p").notStarted, null, "a state change clears it");
  store.setStatus("p", status({ name: "Ready" }, JOB));
  assert.equal(notStartedLine(store.get("p")), null, "and it does not come back");
  const plainHost = fakeHost({ kind: "resolved", jobId: JOB, resolution: { kind: "accepted" } });
  const plain = createLinkStore(plainHost);
  await plain.sendPage("p", "a", { token: 2, name: "page", bytes: new Uint8Array([71]) }, () => {});
  assert.equal(plainHost.calls.find((c) => c.command === "connector_send_page_job")?.args?.start, false, "a send-only press");
  assert.equal(plain.get("p").notStarted, null);
});

test("P8: a started job's Ready is marked until it leaves Ready, so the footer does not say it in between", async () => {
  const started = { kind: "resolved", jobId: JOB, resolution: { kind: "accepted" }, start: { kind: "started" } };
  const store = createLinkStore(fakeHost(started));
  store.setStatus("p", status({ name: "Idle" }));
  const sent = store.sendPage("p", "a", { token: 1, name: "page", bytes: new Uint8Array([71]), start: true }, () => {});
  store.setStatus("p", status({ name: "Ready" }, JOB));
  await sent;
  assert.equal(store.get("p").startedJob, JOB, "the answer came before Running");
  store.setStatus("p", status({ name: "Ready" }, JOB));
  assert.equal(store.get("p").startedJob, JOB, "a repeated Ready keeps it");
  store.setStatus("p", status({ name: "Running" }, JOB));
  assert.equal(store.get("p").startedJob, null, "Running ends it");
  const refused = { ...started, start: { kind: "notStarted", reason: "WrongState" } };
  const other = createLinkStore(fakeHost(refused));
  const again = other.sendPage("p", "a", { token: 2, name: "page", bytes: new Uint8Array([71]), start: true }, () => {});
  other.setStatus("p", status({ name: "Ready" }, JOB));
  await again;
  assert.equal(other.get("p").startedJob, null, "a start that did not happen is said (SS6)");
  const plain = createLinkStore(fakeHost(started));
  const only = plain.sendPage("p", "a", { token: 3, name: "page", bytes: new Uint8Array([71]) }, () => {});
  plain.setStatus("p", status({ name: "Ready" }, JOB));
  await only;
  assert.equal(plain.get("p").startedJob, null, "a send-only press is said as 0125");
});

test("P17: a refused page send while a job is held reads under the job's line, which keeps its bar", () => {
  const store = createLinkStore(fakeHost(null));
  const refused = { ...store.get("p"), link: "paired" as const, sendFailure: { text: "Connector busy", atState: "AwaitingLocalStart" } };
  for (const name of ["AwaitingLocalStart", "Running", "Paused", "Ready"] as const) {
    const entry = { ...refused, status: status({ name }, JOB) };
    const line = controlsLine(entry);
    assert.notEqual(line?.text, "Connector busy", name);
    assert.equal(line?.failure, false, name);
    assert.equal(sendFailureBelow(entry), "Connector busy", name);
  }
  assert.notEqual(controlsLine({ ...refused, status: status({ name: "Running" }, JOB) })?.bar, null, "the bar stays");
  const idle = { ...refused, status: status({ name: "Idle" }) };
  assert.equal(controlsLine(idle)?.text, "Connector busy", "nothing held: the refusal is the line, as 0125");
  assert.equal(sendFailureBelow(idle), null);
  const lost = { ...refused, lost: true, status: status({ name: "Running" }, JOB) };
  assert.equal(controlsLine(lost)?.text, "Connector busy", "a lost connection keeps the 0125 line");
  assert.equal(sendFailureBelow(lost), null);
});

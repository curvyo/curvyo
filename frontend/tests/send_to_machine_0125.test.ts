// `specs/0125-send-to-machine`: the words of Send to machine, the state line and bar of a send,
// the decoded `send` of the group view, and the shared link store (commit once, one status source).
// Since `specs/0118` (Frage 262 A, amending S13 to S15) the one state line of the Operations tab is
// the Machine controls footer's (`controlsLine`), in the `0064` form: "Running" with the bar and the
// counts line, no " · <name>" suffix.
import assert from "node:assert/strict";
import { test } from "node:test";

import { type LinkHost, type Resolution, UNRESOLVED_SEND_MS, createLinkStore, sendLinkOf } from "../src/lib/connectorLink.ts";
import type { JobState, StatusMessage } from "../src/lib/connectorText.ts";
import { decodeOutputGroup } from "../src/lib/outputView.ts";
import { controlsLine as stateLine } from "../src/lib/machineControls.ts";
import { LINE_CHARS, withName } from "../src/lib/sendLine.ts";
import { sendCauseText } from "../src/lib/sendText.ts";

const JOB = "11111111-1111-4111-8111-111111111111";
const OTHER = "22222222-2222-4222-8222-222222222222";

const status = (state: JobState, job: { id: string; line: number; lines: number } | null = null, alarm: string | null = null): StatusMessage => ({
  type: "Status",
  state,
  job: job && { job_id: job.id, line: job.line, lines: job.lines, elapsed_ms: 0 },
  position: null,
  alarm,
});

interface FakeHost extends LinkHost {
  calls: { command: string; args?: Record<string, unknown> }[];
  emit: (payload: Record<string, unknown>) => void;
  answers: Record<string, unknown>;
}

function fakeHost(answers: Record<string, unknown> = {}): FakeHost {
  let handler: ((event: { payload: never }) => void) | null = null;
  const host: FakeHost = {
    calls: [],
    answers,
    emit: (payload) => handler?.({ payload: payload as never }),
    invoke: (command, args) => {
      host.calls.push({ command, args });
      const answer = host.answers[command];
      return Promise.resolve((typeof answer === "function" ? answer(args) : answer) as never);
    },
    listen: (_event, next) => {
      handler = next as never;
      return Promise.resolve(() => {
        handler = null;
      });
    },
  };
  return host;
}

const tick = () => new Promise((resolve) => setImmediate(resolve));

test("S2 to S5: each cause reads in the words of the notes", () => {
  const base = { enabled: false, text: null, profileId: "p", address: "a" } as const;
  assert.equal(sendCauseText({ ...base, cause: "browser" }), "Needs the desktop app");
  assert.equal(sendCauseText({ ...base, cause: "not_paired" }), "Not paired");
  assert.equal(sendCauseText({ ...base, cause: "no_longer_paired" }), "No longer paired");
  assert.equal(sendCauseText({ ...base, cause: "key_changed" }), "This connector's key changed");
  assert.equal(sendCauseText({ ...base, cause: "sending" }), "Sending");
  assert.equal(sendCauseText({ ...base, cause: "blocked", text: "Nothing runs." }), "Nothing runs.");
  assert.equal(sendCauseText({ ...base, cause: "format_not_spoken", text: "The connector does not speak X" }), "The connector does not speak X");
  assert.equal(sendCauseText({ ...base, enabled: true, cause: null }), null);
});

test("S1: the group view decodes the send button and its absence", () => {
  const view = {
    machine: "K40 · GRBL",
    start: { value: "machine_origin", damaged: false, options: [{ value: "machine_origin", label: "Machine origin" }] },
    checks: [],
    total: null,
    export: { enabled: true, tooltip: null, file_type: "GRBL G-code", extension: "gcode" },
    toolpath_view: false,
  };
  assert.equal(decodeOutputGroup(view)?.send, null);
  const send = { enabled: false, cause: "not_paired", profile_id: "p1", address: "192.168.1.20:7447" };
  assert.deepEqual(decodeOutputGroup({ ...view, send })?.send, {
    enabled: false,
    cause: "not_paired",
    text: null,
    profileId: "p1",
    address: "192.168.1.20:7447",
  });
  assert.equal(decodeOutputGroup({ ...view, send: { ...send, cause: "nonsense" } }), null, "a broken value shows nothing");
});

test("S1: an unknown link reads as not paired", () => {
  const store = createLinkStore(fakeHost());
  assert.equal(sendLinkOf(store.get("p")), "not_paired");
});

test("S13: the state line follows the table", () => {
  const entry = (s: StatusMessage | null, extra: object = {}) => ({ ...createLinkStore(fakeHost()).get("x"), status: s, ...extra });
  const held = { name: "Coaster", lines: 412, bytes: 9421, jobId: JOB };
  const line = (s: StatusMessage | null, extra: object = {}) => stateLine(entry(s, extra));
  assert.equal(line(null), null);
  assert.equal(line(status({ name: "Idle" })), null);
  assert.equal(line(status({ name: "NoController" }))?.text, "No controller", "the footer's line names it (0118 §4)");
  assert.equal(line(status({ name: "Ready" }, { id: JOB, line: 0, lines: 412 }), { held, pageJobId: JOB })?.text, "Ready: Coaster, 412 lines, 9.2 KiB");
  assert.equal(line(status({ name: "Ready" }, { id: OTHER, line: 0, lines: 412 }), { held, pageJobId: JOB })?.text, "Ready");
  const running = line(status({ name: "Running" }, { id: JOB, line: 120, lines: 412 }), { held, pageJobId: JOB });
  assert.equal(running?.text, "Running", "the counts are in the counts line (`0118` §6)");
  assert.deepEqual(running?.bar, { line: 120, lines: 412, failure: false });
  assert.equal(line(status({ name: "Paused" }, { id: JOB, line: 120, lines: 412 }), { held, pageJobId: JOB })?.text, "Paused");
  assert.equal(line(status({ name: "AwaitingLocalStart" }, { id: JOB, line: 0, lines: 412 }), { held, pageJobId: JOB })?.text, "Waiting for the start at the machine");
  const done = line(status({ name: "Done", result: "Completed" }, { id: JOB, line: 412, lines: 412 }), { held, pageJobId: JOB });
  assert.equal(done?.text, "Completed, 412 lines");
  assert.equal(done?.bar?.line, 412);
  assert.equal(line(status({ name: "Done", result: "Stopped" }, { id: JOB, line: 7, lines: 412 }), { held, pageJobId: JOB })?.text, "Stopped at line 7");
  const failed = line(status({ name: "Done", result: "Failed" }, { id: JOB, line: 7, lines: 412 }), { held, pageJobId: JOB });
  assert.equal(failed?.alerts, false, "a failed job is a polite status");
  assert.equal(failed?.text, "Failed at line 7");
  assert.equal(failed?.failure, true);
  assert.equal(failed?.bar?.failure, true);
  const alarm = line(status({ name: "Alarm" }, null, "Hard limit"));
  assert.equal(alarm?.text, "Alarm: Hard limit");
  assert.equal(alarm?.alarm, true);
  assert.equal(alarm?.bar, null);
  const lost = line(status({ name: "Running" }, { id: JOB, line: 120, lines: 412 }), { held, pageJobId: JOB, lost: true });
  assert.equal(lost?.text, "Connection lost");
  assert.equal(lost?.bar?.line, 120, "the last values stay");
});

test("S15 (amended by 0118): one line for every job; a Ready of another app names no job of this app", () => {
  const base = createLinkStore(fakeHost()).get("x");
  const manual = { name: "layer-test.gcode", lines: 5, bytes: 90, jobId: OTHER };
  const running = status({ name: "Running" }, { id: OTHER, line: 120, lines: 412 });
  assert.equal(stateLine({ ...base, status: running, held: manual, pageJobId: JOB })?.text, "Running");
  assert.equal(stateLine({ ...base, status: running, held: null })?.text, "Running");
  const ready = status({ name: "Ready" }, { id: OTHER, line: 0, lines: 412 });
  assert.equal(stateLine({ ...base, status: ready, held: { ...manual, jobId: JOB } })?.text, "Ready");
  assert.equal(stateLine({ ...base, status: ready, held: manual })?.text, "Ready: layer-test.gcode, 5 lines, 90 B");
});

test("S13: the job name is cut so a line stays within 70 characters", () => {
  const long = "x".repeat(200);
  const text = withName("Ready: ", long, ", 412 lines, 9.2 KiB");
  assert.equal(text.length, LINE_CHARS);
  assert.ok(text.includes("…"));
  assert.equal(withName("Sending ", "Coaster"), "Sending Coaster");
});

test("S7: a press shows Sending, a failure shows its text and nothing is committed", async () => {
  const host = fakeHost({ connector_send_page_job: { kind: "resolved", jobId: JOB, resolution: { kind: "rejected", reason: "Busy" } } });
  const store = createLinkStore(host);
  const settled: boolean[] = [];
  const pending = store.sendPage("p", "a", { token: 1, name: "Page", bytes: new Uint8Array(3) }, (ok) => settled.push(ok));
  assert.equal(stateLine(store.get("p"))?.text, "Sending Page");
  await pending;
  assert.deepEqual(settled, [false]);
  assert.equal(stateLine(store.get("p"))?.text, "Connector busy");
  assert.equal(stateLine(store.get("p"))?.failure, true);
});

test("S7: Busy while the machine is in alarm names the alarm; a lost pairing sets the link", async () => {
  const host = fakeHost({ connector_send_page_job: { kind: "resolved", jobId: JOB, resolution: { kind: "rejected", reason: "Busy" } } });
  const store = createLinkStore(host);
  store.setStatus("p", status({ name: "Alarm" }, null, "Hard limit"));
  await store.sendPage("p", "a", { token: 1, name: "Page", bytes: new Uint8Array(1) }, () => undefined);
  assert.equal(store.get("p").sendFailure?.text, "Machine in alarm: Hard limit");
  host.answers.connector_send_page_job = { kind: "failed", error: { kind: "noLongerPaired" } };
  await store.sendPage("p", "a", { token: 2, name: "Page", bytes: new Uint8Array(1) }, () => undefined);
  assert.equal(store.get("p").sendFailure?.text, "No longer paired");
  assert.equal(sendLinkOf(store.get("p")), "no_longer_paired");
  host.answers.connector_send_page_job = { kind: "failed", error: { kind: "unreachable", address: "10.0.0.5:7447" } };
  await store.sendPage("p", "a", { token: 3, name: "Page", bytes: new Uint8Array(1) }, () => undefined);
  assert.equal(store.get("p").sendFailure?.text, "Nothing answers at 10.0.0.5:7447");
  host.answers.connector_send_page_job = { kind: "failed", error: { kind: "timedOut" } };
  await store.sendPage("p", "a", { token: 4, name: "Page", bytes: new Uint8Array(1) }, () => undefined);
  assert.equal(store.get("p").sendFailure?.text, "Timed out");
});

test("S11: not received shows its text and commits nothing", async () => {
  const host = fakeHost({ connector_send_page_job: { kind: "resolved", jobId: JOB, resolution: { kind: "notReceived" } } });
  const store = createLinkStore(host);
  const settled: boolean[] = [];
  await store.sendPage("p", "a", { token: 1, name: "Page", bytes: new Uint8Array(1) }, (ok) => settled.push(ok));
  assert.deepEqual(settled, [false]);
  assert.equal(stateLine(store.get("p"))?.text, "Not received");
});

test("S11: the answer and the watcher's event commit once, in either order", async () => {
  const accepted: Resolution = { kind: "accepted" };
  for (const order of ["answer first", "event first"]) {
    const host = fakeHost();
    const store = createLinkStore(host);
    const unwatched = store.acquire("p", "a", false);
    await tick();
    let answer: (value: unknown) => void = () => undefined;
    host.answers.connector_send_page_job = () => new Promise((resolve) => (answer = resolve));
    const settled: boolean[] = [];
    const pending = store.sendPage("p", "a", { token: 1, name: "Page", bytes: new Uint8Array(30) }, (ok) => settled.push(ok));
    host.emit({ profileId: "p", kind: "sendStarted", jobId: JOB, lines: 6, bytes: 30 });
    const event = () => host.emit({ profileId: "p", kind: "sendResolved", jobId: JOB, resolution: accepted });
    const reply = () => answer({ kind: "resolved", jobId: JOB, resolution: accepted });
    if (order === "event first") {
      event();
      reply();
    } else {
      reply();
      await pending;
      event();
    }
    await pending;
    event();
    assert.deepEqual(settled, [true], order);
    const entry = store.get("p");
    assert.deepEqual(entry.held, { name: "Page", lines: 6, bytes: 30, jobId: JOB });
    assert.equal(entry.sendFailure, null);
    assert.equal(stateLine(entry), null, "no status yet and the send is over: nothing to say");
    unwatched();
  }
});

test("S11: an event for another job id commits nothing", async () => {
  const host = fakeHost({ connector_send_page_job: { kind: "resolvedElsewhere" } });
  const store = createLinkStore(host);
  store.acquire("p", "a", false);
  await tick();
  const settled: boolean[] = [];
  await store.sendPage("p", "a", { token: 1, name: "Page", bytes: new Uint8Array(1) }, (ok) => settled.push(ok));
  host.emit({ profileId: "p", kind: "sendStarted", jobId: JOB, lines: 1, bytes: 1 });
  host.emit({ profileId: "p", kind: "sendResolved", jobId: OTHER, resolution: { kind: "accepted" } });
  assert.deepEqual(settled, []);
  host.emit({ profileId: "p", kind: "sendResolved", jobId: JOB, resolution: { kind: "accepted" } });
  assert.deepEqual(settled, [true]);
});

test("S9: after Ready the line names the job and the Connector row's held job is the same", async () => {
  const host = fakeHost();
  const store = createLinkStore(host);
  store.acquire("p", "a", false);
  await tick();
  let answer: (value: unknown) => void = () => undefined;
  host.answers.connector_send_page_job = () => new Promise((resolve) => (answer = resolve));
  const pending = store.sendPage("p", "a", { token: 1, name: "Coaster", bytes: new Uint8Array(9421) }, () => undefined);
  host.emit({ profileId: "p", kind: "sendStarted", jobId: JOB, lines: 412, bytes: 9421 });
  answer({ kind: "resolved", jobId: JOB, resolution: { kind: "accepted" } });
  await pending;
  host.emit({ profileId: "p", kind: "status", status: status({ name: "Ready" }, { id: JOB, line: 0, lines: 412 }) });
  const entry = store.get("p");
  assert.equal(stateLine(entry)?.text, "Ready: Coaster, 412 lines, 9.2 KiB");
  assert.equal(entry.held?.name, "Coaster");
});

test("S14: the Connector row and the Output group read one entry, one watcher", async () => {
  const host = fakeHost();
  const store = createLinkStore(host);
  const row = store.acquire("p", "a", false);
  const group = store.acquire("p", "a", false);
  host.emit({ profileId: "p", kind: "status", status: status({ name: "Running" }, { id: JOB, line: 5, lines: 10 }) });
  assert.equal(store.get("p").status?.job?.line, 5, "the same object for both readers");
  row();
  assert.equal(host.calls.filter((c) => c.command === "connector_unwatch").length, 0, "the group still reads");
  group();
  await tick();
  assert.equal(host.calls.filter((c) => c.command === "connector_unwatch").length, 1);
});

test("S14: a dropped connection keeps the last values with Connection lost, and a status ends it", () => {
  const host = fakeHost();
  const store = createLinkStore(host);
  store.acquire("p", "a", false);
  host.emit({ profileId: "p", kind: "status", status: status({ name: "Running" }, { id: JOB, line: 5, lines: 10 }) });
  host.emit({ profileId: "p", kind: "lost" });
  assert.equal(stateLine(store.get("p"))?.text, "Connection lost");
  host.emit({ profileId: "p", kind: "status", status: status({ name: "Running" }, { id: JOB, line: 6, lines: 10 }) });
  assert.equal(stateLine(store.get("p"))?.text, "Running");
  host.emit({ profileId: "p", kind: "status", status: status({ name: "Idle" }) });
  host.emit({ profileId: "p", kind: "lost" });
  assert.equal(store.get("p").lost, false, "an idle connector is not 'lost'");
});

test("the webview never holds the pairing token: nothing in the store or its answers names one", () => {
  const entry = createLinkStore(fakeHost()).get("p");
  assert.ok(!JSON.stringify(entry).toLowerCase().includes("token"));
});

function fakeTimers() {
  const due: { run: () => void; ms: number; live: boolean }[] = [];
  return {
    timers: {
      set: (run: () => void, ms: number) => {
        const entry = { run, ms, live: true };
        due.push(entry);
        return entry;
      },
      clear: (handle: unknown) => {
        (handle as { live: boolean }).live = false;
      },
    },
    due,
    fire: () => due.filter((entry) => entry.live).forEach((entry) => entry.run()),
  };
}

test("S7/S8: an unresolved send times out, Send is free, and a late Ready commits once", async () => {
  const host = fakeHost({ connector_send_page_job: { kind: "unresolved", error: { kind: "unreachable", address: "a" } } });
  const clock = fakeTimers();
  const store = createLinkStore(host, clock.timers);
  store.acquire("p", "a", false);
  await tick();
  const settled: boolean[] = [];
  let timedOut = 0;
  await store.sendPage("p", "a", { token: 1, name: "Page", bytes: new Uint8Array(1) }, (ok) => settled.push(ok), () => timedOut++);
  host.emit({ profileId: "p", kind: "sendStarted", jobId: JOB, lines: 2, bytes: 1 });
  assert.equal(clock.due.length, 1);
  assert.equal(clock.due[0]?.ms, UNRESOLVED_SEND_MS);
  assert.equal(store.sending("p"), true);
  assert.equal(stateLine(store.get("p"))?.text, "Sending Page");
  clock.fire();
  assert.equal(timedOut, 1);
  assert.equal(store.sending("p"), false, "Send is enabled again");
  assert.equal(stateLine(store.get("p"))?.text, "Timed out");
  assert.deepEqual(settled, [], "nothing is committed by the timeout");
  host.emit({ profileId: "p", kind: "sendResolved", jobId: JOB, resolution: { kind: "accepted" } });
  host.emit({ profileId: "p", kind: "sendResolved", jobId: JOB, resolution: { kind: "accepted" } });
  assert.deepEqual(settled, [true], "the late Ready commits once");
  assert.equal(store.get("p").sendFailure, null);
});

test("S7/S8: a resolution before the bound cancels it, a new send replaces a timed out one", async () => {
  const host = fakeHost({ connector_send_page_job: { kind: "unresolved", error: { kind: "unreachable", address: "a" } } });
  const clock = fakeTimers();
  const store = createLinkStore(host, clock.timers);
  store.acquire("p", "a", false);
  await tick();
  const settled: boolean[] = [];
  let timedOut = 0;
  await store.sendPage("p", "a", { token: 1, name: "Page", bytes: new Uint8Array(1) }, (ok) => settled.push(ok), () => timedOut++);
  host.emit({ profileId: "p", kind: "sendStarted", jobId: JOB, lines: 1, bytes: 1 });
  host.emit({ profileId: "p", kind: "sendResolved", jobId: JOB, resolution: { kind: "notReceived" } });
  clock.fire();
  assert.deepEqual(settled, [false]);
  assert.equal(timedOut, 0, "the bound was cancelled");
  await store.sendPage("p", "a", { token: 2, name: "Page", bytes: new Uint8Array(1) }, (ok) => settled.push(ok), () => timedOut++);
  clock.fire();
  assert.equal(timedOut, 1);
  await store.sendPage("p", "a", { token: 3, name: "Page", bytes: new Uint8Array(1) }, (ok) => settled.push(ok), () => timedOut++);
  host.emit({ profileId: "p", kind: "sendStarted", jobId: OTHER, lines: 1, bytes: 1 });
  host.emit({ profileId: "p", kind: "sendResolved", jobId: JOB, resolution: { kind: "accepted" } });
  assert.deepEqual(settled, [false], "the old job id commits nothing any more");
});

test("UX §4 (amended by 0118): a send's failure and an alarm are alerts of the one line, a plain state is not", () => {
  const empty = createLinkStore(fakeHost()).get("x");
  const idle = { ...empty, status: status({ name: "Idle" }) };
  const alarm = { ...empty, status: status({ name: "Alarm" }, null, "Hard limit") };
  const busy = { ...idle, sendFailure: { text: "Connector busy", atState: "Idle" } };
  assert.equal(stateLine(alarm)?.alerts, true);
  assert.equal(stateLine(busy)?.text, "Connector busy");
  assert.equal(stateLine(busy)?.alerts, true);
  assert.equal(stateLine(idle), null, "nothing to say, nothing read out");
  const ready = { ...empty, status: status({ name: "Ready" }, { id: JOB, line: 0, lines: 4 }) };
  assert.equal(stateLine(ready)?.alerts, false);
});

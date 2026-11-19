// `specs/0133-fix-pause-while-homing` criterion 8 (UX notes H1, H3 to H8): a Pause the connector
// keeps pending while the job homes reads "Pause after homing", Pause stays busy driven by the
// status, Resume and Stop are enabled, and a refused Pause reads "No job to pause". Everything
// follows `pause_pending` in the status, so a second app reads the same.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { type LinkEntry, type LinkHost, createLinkStore } from "../src/lib/connectorLink.ts";
import {
  NO_JOB_TO_PAUSE_TEXT,
  PAUSE_PENDING_TEXT,
  type JobState,
  type StatusMessage,
  jobButtons,
  jobStateLine,
} from "../src/lib/connectorText.ts";
import { SETTLES, controlsLine, pauseButton, pausePending, refusalEnds } from "../src/lib/machineControls.ts";

const JOB = "11111111-1111-4111-8111-111111111111";
const job = { job_id: JOB, line: 3, lines: 12, elapsed_ms: 4_000 };

const status = (state: JobState, extra: Partial<StatusMessage> = {}): StatusMessage => ({
  type: "Status",
  state,
  job,
  position: null,
  alarm: null,
  ...extra,
});

const host = (): LinkHost => ({
  invoke: () => Promise.resolve(undefined as never),
  listen: () => Promise.resolve(() => undefined),
});

const entry = (s: StatusMessage | null, extra: Partial<LinkEntry> = {}): LinkEntry => ({
  ...createLinkStore(host()).get("x"),
  link: "paired",
  status: s,
  ...extra,
});

const homing = status({ name: "Running" }, { detail: "Controller state: Home" });
const pending = status({ name: "Running" }, { detail: "Controller state: Home", pause_pending: true });
const paused = status({ name: "Paused" });

test("H1: the state line reads Pause after homing while pending, Paused once held", () => {
  assert.equal(PAUSE_PENDING_TEXT, "Pause after homing");
  assert.deepEqual(jobStateLine(homing, null), { text: "Running", failure: false });
  assert.deepEqual(jobStateLine(pending, null), { text: "Pause after homing", failure: false });
  assert.deepEqual(jobStateLine(paused, null), { text: "Paused", failure: false });
  const line = controlsLine(entry(pending));
  assert.equal(line?.text, "Pause after homing");
  assert.equal(line?.alerts, false, "polite, not an alert");
  assert.deepEqual(line?.bar, { line: 3, lines: 12, failure: false }, "bar and counts as reported");
});

test("H3, H5, H7: Pause is enabled during the job's homing, busy with its second line while pending", () => {
  assert.deepEqual(pauseButton(entry(homing)), { enabled: true, busy: false, cause: null });
  assert.deepEqual(pauseButton(entry(pending)), { enabled: false, busy: true, cause: "Pause after homing" });
  assert.deepEqual(pauseButton(entry(paused)), { enabled: false, busy: false, cause: null });
  // The Home button's homing: Pause greyed, never sent.
  assert.deepEqual(pauseButton(entry(status({ name: "Homing" }))), { enabled: false, busy: false, cause: null });
  // The short press wait greys it as built.
  assert.deepEqual(pauseButton(entry(homing, { busy: "pause" })), { enabled: false, busy: true, cause: null });
});

test("H6: Resume and Stop are enabled while pending and when paused", () => {
  assert.equal(pausePending(pending), true);
  assert.equal(pausePending(homing), false);
  assert.equal(pausePending(status({ name: "Paused" }, { pause_pending: true })), false, "Running only");
  assert.deepEqual(jobButtons(pending.state, pausePending(pending)), {
    send: false,
    start: true,
    startName: "Resume",
    pause: false,
    stop: true,
  });
  assert.deepEqual(jobButtons(paused.state, pausePending(paused)), {
    send: false,
    start: true,
    startName: "Resume",
    pause: false,
    stop: true,
  });
  assert.deepEqual(jobButtons(homing.state, pausePending(homing)).startName, "Start");
});

test("H3: the press wait ends with the pending status; Resume's ends once it is cancelled", () => {
  assert.equal(SETTLES.pause.done(homing), false);
  assert.equal(SETTLES.pause.done(pending), true, "the status drives Pause from here, no time limit");
  assert.equal(SETTLES.pause.done(paused), true);
  assert.equal(SETTLES.resume.done(pending), false);
  assert.equal(SETTLES.resume.done(homing), true, "cancelled: back to Running");
  assert.equal(SETTLES.resume.done(paused), false);
});

test("PH8: a second app reads the same from the status alone", () => {
  const other = entry(pending, { held: null, busy: null });
  assert.equal(controlsLine(other)?.text, "Pause after homing");
  assert.equal(pauseButton(other).busy, true);
});

test("H8: a refused Pause reads No job to pause in the failure look, as an alert", () => {
  assert.equal(NO_JOB_TO_PAUSE_TEXT, "No job to pause");
  const line = controlsLine(entry(status({ name: "Homing" })), {
    sending: false,
    failure: { text: NO_JOB_TO_PAUSE_TEXT, failure: true },
  });
  assert.equal(line?.text, "No job to pause");
  assert.equal(line?.failure, true);
  assert.equal(line?.alerts, true);
  const hook = readFileSync(new URL("../src/hooks/useMachineControls.ts", import.meta.url), "utf8");
  assert.match(hook, /wrongState" && kind === "pause"\)[\s\S]*?NO_JOB_TO_PAUSE_TEXT/);
});

test("H4: Pause is never a toggle", () => {
  const view = readFileSync(new URL("../src/components/panel/MachineControls.tsx", import.meta.url), "utf8");
  assert.doesNotMatch(view, /aria-pressed/);
});

test("H8: No job to pause ends with the next state change, not only at a line count", () => {
  const home = status({ name: "Homing" }, { job: null });
  // The refresh after the refusal reads Homing: the line stays.
  assert.equal(refusalEnds(status({ name: "Running" }), home, "Homing"), false);
  assert.equal(refusalEnds(home, home, "Homing"), false);
  // Homing ends: Ready or Idle replaces the refusal (neither has a line count).
  assert.equal(refusalEnds(home, status({ name: "Ready" }), "Homing"), true);
  assert.equal(refusalEnds(home, status({ name: "Idle" }, { job: null }), "Homing"), true);
  // Other refusals keep the built rule.
  assert.equal(refusalEnds(home, status({ name: "Idle" }, { job: null }), null), false);
  assert.equal(refusalEnds(status({ name: "Ready" }), status({ name: "Running" }), null), true);
});

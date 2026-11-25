// `specs/0118-machine-control`, slice 2d: the Focus point toggle's causes in their order (criterion
// 34, UX notes W55), the mode's end on a new cause (W56), the press rules (W57, W58), the target
// ring's place (W59) and the pulse built from the profile copy (criterion 35 a). The session maps
// the press and the connector clamps and fires; these are the host's rules.
import assert from "node:assert/strict";
import { test } from "node:test";

import { type LinkEntry, type LinkHost, createLinkStore } from "../src/lib/connectorLink.ts";
import { type JobState, type StatusMessage, jobButtons, jobLineAlerts, jobStateLine } from "../src/lib/connectorText.ts";
import { focusCause, isClick, ownFocus, pageCause, pressTakes, pulseOf, ringTarget, targetOf } from "../src/lib/focusPoint.ts";
import { homeButton, jogCause, setOriginCause } from "../src/lib/machineControls.ts";
import { frameCause } from "../src/lib/frameControls.ts";

const status = (state: JobState, extra: Partial<StatusMessage> = {}): StatusMessage => ({
  type: "Status",
  state,
  job: null,
  position: [10, 20, 0],
  alarm: null,
  homed: true,
  ...extra,
});
const host = (): LinkHost => ({ invoke: () => Promise.resolve(undefined as never), listen: () => Promise.resolve(() => undefined) });
const entry = (s: StatusMessage | null, extra: Partial<LinkEntry> = {}): LinkEntry => ({
  ...createLinkStore(host()).get("x"),
  link: "paired",
  status: s,
  ...extra,
});
const none = { jog: false, focus: false };
const cause = (s: StatusMessage | null, page: string | null = null, own = none, extra: Partial<LinkEntry> = {}) =>
  focusCause(entry(s, extra), page, own);

test("W55: the toggle is enabled on a homed controller in Idle, Ready and Done", () => {
  for (const state of [{ name: "Idle" }, { name: "Ready" }, { name: "Done", result: "Completed" }] as JobState[]) {
    assert.equal(cause(status(state)), null, state.name);
  }
});

test("W55: the causes, connector first, then the trust in the position, then the page", () => {
  assert.equal(cause(status({ name: "Idle" }), null, none, { lost: true }), "Connection lost");
  assert.equal(cause(status({ name: "NoController" } as JobState, { detail: "No controller on COM3" })), "No controller on COM3");
  const words: [string, string][] = [
    ["Homing", "Homing"],
    ["Jogging", "Jogging"],
    ["Framing", "Framing"],
    ["Focusing", "Focus point"],
    ["Pulsing", "Pulse"],
    ["Running", "A job is running"],
    ["Paused", "A job is running"],
    ["Receiving", "Sending"],
    ["Alarm", "Alarm"],
  ];
  for (const [name, text] of words) {
    assert.equal(cause(status({ name } as JobState), "Start is Head position"), text, name);
  }
  assert.equal(cause(status({ name: "AwaitingLocalStart" } as JobState)), "Waiting for the start at the machine");
  assert.equal(cause(status({ name: "Idle" }, { detail: "Controller state: Door" })), "Controller state: Door");
  assert.equal(cause(status({ name: "Idle" }, { homed: false }), "Start is Head position"), "Not homed");
  assert.equal(cause(status({ name: "Idle" }), "Start is Head position"), "Start is Head position");
  assert.equal(cause(status({ name: "Idle" }), "Nothing to place"), "Nothing to place");
});

test("W56: this app's own jog, move, pulse and wait are no causes, so the mode stays on", () => {
  assert.equal(cause(status({ name: "Jogging" } as JobState), null, { jog: true, focus: false }), null);
  for (const name of ["Focusing", "Pulsing"]) {
    assert.equal(cause(status({ name } as JobState), null, { jog: false, focus: true }), null, name);
  }
  const waiting = status({ name: "AwaitingLocalStart" } as JobState, { target: [1, 2] });
  assert.equal(cause(waiting, null, { jog: false, focus: true }), null);
  // Laser mode is no cause: the controller's warning names it, the move is laser off.
  assert.equal(cause(status({ name: "Idle" })), null);
});

test("W58: a press during this app's own move, pulse or wait does nothing; another app's never counts as own", () => {
  const focusing = status({ name: "Focusing" } as JobState, { target: [1, 2] });
  assert.equal(ownFocus(focusing, true), true);
  assert.equal(ownFocus(focusing, false), false);
  assert.equal(pressTakes(focusing, true), false);
  assert.equal(pressTakes(status({ name: "Idle" }), true), true);
  assert.equal(pressTakes(status({ name: "AwaitingLocalStart" } as JobState, { target: [1, 2] }), true), false);
  assert.equal(pressTakes(status({ name: "AwaitingLocalStart" } as JobState), true), true, "a waiting job is no own wait");
});

test("W57: a press is a click within the tolerance, a drag is nothing", () => {
  assert.equal(isClick({ x: 10, y: 10 }, { x: 12, y: 13 }), true);
  assert.equal(isClick({ x: 10, y: 10 }, { x: 15, y: 10 }), false);
});

test("W59: the target ring stands at the connector's clamped target of this app's go to point", () => {
  const pulsing = status({ name: "Pulsing" } as JobState, { target: [400, 0] });
  assert.deepEqual(ringTarget(pulsing, true), [400, 0]);
  assert.equal(ringTarget(pulsing, false), null, "another app's");
  assert.equal(ringTarget(status({ name: "Idle" }), true), null);
});

test("criterion 34, W58: Focus point and Pulse read in the state line, Stop stays the one button", () => {
  assert.equal(jobStateLine(status({ name: "Focusing" } as JobState), null)?.text, "Focus point");
  assert.equal(jobStateLine(status({ name: "Pulsing" } as JobState), null)?.text, "Pulse");
  for (const name of ["Focusing", "Pulsing"]) {
    const buttons = jobButtons({ name } as JobState);
    assert.deepEqual([buttons.stop, buttons.start, buttons.send, buttons.pause], [true, false, false, false], name);
  }
  const focusing = entry(status({ name: "Focusing" } as JobState));
  assert.equal(homeButton(focusing).cause, "Focus point");
  assert.equal(jogCause(focusing, false), "Focus point");
  assert.equal(frameCause(focusing, { enabled: true, cause: null }), "Focus point");
  assert.equal(setOriginCause(focusing), "Machine is moving");
  const pulsing = entry(status({ name: "Pulsing" } as JobState));
  assert.equal(homeButton(pulsing).cause, "Pulse");
  assert.equal(jogCause(pulsing, false), "Pulse");
  assert.equal(frameCause(pulsing, { enabled: true, cause: null }), "Pulse");
});

test("criterion 35 a: the pulse comes from the copy as the session says it, or not at all", () => {
  assert.deepEqual(pulseOf('{"power_permille":10,"length_ms":300}'), { powerPermille: 10, lengthMs: 300 });
  assert.equal(pulseOf("null"), null);
  assert.equal(pulseOf(""), null);
  assert.deepEqual(targetOf('{"x":10,"y":280}'), { x: 10, y: 280 });
  assert.deepEqual(targetOf('{"cause":"Start is Head position"}'), { cause: "Start is Head position" });
  assert.equal(targetOf("null"), null);
  assert.equal(pageCause({ x: 1, y: 2 }), null);
  assert.equal(pageCause({ cause: "Nothing to place" }), "Nothing to place");
});

test("35 c: a go to point from Done that ends short reads its failure line as from Idle and Ready", () => {
  const short = { detail: "Did not reach the point" };
  for (const state of [
    { name: "Idle" },
    { name: "Ready" },
    { name: "Done", result: "Completed" },
    { name: "Done", result: "Stopped" },
  ] as JobState[]) {
    const line = jobStateLine(status(state, short), null);
    assert.deepEqual(line, { text: "Did not reach the point", failure: true }, JSON.stringify(state));
    assert.equal(jobLineAlerts(status(state, short)), true, JSON.stringify(state));
  }
  const done = status({ name: "Done", result: "Completed" } as JobState, { job: { job_id: "j", line: 3, lines: 3, elapsed_ms: 0 } });
  assert.deepEqual(jobStateLine(done, null), { text: "Completed, 3 lines", failure: false }, "no detail, the job's line");
  assert.equal(jobLineAlerts(done), false);
});

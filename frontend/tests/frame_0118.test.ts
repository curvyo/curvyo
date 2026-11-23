// `specs/0118-machine-control`, slice 2b: the Frame row (criteria 22 to 27; UX notes W28 to W38):
// the button's causes in their order, the owner's "Confirm frame", the busy look driven by the
// status, the state line, the head ring's place and the pick's keys.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { type LinkEntry, type LinkHost, createLinkStore, discardedName } from "../src/lib/connectorLink.ts";
import {
  type JobState,
  type Session,
  type StatusMessage,
  confirmsFrame,
  confirmsHome,
  jobButtons,
  jobIsActive,
  jobStateLine,
  waitingText,
} from "../src/lib/connectorText.ts";
import { FRAME_SHAPES, frameCause, frameSlot, headPlace, nextShape } from "../src/lib/frameControls.ts";
import { controlsLine, homeButton, jogCause } from "../src/lib/machineControls.ts";
import { decodeOutputGroup } from "../src/lib/outputView.ts";

const FRAME = "22222222-2222-4222-8222-222222222222";
const JOB = "11111111-1111-4111-8111-111111111111";

const status = (state: JobState, extra: Partial<StatusMessage> = {}): StatusMessage => ({
  type: "Status",
  state,
  job: null,
  position: null,
  alarm: null,
  ...extra,
});

const host: LinkHost = { invoke: () => Promise.resolve(undefined as never), listen: () => Promise.resolve(() => undefined) };
const base = (): LinkEntry => createLinkStore(host).get("x");
const entry = (s: StatusMessage | null, extra: Partial<LinkEntry> = {}): LinkEntry => ({ ...base(), link: "paired", status: s, ...extra });
const owner: Session = { role: "Owner", clientName: "Marc", ownerLabel: "Marc" };
const member: Session = { role: "Member", clientName: "Phone", ownerLabel: "Marc" };
const page = { enabled: true, cause: null };
const waiting = (extra: Partial<StatusMessage> = {}) =>
  status({ name: "AwaitingLocalStart" }, { requested_by: "Phone", frame_id: FRAME, ...extra });

test("Frame is enabled in Idle, Ready and Done and greyed elsewhere with the connector's cause first", () => {
  for (const state of [{ name: "Idle" }, { name: "Ready" }, { name: "Done", result: "Completed" }] as JobState[]) {
    assert.equal(frameCause(entry(status(state)), page), null, state.name);
  }
  const table: [JobState, string][] = [
    [{ name: "Homing" }, "Homing"],
    [{ name: "Jogging" }, "Jogging"],
    [{ name: "Running" }, "A job is running"],
    [{ name: "Paused" }, "A job is running"],
    [{ name: "Receiving" }, "Sending"],
    [{ name: "Alarm" }, "Alarm"],
  ];
  for (const [state, cause] of table) assert.equal(frameCause(entry(status(state)), page), cause, state.name);
  assert.equal(frameCause(entry(status({ name: "Idle" }), { lost: true }), page), "Connection lost");
  assert.equal(frameCause(entry(status({ name: "NoController" }, { detail: "Port gone" })), page), "Port gone");
  assert.equal(
    frameCause(entry(status({ name: "Idle" }, { detail: "Controller state: Door" })), page),
    "Controller state: Door",
  );
});

test("the page's causes come after the connector's, and laser mode and not-homed are no causes", () => {
  const idle = entry(status({ name: "Idle" }, { homed: false }));
  assert.equal(frameCause(idle, { enabled: false, cause: "Nothing to frame" }), "Nothing to frame");
  assert.equal(frameCause(idle, { enabled: false, cause: "Outside the work area" }), "Outside the work area");
  assert.equal(frameCause(entry(status({ name: "Homing" })), { enabled: false, cause: "Nothing to frame" }), "Homing");
  assert.equal(frameCause(idle, null), "Nothing to frame");
});

test("a waiting request names its cause as the state line does", () => {
  const owners = entry(waiting(), { session: owner });
  assert.equal(frameCause(owners, page), "Frame requested by Phone");
  assert.equal(frameCause(entry(waiting(), { session: member }), page), "Waiting for Marc to confirm");
  const other: Session = { role: "Member", clientName: "Watch", ownerLabel: "Marc" };
  assert.equal(frameCause(entry(waiting(), { session: other }), page), "Phone waits for Marc to confirm");
  assert.equal(frameCause(entry(status({ name: "AwaitingLocalStart" })), page), "Waiting for the start at the machine");
});

test("an owner app's Frame slot confirms a member's frame, not armed, and Home does not", () => {
  const s = waiting();
  assert.equal(confirmsFrame(s, owner), true);
  assert.equal(confirmsFrame(s, member), false);
  assert.equal(confirmsHome(s, owner), false, "a frame is not a Home");
  const slot = frameSlot(entry(s, { session: owner }), page);
  assert.deepEqual(slot, { enabled: true, busy: false, cause: null, name: "Confirm frame" });
  assert.equal(homeButton(entry(s, { session: owner })).cause, "Frame requested by Phone");
  assert.equal(frameSlot(entry(s, { session: member }), page).name, "Frame");
  // A waiting Home still confirms in the Home slot.
  const home = status({ name: "AwaitingLocalStart" }, { requested_by: "Phone" });
  assert.equal(confirmsHome(home, owner), true);
  assert.equal(confirmsFrame(home, owner), false);
});

test("Frame is busy from the press and while the connector reports Framing, in every app", () => {
  const idle = status({ name: "Idle" });
  assert.equal(frameSlot(entry(idle, { busy: "frame" }), page).busy, true);
  const framing = frameSlot(entry(status({ name: "Framing" }, { frame_id: FRAME }), { session: member }), page);
  assert.equal(framing.busy, true);
  assert.equal(framing.enabled, false);
});

test("while framing Home, the arrows, Start and Send are greyed and Stop is enabled", () => {
  const framing = status({ name: "Framing" }, { frame_id: FRAME });
  assert.equal(homeButton(entry(framing)).cause, "Framing");
  assert.equal(jogCause(entry(framing), false), "Framing");
  const buttons = jobButtons(framing.state);
  assert.deepEqual(
    [buttons.send, buttons.start, buttons.pause, buttons.stop],
    [false, false, false, true],
  );
  assert.equal(jobIsActive(framing.state), true);
});

test("the state line says Framing, neutral, with no bar, and a held Ready job is not named", () => {
  const line = jobStateLine(status({ name: "Framing" }, { frame_id: FRAME, job: { job_id: JOB, line: 0, lines: 6, elapsed_ms: 0 } }), null);
  assert.deepEqual(line, { text: "Framing", failure: false });
  assert.equal(waitingText(waiting(), owner), "Frame requested by Phone");
});

test("the head ring follows the position only while framing, relative for Head position", () => {
  const at = (state: JobState, position: [number, number, number] | null) => status(state, { position });
  assert.deepEqual(headPlace(at({ name: "Framing" }, [12, 7, 0]), false, null), { x: 12, y: 7 });
  assert.deepEqual(headPlace(at({ name: "Framing" }, [12, 7, 0]), true, [10, 5]), { x: 2, y: 2 });
  assert.equal(headPlace(at({ name: "Idle" }, [12, 7, 0]), false, null), null, "it leaves at once");
  assert.equal(headPlace(at({ name: "Framing" }, null), false, null), null);
  assert.equal(headPlace(null, false, null), null);
});

test("the pick has two values, default Rectangle, and the arrow keys move without wrapping", () => {
  assert.deepEqual(FRAME_SHAPES.map((s) => s.label), ["Rectangle", "Rubber band"]);
  assert.equal(nextShape("rectangle", "ArrowRight"), "rubber_band");
  assert.equal(nextShape("rubber_band", "ArrowRight"), "rubber_band");
  assert.equal(nextShape("rubber_band", "ArrowLeft"), "rectangle");
  assert.equal(nextShape("rectangle", "ArrowLeft"), "rectangle");
  assert.equal(nextShape("rectangle", "a"), "rectangle");
});

test("the output group carries the page's Frame view", () => {
  const source = readFileSync(new URL("../src/lib/outputView.ts", import.meta.url), "utf8");
  assert.match(source, /frame: FrameButton/);
  assert.equal(decodeOutputGroup({}), null);
});

test("the row is its own group above the toolbar with a radiogroup and no hint label", () => {
  const row = readFileSync(new URL("../src/components/panel/FrameRow.tsx", import.meta.url), "utf8");
  assert.match(row, /role="group" aria-label="Frame"/);
  assert.match(row, /role="radiogroup" aria-label="Frame shape"/);
  const controls = readFileSync(new URL("../src/components/panel/MachineControls.tsx", import.meta.url), "utf8");
  assert.ok(controls.indexOf("<FrameRow") < controls.indexOf("<Toolbar\n"), "the row sits above the toolbar");
  assert.ok(controls.indexOf("<JogPad") < controls.indexOf("<FrameRow"), "the pad opens above the row");
});

test("a member's frame discarded by someone else reads Not started: Frame, muted (W35)", () => {
  const waiting = status({ name: "AwaitingLocalStart" }, { requested_by: "Phone", frame_id: FRAME });
  const idle = status({ name: "Idle" });
  assert.equal(discardedName(waiting, idle, member, false), "Frame");
  assert.equal(discardedName(waiting, idle, member, true), null, "its own Stop gives no line");
  const held = status({ name: "AwaitingLocalStart" }, { requested_by: "Phone", frame_id: FRAME, job: { job_id: JOB, line: 0, lines: 9, elapsed_ms: 0 } });
  assert.equal(discardedName(held, status({ name: "Ready" }, { job: { job_id: JOB, line: 0, lines: 9, elapsed_ms: 0 } }), member, false), "Frame", "the held job stays Ready");
  const line = controlsLine(entry(idle, { session: member, discarded: "Frame" }));
  assert.equal(line?.text, "Not started: Frame");
  assert.equal(line?.muted, true);
});

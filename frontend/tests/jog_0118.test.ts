// `specs/0118-machine-control`, slice 2a: the jog pad (criteria 15 to 20) and a member's Home that
// waits for an owner (criterion 37); UX notes W16 to W27. The press reducer takes its times as
// data, so nothing here waits.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { discardedName, type LinkEntry, type LinkHost, createLinkStore } from "../src/lib/connectorLink.ts";
import {
  type JobState,
  type Session,
  type StatusMessage,
  confirmKind,
  confirmsHome,
  jobButtons,
  jobIsActive,
  jobLineAlerts,
  jobStateLine,
  waitingText,
} from "../src/lib/connectorText.ts";
import {
  ARROWS,
  DEFAULT_STEP,
  HOLD_AFTER_MS,
  IDLE,
  KEEP_EVERY_MS,
  STEPS,
  type Arrow,
  type Corner,
  type PressEvent,
  type PressState,
  focusAfter,
  keyPress,
  press,
  readJogView,
  requestOf,
  writeJogView,
} from "../src/lib/jog.ts";
import { controlsLine, homeButton, jogCause, workOffsetNotice } from "../src/lib/machineControls.ts";

const JOB = "11111111-1111-4111-8111-111111111111";
const status = (state: JobState, extra: Partial<StatusMessage> = {}): StatusMessage => ({
  type: "Status",
  state,
  job: null,
  position: null,
  alarm: null,
  ...extra,
});
const host = (): LinkHost => ({ invoke: () => Promise.resolve(undefined as never), listen: () => Promise.resolve(() => undefined) });
const entry = (s: StatusMessage | null, extra: Partial<LinkEntry> = {}): LinkEntry => ({
  ...createLinkStore(host()).get("x"),
  link: "paired",
  status: s,
  ...extra,
});
const owner: Session = { role: "Owner", clientName: "Laptop", ownerLabel: "Marc" };
const member: Session = { role: "Member", clientName: "Phone", ownerLabel: "Marc" };
const source = (file: string) =>
  readFileSync(new URL(`../src/${file}`, import.meta.url), "utf8")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/\/\/.*$/gm, "");

test("criterion 16, W20: the arrows follow the screen and are named by the machine axis per origin corner", () => {
  const names = (corner: Corner) => (["up", "down", "left", "right"] as Arrow[]).map((a) => ARROWS[corner][a].name);
  assert.deepEqual(names("bottom_left"), ["Y+", "Y-", "X-", "X+"]);
  assert.deepEqual(names("top_left"), ["Y-", "Y+", "X-", "X+"]);
  assert.deepEqual(names("bottom_right"), ["Y+", "Y-", "X+", "X-"]);
  assert.deepEqual(names("top_right"), ["Y-", "Y+", "X+", "X-"]);
  // A press is the machine axis, the way and the step, never the screen direction.
  assert.deepEqual(requestOf("top_right", "up", "10"), { axis: "Y", direction: "Minus", step: "10" });
  assert.deepEqual(requestOf("bottom_left", "right", "0.1"), { axis: "X", direction: "Plus", step: "0.1" });
});

test("criterion 15, Frage 315 A: the steps are the closed set and 1 mm is the default", () => {
  assert.deepEqual([...STEPS], ["0.1", "1", "10", "50"]);
  assert.equal(DEFAULT_STEP, "1");
});

test("criterion 16, 18: released within 300 ms is one step, held past it a hold with keep-alives and a release", () => {
  const run = (events: PressEvent[]) => {
    let state: PressState = IDLE;
    return events.map((event) => {
      const [next, effect] = press(state, event);
      state = next;
      return effect;
    });
  };
  assert.equal(HOLD_AFTER_MS, 300);
  assert.ok(KEEP_EVERY_MS <= 100);
  assert.deepEqual(run([{ kind: "down", at: 1000 }, { kind: "end" }]), [null, "step"]);
  assert.deepEqual(run([{ kind: "down", at: 1000 }, { kind: "tick", at: 1299 }, { kind: "end" }]), [null, null, "step"], "299 ms is a step");
  assert.deepEqual(
    run([
      { kind: "down", at: 1000 },
      { kind: "tick", at: 1300 },
      { kind: "tick", at: 1400 },
      { kind: "tick", at: 1500 },
      { kind: "end" },
    ]),
    [null, "hold", "keep", "keep", "release"],
  );
  // Nothing without a press, and a second down while one is under way changes nothing.
  assert.deepEqual(run([{ kind: "end" }, { kind: "tick", at: 5 }]), [null, null]);
  assert.deepEqual(run([{ kind: "down", at: 0 }, { kind: "down", at: 10 }, { kind: "end" }]), [null, null, "step"]);
  // After the release the next press starts afresh.
  assert.deepEqual(
    run([{ kind: "down", at: 0 }, { kind: "tick", at: 300 }, { kind: "end" }, { kind: "down", at: 900 }, { kind: "end" }]),
    [null, "hold", "release", null, "step"],
  );
});

test("W24: Space can hold, Enter is one step, a key repeat is ignored, arrow keys move the focus only", () => {
  assert.equal(keyPress(" ", false), "space");
  assert.equal(keyPress("Enter", false), "enter");
  assert.equal(keyPress(" ", true), null);
  assert.equal(keyPress("Enter", true), null);
  assert.equal(keyPress("a", false), null);
  assert.equal(focusAfter("up", "ArrowDown"), "down");
  assert.equal(focusAfter("left", "ArrowRight"), "right");
  assert.equal(focusAfter("up", "ArrowUp"), null);
  assert.equal(focusAfter("up", "Home"), null, "Home and End are not used");
});

test("W16: the pad's view is remembered per app, closed at 1 mm by default, and a blocked storage is no failure", () => {
  assert.deepEqual(readJogView(null), { open: false, step: "1" });
  assert.deepEqual(readJogView({ getItem: () => null }), { open: false, step: "1" });
  assert.deepEqual(readJogView({ getItem: () => '{"open":true,"step":"10"}' }), { open: true, step: "10" });
  assert.deepEqual(readJogView({ getItem: () => '{"open":true,"step":"7"}' }), { open: true, step: "1" }, "never a typed step");
  assert.deepEqual(readJogView({ getItem: () => "not json" }), { open: false, step: "1" });
  assert.deepEqual(readJogView({ getItem: () => { throw new Error("blocked"); } }), { open: false, step: "1" });
  const kept: string[] = [];
  writeJogView({ setItem: (_k, v) => void kept.push(v) }, { open: true, step: "50" });
  assert.deepEqual(JSON.parse(kept[0]), { open: true, step: "50" });
  assert.doesNotThrow(() => writeJogView({ setItem: () => { throw new Error("full"); } }, { open: false, step: "1" }));
});

test("criterion 19: Jogging is a state of its own: the line reads it, only Stop applies, it counts as active", () => {
  assert.deepEqual(jobStateLine(status({ name: "Jogging" }), null), { text: "Jogging", failure: false });
  assert.deepEqual(jobButtons({ name: "Jogging" }), { send: false, start: false, startName: "Start", pause: false, stop: true });
  assert.equal(jobIsActive({ name: "Jogging" }), true);
  assert.equal(jobLineAlerts(status({ name: "Jogging" })), false, "not read out: each step would be (W23)");
  const line = controlsLine(entry(status({ name: "Jogging" }, { job: { job_id: JOB, line: 0, lines: 6, elapsed_ms: 0 } })));
  assert.equal(line?.text, "Jogging");
  assert.equal(line?.bar, null, "no bar, no counts line");
  assert.equal(homeButton(entry(status({ name: "Jogging" }))).cause, "Jogging");
  assert.equal(homeButton(entry(status({ name: "Jogging" }))).enabled, false);
});

test("criterion 17: error 15 is named in the state line as a failure that answers the press", () => {
  const text = "Jog beyond the travel (error 15)";
  for (const name of ["Idle", "Ready"] as const) {
    const s = status({ name }, { detail: text });
    assert.deepEqual(jobStateLine(s, null), { text, failure: true }, name);
    assert.equal(jobLineAlerts(s), true, name);
  }
});

test("criteria 8, 20: the arrows' causes, the first that applies, and none for laser mode or Not homed", () => {
  const cause = (s: StatusMessage | null, own = false, extra: Partial<LinkEntry> = {}) => jogCause(entry(s, extra), own);
  assert.equal(cause(status({ name: "Idle" })), null);
  assert.equal(cause(status({ name: "Idle" }, { homed: false })), null, "an unhomed jog runs (criterion 17)");
  assert.equal(cause(status({ name: "Ready" })), null);
  assert.equal(cause(status({ name: "Done", result: "Completed" })), null);
  assert.equal(cause(status({ name: "Homing" })), "Homing");
  assert.equal(cause(status({ name: "Jogging" })), "Jogging", "another app's jog");
  assert.equal(cause(status({ name: "Jogging" }), true), null, "this pad's own jog queues one step");
  assert.equal(cause(status({ name: "Running" })), "A job is running");
  assert.equal(cause(status({ name: "Paused" })), "A job is running");
  assert.equal(cause(status({ name: "Receiving" })), "Sending");
  assert.equal(cause(status({ name: "Alarm" })), "Alarm");
  assert.equal(cause(status({ name: "NoController" })), "No controller");
  assert.equal(cause(status({ name: "NoController" }, { detail: "Port /dev/ttyUSB0 not found" })), "Port /dev/ttyUSB0 not found");
  assert.equal(cause(status({ name: "Idle" }, { detail: "Controller state: Door" })), "Controller state: Door");
  assert.equal(cause(status({ name: "Idle" }), false, { lost: true }), "Connection lost");
  assert.equal(cause(null, false, { reading: true }), "Testing");
  assert.equal(
    cause(status({ name: "AwaitingLocalStart" }), false, { session: owner }),
    "Waiting for the start at the machine",
  );
  assert.equal(
    cause(status({ name: "AwaitingLocalStart" }, { requested_by: "Phone" }), false, { session: owner }),
    "Home requested by Phone",
  );
});

test("criterion 14: a work offset on the controller is told, a zero one is not", () => {
  assert.equal(workOffsetNotice(status({ name: "Idle" })), null);
  assert.equal(workOffsetNotice(status({ name: "Idle" }, { work_offset: [0, 0, 0] })), null);
  assert.equal(workOffsetNotice(status({ name: "Idle" }, { work_offset: [0.0005, -0.0005, 0] })), null);
  assert.equal(
    workOffsetNotice(status({ name: "Idle" }, { work_offset: [100, 5.25, 0] })),
    "Work offset on the controller: X 100, Y 5.25",
  );
});

test("criterion 37, W25 to W27: a member's waiting Home reads for each app and an owner's Home slot confirms it", () => {
  const waiting = status({ name: "AwaitingLocalStart" }, { requested_by: "Phone" });
  assert.equal(waitingText(waiting, member), "Waiting for Marc to confirm", "the requester");
  assert.equal(waitingText(waiting, { ...member, clientName: "Tablet" }), "Phone waits for Marc to confirm", "another member");
  assert.equal(waitingText(waiting, owner), "Home requested by Phone");
  // A member's start still names the job; a waiting Home has none and is no Confirm start.
  const start = status({ name: "AwaitingLocalStart" }, { requested_by: "Phone", job: { job_id: JOB, line: 0, lines: 6, elapsed_ms: 0 } });
  assert.equal(waitingText(start, owner), "Start requested by Phone");
  assert.equal(confirmKind(start, owner), "Confirm start");
  assert.equal(confirmKind(waiting, owner), null);
  assert.equal(confirmsHome(waiting, owner), true);
  assert.equal(confirmsHome(waiting, member), false, "a member never confirms");
  assert.equal(confirmsHome(start, owner), false);
  assert.equal(confirmsHome(status({ name: "AwaitingLocalStart" }), owner), false, "the foreground wait for Enter");
  const owners = homeButton(entry(waiting, { session: owner }));
  assert.deepEqual(owners, { enabled: true, busy: false, cause: null, name: "Confirm home" });
  const members = homeButton(entry(waiting, { session: member }));
  assert.equal(members.enabled, false);
  assert.equal(members.cause, "Waiting for Marc to confirm");
  assert.equal(jobButtons({ name: "AwaitingLocalStart" }, false, null).start, false, "Start is greyed meanwhile");
});

test("W25: a Home that was thrown away reads 'Not started: Home' for the member that asked, and the Ready job stays", () => {
  const waiting = status({ name: "AwaitingLocalStart" }, { requested_by: "Phone" });
  assert.equal(discardedName(waiting, status({ name: "Idle" }), member, false), "Home");
  assert.equal(discardedName(waiting, status({ name: "Ready" }), member, false), "Home", "a kept job");
  assert.equal(discardedName(waiting, status({ name: "Idle" }), member, true), null, "its own Stop says nothing");
  assert.equal(discardedName(waiting, status({ name: "Idle" }), owner, false), null);
  assert.equal(discardedName(waiting, status({ name: "Homing" }), member, false), null, "confirmed, not discarded");
  const line = controlsLine(entry(status({ name: "Ready" }), { discarded: "Home" }));
  assert.equal(line?.text, "Not started: Home");
  assert.equal(line?.muted, true);
});

test("criteria 12, 15: a member's app has no pad and the toggle is never greyed; presses go to the host's jog commands", () => {
  const controls = source("components/panel/MachineControls.tsx");
  assert.match(controls, /session\?\.role !== "Member"/);
  assert.match(controls, /enabled: true,\s*busy: false,\s*toggle:/);
  const hook = source("hooks/useMachineControls.ts");
  for (const command of ["connector_jog_hold", "connector_jog_keep", "connector_jog_release"]) {
    assert.ok(hook.includes(`"${command}"`), command);
  }
  assert.match(hook, /command: "jog"/);
  const pad = source("components/panel/JogPad.tsx");
  assert.match(pad, /aria-label="Jog"/);
  assert.match(pad, /role="radiogroup"/);
  // No hint or tip text in the pad: only names of controls and states.
  assert.doesNotMatch(pad, /[Tt]ip:|[Hh]int|Hold to|Press and hold/);
});

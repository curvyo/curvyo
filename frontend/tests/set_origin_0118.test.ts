// `specs/0118-machine-control`, slice 2c: Set origin here's causes (criterion 29, UX notes W40), the
// Start pick's fourth value and its User origin row (W42, W43), and the canvas marks' JSON (W46,
// W47). The session holds the rules of the point itself; these are the host's.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { type LinkEntry, type LinkHost, createLinkStore } from "../src/lib/connectorLink.ts";
import { type JobState, type StatusMessage } from "../src/lib/connectorText.ts";
import { setOriginCause } from "../src/lib/machineControls.ts";
import { decodeOutputGroupText, decodeUserOriginMarks, userOriginWords } from "../src/lib/outputView.ts";

const status = (state: JobState, extra: Partial<StatusMessage> = {}): StatusMessage => ({
  type: "Status",
  state,
  job: null,
  position: [120, 85.5, 0],
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
const cause = (s: StatusMessage | null, extra: Partial<LinkEntry> = {}) => setOriginCause(entry(s, extra));

test("criterion 29, W40: Set origin here is a read, enabled wherever a homed position is reported", () => {
  for (const state of [
    { name: "Idle" },
    { name: "Ready" },
    { name: "Done", result: "Completed" },
    { name: "Receiving" },
    { name: "Paused" },
  ] as JobState[]) {
    assert.equal(cause(status(state)), null, state.name);
  }
});

test("criterion 29, W40: the causes, the first that applies", () => {
  assert.equal(cause(status({ name: "Idle" }), { lost: true }), "Connection lost");
  assert.equal(cause(status({ name: "NoController" } as JobState, { detail: "No controller on COM3" })), "No controller on COM3");
  assert.equal(cause(status({ name: "Idle" }, { position: null })), "No position from the machine");
  for (const name of ["Running", "Homing", "Jogging", "Framing"]) {
    assert.equal(cause(status({ name } as JobState)), "Machine is moving", name);
  }
  assert.equal(cause(status({ name: "Alarm" } as JobState)), "Alarm");
  assert.equal(cause(status({ name: "Idle" }, { detail: "Controller state: Door" })), "Controller state: Door");
  assert.equal(cause(status({ name: "Idle" }, { homed: false })), "Not homed");
  assert.equal(cause(status({ name: "Idle" }, { homed: false, position: null })), "No position from the machine");
});

test("W41: the point is said once in millimetres with two decimals", () => {
  assert.equal(userOriginWords(120, 85.5), "User origin X 120.00 mm, Y 85.50 mm");
});

const group = (start: unknown) =>
  JSON.stringify({
    machine: "Diode 10 W · GRBL",
    start,
    checks: [],
    total: null,
    export: { enabled: false, tooltip: null, file_type: "GRBL G-code", extension: "gcode" },
    toolpath_view: false,
    frame: { enabled: false, cause: "Nothing to frame" },
  });
const options = [
  { value: "machine_origin", label: "Machine origin" },
  { value: "head_position", label: "Head position" },
  { value: "home_then_origin", label: "Home, then machine origin" },
  { value: "user_origin", label: "User origin" },
];

test("criterion 30, W42, W43: User origin is the fourth value and its row carries the point", () => {
  const picked = decodeOutputGroupText(group({ value: "user_origin", damaged: false, options, user_origin: { x: 120, y: 85.5 } }));
  assert.ok(picked);
  assert.equal(picked.start.value, "user_origin");
  assert.equal(picked.start.options.at(-1)?.label, "User origin");
  assert.deepEqual(picked.start.userOrigin, { x: 120, y: 85.5 });
  const other = decodeOutputGroupText(group({ value: "machine_origin", damaged: false, options: options.slice(0, 3) }));
  assert.ok(other);
  assert.equal(other.start.userOrigin, null, "no row with another pick");
});

test("criterion 32, W46, W47: the marks decode, and unreadable text is no marks", () => {
  assert.deepEqual(decodeUserOriginMarks('{"cross":[10,20],"box":[10,5,30,15]}'), { cross: [10, 20], box: [10, 5, 30, 15] });
  assert.deepEqual(decodeUserOriginMarks('{"cross":null,"box":null}'), { cross: null, box: null });
  assert.deepEqual(decodeUserOriginMarks("nope"), { cross: null, box: null });
});

test("W39, W51: Set origin here sits in the centre cell, its own Tab stop outside the arrows' roving", () => {
  const pad = readFileSync(new URL("../src/components/panel/JogPad.tsx", import.meta.url), "utf8");
  assert.match(pad, /col-start-2 row-start-2/);
  assert.match(pad, /LocateFixed/);
  assert.match(pad, /aria-label="Set origin here"/);
  const origin = pad.slice(pad.indexOf("function SetOriginHere"), pad.indexOf("function StepPick"));
  assert.doesNotMatch(origin, /tabIndex/, "always a Tab stop of its own");
});

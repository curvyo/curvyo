// `specs/0127-connector-grbl-serial`: the words the Connector row and the Output group show for a
// real GRBL controller (UX notes sections 2 to 8): failures with their cause, a lost job, an
// unconfirmed stop, the Controller block's rows and the laser-mode refusal.
import assert from "node:assert/strict";
import { test } from "node:test";

import { type LinkHost, createLinkStore } from "../src/lib/connectorLink.ts";
import {
  type ControllerInfo,
  type JobState,
  type StatusMessage,
  controllerRows,
  clearsRefusal,
  controllerWord,
  isLossText,
  jobLineAlerts,
  jobStateLine,
  refusalText,
} from "../src/lib/connectorText.ts";
import { controlsLine as stateLine } from "../src/lib/machineControls.ts";
import { LINE_CHARS } from "../src/lib/sendLine.ts";

const JOB = "11111111-1111-4111-8111-111111111111";

const status = (state: JobState, detail: string | null = null, line = 11): StatusMessage => ({
  type: "Status",
  state,
  job: state.name === "Done" || state.name === "Running" ? { job_id: JOB, line, lines: 412, elapsed_ms: 0 } : null,
  position: null,
  alarm: null,
  ...(detail === null ? {} : { detail }),
});

const host: LinkHost = {
  invoke: async () => undefined as never,
  listen: async () => () => undefined,
};
const entry = (s: StatusMessage | null) => ({ ...createLinkStore(host).get("x"), status: s });

test("G7, G5: a failed job shows its cause, in the row and the Output group alike", () => {
  const failed = status({ name: "Done", result: "Failed" }, "Failed at line 12: No feed rate (error 22)");
  assert.deepEqual(jobStateLine(failed, null), { text: "Failed at line 12: No feed rate (error 22)", failure: true });
  assert.equal(stateLine(entry(failed))?.text, "Failed at line 12: No feed rate (error 22)");
  assert.equal(stateLine(entry(failed))?.failure, true);
  // The fake controller names no cause: as built.
  assert.equal(jobStateLine(status({ name: "Done", result: "Failed" }), null)?.text, "Failed at line 11");
  assert.equal(jobLineAlerts(failed), false, "a failed line is a polite status (V9)");
});

test("G15, G12: a lost job and an unconfirmed stop are alerts and stay after the reconnect", () => {
  for (const text of ["Lost the controller at line 120", "Lost the controller", "Could not confirm the machine stopped"]) {
    assert.equal(isLossText(text), true);
    const lost = status({ name: "NoController" }, text);
    assert.deepEqual(jobStateLine(lost, null), { text, failure: true });
    assert.equal(jobLineAlerts(lost), true);
    const back = status({ name: "Idle" }, text);
    assert.deepEqual(jobStateLine(back, null), { text, failure: true }, "V7: stays in Idle");
    assert.equal(stateLine(entry(back))?.text, text);
    assert.equal(stateLine(entry(back))?.bar, null, "no bar with a loss");
    assert.ok(text.length <= LINE_CHARS);
  }
  const after = entry(status({ name: "NoController" }, "Lost the controller at line 120"));
  assert.equal(stateLine(after)?.text, "Lost the controller at line 120");
  assert.equal(stateLine(after)?.alerts, true);
});

test("G2, G3: why there is no controller shows in the footer's one line (since 0118 §4 row 5)", () => {
  const open = status({ name: "NoController" }, "Port /dev/ttyUSB0 not found");
  assert.deepEqual(jobStateLine(open, null), { text: "Port /dev/ttyUSB0 not found", failure: true });
  assert.equal(jobLineAlerts(open), false);
  assert.equal(stateLine(entry(open))?.text, "Port /dev/ttyUSB0 not found");
  assert.equal(stateLine(entry(open))?.alerts, false, "polite");
  assert.deepEqual(jobStateLine(status({ name: "NoController" }), null), { text: "No controller", failure: false });
  assert.equal(jobStateLine(status({ name: "Idle" }), null), null);
});

test("G19, G4: the Controller block's rows", () => {
  const info: ControllerInfo = {
    name: "GRBL",
    firmware: "1.1h",
    port: "/dev/ttyUSB0",
    baud: 115200,
    settings: { laser_mode: true, s_max: 1000, travel: [406.4, 300] },
  };
  const idle = { ...status({ name: "Idle" }, "Controller state: Door"), position: [120, 45.25, 0] as [number, number, number] };
  assert.equal(controllerWord(idle), "Door");
  assert.deepEqual(controllerRows(info, idle), [
    ["Controller", "GRBL 1.1h"],
    ["Port", "/dev/ttyUSB0, 115200 baud"],
    ["State", "Door"],
    ["Laser mode", "On"],
    ["Max power", "1,000"],
    ["Travel", "406.4 × 300 mm"],
    ["Position", "X 120.0, Y 45.3 mm"],
  ]);
  const nearZero = { ...status({ name: "Idle" }), position: [-0.04, -12.36, 0] as [number, number, number] };
  assert.deepEqual(controllerRows(info, nearZero).at(-1), ["Position", "X 0.0, Y -12.4 mm"], "never -0.0");
  const fake: ControllerInfo = { name: "Fake controller", firmware: "curvyo-fake 0" };
  assert.deepEqual(controllerRows(fake, status({ name: "Idle" })), [["Controller", "Fake controller curvyo-fake 0"]]);
  // "Controller state: Door" is the State row, never a job line.
  assert.equal(jobStateLine(idle, null), null);
});

test("G34: laser mode off is a refusal in words", () => {
  assert.equal(refusalText("LaserModeOff", null), "Laser mode is off ($32=0)");
  assert.equal(refusalText("LaserModeUnknown", null), "Laser mode unknown ($32 not read)");
});

test("G33: an alarm or a lost controller ends a shown refusal, so the alarm line and Unlock show", () => {
  const idle = status({ name: "Idle" });
  assert.equal(clearsRefusal(idle, { ...status({ name: "Alarm" }), alarm: "Locked, not homed" }), true);
  assert.equal(clearsRefusal(idle, status({ name: "NoController" }, "Port /dev/ttyUSB0 not found")), true);
  // A refused send still survives its own late Receiving and Idle pushes.
  assert.equal(clearsRefusal(idle, status({ name: "Receiving" })), false);
  assert.equal(clearsRefusal(status({ name: "Receiving" }), idle), false);
  // No change of state, no clearing.
  assert.equal(clearsRefusal(status({ name: "NoController" }), status({ name: "NoController" })), false);
});

// `specs/0121-print-and-cut`: the pure helpers of the Print and cut group (typed measures, the
// Take head position state, the view decoding).
import assert from "node:assert/strict";
import { test } from "node:test";

import { decodeJson, headPositionState, parseMeasure, stateIsWarning } from "../src/lib/printAndCutView.ts";

test("a typed measure takes a point or a comma and a minus sign within 10000 mm", () => {
  assert.equal(parseMeasure("83.4"), 83.4);
  assert.equal(parseMeasure("-12,5"), -12.5);
  assert.equal(parseMeasure("  "), null);
  assert.equal(parseMeasure("abc"), undefined);
  assert.equal(parseMeasure("-"), undefined);
  assert.equal(parseMeasure("10000.01"), undefined);
});

test("Take head position is greyed while the machine moves or reports no position", () => {
  assert.deepEqual(headPositionState("Running", true), { enabled: false, cause: "Machine is moving" });
  assert.deepEqual(headPositionState("Homing", true), { enabled: false, cause: "Machine is moving" });
  assert.deepEqual(headPositionState("Idle", false), { enabled: false, cause: "No position from the machine" });
  assert.deepEqual(headPositionState("Idle", true), { enabled: true });
});

test("the view decodes JSON and tolerates none", () => {
  assert.deepEqual(decodeJson<{ a: number }>('{"a":1}'), { a: 1 });
  assert.equal(decodeJson(undefined), null);
  assert.equal(decodeJson("{"), null);
});

test("Enter moves to the next field and Take head position to the next unmeasured mark", async () => {
  const { nextField, nextUnmeasured } = await import("../src/lib/printAndCutView.ts");
  const keys = ["1x", "1y", "2x", "2y", "tolerance", "apply"];
  assert.equal(nextField(keys, "1x"), "1y");
  assert.equal(nextField(keys, "2y"), "tolerance");
  assert.equal(nextField(keys, "apply"), null);
  const rows = [
    { number: 1, x: "1.00", kind: "mark" },
    { number: 2, x: null, kind: "mark" },
    { number: 3, x: null, kind: "mark" },
    { number: 4, x: "4.00", kind: "mark" },
  ];
  assert.equal(nextUnmeasured(rows, 2), 3);
  assert.equal(nextUnmeasured(rows, 4), 2);
  assert.equal(nextUnmeasured([{ number: 1, x: null, kind: "mark" }], 1), null);
});

test("Take head position reads in every state with a position but not during a jog or a controller state (criterion 8)", () => {
  assert.deepEqual(headPositionState("Done", true), { enabled: true });
  assert.deepEqual(headPositionState("Paused", true), { enabled: true });
  assert.deepEqual(headPositionState("Alarm", true), { enabled: true });
  assert.deepEqual(headPositionState("Idle", true, "Jog"), { enabled: false, cause: "Machine is moving" });
  assert.deepEqual(headPositionState("Idle", true, "Check"), { enabled: false, cause: "Controller state: Check" });
});

test("the state line is a warning only for too close (2 or more measured rows, no fit)", () => {
  const row = (number: number, x: string | null) => ({ number, x, y: x, kind: "mark" as const, note: null });
  assert.equal(stateIsWarning({ state: "Measure at least 2 marks", rows: [row(1, "1.00"), row(2, null)] }), false);
  assert.equal(stateIsWarning({ state: "Marks too close to fit", rows: [row(1, "1.00"), row(2, "1.20")] }), true);
  assert.equal(stateIsWarning({ state: null, rows: [row(1, "1.00"), row(2, "90.00")] }), false);
});

test("the canvas tags keep only well-formed entries", async () => {
  const { readPrintAndCutLabels } = await import("../src/lib/printAndCutView.ts");
  const text = JSON.stringify([
    { kind: "mark", text: "1", x: 1, y: 2 },
    { kind: "fit", text: "Fit", x: 3, y: 4 },
    { kind: "other", text: "x", x: 0, y: 0 },
    { kind: "ring", text: 2, x: 0, y: 0 },
  ]);
  assert.deepEqual(
    readPrintAndCutLabels(text).map((l) => l.kind),
    ["mark", "fit"],
  );
  assert.deepEqual(readPrintAndCutLabels("{"), []);
});

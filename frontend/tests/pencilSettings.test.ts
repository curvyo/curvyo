// The Pencil bar's rules (`specs/0158-pen-display` slice 1 criterion 4, UX notes §2, §4).
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  PENCIL_DEFAULTS,
  afterStroke,
  applyPencilSettings,
  widthsDimmed,
  withMax,
  withMin,
} from "../src/lib/pencilSettings.ts";

test("Min and Max keep their ranges and Max never falls below Min", () => {
  assert.equal(withMax(PENCIL_DEFAULTS, 0.01).max, 0.2, "raised to Min");
  const raised = withMin(PENCIL_DEFAULTS, 5);
  assert.deepEqual([raised.min, raised.max], [5, 5], "Min above Max raises Max");
  assert.equal(withMin(PENCIL_DEFAULTS, 100).min, 20);
  assert.equal(withMax(PENCIL_DEFAULTS, 100).max, 50);
  assert.equal(withMin(PENCIL_DEFAULTS, 0).min, 0.05);
  assert.equal(withMin(PENCIL_DEFAULTS, Number.NaN).min, 0.05);
});

test("the width group is dimmed until a pen with pressure or Simulate pressure", () => {
  assert.equal(widthsDimmed(PENCIL_DEFAULTS), true, "mouse only");
  assert.equal(widthsDimmed({ ...PENCIL_DEFAULTS, simulate: true }), false);
  const pen = afterStroke(PENCIL_DEFAULTS, true, 1);
  assert.equal(widthsDimmed(pen), false);
  const flat = afterStroke(pen, true, 0);
  assert.equal(flat.noPressure, true, "No pen pressure");
  assert.equal(widthsDimmed(flat), true);
  assert.equal(afterStroke(flat, false, 0), flat, "a mouse stroke changes nothing");
  assert.equal(afterStroke(flat, true, 1).noPressure, false, "a stroke with pressure clears it");
});

test("the values reach the session with Smoothing as mm", () => {
  const calls: string[] = [];
  applyPencilSettings({
    set_pencil_smoothing: (mm) => (calls.push(`smoothing ${mm}`), true),
    set_pencil_simulate: (on) => void calls.push(`simulate ${on}`),
    set_pencil_min_mm: (mm) => void calls.push(`min ${mm}`),
    set_pencil_max_mm: (mm) => void calls.push(`max ${mm}`),
    set_pencil_curve: (name) => (calls.push(`curve ${name}`), true),
  });
  assert.deepEqual(calls, ["smoothing 0.4", "simulate false", "min 0.2", "max 2", "curve linear"]);
});

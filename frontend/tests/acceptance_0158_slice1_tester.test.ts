// Tester checks for 0158 slice 1 on the frontend: the sample feed, the bar states, the rail slot.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import {
  PENCIL_DEFAULTS,
  afterStroke,
  widthsDimmed,
  withMax,
  withMin,
} from "../src/lib/pencilSettings.ts";
import { PEN_SAMPLE_STRIDE, PenSampleFeed, type PointerLike } from "../src/platform/penSamples.ts";

const RECT = { left: 0, top: 0, width: 800, height: 600 };
const read = (p: string) => readFileSync(new URL(p, import.meta.url), "utf8");

function ev(type: string, x: number, y: number, extra: Partial<PointerLike> = {}): PointerLike {
  return { type, pointerType: "pen", clientX: x, clientY: y, pressure: 0.5, tiltX: 0, tiltY: 0,
    buttons: type === "pointerup" ? 0 : 1, timeStamp: 1, ...extra };
}

test("a mouse stroke has NaN pressure even where the browser reports 0.5", () => {
  const f = new PenSampleFeed();
  f.pointer(ev("pointerdown", 5, 5, { pointerType: "mouse", pressure: 0.5 }), RECT);
  f.pointer(ev("pointerup", 9, 9, { pointerType: "mouse", pressure: 0 }), RECT);
  const flush = f.take()!;
  assert.ok(Number.isNaN(flush.flat[2]));
  assert.ok(!f.penSeen);
  assert.ok(flush.ended && !flush.penEnded);
});

test("pointercancel ends the contact as phase out; touch adds nothing", () => {
  const f = new PenSampleFeed();
  f.pointer(ev("pointerdown", 5, 5), RECT);
  f.pointer(ev("pointercancel", 6, 6), RECT);
  f.pointer(ev("pointermove", 7, 7, { pointerType: "touch" }), RECT);
  const flush = f.take()!;
  assert.equal(flush.flat.length, 2 * PEN_SAMPLE_STRIDE);
  assert.equal(flush.flat[PEN_SAMPLE_STRIDE + 6], 2);
  assert.ok(flush.ended);
});

test("a hovering pen is a hover record and a take with nothing is null", () => {
  const f = new PenSampleFeed();
  assert.equal(f.take(), null);
  f.pointer(ev("pointermove", 5, 5, { buttons: 0, pressure: 0 }), RECT);
  const flush = f.take()!;
  assert.equal(flush.flat[6], 0);
  assert.ok(!flush.ended);
});

test("a stroke over many frames equals one batch of records", () => {
  const one = new PenSampleFeed();
  const many = new PenSampleFeed();
  const events = [ev("pointerdown", 1, 1), ...[2, 3, 4, 5].map((i) => ev("pointermove", i, i)), ev("pointerup", 5, 5)];
  for (const e of events) one.pointer(e, RECT);
  const all = [...one.take()!.flat];
  const parts: number[] = [];
  for (const e of events) {
    many.pointer(e, RECT);
    parts.push(...(many.take()?.flat ?? []));
  }
  assert.deepEqual(parts, all);
});

test("pen bar states: dimming matrix and No pen pressure", () => {
  const s = PENCIL_DEFAULTS;
  assert.equal(widthsDimmed(s), true, "mouse only");
  assert.equal(widthsDimmed({ ...s, simulate: true }), false);
  const seen = afterStroke(s, true, 1);
  assert.equal(widthsDimmed(seen), false);
  const none = afterStroke(seen, true, 0);
  assert.ok(none.noPressure && widthsDimmed(none));
  assert.equal(widthsDimmed({ ...none, simulate: true }), false);
  assert.ok(!afterStroke(none, true, 1).noPressure, "goes after the next stroke with pressure");
  assert.equal(afterStroke(none, false, 1), none, "a mouse stroke changes nothing");
  assert.equal(afterStroke(s, true, -1), s);
  const hi = withMin(s, 5);
  assert.ok(hi.max >= 5);
  assert.ok(withMax(hi, 0.1).max >= hi.min);
  assert.equal(withMin(s, Number.NaN).min, 0.05);
});

test("rail: Pencil is the eighth Tools slot after Text, key P, column A scrolls", () => {
  const rail = read("../src/components/ToolRail.tsx");
  const iText = rail.indexOf('tool="text"');
  const iPencil = rail.indexOf('tool="pencil"');
  assert.ok(iText > 0 && iPencil > iText);
  assert.match(rail, /name="Pencil tool \(P\)"/);
  assert.match(rail, /overflow-y-auto/);
  const tools = rail.slice(0, iPencil).match(/<ToolButton\s/g) ?? [];
  assert.ok(tools.length >= 7, `Pencil should be at least the 8th ToolButton, saw ${tools.length + 1}`);
});

test("bar: names, Pen probe button and the status label", () => {
  const bar = read("../src/components/PencilBar.tsx");
  for (const s of ["Pencil options", "Pencil smoothing", "Simulate pressure", "Minimum width", "Maximum width",
    "Pressure curve", "No pen pressure", "Pen probe...", 'role="status"', "openPenProbe"]) {
    assert.ok(bar.includes(s), s);
  }
});

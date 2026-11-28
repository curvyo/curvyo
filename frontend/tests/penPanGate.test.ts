// The Pan toggle stops the pen sample feed (`specs/0158-pen-display` slice 2a): while Pan is on, no
// pen sample reaches the session from native batches or Pointer Events; switching Pan on cancels a
// stroke in progress; switching it off resumes drawing with the next contact.
import assert from "node:assert/strict";
import { test } from "node:test";

import type { NativeBatch } from "../src/lib/penProbe.ts";
import { PenPanGate } from "../src/lib/penPanGate.ts";
import { PenSampleFeed, type PointerLike } from "../src/platform/penSamples.ts";

const RECT = { left: 100, top: 50, width: 800, height: 600 };

function pointer(type: string, x: number, y: number): PointerLike {
  return {
    type,
    pointerType: "pen",
    clientX: x,
    clientY: y,
    pressure: 0.4,
    tiltX: 0,
    tiltY: 0,
    buttons: type === "pointerup" ? 0 : 1,
    timeStamp: 10,
  };
}

function native(x: number, y: number): NativeBatch {
  return {
    samples: [{ x, y, pressure: 0.7, tilt: null, end: "tip", phase: "contact", barrel: 0, time_ms: 1000 }],
    dropped: 0,
    device: "test pen",
    age_ms: null,
  };
}

function setup() {
  const state = { pan: false, cancels: 0 };
  const gate = new PenPanGate(new PenSampleFeed(), () => state.pan);
  const frame = () => gate.take(() => (state.cancels += 1));
  return { state, gate, frame };
}

test("with Pan off, native batches and Pointer Events reach the session", () => {
  const { gate, frame } = setup();
  gate.pointer(pointer("pointerdown", 110, 60), RECT);
  gate.native(native(111, 61), RECT);
  assert.ok(frame(), "samples are sent");
});

test("with Pan on, neither route sends a sample", () => {
  const { state, gate, frame } = setup();
  state.pan = true;
  gate.pointer(pointer("pointerdown", 110, 60), RECT);
  gate.pointer(pointer("pointermove", 120, 70), RECT);
  gate.native(native(121, 71), RECT);
  gate.pointer(pointer("pointerup", 120, 70), RECT);
  assert.equal(frame(), null);
  assert.equal(frame(), null);
});

test("switching Pan on mid-stroke cancels the stroke once and drops its queued samples", () => {
  const { state, gate, frame } = setup();
  gate.pointer(pointer("pointerdown", 110, 60), RECT);
  gate.native(native(111, 61), RECT);
  state.pan = true;
  assert.equal(frame(), null, "the queued samples are dropped");
  assert.equal(state.cancels, 1);
  frame();
  assert.equal(state.cancels, 1, "cancelled once, not every frame");
});

test("switching Pan off resumes drawing with the next contact", () => {
  const { state, gate, frame } = setup();
  state.pan = true;
  gate.pointer(pointer("pointerdown", 110, 60), RECT);
  frame();
  state.pan = false;
  gate.pointer(pointer("pointerup", 110, 60), RECT);
  frame();
  gate.pointer(pointer("pointerdown", 130, 80), RECT);
  gate.native(native(131, 81), RECT);
  const flush = frame();
  assert.ok(flush, "the next contact is sent");
  assert.equal(state.cancels, 1);
});

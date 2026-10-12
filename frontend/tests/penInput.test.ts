// Run with `npm test`. How a Pencil pointer event becomes the sample batch of one wasm call
// (`specs/0032-pen-tablet-input` criteria 1, 2 and 15).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  pencilSampleValues,
  penReadoutText,
  type PencilPointerEvent,
  SAMPLE_VALUES,
} from "../src/lib/penInput.ts";

const ORIGIN = { left: 100, top: 50 };

function reading(x: number, pressure: number): PencilPointerEvent {
  return {
    clientX: 100 + x,
    clientY: 60,
    pressure,
    tiltX: 12,
    tiltY: -3,
    timeStamp: x,
  };
}

test("a move with five coalesced events gives five samples, the event's values in order", () => {
  const coalesced = [1, 2, 3, 4, 5].map((x) => reading(x, x / 10));
  const event = { ...reading(5, 0.5), getCoalescedEvents: () => coalesced };
  const values = pencilSampleValues(event, ORIGIN, true);
  assert.equal(values.length, 5 * SAMPLE_VALUES);
  assert.deepEqual(Array.from(values.slice(0, SAMPLE_VALUES)), [1, 10, 0.1, 12, -3, 1]);
  assert.deepEqual(Array.from(values.slice(4 * SAMPLE_VALUES)), [5, 10, 0.5, 12, -3, 5]);
});

test("a move without getCoalescedEvents gives one sample", () => {
  const values = pencilSampleValues(reading(7, 0.3), ORIGIN, true);
  assert.equal(values.length, SAMPLE_VALUES);
  assert.equal(values[0], 7);
});

test("an empty coalesced list falls back to the event itself", () => {
  const event = { ...reading(7, 0.3), getCoalescedEvents: () => [] };
  assert.equal(pencilSampleValues(event, ORIGIN, true).length, SAMPLE_VALUES);
});

test("a press or release takes only the event, never its coalesced list", () => {
  const event = { ...reading(5, 0.5), getCoalescedEvents: () => [reading(1, 0.1), reading(2, 0.2)] };
  assert.equal(pencilSampleValues(event, ORIGIN, false).length, SAMPLE_VALUES);
});

test("missing tilt reads as zero", () => {
  const event = { clientX: 110, clientY: 60, pressure: 0.5, timeStamp: 3 } as PencilPointerEvent;
  assert.deepEqual(Array.from(pencilSampleValues(event, ORIGIN, false)), [10, 10, 0.5, 0, 0, 3]);
});

test("the readout reads pressure to two places and tilt in whole degrees", () => {
  assert.equal(penReadoutText(0.623, 12.4, -3), "Pen: pressure 0.62, tilt 12° / −3°");
  assert.equal(penReadoutText(1, 0, 0), "Pen: pressure 1.00, tilt 0° / 0°");
});

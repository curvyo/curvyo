// The pen sample feed (`specs/0158-pen-display` slice 1 criteria 10, 11; ADR 0095 decision 2):
// synthetic Pointer Events, coalesced events, touch, the eraser end and native batches become the
// session's flat records, one flush per frame.
import assert from "node:assert/strict";
import { test } from "node:test";

import type { NativeBatch } from "../src/lib/penProbe.ts";
import { PEN_SAMPLE_STRIDE, PenSampleFeed, type PointerLike } from "../src/platform/penSamples.ts";

const RECT = { left: 100, top: 50, width: 800, height: 600 };

function pointer(type: string, x: number, y: number, extra: Partial<PointerLike> = {}): PointerLike {
  return {
    type,
    pointerType: "pen",
    clientX: x,
    clientY: y,
    pressure: 0.4,
    tiltX: 45,
    tiltY: -9,
    buttons: type === "pointerup" ? 0 : 1,
    timeStamp: 10,
    ...extra,
  };
}

function records(flat: Float64Array): number[][] {
  const out: number[][] = [];
  for (let i = 0; i < flat.length; i += PEN_SAMPLE_STRIDE) out.push([...flat.slice(i, i + PEN_SAMPLE_STRIDE)]);
  return out;
}

test("a pen stroke becomes contact records in canvas px, then a hover that ends it", () => {
  const feed = new PenSampleFeed();
  feed.pointer(pointer("pointerdown", 110, 60), RECT);
  feed.pointer(pointer("pointermove", 120, 70, { timeStamp: 20 }), RECT);
  feed.pointer(pointer("pointerup", 120, 70, { timeStamp: 30, pressure: 0 }), RECT);
  const flush = feed.take();
  assert.ok(flush);
  const r = records(flush.flat);
  assert.equal(r.length, 3);
  assert.deepEqual(r[0], [10, 10, 0.4, 0.5, -0.1, 0, 1, 0, 10]);
  assert.equal(r[1][6], 1);
  assert.equal(r[2][6], 0, "the release is a hover sample");
  assert.ok(flush.ended && flush.penEnded);
  assert.ok(feed.penSeen);
  assert.equal(feed.take(), null, "nothing left for the next frame");
});

test("coalesced events each become a record", () => {
  const feed = new PenSampleFeed();
  feed.pointer(pointer("pointerdown", 110, 60), RECT);
  const coalesced = [1, 2, 3].map((k) => pointer("pointermove", 110 + k, 60, { timeStamp: 10 + k, pressure: 0.1 * k }));
  feed.pointer({ ...pointer("pointermove", 113, 60), getCoalescedEvents: () => coalesced }, RECT);
  const r = records(feed.take()!.flat);
  assert.equal(r.length, 4);
  assert.deepEqual(r.slice(1).map((x) => x[0]), [11, 12, 13]);
  assert.deepEqual(r.slice(1).map((x) => x[8]), [11, 12, 13]);
});

test("a mouse has no pressure or tilt, and touch is ignored", () => {
  const feed = new PenSampleFeed();
  feed.pointer(pointer("pointerdown", 110, 60, { pointerType: "touch" }), RECT);
  assert.equal(feed.take(), null);
  feed.pointer(pointer("pointerdown", 110, 60, { pointerType: "mouse", pressure: 0.5 }), RECT);
  feed.pointer(pointer("pointerup", 110, 60, { pointerType: "mouse" }), RECT);
  const flush = feed.take()!;
  const [down] = records(flush.flat);
  assert.ok(Number.isNaN(down[2]) && Number.isNaN(down[3]) && Number.isNaN(down[4]));
  assert.ok(flush.ended && !flush.penEnded);
  assert.equal(feed.penSeen, false);
});

test("the eraser end and the barrel button", () => {
  const feed = new PenSampleFeed();
  feed.pointer(pointer("pointerdown", 110, 60, { buttons: 32 | 2 }), RECT);
  feed.pointer(pointer("pointerup", 110, 60), RECT);
  const [down, up] = records(feed.take()!.flat);
  assert.equal(down[5], 1, "eraser end");
  assert.equal(down[7], 1, "barrel bit");
  assert.equal(up[5], 1, "the lift keeps the end");
  assert.equal(up[6], 0);
});

function nativeBatch(samples: [number, number, "hover" | "contact" | "out"][]): NativeBatch {
  return {
    samples: samples.map(([x, y, phase], i) => ({ x, y, pressure: 0.7, tilt: null, end: "tip", phase, barrel: 0, time_ms: 1000 + i })),
    dropped: 0,
    device: "test pen",
    age_ms: null,
  };
}

test("native samples carry the contact once they arrive and end it", () => {
  const feed = new PenSampleFeed();
  feed.pointer(pointer("pointerdown", 110, 60), RECT);
  feed.native(nativeBatch([[111, 61, "contact"], [112, 62, "contact"]]), RECT);
  feed.pointer(pointer("pointermove", 113, 63), RECT);
  feed.native(nativeBatch([[114, 64, "hover"]]), RECT);
  feed.pointer(pointer("pointerup", 114, 64), RECT);
  const r = records(feed.take()!.flat);
  assert.deepEqual(r.map((x) => [x[0], x[6]]), [[10, 1], [11, 1], [12, 1], [14, 0], [14, 0]]);
  assert.equal(r[1][2], 0.7);
});

test("a release ends a native contact after one frame when the native end does not come", () => {
  const feed = new PenSampleFeed();
  feed.pointer(pointer("pointerdown", 110, 60), RECT);
  feed.native(nativeBatch([[111, 61, "contact"]]), RECT);
  feed.pointer(pointer("pointerup", 111, 61), RECT);
  const first = feed.take()!;
  assert.equal(first.ended, false, "waits one frame for the last native batch");
  const second = feed.take()!;
  const [out] = records(second.flat);
  assert.equal(out[6], 2);
  assert.ok(second.ended);
});

test("native contact without a press on the canvas and hover outside it are dropped", () => {
  const feed = new PenSampleFeed();
  feed.native(nativeBatch([[50, 20, "contact"], [50, 20, "hover"], [150, 80, "hover"]]), RECT);
  const r = records(feed.take()!.flat);
  assert.equal(r.length, 1);
  assert.deepEqual([r[0][0], r[0][1], r[0][6]], [50, 30, 0]);
});

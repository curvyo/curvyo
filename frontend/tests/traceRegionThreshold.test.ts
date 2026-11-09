// `specs/0089` UX notes §25.2: switching Inside / Outside (any new mask region) changes the
// Threshold at most once and never through a default. The field's values are recorded over a
// switch with a fake runner whose analysis answers when the test says so.

import { strict as assert } from "node:assert";
import { test } from "node:test";

import { type RegionRead, readRegion, regionAnswer, viewValues } from "../src/lib/traceMaskCaches.ts";
import { readAnalysis } from "../src/lib/traceAnalysis.ts";
import { firstChoice, type TraceValues } from "../src/lib/traceText.ts";

function answer(threshold: number): RegionRead {
  return {
    threshold: threshold + 7,
    background: "#FFFFFF",
    analysis: JSON.stringify({ type: "logo", capture: "digital", confidence: 0.9, low: false, method: 0, threshold, background: "#FFFFFF", reasons: [], properties: {} }),
  };
}

/** A runner whose `defaults` answers only when `resolve` is called; counts its calls. */
function fakeRunner() {
  const runner = {
    calls: 0,
    resolve: (() => undefined) as (value: RegionRead) => void,
    reject: (() => undefined) as (reason: unknown) => void,
    defaults(): Promise<RegionRead> {
      runner.calls += 1;
      return new Promise((resolve, reject) => {
        runner.resolve = resolve;
        runner.reject = reject;
      });
    },
  };
  return runner;
}

/** The field's values as shown, consecutive repeats collapsed. */
function recorder(start: TraceValues) {
  const shown: (number | null)[] = [start.threshold];
  return {
    shown,
    see(v: TraceValues) {
      if (shown[shown.length - 1] !== v.threshold) {
        shown.push(v.threshold);
      }
    },
  };
}

const start = (): TraceValues => ({ ...firstChoice().values, threshold: 128, backgroundAuto: true, background: "#EEEEEE" });

test("§25.2: a switch shows the old value until the region's analysis answers, then the new one, once", async () => {
  const cache = new Map<string, RegionRead>();
  const runner = fakeRunner();
  let values = start();
  const field = recorder(values);
  // The sync after the switch: the same image, a new region.
  values = viewValues(values, true, undefined, undefined);
  field.see(values);
  assert.equal(values.background, "#EEEEEE", "the background is not blanked either");
  const pending = readRegion(cache, "img", '{"outside":true}', () => runner.defaults());
  field.see(values);
  runner.resolve(answer(90));
  const read = await pending;
  values = regionAnswer(values, readAnalysis(read.analysis), new Set());
  field.see(values);
  assert.deepEqual(field.shown, [128, 90], "old value, then the new one, nothing between");
});

test("§25.2: a new image starts from its kept Threshold (or none), not from the last image's", () => {
  assert.equal(viewValues(start(), false, 77, "#000000").threshold, 77);
  assert.equal(viewValues(start(), false, undefined, undefined).threshold, null);
  assert.equal(viewValues(start(), false, 77, "#000000").background, "#000000");
});

test("§25.2: a failed region analysis keeps the old value, no second change", async () => {
  let values = start();
  const field = recorder(values);
  values = viewValues(values, true, undefined, undefined);
  values = regionAnswer(values, readAnalysis(""), new Set());
  field.see(values);
  assert.deepEqual(field.shown, [128]);
});

test("§25.2: a value the maker sets before the answer wins; the answer does not touch it", async () => {
  const cache = new Map<string, RegionRead>();
  const runner = fakeRunner();
  let values = viewValues(start(), true, undefined, undefined);
  const pending = readRegion(cache, "img", '{"outside":true}', () => runner.defaults());
  values = { ...values, threshold: 100 };
  runner.resolve(answer(90));
  const read = await pending;
  values = regionAnswer(values, readAnalysis(read.analysis), new Set<keyof TraceValues>(["threshold"]));
  assert.equal(values.threshold, 100);
});

test("§25.2: a region analysed earlier in the session answers from the cache, without a new read", async () => {
  const cache = new Map<string, RegionRead>();
  const runner = fakeRunner();
  const first = readRegion(cache, "img", '{"outside":true}', () => runner.defaults());
  runner.resolve(answer(90));
  await first;
  const inside = readRegion(cache, "img", '{"outside":false}', () => runner.defaults());
  runner.resolve(answer(60));
  assert.equal(readAnalysis((await inside).analysis)?.threshold, 60);
  const again = await readRegion(cache, "img", '{"outside":true}', () => runner.defaults());
  assert.equal(readAnalysis(again.analysis)?.threshold, 90);
  assert.equal(runner.calls, 2, "the region read before is not read again");
  // Another image with the same mask is its own region.
  void readRegion(cache, "other", '{"outside":true}', () => runner.defaults());
  assert.equal(runner.calls, 3);
});

test("§25.2: a read that failed is not kept, so the region is read again", async () => {
  const cache = new Map<string, RegionRead>();
  const runner = fakeRunner();
  const failed = readRegion(cache, "img", "m", () => runner.defaults());
  runner.reject(new Error("worker died"));
  await assert.rejects(failed);
  void readRegion(cache, "img", "m", () => runner.defaults());
  assert.equal(runner.calls, 2);
});

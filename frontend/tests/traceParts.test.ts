// `specs/0094` slice 2 (criteria 25, 26, 32): the parts' jobs traced one by one with a cache per
// part, one rising progress over the traced parts, a stop that joins nothing, and the chips' names.

import { strict as assert } from "node:assert";
import { test } from "node:test";

import type { TraceOutput, TracePixels } from "../src/lib/traceJobs.ts";
import { firstChoice } from "../src/lib/traceText.ts";
import {
  PartCache,
  type PartJob,
  type PartsRunner,
  type PartsSession,
  type TracePartInfo,
  joinTooltip,
  mayCrop,
  partEmptyText,
  partKey,
  partName,
  partTooltip,
  partsCount,
  readParts,
  removePartText,
  traceParts,
} from "../src/lib/traceParts.ts";

const pixels = (): TracePixels => ({ width: 2, height: 2, rgba: new Uint8Array(16), map: [1, 0, 0, 1, 0, 0] });

function fakes(stop?: { at: number; flag: { stale: boolean } }) {
  const calls: { settings: string; lines: string; crop: boolean }[] = [];
  const crops: boolean[] = [];
  const joins: (Float64Array | null)[][] = [];
  const session: PartsSession = {
    pixels: (_index, crop) => {
      crops.push(crop);
      return pixels();
    },
    join: (wires) => {
      joins.push(wires);
      return new Float64Array([0, wires.length, 0, 0, 0, 0, 0]);
    },
  };
  const runner: PartsRunner = {
    trace: async (_pixels, settings, onProgress, lines = ""): Promise<TraceOutput> => {
      calls.push({ settings, lines, crop: crops[crops.length - 1] });
      onProgress?.(50);
      onProgress?.(100);
      if (stop && calls.length === stop.at) {
        stop.flag.stale = true;
      }
      return { wire: new Float64Array([0, 1, 4, 0, 0, 0, 0, calls.length]), seconds: 0.25 };
    },
  };
  return { session, runner, calls, joins };
}

const job = (settings: string, text = false): PartJob => ({ settings, text, key: partKey(settings, text, false), crop: !text });

test("criterion 25: after one part's setting changes only that part is traced again", async () => {
  const { session, runner, calls, joins } = fakes();
  const cache = new PartCache();
  const first = await traceParts(session, runner, [job("a"), job("b"), null], cache, "", () => undefined, () => false);
  assert.equal(calls.length, 2, "both parts traced once");
  assert.equal(first?.seconds, 0.5);
  assert.equal(joins[0].length, 3);
  assert.equal(joins[0][2], null, "a part that adds nothing has no wire");
  calls.length = 0;
  await traceParts(session, runner, [job("a"), job("b2"), null], cache, "", () => undefined, () => false);
  assert.equal(calls.length, 1, "a trace-call counter of 1");
  assert.equal(calls[0].settings, "b2");
  assert.equal(joins.length, 2, "the preview shows the joined result");
  calls.length = 0;
  await traceParts(session, runner, [job("a"), job("b2"), null], cache, "", () => undefined, () => false);
  assert.equal(calls.length, 0, "nothing changed: everything from the cache");
});

test("criterion 26: one progress from 0 to 100 over the traced parts, never backwards", async () => {
  const { session, runner } = fakes();
  const cache = new PartCache();
  cache.set(job("kept").key, new Float64Array(7));
  const seen: number[] = [];
  await traceParts(session, runner, [job("a"), job("kept"), job("b"), job("c")], cache, "", (p) => seen.push(p), () => false);
  assert.ok(seen.length > 2);
  assert.ok(seen.every((p, i) => i === 0 || p > seen[i - 1]), `${seen}`);
  assert.equal(seen[seen.length - 1], 100);
  assert.ok(seen[0] <= 20, `${seen}`);
});

test("criterion 26: a stop during the parts joins nothing", async () => {
  const flag = { stale: false };
  const { session, runner, calls, joins } = fakes({ at: 1, flag });
  const out = await traceParts(session, runner, [job("a"), job("b")], new PartCache(), "", () => undefined, () => flag.stale);
  assert.equal(out, null);
  assert.equal(calls.length, 1, "the later parts are not traced");
  assert.equal(joins.length, 0, "no preview, no result, no document change");
});

test("criterion 30 and 31: the read lines go to the parts that read text, the jobs that may be cropped are", async () => {
  const { session, runner, calls } = fakes();
  await traceParts(session, runner, [job("a", true), job("b")], new PartCache(), "LINES", () => undefined, () => false);
  assert.deepEqual(
    calls.map((c) => [c.lines, c.crop]),
    [
      ["LINES", false],
      ["", true],
    ],
  );
});

test("the cache keeps at most its limit, the oldest leaving first", () => {
  const cache = new PartCache(2);
  cache.set("a", new Float64Array(1));
  cache.set("b", new Float64Array(1));
  cache.set("c", new Float64Array(1));
  assert.equal(cache.get("a"), undefined);
  assert.ok(cache.get("b") && cache.get("c"));
  assert.notEqual(partKey("s", false, true), partKey("s", false, false));
  assert.notEqual(partKey("s", true, false), partKey("s", false, false));
});

const part = (over: Partial<TracePartInfo>): TracePartInfo => ({
  id: 0,
  paths: 0,
  outside: false,
  whole: false,
  muted: false,
  active: false,
  contributes: true,
  mask: "",
  ...over,
});

test("criterion 32: a chip is named by its number, mode, paths and state", () => {
  assert.equal(partName(1, 3, part({ paths: 2, active: true })), "Part 2 of 3, inside, 2 paths, active");
  assert.equal(partName(2, 3, part({ paths: 1, outside: true })), "Part 3 of 3, outside, 1 path");
  assert.equal(partName(0, 3, part({ whole: true })), "Part 1 of 3, whole image");
  assert.equal(partName(1, 2, part({ muted: true })), "Part 2 of 2, no paths");
  assert.equal(partTooltip(1, 3, part({ paths: 2, active: true })), "Part 2 of 3, inside, 2 paths");
});

test("the counts and the trash name follow the parts", () => {
  const one = [part({ active: true, whole: true })];
  const three = [part({ whole: true }), part({ paths: 1 }), part({ muted: true, active: true })];
  assert.equal(partsCount(1), "");
  assert.equal(partsCount(3), "3 parts");
  assert.equal(partEmptyText(three), "Part 3 has no paths");
  assert.equal(partEmptyText(one), "");
  assert.equal(joinTooltip(3), "Create paths (Enter), 3 parts joined");
  assert.equal(removePartText(1, false), "Remove part 2");
  assert.equal(removePartText(1, true), "Press again to remove part 2");
  assert.equal(readParts("").length, 1);
  assert.equal(readParts('[{"id":4,"paths":1,"outside":true,"whole":false,"muted":false,"active":true,"contributes":true,"mask":""}]')[0].id, 4);
});

test("criterion 31: a part is cut out only without text, Drop border shapes and Denoise", () => {
  const v = firstChoice().values;
  assert.equal(mayCrop(v, false), v.denoise === 0 && !v.ignoreBackground);
  assert.equal(mayCrop({ ...v, denoise: 0, ignoreBackground: false }, false), true);
  assert.equal(mayCrop({ ...v, denoise: 0, ignoreBackground: false }, true), false, "text reads the whole picture");
  assert.equal(mayCrop({ ...v, denoise: 1, ignoreBackground: false }, false), false, "Denoise: the fit of a shape across the cut is not local");
  assert.equal(mayCrop({ ...v, denoise: 0, ignoreBackground: true }, false), false, "Drop border shapes: the cut's edge is no border");
});

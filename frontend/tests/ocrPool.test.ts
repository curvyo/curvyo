// `specs/0091` round 2: the text lines of one picture shared over the reading pool and put back
// in their order.

import { strict as assert } from "node:assert";
import { test } from "node:test";

import { MAX_POOL, mergeReads, poolPercent, poolSize, readingCost, shareLines } from "../src/lib/ocrPool.ts";

test("0091 pool: one fewer worker than cores, at most four, at least one", () => {
  assert.equal(poolSize(8), MAX_POOL);
  assert.equal(poolSize(4), 3);
  assert.equal(poolSize(2), 1);
  assert.equal(poolSize(1), 1);
  assert.equal(poolSize(undefined), 1);
});

test("0091 pool: every line read once, the long ones spread, each share in line order", () => {
  const widths = [300, 4000, 200, 2500, 100, 900, 3000];
  const shares = shareLines(widths.map(readingCost), 3);
  assert.deepEqual(shares.flat().sort((a, b) => a - b), [0, 1, 2, 3, 4, 5, 6]);
  for (const share of shares) {
    assert.deepEqual(share, [...share].sort((a, b) => a - b));
  }
  // The three longest lines go to three different workers.
  const owner = (line: number) => shares.findIndex((s) => s.includes(line));
  assert.equal(new Set([owner(1), owner(3), owner(6)]).size, 3);
  // Fewer lines than workers: no empty share.
  assert.deepEqual(shareLines([10], 4), [[0]]);
  assert.deepEqual(shareLines([], 4), []);
});

test("0091 pool: the reads come back in line order, empty reads left out", () => {
  const shares = [[0, 3], [1, 2, 4]];
  const reads = [["a", null], ["b", "c", "e"]];
  assert.deepEqual(mergeReads(shares, reads), ["a", "b", "c", "e"]);
  assert.throws(() => mergeReads(shares, [["a"], ["b", "c", "e"]]));
});

test("0091 pool: one percent from the detection's 20 to 100 over the lines", () => {
  assert.equal(poolPercent(0, 78), 20);
  assert.equal(poolPercent(39, 78), 60);
  assert.equal(poolPercent(78, 78), 100);
  assert.equal(poolPercent(0, 0), 20);
});

test("0091 pool: lines of equal cost are dealt in turn", () => {
  assert.deepEqual(shareLines([1, 1, 1, 1, 1], 2), [[0, 2, 4], [1, 3]]);
  assert.equal(readingCost(300), 1024);
  assert.equal(readingCost(2000), 2000);
});

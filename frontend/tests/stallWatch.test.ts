// The share and switch watchdog (customer feedback on PR #281): progress keeps it alive, a
// standstill of 30 s fails it, never a long upload that keeps moving.
import assert from "node:assert/strict";
import { test } from "node:test";

import { STALL_MS, lookAt, startWatch } from "../src/lib/stallWatch.ts";

test("a standstill fails once it lasted the stall time", () => {
  const watch = startWatch("waiting", 1000);
  assert.equal(lookAt(watch, "waiting", 1000 + STALL_MS - 1).stalled, false);
  assert.equal(lookAt(watch, "waiting", 1000 + STALL_MS).stalled, true);
});

test("progress restarts the watch, so a long upload that moves never fails", () => {
  let watch = startWatch("uploading:0", 0);
  for (let done = 1; done <= 10; done += 1) {
    const look = lookAt(watch, `uploading:${done}`, done * (STALL_MS - 1));
    assert.equal(look.stalled, false);
    watch = look.watch;
  }
  assert.equal(watch.key, "uploading:10");
  assert.equal(lookAt(watch, "uploading:10", 10 * (STALL_MS - 1) + STALL_MS).stalled, true);
});

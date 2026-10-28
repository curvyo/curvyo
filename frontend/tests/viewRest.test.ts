// `specs/0025-raster-trace` criterion 56: the view moves on a change and rests after 150 ms
// without one.

import { strict as assert } from "node:assert";
import { test } from "node:test";

import { VIEW_REST_MS, ViewRest } from "../src/lib/viewRest.ts";

test("criterion 56: a change moves the view, 150 ms without one rests it", () => {
  const rest = new ViewRest();
  assert.equal(rest.update("a", 0), null, "the first frame only remembers");
  assert.equal(rest.update("a", 16), null);
  assert.equal(rest.update("b", 32), true, "moving");
  assert.equal(rest.update("c", 48), null, "still moving");
  assert.equal(rest.update("c", 48 + VIEW_REST_MS - 1), null);
  assert.equal(rest.update("c", 48 + VIEW_REST_MS), false, "rested");
  assert.equal(rest.update("c", 1000), null);
  assert.equal(rest.update("d", 1016), true);
});

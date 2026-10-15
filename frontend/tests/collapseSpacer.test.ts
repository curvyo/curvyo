// The spacer of the switch back to Original's (specs/0057-clone-overrides, UX review must-fix 2).

import assert from "node:assert/strict";
import test from "node:test";

import { collapseSpacer } from "../src/lib/collapseSpacer.ts";

test("the spacer is the height the body lost between the press and the collapse", () => {
  assert.equal(collapseSpacer(704, 554), 150);
  assert.equal(collapseSpacer(620.4, 84.1), 536);
});

test("a body that did not shrink gets no spacer", () => {
  assert.equal(collapseSpacer(300, 300), 0);
  assert.equal(collapseSpacer(300, 420), 0);
  assert.equal(collapseSpacer(Number.NaN, 100), 0);
});

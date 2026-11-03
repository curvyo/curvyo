import assert from "node:assert/strict";
import { test } from "node:test";

import { parseDeleted, parseDiskPlan, parsePlace, sharedStates, trashChoice } from "../src/lib/sharedTree.ts";

test("a deleted document's file is trashed, the open one closed first or held while unsaved", () => {
  assert.equal(trashChoice(["Lid.curvyo"], null, true), "trash");
  assert.equal(trashChoice(["Lid.curvyo"], ["Lid.curvyo"], false), "close-then-trash");
  assert.equal(trashChoice(["Lid.curvyo"], ["Lid.curvyo"], true), "hold");
});

test("plans, places and the Deleted list read what the session answers", () => {
  assert.equal(parseDiskPlan(""), null);
  const plan = parseDiskPlan('{"steps":[{"kind":"trash","path":["Sides"]}],"missing":[["Lid.curvyo"]]}');
  assert.deepEqual(plan?.steps, [{ kind: "trash", path: ["Sides"] }]);
  assert.deepEqual(plan?.outside, []);
  assert.deepEqual(parsePlace('{"document":"11","path":["Lid.curvyo"]}'), { kind: "file", path: ["Lid.curvyo"] });
  assert.deepEqual(parsePlace('{"document":"11","deleted":true}'), { kind: "deleted" });
  assert.deepEqual(parsePlace(""), { kind: "unknown" });
  assert.deepEqual(parseDeleted("not json"), []);
  assert.equal(sharedStates(null, []), '{"shared":false}');
  assert.deepEqual(JSON.parse(sharedStates(plan, [["New.curvyo"]])), {
    shared: true,
    outside: [],
    missing: [["Lid.curvyo"]],
    receiving: [["New.curvyo"]],
  });
});

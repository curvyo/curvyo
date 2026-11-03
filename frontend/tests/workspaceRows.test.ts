import assert from "node:assert/strict";
import test from "node:test";

import {
  DOCUMENT_ROW_PX,
  FOLDER_ROW_PX,
  MAX_ROW_ELEMENTS,
  indentPx,
  isInside,
  movedPath,
  parentOf,
  parsePlan,
  parseProbe,
  parseWorkspaceView,
  pathKey,
  rowOffsets,
  rowWindow,
  samePath,
  targetFolder,
  thumbnailKey,
} from "../src/lib/workspaceRows.ts";

const rows = [
  { path: ["Sides"], kind: "folder", name: "Sides", level: 1, posinset: 1, setsize: 3, expanded: true, count: 1, time: "", label: "" },
  { path: ["Sides", "Left.curvyo"], kind: "document", name: "Left", level: 2, posinset: 1, setsize: 1, expanded: false, count: 0, time: "12:03", label: "" },
  { path: ["Lid.curvyo"], kind: "document", name: "Lid", level: 1, posinset: 2, setsize: 3, expanded: false, count: 0, time: "12:03", label: "" },
] as const;

test("folder rows are 32 px and document rows 44 px, 36 and 48 with a coarse pointer (ux-notes 3)", () => {
  const view = parseWorkspaceView(JSON.stringify({ documents: 2, header: "2 documents", rows }));
  assert.deepEqual(rowOffsets(view.rows, false), [0, 32, 76, 120]);
  assert.deepEqual(rowOffsets(view.rows, true), [0, 36, 84, 132]);
  assert.equal(FOLDER_ROW_PX, 32);
  assert.equal(DOCUMENT_ROW_PX, 44);
});

test("an unreadable view is an empty tree", () => {
  assert.deepEqual(parseWorkspaceView("nope"), { documents: 0, header: "0 documents", rows: [] });
});

test("the window of 500 rows never holds more than 60 elements and follows the scroll (criterion 5)", () => {
  const many = Array.from({ length: 500 }, (_, index) => ({
    ...rows[2],
    path: [`Doc ${index}.curvyo`],
  }));
  const offsets = rowOffsets(many, false);
  for (const scrollTop of [0, 3000, 500 * 44]) {
    const { first, count } = rowWindow(offsets, scrollTop, 300);
    assert.ok(count <= MAX_ROW_ELEMENTS, `${count}`);
    assert.ok(first >= 0 && first + count <= 500);
    const top = Math.min(scrollTop, offsets[500] - 300);
    assert.ok(offsets[first] <= top && offsets[first + count] >= top + 300, `${scrollTop}`);
  }
  assert.deepEqual(rowWindow([0], 0, 300), { first: 0, count: 0 });
});

test("paths compare by component and know their folder", () => {
  assert.equal(pathKey(["a", "b"]), '["a","b"]');
  assert.ok(samePath(["a"], ["a"]));
  assert.ok(!samePath(["a"], ["a", "b"]));
  assert.deepEqual(parentOf(["a", "b.curvyo"]), ["a"]);
  assert.ok(isInside(["a", "b"], ["a"]));
  assert.ok(isInside(["a"], ["a"]));
  assert.ok(!isInside(["ab"], ["a"]));
  assert.equal(thumbnailKey(["a.curvyo"], "12:03"), '["a.curvyo"]@12:03');
  assert.deepEqual(movedPath(["a", "b.curvyo"], ["a"], ["c", "a"]), ["c", "a", "b.curvyo"]);
  assert.deepEqual(movedPath(["x.curvyo"], ["a"], ["c"]), ["x.curvyo"]);
});

test("New document acts on the focused folder, the folder of a focused document, or the root", () => {
  assert.deepEqual(targetFolder(rows[0]), ["Sides"]);
  assert.deepEqual(targetFolder(rows[1]), ["Sides"]);
  assert.deepEqual(targetFolder(rows[2]), []);
  assert.deepEqual(targetFolder(null), []);
});

test("rows indent 12 px per level up to level 6 (ux-notes 3)", () => {
  assert.equal(indentPx(1), 0);
  assert.equal(indentPx(3), 24);
  assert.equal(indentPx(12), 72);
});

test("plans and probes read what the session and the host answer", () => {
  const plan = parsePlan('{"op":{"kind":"move","from":["a"],"to":["b","a"]},"name":"a","status":"Moved a to b."}');
  assert.equal(plan.kind, "planned");
  assert.ok(plan.kind === "planned" && plan.status === "Moved a to b.");
  assert.deepEqual(parsePlan('{"refusal":"Enter a name."}'), { kind: "refused", text: "Enter a name." });
  assert.deepEqual(parsePlan("{}"), { kind: "refused", text: "" });
  assert.deepEqual(parseProbe({ error: "damaged" }), { kind: "unreadable", code: "damaged" });
  assert.deepEqual(parseProbe({ png: new Uint8Array() }), { kind: "readable", png: null });
  assert.deepEqual(parseProbe({ png: new Uint8Array([137]) }), { kind: "readable", png: new Uint8Array([137]) });
  assert.equal(parseProbe({ error: "not-found" }), null);
});

// The Operations view toggle of a laser page's label (`specs/0111-laser-operation-tools`
// criterion 15, `ux-notes.md` 5): it shows on laser pages only, pressed or not as the session says.
import { test } from "node:test";
import assert from "node:assert/strict";
import { OPERATIONS_VIEW_NAME, operationsToggle, readPages } from "../src/lib/pages.ts";

test("a laser page's label carries the toggle with its state; other pages carry none", () => {
  const pages = readPages(
    JSON.stringify([
      { id: "a", operationsView: true },
      { id: "b", operationsView: false },
      { id: "c", operationsView: null },
      { id: "d" },
    ]),
  );
  assert.deepEqual(pages.map((page) => operationsToggle(page)), [{ pressed: true }, { pressed: false }, null, null]);
});

test("the toggle keeps one constant name; the state is aria-pressed", () => {
  assert.equal(OPERATIONS_VIEW_NAME, "Operations view");
});

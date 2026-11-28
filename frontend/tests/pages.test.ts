// The Operations view toggle of a laser page's label (`specs/0111-laser-operation-tools`
// criterion 15, `ux-notes.md` 5): it shows on laser pages only, pressed or not as the session says.
import { test } from "node:test";
import assert from "node:assert/strict";
import { OPERATIONS_VIEW_NAME, operationsToggle, pageLabelTop, readPages } from "../src/lib/pages.ts";

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

// `specs/0159-ui-consistency` F50: the page label is never under a bar of any height.
test("the page label sits above its frame, or just below a bar that covers that place", () => {
  const page = { screenX: 200, screenY: 72, screenWidth: 400, screenHeight: 500 };
  assert.equal(pageLabelTop(page, null), -20, "no bar: 20 px above the frame");
  const oneRow = { left: 152, right: 700, bottom: 48 };
  assert.equal(pageLabelTop(page, oneRow), -20, "a one-row bar leaves the label above the frame");
  const threeRows = { left: 152, right: 470, bottom: 120 };
  assert.equal(pageLabelTop(page, threeRows), 52, "4 px below a three-row bar");
  const aside = { left: 700, right: 900, bottom: 120 };
  assert.equal(pageLabelTop(page, aside), -20, "a bar beside the label leaves it");
  const scrolledAway = { ...page, screenY: -480 };
  assert.equal(pageLabelTop(scrolledAway, threeRows), -20, "a page that ends under the bar keeps its place");
});

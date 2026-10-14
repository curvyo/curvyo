import assert from "node:assert/strict";
import test from "node:test";

import {
  MAX_ROW_ELEMENTS,
  ROW_HEIGHT_PX,
  dragBadge,
  dropPlace,
  edgeScroll,
  idText,
  indentPx,
  nextRow,
  parseLayerNotice,
  parseLayersView,
  parseReply,
  reorderFor,
  listWindow,
  toolbarStep,
  withExtraRow,
  windowRange,
} from "../src/lib/layerRows.ts";

test("the window never holds more than 60 rows, even for 10,000 (criterion 48)", () => {
  for (const scrollTop of [0, 5000, 10_000 * ROW_HEIGHT_PX]) {
    const { first, count } = windowRange(scrollTop, 418, 10_000);
    assert.ok(count <= MAX_ROW_ELEMENTS - 1, `${count}`);
    assert.ok(first >= 0 && first + count <= 10_000);
  }
  const end = windowRange(10_000 * ROW_HEIGHT_PX, 418, 10_000);
  assert.equal(end.first + end.count, 10_000, "a jump to the end shows the last rows");
  assert.deepEqual(windowRange(0, 418, 0), { first: 0, count: 0 });
  assert.deepEqual(windowRange(0, 418, 3), { first: 0, count: 3 });
});

test("the arrows, Home, End and the page keys move the roving row", () => {
  assert.equal(nextRow("ArrowDown", 0, 5, 3), 1);
  assert.equal(nextRow("ArrowUp", 0, 5, 3), 0);
  assert.equal(nextRow("End", 0, 5, 3), 4);
  assert.equal(nextRow("PageDown", 3, 5, 3), 4);
  assert.equal(nextRow("x", 0, 5, 3), null);
  assert.equal(nextRow("ArrowDown", 0, 0, 3), null);
});

test("the Alt keys name their reorder (criterion 32)", () => {
  assert.equal(reorderFor("ArrowUp"), "forward");
  assert.equal(reorderFor("ArrowLeft"), "out");
  assert.equal(reorderFor("ArrowRight"), "in");
  assert.equal(reorderFor("End"), "back");
  assert.equal(reorderFor("a"), null);
  // The browser build takes Shift+Alt+Home and Shift+Alt+End; the desktop takes Alt+Home.
  assert.equal(reorderFor("Home", false, true), null);
  assert.equal(reorderFor("Home", true, true), "front");
  assert.equal(reorderFor("End", true, true), "back");
  assert.equal(reorderFor("Home", false, false), "front");
  assert.equal(reorderFor("ArrowUp", false, true), "forward", "Alt+Up needs no Shift");
});

test("a drop goes above, below or into a container row (criterion 29)", () => {
  assert.equal(dropPlace(2, 28, true), "above");
  assert.equal(dropPlace(14, 28, true), "into");
  assert.equal(dropPlace(26, 28, true), "below");
  assert.equal(dropPlace(13, 28, false), "above");
  assert.equal(dropPlace(15, 28, false), "below");
});

test("the list scrolls in one row-height zone at each edge", () => {
  assert.ok(edgeScroll(105, 100, 400, 28) < 0);
  assert.ok(edgeScroll(490, 100, 400, 28) > 0);
  assert.equal(edgeScroll(300, 100, 400, 28), 0);
});

test("indent, badge, ids and parsing", () => {
  assert.equal(indentPx(1), 0);
  assert.equal(indentPx(2), 16);
  assert.equal(indentPx(20), 96, "deeper rows keep six levels of indent");
  assert.equal(dragBadge(1), "");
  assert.equal(dragBadge(12), "12");
  assert.equal(idText({ peer: "1", counter: 3 }), '{"peer":"1","counter":3}');
  assert.equal(idText(null), "");
  assert.equal(parseLayersView("not json").count, 0);
  assert.equal(parseLayersView('{"count":3,"rows":[]}').count, 3);
  assert.deepEqual(parseReply('{"kind":"notice","text":"Deleted layer Cut."}'), {
    kind: "notice",
    text: "Deleted layer Cut.",
  });
  assert.deepEqual(parseReply(""), { kind: "", text: "" });
});

test("a name with markup is data, never markup (criterion 17)", () => {
  const view = parseLayersView(
    JSON.stringify({ count: 1, rows: [{ name: "<img src=x onerror=alert(1)>" }] }),
  );
  assert.equal(view.rows[0]?.name, "<img src=x onerror=alert(1)>");
});

test("an empty layer notice is never shown (customer feedback 2026-10-14)", () => {
  assert.equal(parseLayerNotice(""), null, "no notice");
  assert.equal(parseLayerNotice('{"kind":"","text":""}'), null, "an empty one is dropped too");
  assert.equal(parseLayerNotice("not json"), null);
  assert.deepEqual(parseLayerNotice('{"kind":"status","text":"Already at the front"}'), {
    kind: "status",
    text: "Already at the front",
  });
  assert.deepEqual(
    parseLayerNotice('{"kind":"alert","text":"Layer Cut is locked. Unlock it or choose another layer."}'),
    { kind: "alert", text: "Layer Cut is locked. Unlock it or choose another layer." },
  );
});

test("the view carries the toolbar and the opacity bar, with safe defaults", () => {
  const empty = parseLayersView("");
  assert.deepEqual(empty.toolbar, {
    newLayer: true,
    deleteLayer: false,
    moveUp: false,
    moveDown: false,
    duplicate: false,
    linkedClone: false,
  });
  assert.equal(empty.opacity.bar, 0);
});

test("the toolbar's roving focus moves with Left, Right, Home and End, without wrapping", () => {
  assert.equal(toolbarStep("ArrowRight", 0, 5), 1);
  assert.equal(toolbarStep("ArrowRight", 4, 5), 4, "no wrap at the end");
  assert.equal(toolbarStep("ArrowLeft", 0, 5), 0, "no wrap at the start");
  assert.equal(toolbarStep("Home", 3, 5), 0);
  assert.equal(toolbarStep("End", 1, 5), 4);
  assert.equal(toolbarStep("ArrowDown", 1, 5), null);
});

test("the window follows the scroll and the focused row comes on its own (0049 UX must-fix 9)", () => {
  // 4,096 rows, scrolled to the end, the focus on row 1: the rows on screen are read.
  const range = windowRange(4096 * 28 - 400, 400, 4096, 28);
  const window = listWindow(range, 1, 4096);
  assert.equal(window.first, range.first);
  assert.ok(window.first + window.count >= 4096 - 14, "the window holds the rows on screen");
  assert.equal(window.extra, 1, "the focused row is read on its own");
  assert.ok(window.count + 1 <= 60, "never more than 60 row elements");
  // With the focus inside the window nothing extra is read.
  assert.equal(listWindow(range, range.first + 2, 4096).extra, null);
  const row = (index: number) => ({ index }) as never;
  const merged = withExtraRow(
    { ...parseLayersView(""), rows: [row(4090), row(4091)] },
    { ...parseLayersView(""), rows: [row(1)] },
  );
  assert.deepEqual(
    merged.rows.map((r) => (r as { index: number }).index),
    [4090, 4091, 1],
  );
});

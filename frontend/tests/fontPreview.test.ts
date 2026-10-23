// `specs/0077-font-preview`: the preview's stored state, sample rule, geometry, words and keys.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  DEFAULT_SAMPLE,
  DEFAULT_SIZE,
  FONT_PREVIEW_KEY,
  type PreviewStorage,
  clampSize,
  readPreviewPrefs,
  writePreviewPrefs,
} from "../src/lib/fontPreviewState.ts";
import { followedSample, openingSample } from "../src/lib/fontPreviewSample.ts";
import {
  drawingLoss,
  drawingWidthDelta,
  gridColumns,
  headerRows,
  heldFull,
  minColumnWidth,
  PANE_CHROME_PX,
  oneColumnPane,
  paneLayout,
  SPLIT_KEY_STEP_PX,
  splitAfterDrag,
  switchedView,
  rowsInPage,
  sampleAreaHeight,
  tileHeight,
  tileTagRoom,
  TILE_DRAW_CAPS,
  TILE_FOOTER_PX,
  sameGridArea,
  tileColumnCss,
  GRID_PAD_PX,
} from "../src/lib/fontPreviewLayout.ts";
import { missingText, sizeTip, subjectLine, tileDescription, tileName } from "../src/lib/fontPreviewText.ts";
import { canvasLeavesKey, escapeClosesPreview, gridMove, tileTagKey, typesIntoSample } from "../src/lib/fontPreviewKeys.ts";
import { latestPerFrame } from "../src/lib/latestPerFrame.ts";
import { PRERENDER_MAX, createPrerenderer, prerenderOrder } from "../src/lib/fontPreviewPrerender.ts";

function memory(): PreviewStorage & { values: Map<string, string> } {
  const values = new Map<string, string>();
  return { values, getItem: (key) => values.get(key) ?? null, setItem: (key, value) => void values.set(key, value) };
}

const throwing: PreviewStorage = {
  getItem: () => {
    throw new Error("blocked");
  },
  setItem: () => {
    throw new Error("blocked");
  },
};

// Criterion 6 and adrs.md decision 4.
test("a first start, broken and throwing storage give the defaults", () => {
  const defaults = { sample: "Anna & Jonas 2026", size: 32, full: false };
  assert.equal(FONT_PREVIEW_KEY, "curvyo.font-preview");
  assert.deepEqual(readPreviewPrefs(memory()), defaults);
  assert.deepEqual(readPreviewPrefs(throwing), defaults);
  const broken = memory();
  broken.values.set(FONT_PREVIEW_KEY, "{not json");
  assert.deepEqual(readPreviewPrefs(broken), defaults);
  assert.doesNotThrow(() => writePreviewPrefs(defaults, throwing));
});

test("sample, size and full view are read back; out-of-range values fall back one by one", () => {
  const storage = memory();
  writePreviewPrefs({ sample: "Jürgen", size: 96, full: true }, storage);
  assert.deepEqual(readPreviewPrefs(storage), { sample: "Jürgen", size: 96, full: true });
  storage.values.set(FONT_PREVIEW_KEY, JSON.stringify({ sample: "x".repeat(201), size: 400, full: "yes" }));
  assert.deepEqual(readPreviewPrefs(storage), { sample: DEFAULT_SAMPLE, size: DEFAULT_SIZE, full: false });
  storage.values.set(FONT_PREVIEW_KEY, JSON.stringify({ sample: "", size: 16 }));
  assert.deepEqual(readPreviewPrefs(storage), { sample: "", size: 16, full: false });
});

test("a size is whole px from 16 to 160", () => {
  assert.equal(clampSize(4), 16);
  assert.equal(clampSize(48.4), 48);
  assert.equal(clampSize(999), 160);
  assert.equal(clampSize(Number.NaN), 32);
});

// Criteria 8 to 10, adrs.md decision 5.
test("the preview opens with the selected text's line, else the kept sample", () => {
  assert.equal(openingSample({ node: "n1", sample: "Lea" }, "Typed"), "Lea");
  assert.equal(openingSample({ node: "", sample: "" }, "Typed"), "Typed");
});

test("the field follows a new single text or a changed line, and nothing else", () => {
  const none = { node: "", sample: "" };
  assert.equal(followedSample(none, { node: "a", sample: "Lea" }), "Lea");
  assert.equal(followedSample({ node: "a", sample: "Lea" }, { node: "a", sample: "Lea!" }), "Lea!");
  // The same state again keeps what the maker typed.
  assert.equal(followedSample({ node: "a", sample: "Lea" }, { node: "a", sample: "Lea" }), null);
  // Another text with the same line still resets a typed sample.
  assert.equal(followedSample({ node: "a", sample: "Lea" }, { node: "b", sample: "Lea" }), "Lea");
  // No single text: the field keeps its sample.
  assert.equal(followedSample({ node: "a", sample: "Lea" }, none), null);
});

// Criteria 2 to 5.
test("split is 35 % of the canvas area, or one tile column if wider; full when the drawing would keep under 360px", () => {
  // 1280 x 800: canvas area 968px. Size 32: 35 % = 339px beats one column (240 + 24 + 18 = 282px)
  // (customer decision 2026-10-23, was 30 %).
  assert.deepEqual(paneLayout(968, 32, false), { full: false, canSplit: true, width: 339 });
  // Size 96: one column 480 + 24 + 18 = 522px; the drawing keeps 446px.
  assert.deepEqual(paneLayout(968, 96, false), { full: false, canSplit: true, width: 522 });
  // Exactly one column fits the grid's inner width (the pane less its border and scrollbar).
  for (const size of [16, 48, 96, 120]) {
    const width = paneLayout(1600, size, false).width;
    assert.ok(gridColumns(width - PANE_CHROME_PX, size).count >= 1, `${size}`);
    assert.ok(width >= oneColumnPane(size));
  }
  assert.equal(gridColumns(oneColumnPane(96) - PANE_CHROME_PX, 96).width, minColumnWidth(96));
  // The drawing must keep 360px: size 120 at 968px needs 642px, leaving 326px: full.
  assert.deepEqual(paneLayout(968, 120, false), { full: true, canSplit: false, width: 968 });
  assert.deepEqual(paneLayout(720, 16, false), { full: false, canSplit: true, width: 282 });
  assert.deepEqual(paneLayout(641, 16, false), { full: true, canSplit: false, width: 641 });
  assert.deepEqual(paneLayout(642, 16, false).full, false);
  assert.deepEqual(paneLayout(1000, 32, true), { full: true, canSplit: true, width: 1000 });
  // 800 x 600: canvas area 488px, forced full.
  assert.equal(paneLayout(488, 32, false).full, true);
});

test("a forced full view holds when the area widens again, until the maker switches or closes", () => {
  // Narrow while open: held; widened again: still full.
  const held = heldFull(false, true, 600, 32);
  assert.equal(held, true);
  assert.equal(heldFull(held, true, 1200, 32), true);
  assert.deepEqual(paneLayout(1200, 32, false, held), { full: true, canSplit: true, width: 1200 });
  // A size whose column leaves the drawing too narrow holds as well.
  assert.equal(heldFull(false, true, 968, 120), true);
  // Closing lets go; a closed preview or a wide area does not hold; an unmeasured area neither.
  assert.equal(heldFull(held, false, 1200, 32), false);
  assert.equal(heldFull(false, false, 600, 32), false);
  assert.equal(heldFull(false, true, 1200, 32), false);
  assert.equal(heldFull(false, true, 0, 32), false);
  // Split view from a held full view splits without overwriting the stored split choice.
  assert.deepEqual(switchedView(false, true), { wantFull: false, held: false });
  // A stored full choice under a hold, and a plain switch, toggle the stored choice.
  assert.deepEqual(switchedView(true, true), { wantFull: false, held: false });
  assert.deepEqual(switchedView(false, false), { wantFull: true, held: false });
  assert.deepEqual(switchedView(true, false), { wantFull: false, held: false });
});

test("the drawing keeps its view: the width deltas of open, full and close add up to zero", () => {
  const split = paneLayout(1000, 32, false);
  const full = paneLayout(1000, 32, true);
  const opened = drawingWidthDelta(drawingLoss(false, split), drawingLoss(true, split));
  assert.equal(opened, -350);
  const toFull = drawingWidthDelta(drawingLoss(true, split), drawingLoss(true, full));
  const closed = drawingWidthDelta(drawingLoss(true, full), drawingLoss(false, full));
  assert.equal(opened + toFull + closed, 0);
  assert.equal(drawingLoss(true, paneLayout(600, 32, false)), 0);
});

// Criterion 27 (customer decision 2026-10-23): the dragged split.
test("a dragged split is a share of the canvas area, held to one column and a 360px drawing", () => {
  // Dragged to 500px of 1000px: half, and it scales with the window.
  assert.equal(splitAfterDrag(1000, 32, 500), 0.5);
  assert.deepEqual(paneLayout(1000, 32, false, false, 0.5), { full: false, canSplit: true, width: 500 });
  assert.equal(paneLayout(1400, 32, false, false, 0.5).width, 700);
  // Too narrow: one column at the Size (282px at 32); too wide: the drawing keeps 360px.
  assert.equal(splitAfterDrag(1000, 32, 100), 0.282);
  assert.equal(splitAfterDrag(1000, 32, 900), 0.64);
  assert.equal(paneLayout(1000, 32, false, false, 0.05).width, 282);
  assert.equal(paneLayout(1000, 32, false, false, 0.95).width, 640);
  // A larger Size widens a narrow stored split to one column.
  assert.equal(paneLayout(1000, 96, false, false, 0.3).width, oneColumnPane(96));
  // When both cannot hold, the forced full view stays as without a stored split.
  assert.deepEqual(paneLayout(600, 32, false, false, 0.5), { full: true, canSplit: false, width: 600 });
  // No stored split: the default 35 % or one column.
  assert.equal(paneLayout(968, 32, false, false, undefined).width, 339);
  assert.equal(SPLIT_KEY_STEP_PX, 16);
});

test("the dragged split is kept with the other view state; reset removes it", () => {
  const storage = memory();
  writePreviewPrefs({ sample: "Jürgen", size: 48, full: false, split: 0.42 }, storage);
  assert.deepEqual(readPreviewPrefs(storage), { sample: "Jürgen", size: 48, full: false, split: 0.42 });
  for (const junk of [0, 1, -0.2, 1.5, "0.4", null]) {
    storage.values.set(FONT_PREVIEW_KEY, JSON.stringify({ sample: "a", size: 48, full: false, split: junk }));
    assert.deepEqual(readPreviewPrefs(storage), { sample: "a", size: 48, full: false }, String(junk));
  }
  writePreviewPrefs({ sample: "a", size: 48, full: false, split: undefined }, storage);
  assert.equal("split" in readPreviewPrefs(storage), false);
});

test("the header takes two rows under 560px", () => {
  assert.equal(headerRows(320), 2);
  assert.equal(headerRows(559), 2);
  assert.equal(headerRows(560), 1);
});

// Criteria 12, 13.
test("columns are at least max(240, 5 x size) wide; 160px gives one column", () => {
  // UX notes §6: full view at 1280 x 800, 968px pane: 3 columns of about 306px.
  const wide = gridColumns(968 - 8, 48);
  assert.equal(wide.count, 3);
  assert.ok(Math.abs(wide.width - 304) < 4);
  assert.equal(gridColumns(968, 160).count, 1);
  assert.equal(gridColumns(200, 48).count, 1);
  assert.equal(gridColumns(1056, 48).count, 4);
});

// Criterion 13 (customer decisions 2026-10-23): one line per tile, and the tags in a row of their
// own under the name, reserved on every tile.
test("all tiles of one size are one line high, about 195px at 48px, the tag row included", () => {
  const one = tileHeight(48);
  assert.ok(one > 185 && one < 210, String(one));
  assert.equal(TILE_FOOTER_PX, 28 + 24);
  assert.equal(one - sampleAreaHeight(48), 12 + 8 + 28 + 24 + 8);
  assert.ok(tileHeight(96) > one);
});

// Criterion 23: only visible tiles plus one screen are in the page.
test("the page holds the rows in view and one screen ahead", () => {
  assert.deepEqual(rowsInPage(0, 600, 180, 1000), { first: 0, last: 6 });
  const far = rowsInPage(180 * 500, 600, 180, 1000);
  assert.ok(far.first >= 499 && far.last - far.first <= 8);
  assert.deepEqual(rowsInPage(0, 600, 180, 0), { first: 0, last: -1 });
  assert.equal(rowsInPage(1e9, 600, 180, 10).last, 9);
});

// Criteria 18, 22.
test("a tile's name and description", () => {
  const none = { favourite: false, cuttable: false, singleLine: false, current: false };
  assert.equal(tileName("Lobster", { ...none, favourite: true, cuttable: true, current: true }), "Lobster, favourite, Cuttable, current");
  assert.equal(tileName("Lobster", none), "Lobster");
  assert.equal(tileDescription("Anna & Jonas 2026", ["ä"]), "Anna & Jonas 2026. No glyph for: ä");
  assert.equal(tileDescription("Anna", []), "Anna");
});

test("the missing-glyph tooltip names at most 10 characters", () => {
  assert.equal(missingText(["ä", "ß"]), "No glyph for: ä, ß");
  assert.equal(missingText([]), "");
  const many = [..."abcdefghijkl"];
  assert.equal(missingText(many), "No glyph for: a, b, c, d, e, f, g, h, i, j, …");
});

test("the Size tooltip has no hint line", () => {
  assert.equal(sizeTip(48), "Size: 48 px");
});

// Criterion 26.
test("the subject line uses the Fonts tab's words", () => {
  assert.equal(subjectLine("All", 1240, 1240, false), "All · 1,240");
  assert.equal(subjectLine("Cuttable", 3, 42, true), "Cuttable · 3 of 42");
});

// Criterion 22.
test("arrows, Home, End and pages move over the grid", () => {
  assert.equal(gridMove("ArrowRight", 0, 10, 3, 2), 1);
  assert.equal(gridMove("ArrowLeft", 0, 10, 3, 2), 0);
  assert.equal(gridMove("ArrowDown", 1, 10, 3, 2), 4);
  assert.equal(gridMove("ArrowUp", 1, 10, 3, 2), 1);
  assert.equal(gridMove("ArrowDown", 9, 10, 3, 2), 9);
  assert.equal(gridMove("ArrowDown", 8, 10, 3, 2), 9);
  // Down from a row above a short last row lands on the last tile.
  assert.equal(gridMove("ArrowDown", 5, 7, 3, 2), 6);
  assert.equal(gridMove("PageDown", 0, 10, 3, 2), 6);
  assert.equal(gridMove("PageUp", 7, 10, 3, 2), 1);
  assert.equal(gridMove("Home", 7, 10, 3, 2), 0);
  assert.equal(gridMove("End", 0, 10, 3, 2), 9);
  assert.equal(gridMove("Enter", 0, 10, 3, 2), null);
  assert.equal(gridMove("ArrowRight", 0, 0, 3, 2), null);
});

test("a printable character types into the sample field", () => {
  assert.equal(typesIntoSample("a", false, false, false), true);
  assert.equal(typesIntoSample("ä", false, false, false), true);
  assert.equal(typesIntoSample("d", true, false, false), false);
  assert.equal(typesIntoSample("ArrowUp", false, false, false), false);
});

// Criterion 27: a divider drag applies at most one width per animation frame, the latest.
test("pointer moves of a drag are coalesced into one delivery per frame, the latest winning", () => {
  const frames: (() => void)[] = [];
  const scheduler = {
    request: (run: () => void) => frames.push(run),
    cancel: (handle: number) => {
      frames[handle - 1] = () => {};
    },
  };
  const delivered: number[] = [];
  const drag = latestPerFrame((value) => delivered.push(value), scheduler);
  drag.push(0.3);
  drag.push(0.31);
  drag.push(0.32);
  assert.equal(frames.length, 1, "one frame requested for three moves");
  frames[0]();
  assert.deepEqual(delivered, [0.32]);
  drag.push(0.4);
  assert.equal(frames.length, 2);
  // The drag ends before the frame: the pending width is delivered now, and only once.
  drag.flush();
  frames[1]();
  assert.deepEqual(delivered, [0.32, 0.4]);
  drag.push(0.5);
  drag.cancel();
  frames[2]();
  assert.deepEqual(delivered, [0.32, 0.4]);
});

test("one drawing per face and sample serves every Size and width: the widest column's caps", () => {
  // A column is narrower than two of the narrowest and a gap, at any size and pane width.
  for (const size of [16, 32, 48, 100, 160]) {
    for (let grid = 200; grid <= 4000; grid += 37) {
      const { width } = gridColumns(grid, size);
      if (grid >= minColumnWidth(size) + 2 * GRID_PAD_PX) {
        assert.ok(width / size <= TILE_DRAW_CAPS, `${grid}/${size}: ${width / size}`);
      }
    }
  }
  assert.equal(TILE_DRAW_CAPS, 31);
});

test("criterion 27: tiles take shares of the grid's width, so a drag re-flows them in CSS", () => {
  assert.deepEqual(tileColumnCss(0, 1), { left: "calc(12px + 0 * ((100% - 24px) / 1 + 12px))", width: "calc((100% - 24px) / 1)" });
  assert.deepEqual(tileColumnCss(2, 3), {
    left: "calc(12px + 2 * ((100% - 48px) / 3 + 12px))",
    width: "calc((100% - 48px) / 3)",
  });
  // While the divider moves, only a change of the column count (or the height) re-lays the grid.
  const a = { width: 480, height: 600 };
  assert.equal(sameGridArea(a, { width: 505, height: 600 }, true, 32), true);
  assert.equal(sameGridArea(a, { width: 520, height: 600 }, false, 32), false);
  assert.equal(sameGridArea(a, { width: 540, height: 600 }, true, 32), false, "two columns fit at 540");
  assert.equal(sameGridArea(a, { width: 480, height: 610 }, true, 32), false);
});

test("criteria 17 and 23: the families in view are prepared first, then the next, then the earlier", () => {
  assert.deepEqual(prerenderOrder(10, 3, 5), [3, 4, 5, 6, 7, 8, 9, 2, 1, 0]);
  assert.deepEqual(prerenderOrder(10, 3, 5, 4), [3, 4, 5, 6]);
  assert.deepEqual(prerenderOrder(4, 0, -1), [0, 1, 2, 3], "nothing in view: from the top");
  assert.deepEqual(prerenderOrder(3, 7, 9), [2, 1, 0], "a range past the end is held to the list");
  assert.deepEqual(prerenderOrder(0, 0, 5), []);
  assert.equal(prerenderOrder(5000, 0, 20).length, PRERENDER_MAX);
});

test("criterion 23: background jobs run one at a time when idle, a new queue replaces the rest", async () => {
  const idle: (() => void)[] = [];
  const runner = createPrerenderer((run) => idle.push(run));
  const ran: string[] = [];
  const job = (name: string) => async () => {
    ran.push(name);
  };
  runner.schedule([job("a"), job("b"), job("c")]);
  assert.equal(idle.length, 1);
  idle.shift()?.();
  assert.deepEqual(ran, ["a"]);
  runner.schedule([job("x")]);
  await Promise.resolve();
  await Promise.resolve();
  idle.shift()?.();
  assert.deepEqual(ran, ["a", "x"]);
  await Promise.resolve();
  await Promise.resolve();
  idle.shift()?.();
  assert.equal(runner.pending(), 0);
  assert.equal(idle.length, 0, "an empty queue stops");
  // A failing job does not stop the queue.
  runner.schedule([async () => Promise.reject(new Error("unreadable")), job("y")]);
  idle.shift()?.();
  for (let i = 0; i < 4; i += 1) {
    await Promise.resolve();
  }
  idle.shift()?.();
  assert.deepEqual(ran, ["a", "x", "y"]);
});

test("criterion 5: Escape outside the preview closes it only when nothing else handled it", () => {
  // The canvas keeps every step of its Escape ladder and leaves "nothing to cancel" unhandled.
  for (const step of ["escape-entry", "escape-drag", "escape-state", "escape-group", "escape-tool"]) {
    assert.equal(canvasLeavesKey(step), false, step);
  }
  assert.equal(canvasLeavesKey("escape-none"), true);
  assert.equal(canvasLeavesKey("ignored"), true);
  assert.equal(canvasLeavesKey("deleted"), false);
  // Selection cleared (handled): stays open; the next Escape (nothing to cancel): closes.
  assert.equal(escapeClosesPreview("Escape", true, true), false);
  assert.equal(escapeClosesPreview("Escape", false, true), true);
  // Repeated presses are never swallowed while the preview is open.
  assert.equal(escapeClosesPreview("Escape", false, true), true);
  assert.equal(escapeClosesPreview("Escape", false, false), false);
  assert.equal(escapeClosesPreview("Backspace", false, true), false);
});

test("criterion 2: the pane lies over the drawing's canvas by its split width only", () => {
  for (const size of [16, 32, 64]) {
    const split = paneLayout(1400, size, false, false, splitAfterDrag(1400, size, 0));
    assert.equal(drawingLoss(true, split), split.width);
    assert.equal(drawingLoss(true, paneLayout(1400, size, true)), 0, "full view hides the drawing");
    assert.equal(drawingLoss(false, split), 0);
  }
});

test("criterion 14: a tile's tag line room and its keys", () => {
  // A row of its own: the column less the tile's 16px side paddings.
  assert.equal(tileTagRoom(300), 300 - 32);
  assert.equal(tileTagRoom(20), 0);
  assert.equal(tileTagKey("F2", false), "add");
  assert.equal(tileTagKey("F2", true), "enter-line");
  assert.equal(tileTagKey("Delete", false), null);
});

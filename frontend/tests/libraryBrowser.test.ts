// The library browser's pure parts (`specs/0152-library-browser` criteria 2 to 4 and 10): the
// per-user prefs and their guards, the tile and card geometry, and the one side pane.
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  DEFAULT_BROWSER_PREFS,
  LIBRARY_BROWSER_KEY,
  readBrowserPrefs,
  writeBrowserPrefs,
  clampBrowserSize,
} from "../src/lib/libraryBrowserState.ts";
import {
  columnsFor,
  isWide,
  paneLayoutSize,
  rowsInView,
  tileHeight,
  tileWidth,
} from "../src/lib/libraryBrowserLayout.ts";
import { oneColumnPane } from "../src/lib/fontPreviewLayout.ts";
import { claimPane, openPane, registerPaneCloser, releasePane } from "../src/lib/sidePaneStore.ts";

function storage(initial: Record<string, string> = {}) {
  const data = new Map(Object.entries(initial));
  return {
    getItem: (key: string) => data.get(key) ?? null,
    setItem: (key: string, value: string) => void data.set(key, value),
    data,
  };
}

test("criterion 3: the view is kept under its own key, Source and Category are not", () => {
  const store = storage();
  writeBrowserPrefs({ ...DEFAULT_BROWSER_PREFS, size: 128, sort: "newest", text: "hrat", split: 0.4 }, store);
  const raw = JSON.parse(store.data.get(LIBRARY_BROWSER_KEY) ?? "{}") as Record<string, unknown>;
  assert.equal(raw.size, 128);
  assert.equal("sources" in raw || "categories" in raw, false);
  const back = readBrowserPrefs(store);
  assert.deepEqual(back, { ...DEFAULT_BROWSER_PREFS, size: 128, sort: "newest", text: "hrat", split: 0.4 });
});

test("criterion 3: unreadable values fall back one by one", () => {
  const store = storage({
    [LIBRARY_BROWSER_KEY]: JSON.stringify({ size: 300, sort: "best", colour: "mono", full: "yes", text: "x".repeat(101) }),
  });
  const prefs = readBrowserPrefs(store);
  assert.equal(prefs.size, 96, "out of range");
  assert.equal(prefs.sort, "name", "Best match is never kept");
  assert.equal(prefs.colour, "mono");
  assert.equal(prefs.full, false);
  assert.equal(prefs.text, "");
  assert.deepEqual(readBrowserPrefs(storage({ [LIBRARY_BROWSER_KEY]: "{" })), DEFAULT_BROWSER_PREFS);
  assert.equal(clampBrowserSize(10), 48);
  assert.equal(clampBrowserSize(1000), 256);
});

test("criterion 4: tiles are Size + 16 wide and the preview alone under 96 px", () => {
  assert.equal(tileWidth(96), 112);
  assert.equal(tileHeight(96), 152);
  assert.equal(tileHeight(48), 64);
  assert.equal(columnsFor(12 + 112 + 8 + 112 + 12, 112), 2);
  assert.equal(columnsFor(100, 112), 1, "at least one column");
  assert.equal(isWide(640), true);
  assert.equal(isWide(639), false);
  assert.deepEqual(rowsInView(0, 300, 100, 0), { first: 0, last: -1 });
  assert.deepEqual(rowsInView(1000, 300, 100, 100), { first: 10, last: 15 });
});

test("criterion 2: the narrowest split holds one tile column", () => {
  for (const size of [48, 96, 200, 256]) {
    const pane = oneColumnPane(paneLayoutSize(size));
    assert.ok(pane >= tileWidth(size) + 24, `${size}: ${pane}`);
  }
});

test("criterion 2: one side pane at a time", () => {
  const closed: string[] = [];
  const off = [
    registerPaneCloser("font-preview", () => {
      closed.push("font");
      releasePane("font-preview");
    }),
    registerPaneCloser("library-browser", () => {
      closed.push("browser");
      releasePane("library-browser");
    }),
  ];
  claimPane("font-preview");
  assert.equal(openPane(), "font-preview");
  claimPane("library-browser");
  assert.equal(openPane(), "library-browser");
  assert.deepEqual(closed, ["font"]);
  releasePane("font-preview");
  assert.equal(openPane(), "library-browser", "releasing a closed kind changes nothing");
  releasePane("library-browser");
  assert.equal(openPane(), null);
  for (const unregister of off) {
    unregister();
  }
});

test("criterion 6: pictures are asked for the rows in view and one screen ahead, never behind", () => {
  // Scrolled down to 1,000 px of 100 px rows in a 300 px view: rows 10 to 12 in view, 13 to 15 ahead.
  assert.deepEqual(rowsInView(1000, 300, 100, 100, 1), { first: 10, last: 15 });
  // Scrolling up: the screen above is ahead, nothing below the view.
  assert.deepEqual(rowsInView(1000, 300, 100, 100, -1), { first: 7, last: 12 });
  assert.deepEqual(rowsInView(0, 300, 100, 4, 1), { first: 0, last: 3 }, "held to the rows there are");
  assert.deepEqual(rowsInView(0, 300, 100, 100, -1), { first: 0, last: 2 });
});

import { backApplies, isBackKey } from "../src/lib/browserBack.ts";

test("criterion 30: Alt+Left, Cmd+[ and mouse back go Back anywhere but a text field outside the pane", () => {
  const key = (k: string, mods: Partial<{ altKey: boolean; metaKey: boolean; ctrlKey: boolean; shiftKey: boolean }>) => ({
    key: k,
    altKey: false,
    metaKey: false,
    ctrlKey: false,
    shiftKey: false,
    ...mods,
  });
  assert.equal(isBackKey(key("ArrowLeft", { altKey: true })), true);
  assert.equal(isBackKey(key("[", { metaKey: true })), true);
  assert.equal(isBackKey(key("ArrowLeft", {})), false);
  assert.equal(isBackKey(key("ArrowLeft", { altKey: true, shiftKey: true })), false);
  assert.equal(isBackKey(key("Backspace", {})), false, "Backspace no longer goes back");
  // The tester's case: the focus on the Browse button outside the pane.
  assert.equal(backApplies(true, false, false), true);
  assert.equal(backApplies(true, true, true), true, "the pane's own search field");
  assert.equal(backApplies(true, false, true), false, "a text field elsewhere keeps its keys");
  assert.equal(backApplies(false, false, false), false, "closed pane");
});

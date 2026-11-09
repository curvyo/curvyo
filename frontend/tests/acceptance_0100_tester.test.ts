// Tester cases for `specs/0100-bring-content-in` in the front end: the one drag-answer decoder
// against hostile shapes (adrs.md note 4), the badge words and glyphs (criteria 2, 13, 14), the
// page size text of the pick rows (15, 25), the Place page and Place on… wiring (15, 25,
// including the keyboard route of the inline list), and the session the browser loads for
// Place on targets and the row-drag answer (needs `npm run build:wasm`).

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

import {
  BADGE_GLYPHS,
  DROP_BADGES,
  asDragAnswer,
  badgeWord,
  decodeDragAnswer,
  isRefusal,
} from "../src/lib/dragAnswer.ts";
import { sizeText } from "../src/lib/pageSizeText.ts";

const SRC = path.resolve(import.meta.dirname, "..", "src");
const source = (rel: string) => readFileSync(path.join(SRC, rel), "utf8");
const BINDINGS = path.join(SRC, "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");
const skipWasm = { skip: !existsSync(WASM) };

// ---- adrs.md note 4: one decoder, hostile input ------------------------------------------------

test("note 4: wrong field types, arrays, null and extra nesting decode to null", () => {
  const bad: unknown[] = [
    null,
    undefined,
    42,
    "clone",
    [],
    [{ badge: "clone", target: null, faint: false }],
    { badge: "clone", target: 7, faint: false },
    { badge: "clone", target: null, faint: "false" },
    { badge: "clone", target: null, faint: 0 },
    { badge: "Clone", target: null, faint: false },
    { badge: " clone", target: null, faint: false },
    { badge: null, target: null, faint: false },
    { badge: "toString", target: null, faint: false },
    { badge: "__proto__", target: null, faint: false },
    { target: null, faint: false },
  ];
  for (const value of bad) {
    assert.equal(asDragAnswer(value), null, JSON.stringify(value));
  }
  assert.equal(decodeDragAnswer("null"), null);
  assert.equal(decodeDragAnswer("{"), null);
  assert.equal(decodeDragAnswer('"clone"'), null);
});

test("note 4: a good answer decodes to exactly its three fields, extra keys dropped", () => {
  const answer = decodeDragAnswer('{"badge":"placed_page","target":"mfg-1-2","faint":false,"x":1}');
  assert.deepEqual(answer, { badge: "placed_page", target: "mfg-1-2", faint: false });
});

test("criteria 2, 13, 14: every badge but none has a word and a glyph; refusals are the 'not' badges", () => {
  for (const kind of DROP_BADGES) {
    assert.equal(BADGE_GLYPHS[kind] === null, kind === "none", `glyph of ${kind}`);
  }
  assert.equal(badgeWord("clone", ""), "Linked clone");
  assert.equal(badgeWord("move", ""), "Move");
  assert.equal(badgeWord("copy", ""), "Copy");
  assert.equal(badgeWord("placed_page", ""), "Placed page");
  assert.equal(badgeWord("only_on_laser_page", ""), "Only on a laser page");
  assert.ok(isRefusal("only_on_laser_page"));
  for (const kind of ["clone", "move", "copy", "placed_page", "none"] as const) {
    assert.ok(!isRefusal(kind), kind);
  }
  // Glyph and word, never colour alone: the move and copy badges differ in both.
  assert.notEqual(BADGE_GLYPHS.move, BADGE_GLYPHS.copy);
  assert.notEqual(BADGE_GLYPHS.clone, BADGE_GLYPHS.move);
});

// ---- criteria 15 and 25: pick rows ---------------------------------------------------------------

test("criteria 15, 25: the size text reads '125 × 75 mm' and keeps one decimal otherwise", () => {
  assert.equal(sizeText(125, 75), "125 × 75 mm");
  assert.equal(sizeText(210, 297), "210 × 297 mm");
  assert.equal(sizeText(100.25, 50), "100.3 × 50 mm");
});

test("criterion 15: Place page sits between the Machine block and Fixtures and hides without a drawing page", () => {
  const tab = source("components/workbench/ManifestTab.tsx");
  const machine = tab.indexOf("<MachineBlock");
  const place = tab.indexOf("<PlacePage");
  const fixtures = tab.indexOf("<FixtureList");
  assert.ok(machine >= 0 && place > machine && fixtures > place, "Machine, Place page, Fixtures");
  assert.match(tab, /if \(drawing\.length === 0\) return null;/, "hidden, not disabled");
  assert.match(tab, /label="Place page"/);
});

test("criterion 25: Place on… is hidden without a laser page and returns focus to the canvas", () => {
  const bar = source("components/SelectToolbar.tsx");
  assert.match(bar, /placeOn\.rows\.length > 0/, "no rows hides the button");
  assert.match(bar, /label="Place on…"/);
  const hook = source("hooks/usePlaceOn.ts");
  assert.match(hook, /containerRef\.current\?\.focus\(/, "focus on the canvas after a choice");
});

test("criteria 15, 25: the inline list's keyboard route (Enter opens, arrows, Enter/Space choose, Escape returns focus)", () => {
  const list = source("components/InlinePickList.tsx");
  assert.doesNotMatch(list, /createPortal|role="dialog"|popover/i, "an inline list, no popup");
  assert.match(list, /role="listbox"/);
  assert.match(list, /role="option"/);
  assert.match(list, /e\.key === "ArrowDown" \|\| e\.key === "ArrowUp"/);
  assert.match(list, /e\.key === "Enter" \|\| e\.key === " "/);
  assert.match(list, /e\.key === "Escape"[\s\S]{0,200}button\.current\?\.focus\(\)/, "Escape returns focus to the button");
  assert.match(list, /aria-expanded=\{open\}/);
  // A plain <button>: Tab reaches it and Enter or Space opens it.
  assert.match(list, /<button[\s\S]{0,80}type="button"/);
});

test("criterion 23: the new front-end files make no network request", () => {
  for (const rel of [
    "lib/dragAnswer.ts",
    "lib/pageSizeText.ts",
    "hooks/usePlaceOn.ts",
    "components/InlinePickList.tsx",
    "components/DropBadgePill.tsx",
    "components/PlacedPageTags.tsx",
  ]) {
    const text = source(rel);
    assert.doesNotMatch(text, /\bfetch\(|XMLHttpRequest|WebSocket|navigator\.sendBeacon/, rel);
  }
});

// ---- the session the browser loads ---------------------------------------------------------------

type Session = {
  add_page_of_kind: (kind: string, profile: string) => string;
  activate_page: (id: string) => boolean;
  place_on_targets: () => string;
  place_page_on_active: (id: string) => boolean;
  clone_drop_answer: () => string;
  pages_json: () => string;
};

async function open(): Promise<Session> {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: bigint) => Session;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  return new bindings.WasmSession(17n);
}

test("criterion 25: the session's Place on targets carry id, name and size, none without a laser page", skipWasm, async () => {
  const s = await open();
  assert.deepEqual(JSON.parse(s.place_on_targets()), [], "no laser page: no rows");
  const laser = s.add_page_of_kind("curvyo.laser", "");
  const rows = JSON.parse(s.place_on_targets()) as Record<string, unknown>[];
  assert.equal(rows.length, 1);
  assert.equal(rows[0].id, laser);
  assert.equal(typeof rows[0].name, "string");
  assert.equal(typeof rows[0].width_mm, "number");
  assert.equal(typeof rows[0].height_mm, "number");
});

test("note 4: no drag in flight answers an empty text, which decodes to null", skipWasm, async () => {
  const s = await open();
  assert.equal(decodeDragAnswer(s.clone_drop_answer()), null);
});

test("criterion 15: Place page on the active laser page places the drawing page", skipWasm, async () => {
  const s = await open();
  const pages = JSON.parse(s.pages_json()) as { id: string; kind: string }[];
  const drawing = pages.find((p) => p.kind === "drawing");
  assert.ok(drawing);
  const laser = s.add_page_of_kind("curvyo.laser", "");
  s.activate_page(laser);
  assert.equal(s.place_page_on_active(drawing.id), true);
  assert.equal(s.place_page_on_active(laser), false, "a laser page is never a source");
});

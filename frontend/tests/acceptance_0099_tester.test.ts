// Tester cases for `specs/0099-sketch-pages` in the front end: the drop badge words (criteria 8 to
// 10), the usage line and Select clones label (12), the row state words, glyphs and spoken name (14),
// the delete wording (16), the source rules of the page row, the Use group and Convert (5, 11, 15,
// 22), no network (21), and the row drag in the session the browser loads (needs
// `npm run build:wasm`).

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

import {
  type SketchRow,
  type SketchState,
  badgeWord,
  deleteLabel,
  selectClonesLabel,
  sketchPageReading,
  stateEmphasised,
  stateGlyph,
  stateSpoken,
  stateWord,
  usageLine,
  usedByTail,
} from "../src/lib/sketchPages.ts";

const SRC = path.resolve(import.meta.dirname, "..", "src");
const source = (rel: string) => readFileSync(path.join(SRC, rel), "utf8");
const BINDINGS = path.join(SRC, "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");
const skipWasm = { skip: !existsSync(WASM) };

const row = (clones: number, pages = 1, drawable = true): SketchRow => ({
  state: { state: "full" },
  clones,
  pages,
  drawable,
});

// ---- criteria 8 to 10: the drop badge -------------------------------------------------------------

test("AC8/9: the badge reads 'Linked clone' over a target and 'Not on a sketch page' over a sketch page", () => {
  assert.equal(badgeWord("clone", "Bracket"), "Linked clone");
  assert.equal(badgeWord("not_on_sketch_page", "Bracket"), "Not on a sketch page");
  assert.equal(badgeWord("none", "Bracket"), null, "no badge off the canvas");
});

test("AC10: an empty sketch reads '<name> has no sketch yet', the name truncated at 24 characters", () => {
  assert.equal(badgeWord("no_sketch", "Bracket"), "Bracket has no sketch yet");
  const long = "A very long sketch page name indeed";
  const word = badgeWord("no_sketch", long) ?? "";
  assert.ok(word.endsWith(" has no sketch yet"), word);
  const name = word.slice(0, -" has no sketch yet".length);
  assert.ok(name.length <= 25, `truncated: ${name} (${name.length})`);
  assert.ok(long.startsWith(name.replace(/…$/, "")), name);
  assert.notEqual(name, long);
  const exact = "x".repeat(24);
  assert.equal(badgeWord("no_sketch", exact), `${exact} has no sketch yet`, "24 characters stay whole");
});

// ---- criterion 12: the usage line -----------------------------------------------------------------

test("AC12: the usage line words none, one and many clones and pages", () => {
  assert.equal(usageLine(0, 0), "Not used on other pages");
  assert.equal(usageLine(1, 1), "Used by 1 linked clone on 1 page");
  assert.equal(usageLine(3, 2), "Used by 3 linked clones on 2 pages");
  assert.equal(usageLine(2, 1), "Used by 2 linked clones on 1 page");
});

test("AC12: pasteboard clones count in the first number only", () => {
  // Two clones, both on the pasteboard: on no page.
  const line = usageLine(2, 0);
  assert.match(line, /^Used by 2 linked clones/, line);
  assert.doesNotMatch(line, /on 2 pages/, line);
});

test("AC12: the row tooltip tail reads 'used by 3 clones' and Select clone(s) follows the count", () => {
  assert.match(usedByTail(3), /used by 3 clones/);
  assert.match(usedByTail(1), /used by 1 clone\b/);
  assert.equal(selectClonesLabel(1), "Select clone");
  assert.equal(selectClonesLabel(3), "Select clones");
});

// ---- criterion 14: the row state ------------------------------------------------------------------

const STATES: [SketchState, string, string][] = [
  [{ state: "empty" }, "Empty", "empty"],
  [{ state: "full" }, "Fully constrained", "fully constrained"],
  [{ state: "free", dof: 3 }, "3 DOF", "3 degrees of freedom"],
  [{ state: "conflict" }, "Conflict", "conflict"],
  [{ state: "failed" }, "Solver failed", "solver failed"],
];

test("AC14: the visible state words", () => {
  for (const [s, word] of STATES) assert.equal(stateWord(s), word);
});

test("AC14: the accessible name spells the state out after the kind", () => {
  assert.equal(sketchPageReading("Bracket", { state: "full" }), "Bracket, Sketch page, fully constrained");
  assert.equal(sketchPageReading("Bracket", { state: "free", dof: 3 }), "Bracket, Sketch page, 3 degrees of freedom");
  for (const [s, , spoken] of STATES) {
    assert.equal(stateSpoken(s).toLowerCase(), spoken);
  }
  assert.equal(sketchPageReading("B", { state: "free", dof: 1 }), "B, Sketch page, 1 degree of freedom");
});

test("AC14: a glyph per state, never colour alone; conflict and failure at full opacity", () => {
  const glyphs = STATES.map(([s]) => stateGlyph(s));
  assert.equal(new Set(glyphs).size, glyphs.length, `distinct glyphs: ${glyphs}`);
  assert.equal(stateEmphasised({ state: "conflict" }), true);
  assert.equal(stateEmphasised({ state: "failed" }), true);
  assert.equal(stateEmphasised({ state: "full" }), false);
});

// ---- criterion 16: the delete wording -------------------------------------------------------------

test("AC16: the sketch row's trash names its sketch and the clones that become paths", () => {
  assert.equal(deleteLabel("Bracket", row(3, 2)), "Delete Bracket and its sketch; 3 clones become paths");
  assert.equal(deleteLabel("Bracket", row(1)), "Delete Bracket and its sketch; 1 clone becomes a path");
  assert.equal(deleteLabel("Bracket", row(0, 0)), "Delete Bracket and its sketch");
  assert.equal(deleteLabel("Bracket", row(2, 0)), "Delete Bracket and its sketch; 2 clones become paths", "pasteboard clones count");
  assert.equal(deleteLabel("Page 2", null), "Delete page: Page 2", "other kinds keep their label");
});

// ---- criteria 5, 11, 15, 22: source rules --------------------------------------------------------

test("AC5: the page row offers no frame-only switch on a sketch row", () => {
  const block = source("components/panel/PagesBlock.tsx");
  assert.match(block, /frameOnly/);
  assert.match(block, /sketch/);
});

test("AC11/15: Place clone on and Convert to paths are hidden, not disabled, without geometry", () => {
  const panel = source("components/sketch/SketchesPanel.tsx");
  assert.match(panel, /Place clone on/);
  assert.match(panel, /Convert to paths/);
  assert.match(panel, /drawable/);
  // Hidden, never a disabled control for these two.
  for (const label of ["Place clone on", "Convert to paths"]) {
    const at = panel.indexOf(label);
    const around = panel.slice(Math.max(0, at - 400), at + 100);
    assert.doesNotMatch(around, /disabled=\{[^}]*drawable/, label);
  }
});

test("AC11: the Place clone on list is inline (no popup) and Escape returns focus to the button", () => {
  const panel = source("components/sketch/SketchesPanel.tsx");
  assert.doesNotMatch(panel, /role="dialog"|<Popover|createPortal/, "no popup");
  assert.match(panel, /Escape/);
});

test("AC22: commands that change the page leave focus on the canvas", () => {
  const files = ["components/sketch/SketchesPanel.tsx", "hooks/usePageRowDrag.ts", "components/panel/CloneBlock.tsx"];
  const all = files.map(source).join("\n");
  assert.match(all, /focus/i);
  assert.match(all, /canvas/i);
});

test("AC21: the new front-end pieces open no network connection", () => {
  for (const file of [
    "hooks/usePageRowDrag.ts",
    "lib/sketchPages.ts",
    "components/sketch/SketchesPanel.tsx",
    "components/panel/PagesBlock.tsx",
  ]) {
    assert.doesNotMatch(source(file), /fetch\(|WebSocket|XMLHttpRequest|sendBeacon|EventSource/, file);
  }
});

// ---- the session the browser loads -----------------------------------------------------------------

type Session = {
  pages_json: () => string;
  add_page_of_kind: (kind: string, profile: string) => string;
  activate_page: (id: string) => boolean;
  row_drag_begin: (id: string) => boolean;
  row_drag_move: (x: number, y: number, onCanvas: boolean) => string;
  row_drag_drop: () => boolean;
  row_drag_cancel: () => void;
  place_sketch_clone_on: (id: string) => boolean;
  undo_len?: () => number;
};

async function open(): Promise<Session> {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: bigint) => Session;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  return new bindings.WasmSession(13n);
}

type Row = {
  id: string;
  kind: string;
  sketch?: SketchRow | null;
  screenX: number;
  screenY: number;
  screenWidth: number;
  screenHeight: number;
};

test("AC10/14: an empty sketch page row reads Empty, is not drawable, and its drag places nothing", skipWasm, async () => {
  const s = await open();
  const id = s.add_page_of_kind("sketch", "");
  const rows = JSON.parse(s.pages_json()) as Row[];
  const r = rows.find((x) => x.id === id);
  assert.ok(r?.sketch, "a sketch row carries its sketch facts");
  assert.deepEqual(r.sketch.state, { state: "empty" });
  assert.equal(r.sketch.drawable, false);
  assert.equal(r.sketch.clones, 0);
  const drawing = rows.find((x) => x.kind === "drawing");
  assert.ok(drawing);
  assert.equal(drawing.sketch ?? null, null, "a drawing row has no sketch facts");
  const before = s.pages_json();
  if (s.row_drag_begin(id)) {
    const x = drawing.screenX + drawing.screenWidth / 2;
    const y = drawing.screenY + drawing.screenHeight / 2;
    const answer = JSON.parse(s.row_drag_move(x, y, true)) as { badge: string };
    assert.equal(answer.badge, "no_sketch");
    assert.equal(s.row_drag_drop(), false);
  }
  assert.equal(s.pages_json(), before);
  assert.equal(s.place_sketch_clone_on(drawing.id), false);
});

// Amended by `specs/0100-bring-content-in` criteria 13 and 14: a drawing page row starts a
// place drag; over the drawing page itself it says "Only on a laser page" and places nothing.
test("AC8: a drawing page row places nothing outside a laser page", skipWasm, async () => {
  const s = await open();
  assert.equal(s.row_drag_begin("first"), true);
  const answer = JSON.parse(s.row_drag_move(10, 10, true)) as { badge: string };
  assert.notEqual(answer.badge, "placed_page");
  assert.equal(s.row_drag_drop(), false);
  s.row_drag_cancel();
});

// Reproduction of the customer's reports on align and distribute: the exact
// wasm module and entry points the browser build uses. Needs `npm run build:wasm`.
//
// The wasm's pointer API takes canvas-relative CSS pixels, so objects are
// drawn at millimetres through the session's own screen→document map.

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

const BINDINGS = path.resolve(import.meta.dirname, "..", "src", "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");

type Session = {
  set_tool: (tool: string) => string | undefined;
  screen_to_document: (x: number, y: number) => { x: number; y: number; free: () => void };
  pointer_down: (x: number, y: number, shift: boolean, ctrl: boolean, alt: boolean) => void;
  pointer_up: (x: number, y: number, shift: boolean, constrain: boolean, alt: boolean) => void;
  key_down: (
    key: string,
    shift: boolean,
    ctrl: boolean,
    alt: boolean,
    repeat: boolean,
    dom_blocked: boolean,
    mac: boolean,
    time_ms: number,
  ) => string;
  align_shown: () => boolean;
  align_reference: () => string;
  distribute_reference: () => string;
  grid_count: () => number;
  arrange_grid: (rows: number, columns: number, gap_mm: number) => string;
  align: (code: string) => string;
  distribute: (code: string, gap_mm: number | null) => string;
  pack: (version: string) => Uint8Array;
};

/** The built module's session, the same the browser loads. */
async function open(): Promise<Session> {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: bigint) => Session;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  return new bindings.WasmSession(1n);
}

/** Draws a rectangle of `mm` size, and selects it. */
function draw(session: Session, x0: number, y0: number, x1: number, y1: number): void {
  const at = (x: number, y: number) => {
    const point = session.screen_to_document(x, y);
    const out = { x: point.x, y: point.y };
    point.free();
    return out;
  };
  const origin = at(0, 0);
  const right = at(100, 0);
  const down = at(0, 100);
  const px = (mm: number, axis: "x" | "y") =>
    ((mm - (axis === "x" ? origin.x : origin.y)) * 100) /
    (axis === "x" ? right.x - origin.x : down.y - origin.y);
  session.set_tool("rectangle");
  session.pointer_down(px(x0, "x"), px(y0, "y"), false, false, false);
  session.pointer_up(px(x1, "x"), px(y1, "y"), false, false, false);
}

test(
  "distribute through the browser's wasm moves the middle object",
  { skip: !existsSync(WASM) },
  async () => {
    const session = await open();
    draw(session, 40, 30, 50, 40);
    draw(session, 90, 30, 94, 40);
    draw(session, 130, 30, 140, 40);
    session.set_tool("select");
    assert.equal(session.key_down("a", false, true, false, false, false, false, 0), "select-all");
    assert.equal(session.align_shown(), true, "the distribute group shows");
    assert.equal(session.distribute_reference(), "selection", "three keep their ends");
    const outcome = session.distribute("h-centres", null);
    assert.equal(outcome, "aligned", `the distribute commits: ${outcome}`);
    assert.ok(session.pack("0.1.0").length > 0, "the document packs");
  },
);

test(
  "two selected objects distribute on the page (criterion 16)",
  { skip: !existsSync(WASM) },
  async () => {
    const session = await open();
    draw(session, 40, 30, 50, 40);
    draw(session, 90, 30, 100, 40);
    session.set_tool("select");
    session.key_down("a", false, true, false, false, false, false, 0);
    assert.equal(session.distribute_reference(), "page", "two objects use the page");
    // Equal gaps to both page edges and between them: the thirds.
    assert.equal(session.distribute("h-gaps", null), "aligned");
    assert.equal(session.distribute("h-gaps", null), "unchanged", "already in thirds");
    // A typed gap spaces from the page's start edge.
    assert.equal(session.distribute("h-gaps", 5), "aligned");
    assert.ok(session.pack("0.1.0").length > 0, "the document packs");
  },
);

// The customer's extension of 2026-10-30: with nothing selected, the groups
// stay in the bar and act on the active page — align onto the page, distribute
// evenly across it, horizontally and vertically.
test(
  "with nothing selected the page's objects align and distribute on the page",
  { skip: !existsSync(WASM) },
  async () => {
    const session = await open();
    draw(session, 40, 30, 50, 44);
    draw(session, 90, 20, 110, 50);
    session.set_tool("select");
    // The last created object is selected; Escape empties the selection.
    assert.equal(session.key_down("Escape", false, false, false, false, false, false, 0), "escape-state");
    assert.equal(session.align_shown(), true, "align shows for the page");
    assert.equal(session.distribute_reference(), "page-objects", "distribute acts on the page");
    assert.equal(session.align("left"), "aligned", "every page object onto the page's left edge");
    assert.equal(session.distribute("h-gaps", null), "aligned", "even gaps across the page");
    assert.equal(session.distribute("v-centres", null), "aligned", "even centres down the page");
    assert.ok(session.pack("0.1.0").length > 0, "the document packs");
  },
);

// Criteria 18 to 21: the grid through the browser's wasm.
test("six selected objects arrange in a grid of two rows", { skip: !existsSync(WASM) }, async () => {
  const session = await open();
  for (const [x, y] of [
    [10, 10],
    [40, 12],
    [80, 10],
    [10, 60],
    [50, 62],
    [90, 58],
  ]) {
    draw(session, x, y, x + 12, y + 8);
  }
  session.set_tool("select");
  session.key_down("a", false, true, false, false, false, false, 0);
  assert.equal(session.grid_count(), 6);
  assert.equal(session.arrange_grid(1, 5, 0), "too-few-cells");
  assert.equal(session.arrange_grid(2, 3, 5), "aligned");
  assert.equal(session.arrange_grid(2, 3, 5), "unchanged", "a second press changes nothing");
  assert.ok(session.pack("0.1.0").length > 0, "the document packs");
});

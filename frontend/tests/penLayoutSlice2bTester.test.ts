// Acceptance tests of `specs/0158-pen-display` slice 2b written from the criteria (tester): the
// fixed Pen layout's control list and order (1), 48 px targets (1), no configuration entry (1,
// Frage 413 A), the switches in the query (4, 5), every host call on the allow-list (7), the
// second column and the window sizes (8), accessible names (9).
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { test } from "node:test";

import { PEN_ALLOWED } from "../src/lib/windowRole.ts";
import {
  cardHeight,
  ignoresPress,
  type PenLayoutActions,
  type PenLayoutState,
  penControls,
  railHeight,
  readPenFlags,
  savePenFlags,
  TARGET,
  viewInSecondColumn,
} from "../src/pen/penLayout.ts";

const STATE: PenLayoutState = {
  tool: "pencil",
  undo: null,
  redo: null,
  canDelete: false,
  panning: false,
  follow: false,
  canvasOnly: false,
};

function recorder() {
  const log: string[] = [];
  const actions: PenLayoutActions = {
    tool: (t) => log.push(`tool:${t}`),
    undo: () => log.push("undo"),
    redo: () => log.push("redo"),
    delete: () => log.push("delete"),
    zoom: (z) => log.push(`zoom:${z}`),
    fit: () => log.push("fit"),
    pan: () => log.push("pan"),
    follow: (on) => log.push(`follow:${on}`),
    canvasOnly: (on) => log.push(`canvas:${on}`),
  };
  return { log, actions };
}

const src = (path: string) => readFileSync(new URL(`../src/${path}`, import.meta.url), "utf8");

test("criterion 1: the fixed control list and order", () => {
  const controls = penControls(STATE, recorder().actions);
  assert.deepEqual(
    controls.map((c) => c.id),
    ["pencil", "eraser", "select", "node", "undo", "redo", "delete", "zoom-in", "zoom-out", "fit", "pan", "follow", "canvas-only"],
  );
  assert.deepEqual(
    controls.map((c) => c.group),
    ["Tools", "Tools", "Tools", "Tools", "Edit", "Edit", "Edit", "View", "View", "View", "View", "View", "Corner"],
  );
});

test("criterion 1: every control is a 48 px target in the source, in one card width of 56", () => {
  assert.equal(TARGET, 48);
  assert.equal(cardHeight(4), 200);
  assert.equal(cardHeight(3), 152);
  assert.equal(cardHeight(5), 248);
  const rail = src("pen/PenRail.tsx");
  assert.match(rail, /size-12/);
  assert.match(rail, /w-14/);
  assert.doesNotMatch(rail, /size-(?:[1-9]|10|11)\b/);
  assert.match(src("pen/PenStrip.tsx"), /h-14/);
});

test("criterion 1: Eraser second tap returns to the Pencil; others choose themselves", () => {
  const { log, actions } = recorder();
  penControls({ ...STATE, tool: "eraser" }, actions)[1].press();
  penControls(STATE, actions)[1].press();
  penControls(STATE, actions)[2].press();
  assert.deepEqual(log, ["tool:pencil", "tool:eraser", "tool:select"]);
  const eraserOn = penControls({ ...STATE, tool: "eraser" }, actions);
  assert.equal(eraserOn[1].pressed, true);
  assert.equal(eraserOn[0].pressed, false);
});

test("criterion 1: touch presses on a control do nothing, pen and mouse do", () => {
  assert.equal(ignoresPress("touch"), true);
  assert.equal(ignoresPress("pen"), false);
  assert.equal(ignoresPress("mouse"), false);
  const rail = src("pen/PenRail.tsx");
  assert.match(rail, /ignoresPress\(event\.pointerType\)/);
  assert.match(rail, /!fromTouch/);
});

test("criterion 1: no configuration entry of any kind (Frage 413 A)", () => {
  const files = ["pen/PenApp.tsx", "pen/PenRail.tsx", "pen/PenStrip.tsx", "pen/PenLayerPick.tsx", "pen/penLayout.ts", "components/PenViewFrame.tsx"];
  for (const file of files) {
    const text = src(file).toLowerCase();
    for (const word of ["customi", "mirror", "left-handed", "lefthanded", "localstorage", "layout setting", "full layout", "reorder", "hide control"]) {
      assert.ok(!text.includes(word), `${file} mentions ${word}`);
    }
  }
});

test("criterion 4/5: the switches come from the page's query, both off by default", () => {
  assert.deepEqual(readPenFlags(""), { followView: false, canvasOnly: false });
  assert.deepEqual(readPenFlags("?window=pen"), { followView: false, canvasOnly: false });
  assert.deepEqual(readPenFlags("?window=pen&follow=1&canvas=0"), { followView: true, canvasOnly: false });
  assert.deepEqual(readPenFlags("?window=pen&follow=0&canvas=1"), { followView: false, canvasOnly: true });
  assert.deepEqual(readPenFlags("?follow=true&canvas=2"), { followView: false, canvasOnly: false });
});

test("criterion 4: the corner toggle keeps one place, names flip, Canvas only is its own control", () => {
  const off = penControls(STATE, recorder().actions);
  const on = penControls({ ...STATE, canvasOnly: true }, recorder().actions);
  const corner = (cs: typeof off) => cs.filter((c) => c.group === "Corner");
  assert.equal(corner(off).length, 1);
  assert.equal(corner(on).length, 1);
  assert.equal(corner(off)[0].id, corner(on)[0].id);
  assert.equal(corner(off)[0].label, "Canvas only");
  assert.equal(corner(on)[0].label, "Show tools");
  assert.equal(corner(on)[0].pressed, true);
  const { log, actions } = recorder();
  penControls({ ...STATE, canvasOnly: true }, actions).at(-1)?.press();
  assert.deepEqual(log, ["canvas:false"]);
  const pen = src("pen/PenRail.tsx");
  assert.match(pen, /top-3 right-3/);
});

test("criterion 5: Follow view is a toggle on the View card and off by default", () => {
  const controls = penControls(STATE, recorder().actions);
  const follow = controls.find((c) => c.id === "follow");
  assert.equal(follow?.group, "View");
  assert.equal(follow?.pressed, false);
  const { log, actions } = recorder();
  penControls({ ...STATE, follow: true }, actions).find((c) => c.id === "follow")?.press();
  assert.deepEqual(log, ["follow:false"]);
});

test("criterion 7: every host call of the layout is on the allow-list", () => {
  const calls: string[] = [];
  const call = async (command: string) => {
    calls.push(command);
    return undefined;
  };
  void savePenFlags({ followView: true, canvasOnly: true }, call);
  // Press every control with every state: only actions run, no host call.
  for (const tool of ["pencil", "eraser", "select", "node"] as const) {
    for (const flag of [false, true]) {
      const { actions } = recorder();
      for (const control of penControls({ ...STATE, tool, follow: flag, canvasOnly: flag, undo: "x", redo: "y", canDelete: true }, actions)) {
        control.press();
      }
    }
  }
  assert.deepEqual(calls, ["pen_window_flags"]);
  for (const command of calls) assert.ok(PEN_ALLOWED.has(command), command);
  // Every literal command in the pen page's sources.
  for (const dir of ["pen", "components/PenViewFrame.tsx", "hooks/useWindowLink.ts", "lib/windowLink.ts", "lib/penWindowStore.ts"]) {
    const files = dir.endsWith(".tsx") || dir.endsWith(".ts") ? [dir] : readdirSync(new URL(`../src/${dir}/`, import.meta.url)).map((f) => `${dir}/${f}`);
    for (const file of files) {
      for (const match of src(file).matchAll(/\b(?:invoke|call)(?:<[^>]*>)?\(\s*"([a-z_]+)"/g)) {
        if (file === "lib/penWindowStore.ts") continue; // main window only
        assert.ok(PEN_ALLOWED.has(match[1]), `${file}: ${match[1]}`);
      }
    }
  }
});

test("criterion 8: one column fits 1920x1080 and 1366x768, the View card moves below 640", () => {
  const controls = penControls(STATE, recorder().actions);
  const one = railHeight(controls, false);
  assert.equal(one, 640);
  assert.ok(one <= 768 - 38, "fits 1366x768 under a title bar");
  assert.ok(one <= 1080 - 38);
  assert.equal(viewInSecondColumn(639), true);
  assert.equal(viewInSecondColumn(640), false);
  assert.equal(viewInSecondColumn(768), false);
  assert.ok(railHeight(controls, true) <= 639, "two columns fit a 600 px window");
  assert.ok(railHeight(controls, true) < one);
});

test("criterion 9: every control has a distinct accessible name and a visible focus state", () => {
  for (const state of [STATE, { ...STATE, undo: "Erase", redo: "Pencil stroke", canvasOnly: true }]) {
    const controls = penControls(state, recorder().actions);
    const names = controls.map((c) => c.label.trim());
    assert.ok(names.every((n) => n.length > 0));
    assert.equal(new Set(names).size, names.length);
  }
  const rail = src("pen/PenRail.tsx");
  assert.match(rail, /aria-label=\{control\.label\}/);
  assert.match(rail, /focus-visible:ring-2/);
  assert.match(rail, /aria-label="Pen tools"/);
  const strip = src("pen/PenStrip.tsx");
  assert.match(strip, /aria-label="Pen options"/);
  assert.match(src("pen/PenLayerPick.tsx"), /aria-label=\{`Layer: \$\{name\}`\}/);
  // Design-system tokens only: no raw colours in the layout files except the documented fallback.
  for (const file of ["pen/PenRail.tsx", "pen/PenStrip.tsx", "pen/PenLayerPick.tsx"]) {
    assert.doesNotMatch(src(file), /#[0-9a-fA-F]{3,8}\b/, file);
  }
});

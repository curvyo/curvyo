// The Fonts tree's Delete and Enter keys (`specs/0081-inline-delete-confirm` criterion 7):
// Delete arms the focused Curvyo family's Remove trash, Delete or Enter on an armed row removes.
import assert from "node:assert/strict";
import { test } from "node:test";

import { familyKey, type Row } from "../src/lib/fontRows.ts";
import { type FontTreeKeyContext, onFontTreeKey } from "../src/lib/fontTreeKeys.ts";
import type { FontFamily } from "../src/lib/textText.ts";

function family(block: FontFamily["block"]): FontFamily {
  return { block, family: "Lobster", faces: [] };
}

/** Runs one key on a tree whose only row is the family; returns the calls made. */
function press(key: string, fam: FontFamily, armedAtStart: boolean): string[] {
  const calls: string[] = [];
  let armed = armedAtStart;
  const row: Row = { key: familyKey(fam), kind: "family", family: fam, tags: [], open: false, height: 52 };
  const ctx: FontTreeKeyContext = {
    rows: [row],
    focusable: [row],
    activeKey: row.key,
    pageRows: 10,
    focusRow: () => calls.push("focusRow"),
    toggle: () => calls.push("toggle"),
    pickFamily: () => calls.push("pickFamily"),
    pickFace: () => calls.push("pickFace"),
    star: () => calls.push("star"),
    pressRemove: () => {
      calls.push(armed ? "remove" : "arm");
      armed = !armed;
    },
    isArmed: () => armed,
    openAddTag: () => calls.push("openAddTag"),
    focusTagLine: () => calls.push("focusTagLine"),
    returnFocus: () => calls.push("returnFocus"),
    typeInSearch: () => calls.push("typeInSearch"),
  };
  const event = {
    key,
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    shiftKey: false,
    preventDefault: () => {},
    stopPropagation: () => {},
  } as unknown as React.KeyboardEvent<HTMLElement>;
  onFontTreeKey(event, ctx);
  return calls;
}

test("Delete on a Curvyo family arms its Remove trash", () => {
  assert.deepEqual(press("Delete", family("curvyo"), false), ["arm"]);
});

test("Delete or Enter on an armed Curvyo family removes it", () => {
  assert.deepEqual(press("Delete", family("curvyo"), true), ["remove"]);
  assert.deepEqual(press("Enter", family("curvyo"), true), ["remove"]);
});

test("Enter on an unarmed family picks it, Delete on a system family does nothing", () => {
  assert.deepEqual(press("Enter", family("curvyo"), false), ["pickFamily"]);
  assert.deepEqual(press("Delete", family("system"), false), []);
});

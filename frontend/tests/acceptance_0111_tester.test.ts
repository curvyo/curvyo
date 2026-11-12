// Tester acceptance cases for `specs/0111-laser-operation-tools` on the view-only criteria: the
// kind buttons' names, order and single-line tooltip (criteria 1, 2), no kind buttons and no
// view switch on the Operations tab (8, 16), the mixed group line (10), the read-only crumb (11),
// the Operations view toggle at the page label (15), one unfolded row and none for several
// selected rows (17), and the lower part of the unfolded row (20, 21). Written from the spec and
// the UX notes; they use the pure helpers and the shared view fixtures, and read the component
// sources only for the attributes the criteria name.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { crumbRow } from "../src/lib/groupText.ts";
import { enteredBarShown, kindButtonName } from "../src/lib/kindButtons.ts";
import { lowerPart, toggleNames } from "../src/lib/operationOptionRows.ts";
import { rowName } from "../src/lib/operationRows.ts";
import { bodyOpen, pressUnfolds } from "../src/lib/operationUnfold.ts";
import { decodeOperationsText, operationIdKey, type OperationsView } from "../src/lib/operationsView.ts";
import { OPERATIONS_VIEW_NAME, operationsToggle, type PageRow } from "../src/lib/pages.ts";

const FIXTURES = new URL("../../curvyo-editor-wasm/tests/fixtures/operations_view/", import.meta.url);
const SRC = new URL("../src/", import.meta.url);

function source(path: string): string {
  return readFileSync(new URL(path, SRC), "utf8");
}

function view(name: string): OperationsView {
  const decoded = decodeOperationsText(readFileSync(new URL(name, FIXTURES), "utf8"));
  assert.ok(decoded, `${name} decodes`);
  return decoded;
}

const NO_MODS = { ctrl: false, shift: false };

test("c1 c2: five kind buttons in bar order, each named by its kind word only", () => {
  const v = view("select_bar_kinds.json");
  assert.ok(v.selectBarKinds);
  assert.deepEqual(
    v.selectBarKinds.map((k) => k.kind),
    ["cut", "score", "engrave", "engrave_outline", "cut_out"],
  );
  assert.deepEqual(
    v.selectBarKinds.map((k) => kindButtonName(k.kind)),
    ["Cut", "Score", "Engrave", "Engrave + Outline", "Cut-out"],
  );
  assert.equal(view("options_engrave.json").selectBarKinds, null, "no selection: no buttons");
  assert.equal(enteredBarShown(null), false);
  assert.equal(enteredBarShown([]), false);
});

test("c2: the tooltip is the kind name alone and the button has no visible caption", () => {
  const group = source("components/OperationKindGroup.tsx");
  assert.match(group, /content=\{name\}/, "the tooltip content is the name");
  assert.match(group, /aria-label=\{name\}/, "the accessible name is the name");
  assert.doesNotMatch(group, /content=\{`[^`]*description/, "no second tooltip line");
  assert.match(group, /type="button"/, "a real button, reachable by Tab");
  assert.doesNotMatch(group, /tabIndex=\{-1\}/, "not taken out of the Tab order");
});

test("c8 c16: the Operations tab has no kind buttons and no view switch", () => {
  const section = source("components/panel/OperationsSection.tsx");
  assert.doesNotMatch(section, /OperationKindGroup|kindButtonName/, "no kind buttons on the tab");
  assert.doesNotMatch(section, /View:|"Drawn"|>Drawn</, "no view switch on the tab");
  assert.match(section, /Cuts last/, "Cuts last stays");
});

test("c10: a mixed group row says '<group> in part' and never names members", () => {
  const v = view("group_tick_mixed.json");
  const mixed = v.rows.find((r) => r.tick === "some");
  assert.ok(mixed, "a mixed row");
  assert.ok(mixed.groupInPart);
  assert.match(rowName(mixed), new RegExp(`${mixed.groupInPart} in part`));
  const all = v.rows.find((r) => r.tick === "all");
  assert.ok(all);
  assert.doesNotMatch(rowName(all), /in part/);
});

test("c11: the crumb of an entered laser group names the page and the groups, Read-only, Esc: leave", () => {
  const row = crumbRow(2, { root: "Laser 1", groups: ["Group 3", "Group 7"] });
  const text = JSON.stringify(row);
  for (const want of ["Laser 1", "Group 3", "Group 7"]) assert.ok(text.includes(want), want);
  const crumbs = source("components/OpenGroups.tsx");
  assert.match(crumbs, /Read-only/);
  assert.match(crumbs, /Esc: leave/);
});

test("c15: the toggle exists on laser pages only, named Operations view, with aria-pressed", () => {
  const laser = { operationsView: true } as unknown as PageRow;
  const off = { operationsView: false } as unknown as PageRow;
  const drawing = { operationsView: null } as unknown as PageRow;
  assert.deepEqual(operationsToggle(laser), { pressed: true });
  assert.deepEqual(operationsToggle(off), { pressed: false });
  assert.equal(operationsToggle(drawing), null);
  assert.equal(OPERATIONS_VIEW_NAME, "Operations view");
  const frames = source("components/PageFrames.tsx");
  assert.match(frames, /ListTree/);
  assert.match(frames, /aria-pressed=\{pressed\}/);
  assert.match(frames, /size-5/, "a 20 px button face");
});

test("c17: one row opens; Ctrl or Shift presses and several selected rows unfold nothing", () => {
  const v = view("options_engrave.json");
  const row = v.rows[0];
  assert.ok(row.unfolded);
  const key = operationIdKey(row.id);
  assert.equal(bodyOpen(row, [key]), true);
  assert.equal(bodyOpen(row, [key, "other"]), false, "several rows selected: no body");
  assert.equal(pressUnfolds(row, [key], NO_MODS), true, "a press on the open row folds it");
  assert.equal(pressUnfolds(row, [key], { ctrl: true, shift: false }), false);
  assert.equal(pressUnfolds(row, [key], { ctrl: false, shift: true }), false);
  const folded = { ...row, unfolded: false };
  assert.equal(pressUnfolds(folded, [], NO_MODS), true, "a press on a folded row unfolds it");
});

test("c20 c21: the lower part of an Engrave holds two toggles and the fields, of a Cut Z alone", () => {
  const engrave = view("options_engrave.json").rows[0];
  assert.ok(engrave.settings);
  const part = lowerPart(engrave.kind, engrave.settings);
  assert.ok(part);
  assert.deepEqual(
    part.toggles.map((t) => toggleNames(t.option)),
    ["Crosshatch", "Bidirectional fill"],
  );
  assert.deepEqual(
    part.fields.map((f) => f.key),
    ["scan_angle", "overscan", "z_offset"],
    "Z offset with the motorised axis",
  );
  const cut = view("options_cut_z_hidden.json").rows[0];
  assert.ok(cut.settings);
  assert.equal(lowerPart(cut.kind, cut.settings), null, "a Cut without the axis shows no lower part");
  const withZ = { ...cut.settings, zOffsetShown: true };
  assert.deepEqual(lowerPart(cut.kind, withZ)?.fields.map((f) => f.key), ["z_offset"]);
  assert.deepEqual(lowerPart(cut.kind, withZ)?.toggles, []);
  const opts = source("components/panel/OperationOptionsPart.tsx");
  assert.match(opts, /size-7/, "28 px icon toggles");
  assert.match(opts, /aria-pressed/);
});

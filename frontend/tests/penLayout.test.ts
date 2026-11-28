// The fixed Pen layout (`specs/0158-pen-display` slice 2b criteria 1, 2, 4, 5, 7, 8): the controls
// and their order, the Eraser's second tap, the remembered switches, the touch rule, the second
// column and the sizes, and every host call the layout's controls make against the allow-list.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { bracketPath } from "../src/lib/penViewFrame.ts";
import { windowAllows } from "../src/lib/windowRole.ts";
import {
  cardHeight,
  ignoresPress,
  type PenLayoutActions,
  type PenLayoutState,
  penControls,
  railHeight,
  railRight,
  readPenFlags,
  savePenFlags,
  TARGET,
  viewInSecondColumn,
} from "../src/pen/penLayout.ts";

const STATE: PenLayoutState = {
  tool: "pencil",
  undo: "Pencil stroke",
  redo: null,
  canDelete: true,
  panning: false,
  follow: false,
  canvasOnly: false,
};

/** Actions that record what was pressed; the switches go through the layout's host call. */
function recording(calls: string[], hostCalls: string[]): PenLayoutActions {
  const host = (command: string) => {
    hostCalls.push(command);
    return Promise.resolve();
  };
  return {
    tool: (tool) => calls.push(`tool:${tool}`),
    undo: () => calls.push("undo"),
    redo: () => calls.push("redo"),
    delete: () => calls.push("delete"),
    zoom: (zoomIn) => calls.push(zoomIn ? "zoom-in" : "zoom-out"),
    fit: () => calls.push("fit"),
    pan: () => calls.push("pan"),
    follow: (on) => {
      calls.push(`follow:${on}`);
      void savePenFlags({ followView: on, canvasOnly: STATE.canvasOnly }, host);
    },
    canvasOnly: (on) => {
      calls.push(`canvas:${on}`);
      void savePenFlags({ followView: STATE.follow, canvasOnly: on }, host);
    },
  };
}

test("the layout holds the criterion's controls in order, and nothing else (criterion 1)", () => {
  const controls = penControls(STATE, recording([], []));
  assert.deepEqual(
    controls.map((c) => `${c.group}:${c.id}`),
    [
      "Tools:pencil",
      "Tools:eraser",
      "Tools:select",
      "Tools:node",
      "Edit:undo",
      "Edit:redo",
      "Edit:delete",
      "View:zoom-in",
      "View:zoom-out",
      "View:fit",
      "View:pan",
      "View:follow",
      "Corner:canvas-only",
    ],
  );
  for (const control of controls) assert.ok(control.label.length > 0, `${control.id} has a name (criterion 9)`);
  assert.equal(controls.find((c) => c.id === "undo")?.label, "Undo: Pencil stroke (Ctrl+Z)");
  assert.equal(controls.find((c) => c.id === "redo")?.disabled, true);
  assert.equal(TARGET, 48, "every control is a 48 px target");
});

test("the Eraser is a radio item; a second tap returns to the Pencil (criterion 2)", () => {
  const calls: string[] = [];
  penControls(STATE, recording(calls, [])).find((c) => c.id === "eraser")?.press();
  penControls({ ...STATE, tool: "eraser" }, recording(calls, [])).find((c) => c.id === "eraser")?.press();
  assert.deepEqual(calls, ["tool:eraser", "tool:pencil"]);
  const erasing = penControls({ ...STATE, tool: "eraser" }, recording([], []));
  assert.equal(erasing.find((c) => c.id === "eraser")?.pressed, true);
  assert.equal(erasing.find((c) => c.id === "pencil")?.pressed, false);
});

test("Follow view and Canvas only toggle and are remembered (criteria 4, 5)", () => {
  const calls: string[] = [];
  const hostCalls: string[] = [];
  const on = { ...STATE, follow: true, canvasOnly: true };
  for (const state of [STATE, on]) {
    for (const id of ["follow", "canvas-only"]) penControls(state, recording(calls, hostCalls)).find((c) => c.id === id)?.press();
  }
  assert.deepEqual(calls, ["follow:true", "canvas:true", "follow:false", "canvas:false"]);
  assert.deepEqual(hostCalls, Array(4).fill("pen_window_flags"));
  assert.equal(penControls(on, recording([], [])).find((c) => c.id === "canvas-only")?.label, "Show tools");
  assert.deepEqual(readPenFlags("?window=pen&follow=1&canvas=0"), { followView: true, canvasOnly: false });
  assert.deepEqual(readPenFlags("?window=pen"), { followView: false, canvasOnly: false }, "format 1: both off");
});

test("every host call of the layout's controls is on the pen page's allow-list (criterion 7)", () => {
  const hostCalls: string[] = [];
  for (const state of [STATE, { ...STATE, follow: true, canvasOnly: true, tool: "eraser" as const }]) {
    for (const control of penControls(state, recording([], hostCalls))) control.press();
  }
  assert.ok(hostCalls.length > 0);
  // The layout's files and the components and hooks they use: every command they name.
  const files = [
    "../src/pen/PenApp.tsx",
    "../src/pen/PenRail.tsx",
    "../src/pen/PenStrip.tsx",
    "../src/pen/PenLayerPick.tsx",
    "../src/pen/penLayout.ts",
    "../src/components/NodeToolbar.tsx",
    "../src/components/WidthEditToggle.tsx",
    "../src/components/TraceField.tsx",
    "../src/components/panel/ValueField.tsx",
    "../src/hooks/usePencilFeed.ts",
    "../src/hooks/usePenProbe.ts",
    "../src/hooks/useWindowLink.ts",
  ];
  const named = files.flatMap((file) => {
    const text = readFileSync(new URL(file, import.meta.url), "utf8");
    return [...text.matchAll(/(?:invoke(?:<[^>]*>)?\(|call\()\s*"([a-z_]+)"/g)].map((m) => m[1]);
  });
  for (const command of [...hostCalls, ...named]) assert.equal(windowAllows("pen", command), true, command);
  for (const refused of ["pick_project", "save_project_bytes", "pick_import_files", "pick_svg_export_path", "connector_send_job"]) {
    assert.equal(named.includes(refused), false, refused);
  }
});

test("a touch press does nothing; pen and mouse presses work (criterion 1)", () => {
  assert.equal(ignoresPress("touch"), true);
  assert.equal(ignoresPress("pen"), false);
  assert.equal(ignoresPress("mouse"), false);
  assert.equal(ignoresPress(""), false, "a keyboard click");
});

test("one column needs 640 px; below it the View card moves beside the others (criterion 8)", () => {
  const controls = penControls(STATE, recording([], []));
  assert.equal(cardHeight(4), 200);
  assert.equal(cardHeight(3), 152);
  assert.equal(cardHeight(5), 248);
  assert.equal(railHeight(controls, false), 640);
  assert.equal(viewInSecondColumn(640), false);
  assert.equal(viewInSecondColumn(639), true);
  // 1920 x 1080 and 1366 x 768 (about 730 px inside the title bar): one column fits.
  for (const height of [1040, 730]) assert.ok(railHeight(controls, viewInSecondColumn(height)) <= height, `${height}`);
  // A low window: two columns, still inside it.
  assert.ok(railHeight(controls, viewInSecondColumn(480)) <= 480);
  assert.equal(railRight(false), 68);
  assert.equal(railRight(true), 132);
  // The strip (layer 180, two 150 px fields, two 25 px gaps, 8 px padding) and the 68 px corner
  // leave the top row clear at 1366 px.
  const strip = 180 + 25 + 150 + 25 + 150 + 8;
  assert.ok(railRight(true) + 12 + strip + 12 + 68 < 1366);
});

test("the frame's corner brackets have 16 px legs, shorter on a small frame (criterion 6)", () => {
  assert.equal(bracketPath([10, 20, 110, 220]).split("M").length - 1, 4);
  assert.ok(bracketPath([10, 20, 110, 220]).startsWith("M10 36V20H26"));
  assert.ok(bracketPath([0, 0, 10, 10]).startsWith("M0 5V0H5"));
});

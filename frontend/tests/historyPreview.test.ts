import assert from "node:assert/strict";
import test from "node:test";

import {
  MODE_TOOLTIP,
  modeCaption,
  parsePreview,
  previewBanner,
  previewReadout,
  sinceText,
} from "../src/lib/historyPreview.ts";

const now = new Date(2026, 9, 10, 15, 0, 0);
const time = new Date(2026, 9, 10, 12, 3, 0).getTime() / 1000;

test("the readout names the id, the step, the author and the time", () => {
  const state = parsePreview(
    JSON.stringify({ kind: "object", id: "a1b2c3d", name: "Resize", author: "you", time, since: 0 }),
  );
  assert.equal(previewReadout(state, now), "Preview a1b2c3d, Resize, You, 12:03");
});

test("the banner says editing is paused and counts the steps that arrived", () => {
  const state = parsePreview(JSON.stringify({ kind: "document", id: "a1b2c3d", time, since: 3 }));
  assert.equal(previewBanner(state, now), "Previewing a1b2c3d, 12:03. Editing is paused.");
  assert.equal(sinceText(state), "(3 new steps since)");
  assert.equal(sinceText({ kind: "document", since: 1 }), "(1 new step since)");
  assert.equal(sinceText({ kind: "document", since: 0 }), "");
});

test("no preview reads as an empty kind", () => {
  assert.deepEqual(parsePreview(JSON.stringify({ kind: "" })), { kind: "" });
});

test("the Mode captions and tooltip are the words of criterion 14", () => {
  assert.equal(modeCaption(false), "A click shows the version. Nothing changes.");
  assert.equal(modeCaption(true), "A click puts the object into that version. Ctrl+Z undoes it.");
  assert.match(MODE_TOOLTIP, /^Preview shows a version over the canvas and changes nothing\./);
});

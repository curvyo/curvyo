// The words and rules of the On path button, the row-2 dot and the position handle's entry box
// (`specs/0080-text-on-path` criteria 1, 17, 48; UX notes §0.6, §1, §5; customer 2026-10-24).
import assert from "node:assert/strict";
import { test } from "node:test";

import { changedRow2Controls, changedText } from "../src/lib/textBarRows.ts";
import {
  EMPTY_TEXT_PATH,
  ON_PATH_WRITING_LINE,
  onPathEnabled,
  onPathTooltipLine,
  parseTextPath,
  parseTextPathEntry,
  pathFieldText,
  positionUnit,
  writingGreyedOnPath,
} from "../src/lib/textPathText.ts";

test("the session's view reads, damage as the empty view", () => {
  const view = parseTextPath(JSON.stringify({ texts: 1, picking: true, onPath: 1, vertical: false }));
  assert.deepEqual(view, { texts: 1, picking: true, onPath: 1, vertical: false });
  assert.deepEqual(parseTextPath("not json"), EMPTY_TEXT_PATH);
});

test("customer 2026-10-24: the entry box reads its handle and values, closed as null", () => {
  const entry = parseTextPathEntry(
    JSON.stringify({
      handle: { x: 120, y: 80 },
      side: "below",
      line: "top",
      position: 270.04,
      degrees: true,
      distance: -1.5,
    }),
  );
  assert.deepEqual(entry?.handle, { x: 120, y: 80 });
  assert.deepEqual(entry?.box, { left: 120, top: 80, right: 120, bottom: 80 }, "no box: the handle");
  const boxed = parseTextPathEntry(
    JSON.stringify({
      handle: { x: 120, y: 80 },
      box: { left: 60, top: 30, right: 180, bottom: 82 },
      side: "above",
      line: "baseline",
      position: 10,
      distance: 0,
    }),
  );
  assert.deepEqual(boxed?.box, { left: 60, top: 30, right: 180, bottom: 82 });
  assert.equal(entry?.side, "below");
  assert.equal(entry?.line, "top");
  assert.equal(entry ? positionUnit(entry) : "", "°");
  assert.equal(pathFieldText(entry?.position ?? 0), "270.0");
  assert.equal(pathFieldText(entry?.distance ?? 0), "-1.5");
  assert.equal(parseTextPathEntry(""), null);
  assert.equal(parseTextPathEntry(JSON.stringify({ handle: { x: 1, y: 2 }, side: "left" })), null);
});

test("UX §0.6: enabled with exactly one text; with more the tooltip names the state", () => {
  assert.equal(onPathEnabled({ ...EMPTY_TEXT_PATH, texts: 1 }), true);
  assert.equal(onPathEnabled({ ...EMPTY_TEXT_PATH, texts: 3 }), false);
  assert.equal(onPathTooltipLine({ ...EMPTY_TEXT_PATH, texts: 3 }), "3 texts selected");
  assert.equal(onPathTooltipLine({ ...EMPTY_TEXT_PATH, texts: 1 }), null);
});

test("criterion 48: a vertical text on a path greys the writing groups with its line", () => {
  assert.equal(writingGreyedOnPath({ ...EMPTY_TEXT_PATH, onPath: 1, vertical: true }), true);
  assert.equal(writingGreyedOnPath({ ...EMPTY_TEXT_PATH, onPath: 1, vertical: false }), false);
  assert.equal(ON_PATH_WRITING_LINE, "Text on a path runs along the path");
});

test("UX §5: the expand button's dot says 'Changed: On path' for a text on a path", () => {
  const changed = changedRow2Controls({
    writingMode: "horizontal",
    orientation: "auto",
    path: { onPath: 1 },
  });
  assert.equal(changedText(changed), "Changed: On path");
  assert.deepEqual(changedRow2Controls({ writingMode: "horizontal", orientation: "auto" }), []);
});

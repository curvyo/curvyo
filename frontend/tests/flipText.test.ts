import assert from "node:assert/strict";
import { test } from "node:test";

import {
  FLIP_ANNOUNCEMENT,
  FLIP_TEXT,
  FLIP_UNCHANGED_NOTICE,
  flipKeyAxis,
} from "../src/lib/flipText.ts";

test("the texts of criterion 1 and the notice of criterion 3", () => {
  assert.equal(FLIP_TEXT.horizontal.name, "Flip horizontally");
  assert.equal(FLIP_TEXT.vertical.name, "Flip vertically");
  assert.equal(
    `${FLIP_TEXT.horizontal.name}: ${FLIP_TEXT.horizontal.rule.replace(/^M/, "m").replace(/\.$/, "")}`,
    "Flip horizontally: mirror the selection left to right about the middle of its box",
  );
  assert.equal(FLIP_UNCHANGED_NOTICE, "The selection looks the same flipped. Nothing changed.");
  assert.ok(FLIP_UNCHANGED_NOTICE.length <= 70);
  assert.equal(FLIP_ANNOUNCEMENT.vertical, "Flipped vertically");
});

test("the key outcomes of H and V", () => {
  assert.equal(flipKeyAxis("flip-horizontal"), "horizontal");
  assert.equal(flipKeyAxis("flip-vertical"), "vertical");
  assert.equal(flipKeyAxis("flip-unchanged"), null);
  assert.equal(flipKeyAxis("nudge"), null);
});

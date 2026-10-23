// The words of single-line fonts (`specs/0076-single-line-fonts` criteria 4 and 19, UX notes
// §1 and §5).
import assert from "node:assert/strict";
import { test } from "node:test";

import { chipTooltip } from "../src/lib/fontLibraryText.ts";
import { textRefusalOutlines, textRefusalText } from "../src/lib/textText.ts";

test("criterion 19: the area refusal of single-line texts, outlined", () => {
  assert.equal(
    textRefusalText({ kind: "single_line_texts", count: 1 }),
    "Single-line text has no area. Nothing was changed.",
  );
  assert.equal(
    textRefusalText({ kind: "single_line_texts", count: 2 }),
    "2 single-line texts have no area. Nothing was changed.",
  );
  assert.ok(textRefusalOutlines("single_line_texts"));
  assert.ok("2 single-line texts have no area. Nothing was changed.".length <= 70);
});

test("criterion 4: the Single line chip's tooltip", () => {
  assert.equal(chipTooltip("detected:single_line"), "Detected: single-line font");
});

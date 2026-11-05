// Run with `npm test`. The laser page's work area line of `src/lib/workAreaText.ts`
// (`specs/0098-page-kinds` UX notes §9: in the document's display unit).

import assert from "node:assert/strict";
import { test } from "node:test";

import { formatWorkArea } from "../src/lib/workAreaText.ts";

test("the work area reads in the display unit without trailing zeros", () => {
  assert.equal(formatWorkArea(400, 300, "mm"), "400 × 300 mm");
  assert.equal(formatWorkArea(400, 300, "cm"), "40 × 30 cm");
  assert.equal(formatWorkArea(400, 300, "in"), "15.748 × 11.811 in");
  assert.equal(formatWorkArea(600.5, 400, "mm"), "600.5 × 400 mm");
});

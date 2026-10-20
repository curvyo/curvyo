// Run with `npm test`. The words and Gap rules of `src/lib/weldText.ts` against
// `specs/0068-laserable-lettering/specification.md` (criteria 2, 10, 23, 27, 28, UX notes §3-§5).

import assert from "node:assert/strict";
import { test } from "node:test";

import { dimmedPress } from "../src/lib/dimmedPress.ts";
import {
  type WeldResult,
  gapBlurCommit,
  gapClampNotice,
  gapText,
  parseGap,
  readStoredGap,
  stepGap,
  storeGap,
  weldRefusalText,
  weldSuccessText,
} from "../src/lib/weldText.ts";

function result(fields: Partial<WeldResult>): WeldResult {
  return {
    kind: "applied",
    count: 0,
    of: 1,
    textsOnly: false,
    compound: false,
    pieces: 1,
    dashed: false,
    closedOpen: 0,
    removedOpen: 0,
    mixedStyles: false,
    ...fields,
  };
}

test("criterion 23: the success notice counts objects or texts and pieces", () => {
  assert.equal(weldSuccessText(result({})), "Weld: 1 object became 1 path, 1 piece.");
  assert.equal(
    weldSuccessText(result({ of: 1, textsOnly: true, compound: true, pieces: 3 })),
    "Weld: 1 text became 1 compound path, 3 pieces.",
  );
  assert.equal(
    weldSuccessText(result({ of: 3, compound: true, pieces: 4, dashed: true, closedOpen: 1, mixedStyles: true })),
    "Weld: 3 objects became 1 compound path, 4 pieces. Dashes were welded as solid lines. 1 open outline was closed with a straight line. It uses the style of the lowest object.",
  );
});

test("criteria 27 and 28: refusals use the sentences of 0074 and 0016 with Weld", () => {
  assert.equal(
    weldRefusalText(result({ kind: "missing_fonts", count: 1, face: "Source Serif 4 Bold" })),
    "Font Source Serif 4 Bold is missing. Nothing was changed.",
  );
  assert.equal(
    weldRefusalText(result({ kind: "no_area", count: 1, of: 1 })),
    "Weld needs shapes that enclose an area. 1 of 1 selected object has no area. Nothing was changed.",
  );
  assert.match(weldRefusalText(result({ kind: "out_of_range", count: 1, of: 3 })) ?? "", /^Weld works only within 10 km/);
});

test("criterion 2: the Gap field's value, steps, units, clamp notices and storage", () => {
  assert.equal(gapText(0.1), "0.1");
  assert.equal(gapText(0), "0");
  assert.equal(stepGap(0.1, true, false), 0.11);
  assert.equal(stepGap(0.95, true, true), 1);
  assert.equal(stepGap(0.005, false, false), 0);
  assert.equal(parseGap("0.004 in"), 0.1, "a typed unit is rounded to 0.01 mm");
  assert.equal(parseGap("abc"), null);
  assert.equal(gapClampNotice("largest"), "Gap set to 1 mm, the largest gap.");
  assert.equal(gapClampNotice("smallest"), "Gap set to 0 mm, the smallest gap.");
  assert.equal(gapClampNotice(""), null);
  const store = new Map<string, string>();
  const storage = { getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => void store.set(k, v) };
  assert.equal(readStoredGap(storage), 0.1);
  storeGap(storage, 0.25);
  assert.equal(readStoredGap(storage), 0.25);
  const broken = { getItem: () => { throw new Error("blocked"); }, setItem: () => { throw new Error("blocked"); } };
  assert.equal(readStoredGap(broken), 0.1);
  storeGap(broken, 0.3);
});

test("UX review: Escape discards the Gap edit, so the blur after it saves nothing", () => {
  assert.equal(gapBlurCommit("0.5", true), null);
  assert.equal(gapBlurCommit("0.5", false), "0.5");
});

test("UX review: a press on a dimmed button prevents the default, so the tooltip stays", () => {
  let prevented = false;
  const event = { preventDefault: () => { prevented = true; } };
  assert.equal(dimmedPress(event, true), false);
  assert.equal(prevented, false);
  assert.equal(dimmedPress(event, false), true);
  assert.equal(prevented, true);
});

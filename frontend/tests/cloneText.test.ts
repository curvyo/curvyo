import assert from "node:assert/strict";
import { test } from "node:test";

import { NO_CLONE_PANEL, cloneBlockLines, unlinkTitle } from "../src/lib/cloneText.ts";

test("the clone block names one clone's original and a chain's root", () => {
  const one = { ...NO_CLONE_PANEL, clonesOnly: true, count: 1, original: "Path 3f9a1c", deepest: 1 };
  assert.deepEqual(cloneBlockLines(one), {
    title: "Linked clone of Path 3f9a1c.",
    chain: "",
  });
  const chain = { ...one, original: "Clone of Path 3f9a1c", deepest: 3, root: "Path 3f9a1c" };
  assert.equal(cloneBlockLines(chain).chain, "Chain of 3 clones. The root original is Path 3f9a1c.");
  const several = { ...chain, count: 3 };
  assert.equal(cloneBlockLines(several).title, "3 linked clones.");
  assert.equal(cloneBlockLines(several).chain, "Chains of up to 3 clones.");
});

test("Unlink clone of a text clone says it becomes a text (0080 criterion 41)", () => {
  assert.match(unlinkTitle(NO_CLONE_PANEL), /ordinary path/);
  assert.match(unlinkTitle({ ...NO_CLONE_PANEL, texts: true, placementRow: false }), /ordinary text/);
});

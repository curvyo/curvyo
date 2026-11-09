// The drag answer decoder against the Rust-produced fixture (`specs/0100-bring-content-in`
// `adrs.md` note 4; `curvyo-editor-wasm/tests/drag_answer_contract.rs` writes the same file).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  DROP_BADGES,
  asDragAnswer,
  badgeWord,
  decodeDragAnswer,
} from "../src/lib/dragAnswer.ts";

const fixture: unknown[] = JSON.parse(
  readFileSync(new URL("../src/lib/fixtures/drag-answers.json", import.meta.url), "utf8"),
);

test("every Rust answer decodes, one per badge", () => {
  const badges = fixture.map((value) => {
    const answer = asDragAnswer(value);
    assert.ok(answer, JSON.stringify(value));
    return answer.badge;
  });
  assert.deepEqual([...badges].sort(), [...DROP_BADGES].sort());
});

test("an unknown badge and a missing field decode to null", () => {
  assert.equal(decodeDragAnswer('{"badge":"teleport","target":null,"faint":false}'), null);
  assert.equal(decodeDragAnswer('{"badge":"clone","faint":false}'), null);
  assert.equal(decodeDragAnswer('{"badge":"clone","target":null}'), null);
  assert.equal(decodeDragAnswer("not json"), null);
  assert.equal(decodeDragAnswer(""), null);
});

test("every badge but none has a word", () => {
  for (const kind of DROP_BADGES) {
    const word = badgeWord(kind, "Coaster");
    assert.equal(word === null, kind === "none", kind);
  }
  assert.equal(badgeWord("only_on_laser_page", ""), "Only on a laser page");
  assert.equal(badgeWord("placed_page", ""), "Placed page");
});

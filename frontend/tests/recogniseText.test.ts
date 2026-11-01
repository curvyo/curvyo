// The words of Recognise text (`specs/0088-trace-recognition` criteria 3, 4, 6, 7 and 10; UX
// notes §2 and §4): the notices built from the session's outcome, the Faces readout and the
// stored tolerance.
import { strict as assert } from "node:assert";
import { test } from "node:test";

import {
  TOLERANCE_KEY,
  facesReadout,
  guessedText,
  guessedWarning,
  noticeMs,
  nothingText,
  parseOutcome,
  readTolerance,
  searchedReadout,
  writeTolerance,
} from "../src/lib/recogniseText.ts";

const line = {
  string: "Laser 42",
  face: "Source Sans 3 Bold",
  tier: "included",
  letters: 12,
  guessed: [] as [string, number][],
  deviation: 0.123,
};
const none = { images: 0, open_paths: 0, others: 0 };

test("criterion 4: nothing within the tolerance names the nearest face", () => {
  const nothing = { kind: "nothing" as const, no_lines: false, nearest: { face: "Source Serif 4 Regular", deviation: 0.844 }, searched: "system" };
  assert.equal(nothingText(nothing, 0.3), "No text within 0.30 mm; nearest face Source Serif 4 Regular, 0.84 mm.");
  assert.equal(nothingText({ ...nothing, no_lines: true, nearest: null }, 0.3), "No text lines found.");
  assert.equal(searchedReadout("system"), "Searched: system fonts");
  assert.equal(searchedReadout(null), null);
});

test("criterion 10: the Faces readout", () => {
  assert.equal(facesReadout(5), "5 face sources");
  assert.equal(facesReadout(1), "1 face source");
});

test("criterion 6: the tolerance is a stored preference, 0.3 mm by default, clamped", () => {
  const store = new Map<string, string>();
  const storage = { getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => void store.set(k, v) };
  assert.equal(readTolerance(storage), 0.3);
  writeTolerance(storage, 0.45);
  assert.equal(store.get(TOLERANCE_KEY), "0.45");
  assert.equal(readTolerance(storage), 0.45);
  store.set(TOLERANCE_KEY, "9");
  assert.equal(readTolerance(storage), 2);
  store.set(TOLERANCE_KEY, "x");
  assert.equal(readTolerance(storage), 0.3);
  assert.equal(readTolerance(null), 0.3);
});

test("the outcome JSON and the notice life", () => {
  assert.deepEqual(parseOutcome('{"kind":"running","done":1,"total":3}'), { kind: "running", done: 1, total: 3 });
  assert.deepEqual(parseOutcome("nonsense"), { kind: "ignored" });
  assert.equal(noticeMs("short"), 3000);
  assert.equal(noticeMs("x".repeat(71)), 8000);
});

test("lead decision 2026-10-31: the bar's words for a selection without a closed path", async () => {
  const { RECOGNISE_NOT_PATHS } = await import("../src/lib/recogniseText.ts");
  assert.equal(RECOGNISE_NOT_PATHS, "No closed paths selected.");
});

// No "Recognised N letters as ..." notice since the customer rule of 2026-10-31: the written text
// is the feedback. Only guessed letters (criterion 7) are warned about.
test("criterion 7: a clean run has no notice, guessed letters are a warning", () => {
  assert.equal(guessedWarning({ kind: "applied", lines: [line], left: 0, skipped: none }), null);
  assert.equal(
    guessedWarning({ kind: "applied", lines: [{ ...line, guessed: [["e", 5], ["l", 9]] }], left: 0, skipped: none }),
    "2 letters guessed: e (5), l (9).",
  );
  assert.equal(
    guessedText([["e", 5], ["l", 9], ["t", 12], ["a", 14], ["r", 20], ["x", 21], ["y", 22], ["z", 23]]),
    "8 letters guessed: e (5), l (9), t (12), a (14), r (20) and 3 more",
  );
  assert.equal(guessedText([["l", 9]]), "1 letter guessed: l (9)");
});

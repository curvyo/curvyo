// Tester acceptance cases for `specs/0081-inline-delete-confirm`, written from the specification
// before reading the implementation; they add the boundary cases `deleteArm.test.ts` leaves out.
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { test } from "node:test";

import { ARM_GUARD_MS, ARM_TIMEOUT_MS, IDLE, disarm, pressTrash } from "../src/lib/deleteArm.ts";
import { armedRemoveLabel, armedTagLabel, keptNotice } from "../src/lib/fontLibraryText.ts";

const design = readFileSync(new URL("../../docs/design-system.md", import.meta.url), "utf8");

/** The bullet starting with `- **name**`, up to the next bullet or blank line. */
function bullet(name: string): string | undefined {
  const lines = design.split("\n");
  const start = lines.findIndex((l) => l.startsWith(`- **${name}**`));
  if (start < 0) return undefined;
  let end = start + 1;
  while (end < lines.length && lines[end].trim() !== "" && !lines[end].startsWith("- ")) end += 1;
  return lines.slice(start, end).join(" ").replace(/\s+/g, " ");
}

test("c4: the guard is 400 ms and the second press just before the 4 s timeout still removes", () => {
  assert.equal(ARM_GUARD_MS, 400);
  const armed = pressTrash(IDLE, "font:Lobster", 5_000).state;
  assert.equal(pressTrash(armed, "font:Lobster", 5_000 + ARM_TIMEOUT_MS - 1).remove, true);
});

test("c4: a double-click (several presses inside 400 ms) never removes and does not restart the guard", () => {
  let state = pressTrash(IDLE, "font:Lobster", 1_000).state;
  for (const t of [1_050, 1_150, 1_399]) {
    const r = pressTrash(state, "font:Lobster", t);
    assert.equal(r.remove, false);
    state = r.state;
  }
  // The guard counts from the arming press, not from the ignored presses.
  assert.equal(pressTrash(state, "font:Lobster", 1_400).remove, true);
});

test("c4: a clock that steps backwards never removes", () => {
  const armed = pressTrash(IDLE, "tag:Wedding", 10_000).state;
  assert.equal(pressTrash(armed, "tag:Wedding", 9_000).remove, false);
});

test("c5: after disarming, the next press arms again and only a later one removes", () => {
  const armed = pressTrash(IDLE, "font:Lobster", 0).state;
  const back = disarm();
  assert.deepEqual(back, IDLE);
  const again = pressTrash(back, "font:Lobster", 1_000);
  assert.equal(again.remove, false);
  assert.equal(again.state.armed, "font:Lobster");
  assert.notDeepEqual(again.state, armed);
});

test("c6/c13: a font and a tag can be armed one after the other; the first press on the other never removes", () => {
  let state = pressTrash(IDLE, "font:Lobster", 0).state;
  const tag = pressTrash(state, "tag:Wedding", 2_000);
  assert.equal(tag.remove, false);
  state = tag.state;
  const fontAgain = pressTrash(state, "font:Lobster", 2_500);
  assert.equal(fontAgain.remove, false, "pressing the first trash again arms it, it does not remove");
  assert.equal(fontAgain.state.armed, "font:Lobster");
});

test("c6: an armed trash is never shared by two different targets at the start of time", () => {
  // IDLE has at = 0: a first press at t = 0 must arm, not be taken as a second press.
  const r = pressTrash(IDLE, "font:A", 0);
  assert.equal(r.remove, false);
  assert.equal(r.state.armed, "font:A");
});

test("c3/c11: armed labels keep unusual family and tag names verbatim", () => {
  assert.equal(
    armedRemoveLabel("Noto Sans CJK JP", 0),
    "Press again to remove Noto Sans CJK JP from Curvyo fonts.",
  );
  assert.equal(armedTagLabel("Hochzeit & Taufe", 2), "Press again to delete tag Hochzeit & Taufe. Its 2 fonts stay in Curvyo.");
});

test("c8: the TTC notice wording is unchanged", () => {
  assert.equal(keptNotice("Lobster"), "Lobster is in a file with other fonts and was kept.");
});

test("c2: the Remove row component is gone", () => {
  assert.equal(existsSync(new URL("../src/components/panel/FontRemoveRow.tsx", import.meta.url)), false);
});

test("c17: design system has the Inline delete confirm row naming both uses", () => {
  const row = bullet("Inline delete confirm");
  assert.ok(row, "row Inline delete confirm missing");
  for (const part of ["Trash2", "--destructive-soft", "400 ms", "4 s", "Escape", "Delete", "Enter", "Remove from Curvyo fonts", "Delete tag"]) {
    assert.ok(row.includes(part), `Inline delete confirm row lacks ${part}`);
  }
  const old = bullet("Inline confirm");
  assert.ok(old);
  assert.ok(!/Remove from Curvyo fonts|Delete tag/.test(old), "Inline confirm still names the moved uses");
  assert.match(design, /Remove a Curvyo font \| `Delete` on its family row, then `Delete` or `Enter`/);
  assert.match(design, /Delete an own tag \| `Delete` on its cell in the tag strip, then `Delete` or `Enter`/);
  assert.match(design, /`--destructive-soft`/);
});

// White-box: the strip keys of criterion 14.
import { stripKeyAction } from "../src/lib/tagStrip.ts";

test("c14: Delete arms an own tag cell only; Enter is not a strip action by itself", () => {
  assert.equal(stripKeyAction("Delete", true), "delete");
  assert.equal(stripKeyAction("Delete", false), null);
  assert.equal(stripKeyAction("Enter", true), null);
  assert.equal(stripKeyAction("Backspace", true), null);
});

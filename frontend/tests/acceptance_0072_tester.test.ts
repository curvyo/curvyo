// Tester acceptance test for specs/0072-pen-angle-snap-and-typed-values criterion 36: the Pen
// tool's rail tooltip gets no operating hint, and both functions are in the shortcut table.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const rail = readFileSync(new URL("../src/components/ToolRail.tsx", import.meta.url), "utf8");
const design = readFileSync(new URL("../../docs/design-system.md", import.meta.url), "utf8");

test("c36 the Pen rail button has no hint line naming Ctrl or Tab", () => {
  const start = rail.indexOf('tool="pen"');
  assert.ok(start >= 0, "the Pen button is in the rail");
  const button = rail.slice(start, rail.indexOf("/>", start));
  assert.ok(!/\bhint=/.test(button), button);
  assert.ok(!/Ctrl|Tab/.test(button), button);
});

test("c36 the shortcut table lists the angle lock and the typed node", () => {
  const rows = design.split("\n").filter((line) => line.startsWith("| Pen:"));
  assert.ok(rows.some((row) => row.includes("Ctrl") && row.includes("`0072`")), "Ctrl row");
  assert.ok(rows.some((row) => row.startsWith("| Pen: type") && row.includes("Tab")), "Tab row");
});

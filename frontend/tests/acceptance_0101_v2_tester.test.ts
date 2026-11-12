// Tester acceptance cases for `specs/0101-laser-operations` after the customer's design of
// 2026-11-10 (no operation colour, rename in place, armed trash, arrows and the section line,
// no machine settings). Written from the spec and the UX notes before reading the operation
// components; they use the pure helpers the components are built on.
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { test } from "node:test";

import { ARM_GUARD_MS, ARM_TIMEOUT_MS, IDLE, disarm, pressTrash } from "../src/lib/deleteArm.ts";
import { neighbourIndex, sectionOf } from "../src/lib/operationRows.ts";
import { decodeOperationsText, decodeOperationsView, type OperationsView } from "../src/lib/operationsView.ts";

const FIXTURES = new URL("../../curvyo-editor-wasm/tests/fixtures/operations_view/", import.meta.url);
const INDEX_CSS = new URL("../src/index.css", import.meta.url);

function fixtureText(name: string): string {
  return readFileSync(new URL(name, FIXTURES), "utf8");
}

function view(name: string): OperationsView {
  const decoded = decodeOperationsText(fixtureText(name));
  assert.ok(decoded, `${name} decodes`);
  return decoded;
}

/** Every object key in `value`, at any depth. */
function keysDeep(value: unknown, out: string[] = []): string[] {
  if (Array.isArray(value)) {
    for (const item of value) keysDeep(item, out);
  } else if (typeof value === "object" && value !== null) {
    for (const [k, v] of Object.entries(value)) {
      out.push(k);
      keysDeep(v, out);
    }
  }
  return out;
}

// Power, speed, passes and the material link left out since `specs/0028` (criteria 24 to 26 add
// the settings to the Operations tab, amending `0101` Out of scope); colour and kerf stay out.
const COLOUR_OR_MACHINE = /colou?r|swatch|hex|rgb|kerf/i;

// ---- criterion 3 and 4: the armed trash (0081 timings, criterion 4 of 0101) -------------------

test("c4: the first press arms, a press inside 400 ms is ignored, a press at 400 ms removes", () => {
  assert.equal(ARM_GUARD_MS, 400);
  const armed = pressTrash(IDLE, "cut-1", 1000);
  assert.equal(armed.remove, false);
  assert.equal(armed.state.armed, "cut-1");

  const early = pressTrash(armed.state, "cut-1", 1399);
  assert.equal(early.remove, false, "399 ms after arming is ignored");
  assert.equal(early.state.armed, "cut-1", "an ignored press keeps the arm");

  const second = pressTrash(armed.state, "cut-1", 1400);
  assert.equal(second.remove, true, "400 ms after arming deletes");
});

test("c4: the arm disarms by itself after 4 s; a press after that arms again and does not delete", () => {
  assert.equal(ARM_TIMEOUT_MS, 4000);
  const armed = pressTrash(IDLE, "cut-1", 0);
  const late = pressTrash(armed.state, "cut-1", ARM_TIMEOUT_MS);
  assert.equal(late.remove, false, "a press at the timeout does not delete");
  assert.equal(late.state.armed, "cut-1", "it arms again");
});

test("c4: a press just before the timeout still deletes after the guard", () => {
  const armed = pressTrash(IDLE, "cut-1", 0);
  assert.equal(pressTrash(armed.state, "cut-1", ARM_TIMEOUT_MS - 1).remove, true);
});

test("c4: only one trash is armed at a time: arming another row moves the arm", () => {
  const first = pressTrash(IDLE, "cut-1", 0);
  const other = pressTrash(first.state, "score-2", 500);
  assert.equal(other.state.armed, "score-2", "the second row is armed");
  assert.equal(other.remove, false);
  // The first row's trash is no longer armed: a press on it arms it again, it does not delete.
  const back = pressTrash(other.state, "cut-1", 900);
  assert.equal(back.remove, false, "the first row was disarmed by the other arm");
  assert.equal(back.state.armed, "cut-1");
});

test("c4: Escape or a press elsewhere disarms; the next press arms and does not delete", () => {
  const armed = pressTrash(IDLE, "cut-1", 0);
  const off = disarm();
  assert.deepEqual(off, IDLE);
  const next = pressTrash(off, "cut-1", 2000);
  assert.equal(next.remove, false);
  assert.equal(next.state.armed, "cut-1");
  assert.equal(armed.remove, false);
});

// ---- criterion 5 and 6: arrows at the section line, with Cuts last on and off ----------------

test("c5: with Cuts last on, Up at the top row and Down at the bottom of Marking are refused", () => {
  const v = view("five_kinds_cuts_last.json");
  assert.equal(v.cutsLast, true);
  const kinds = v.rows.map((r) => `${r.kind}:${sectionOf(r)}`);
  // The fixture lists the marking section first, then the cutting section.
  const lastMarking = v.rows.findIndex((r, i) => sectionOf(r) === "cutting") - 1;
  assert.ok(lastMarking >= 0, `the fixture has both sections: ${kinds.join(", ")}`);
  assert.equal(neighbourIndex(v, 0, -1), null, "Up at the top of the list is refused");
  assert.equal(neighbourIndex(v, lastMarking, 1), null, "Down at the bottom of Marking is refused");
  assert.equal(neighbourIndex(v, lastMarking + 1, -1), null, "Up at the top of Cutting is refused");
});

test("c5: inside a section the arrows move one place", () => {
  const v = view("five_kinds_cuts_last.json");
  const cutting = v.rows.map((r, i) => [r, i] as const).filter(([r]) => sectionOf(r) === "cutting");
  const marking = v.rows.map((r, i) => [r, i] as const).filter(([r]) => sectionOf(r) === "marking");
  if (cutting.length >= 2) {
    const [, first] = cutting[0]!;
    assert.equal(neighbourIndex(v, first + 1, -1), first, "a Cut-section row moves up inside Cutting");
  }
  if (marking.length >= 2) {
    const [, first] = marking[0]!;
    assert.equal(neighbourIndex(v, first, 1), first + 1, "a Marking row moves down inside Marking");
  }
});

test("c6: with Cuts last off the arrows cross the former section line", () => {
  const v = view("cuts_last_off.json");
  assert.equal(v.cutsLast, false);
  for (let i = 0; i + 1 < v.rows.length; i += 1) {
    assert.equal(neighbourIndex(v, i, 1), i + 1, `row ${i} moves down to ${i + 1}`);
  }
});

test("c5: an unknown operation has no arrows", () => {
  const raw = JSON.parse(fixtureText("unknown_and_clone_members.json")) as { rows: Array<Record<string, unknown>> };
  const v = decodeOperationsView(raw);
  assert.ok(v);
  const at = v.rows.findIndex((r) => r.kind === "unknown");
  assert.ok(at >= 0, "the fixture has an unknown row");
  assert.equal(neighbourIndex(v, at, 1), null);
  assert.equal(neighbourIndex(v, at, -1), null);
});

// ---- criterion 1, 3 and 18: no colour anywhere in the view -----------------------------------

test("c18: no fixture carries a colour, swatch or machine-setting key", () => {
  for (const name of readdirSync(FIXTURES)) {
    if (!name.endsWith(".json")) continue;
    const keys = keysDeep(JSON.parse(fixtureText(name)));
    const bad = keys.filter((k) => COLOUR_OR_MACHINE.test(k));
    assert.deepEqual(bad, [], `${name} has keys that must not exist`);
  }
});

test("c18: the decoder drops a colour key from a row instead of carrying it", () => {
  const raw = JSON.parse(fixtureText("five_kinds_cuts_last.json")) as { rows: Array<Record<string, unknown>> };
  raw.rows[0] = { ...raw.rows[0], colour: "op-1" };
  const v = decodeOperationsView(raw);
  assert.ok(v);
  assert.equal("colour" in (v.rows[0] as object), false, "no colour field on a decoded row");
});

test("c1, c18: the operation UI sources and the index stylesheet carry no operation colour tokens", () => {
  const css = readFileSync(INDEX_CSS, "utf8");
  assert.equal(/--op-\d/.test(css), false, "no --op-1 to --op-8 tokens");
  assert.ok(/--cut:\s*#C8102E/i.test(css), "--cut is the Cut red");
  assert.ok(/--engrave-hatch:\s*#C46A1C/i.test(css), "--engrave-hatch is the pale orange hatch");
  const sources = [
    new URL("../src/lib/operationsView.ts", import.meta.url),
    new URL("../src/lib/operationRows.ts", import.meta.url),
    new URL("../src/components/panel/OperationRow.tsx", import.meta.url),
    new URL("../src/components/panel/OperationsSection.tsx", import.meta.url),
  ];
  for (const url of sources) {
    const text = readFileSync(url, "utf8");
    assert.equal(/colou?r|swatch|--op-/i.test(text), false, `${url.pathname} names no colour`);
  }
});

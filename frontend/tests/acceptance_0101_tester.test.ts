// Independent checks for `specs/0101-laser-operations` on the frontend: the decoder keeps an
// unknown kind as "unknown" and does not drop the list (criterion 26), the summary carries no
// missing count (Frage 169 A, criterion 27), the view fixtures decode with the right counts
// (criterion 17), and the eight operation colours are at least 3:1 on the page ground and on the
// canvas (criterion 18).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { decodeOperationsText, decodeOperationsView } from "../src/lib/operationsView.ts";

const FIXTURES = new URL("../../curvyo-editor-wasm/tests/fixtures/operations_view/", import.meta.url);
const INDEX_CSS = new URL("../src/index.css", import.meta.url);

function fixtureText(name: string): string {
  return readFileSync(new URL(name, FIXTURES), "utf8");
}

function fixtureObject(name: string): Record<string, unknown> {
  return JSON.parse(fixtureText(name)) as Record<string, unknown>;
}

test("c17: the five-kind fixture decodes with its list, Cuts last and counts", () => {
  const view = decodeOperationsText(fixtureText("five_kinds_cuts_last.json"));
  assert.ok(view, "the fixture decodes");
  assert.equal(view.cutsLast, true);
  const kinds = view.rows.map((r) => r.kind).sort();
  assert.deepEqual(kinds, ["cut", "cut", "cut_out", "engrave", "engrave_outline", "score"]);
  assert.equal(typeof view.summary.running, "number");
  assert.equal(typeof view.summary.notAssigned, "number");
  assert.equal(typeof view.summary.runsWhileHidden, "number");
  assert.equal(typeof view.summary.outputOff, "number");
});

test("c26: an unknown kind is kept as the unknown kind and the list is not dropped", () => {
  const raw = fixtureObject("unknown_and_clone_members.json") as { rows: Array<Record<string, unknown>> };
  const before = raw.rows.length;
  const view = decodeOperationsView(raw);
  assert.ok(view, "a list with an unknown kind still decodes");
  assert.equal(view.rows.length, before, "no row is dropped");
  assert.ok(view.rows.some((r) => r.kind === "unknown"), "an unknown kind is listed as unknown");
});

test("c26: a kind string this build does not know decodes as unknown, not as a cut", () => {
  const raw = fixtureObject("five_kinds_cuts_last.json") as { rows: Array<Record<string, unknown>> };
  raw.rows[0] = { ...raw.rows[0], kind: "sawdust" };
  const view = decodeOperationsView(raw);
  assert.ok(view);
  assert.equal(view.rows[0]?.kind, "unknown");
});

test("c27: the summary has no missing count and no missing row (Frage 169 A)", () => {
  for (const name of [
    "empty.json",
    "five_kinds_cuts_last.json",
    "cuts_last_off.json",
    "unknown_and_clone_members.json",
    "placed_and_clone_rows.json",
  ]) {
    const text = fixtureText(name).toLowerCase();
    assert.ok(!text.includes("missing"), `${name} must not name a missing item`);
    const view = decodeOperationsText(fixtureText(name));
    assert.ok(view, `${name} decodes`);
    const keys = Object.keys(view.summary).join(" ").toLowerCase();
    assert.ok(!keys.includes("missing"), `${name} summary has no missing count: ${keys}`);
  }
});

test("c25: the Cuts last off fixture reads its switch as off", () => {
  const view = decodeOperationsText(fixtureText("cuts_last_off.json"));
  assert.ok(view);
  assert.equal(view.cutsLast, false);
});

test("c18: the two role colours --cut and --engrave-hatch are at least 3:1 on the canvas (customer 2026-11-10: no operation colours; tester change)", () => {
  const css = readFileSync(INDEX_CSS, "utf8");
  const token = (name: string): string => {
    const match = new RegExp(`--${name}:\\s*(#[0-9A-Fa-f]{6})\\s*;`).exec(css);
    assert.ok(match, `--${name} is a six-digit hex in index.css`);
    return match[1] ?? "";
  };
  const luminance = (hex: string): number => {
    const channel = (i: number): number => {
      const c = parseInt(hex.slice(1 + i * 2, 3 + i * 2), 16) / 255;
      return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * channel(0) + 0.7152 * channel(1) + 0.0722 * channel(2);
  };
  const ratio = (a: string, b: string): number => {
    const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
    return (hi + 0.05) / (lo + 0.05);
  };
  const canvas = token("canvas-bg");
  for (const name of ["cut", "engrave-hatch"]) {
    const role = token(name);
    assert.ok(ratio(role, canvas) >= 3, `--${name} ${role} is below 3:1 on --canvas-bg ${canvas}`);
  }
});

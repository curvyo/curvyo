// The gradient decoders against the Rust fixtures of `curvyo-editor-wasm/tests/fixtures/
// gradient_view/` (`specs/0124-gradient-fill` criteria 3, 6 to 8). The Rust contract test
// `tests/gradient_view_contract.rs` builds the same states and compares them with these files, so a
// drift fails on both sides.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { decodeGradientHandles, decodeGradientView, decodeText } from "../src/lib/gradientView.ts";
import { breakEveryField } from "./contractWalk.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/gradient_view/", import.meta.url);

function fixture(name: string): { view: unknown; handles: unknown } {
  return JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8")) as { view: unknown; handles: unknown };
}

const NAMES = readdirSync(DIR)
  .filter((file) => file.endsWith(".json"))
  .map((file) => file.slice(0, -".json".length))
  .sort();

test("the six Rust states decode, and nothing else is in the folder", () => {
  assert.deepEqual(NAMES, [
    "end_drag_readout",
    "linear_default",
    "mixed_gradients",
    "nothing_selected",
    "radial_entry",
    "solid_offered",
  ]);
  for (const name of NAMES) {
    assert.ok(decodeGradientView(fixture(name).view), `${name} view`);
    assert.ok(decodeGradientHandles(fixture(name).handles), `${name} handles`);
  }
});

test("every field of every shared fixture is required and typed", () => {
  for (const name of NAMES) {
    const { view, handles } = fixture(name);
    for (const broken of breakEveryField(view)) {
      assert.equal(decodeGradientView(broken.value), null, `${name} view: ${broken.how} ${broken.path}`);
    }
    for (const broken of breakEveryField(handles)) {
      assert.equal(decodeGradientHandles(broken.value), null, `${name} handles: ${broken.how} ${broken.path}`);
    }
  }
});

test("the radial entry box carries the radius field of the radius handle", () => {
  const handles = decodeGradientHandles(fixture("radial_entry").handles);
  assert.ok(handles?.entry);
  assert.equal(handles.entry.field, "radius");
  assert.equal(handles.entry.fieldText, "50");
  assert.equal(handles.handles.find((h) => h.kind === "stop")?.hex, "#3366CC80");
  assert.equal(decodeGradientView(fixture("mixed_gradients").view)?.mixedGradients, true);
});

test("an unknown kind and bad JSON decode to null", () => {
  const view = fixture("linear_default").view as Record<string, unknown>;
  assert.equal(decodeGradientView({ ...view, kind: "conic" }), null);
  assert.equal(decodeText("{", decodeGradientView), null);
  assert.equal(decodeText("", decodeGradientView), null);
  assert.ok(decodeText(JSON.stringify(view), decodeGradientView));
});

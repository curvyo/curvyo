import assert from "node:assert/strict";
import test from "node:test";
import { parseSvgExportAnswer } from "../src/lib/svgExport.ts";

test("unsupported SVG path is not treated as an exportable file", () => {
  assert.deepEqual(parseSvgExportAnswer("unsupported-extended-path"), { kind: "unsupported-extended-path" });
});

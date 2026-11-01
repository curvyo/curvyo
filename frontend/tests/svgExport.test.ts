import { strict as assert } from "node:assert";
import { test } from "node:test";

import {
  DEFAULT_SVG_EXPORT_OPTIONS,
  loadSvgExportOptions,
  parseSvgExportAnswer,
  parseTimedNotice,
  saveSvgExportOptions,
} from "../src/lib/svgExport.ts";

function memory(): Storage {
  const items = new Map<string, string>();
  return {
    getItem: (key: string) => items.get(key) ?? null,
    setItem: (key: string, value: string) => void items.set(key, value),
    removeItem: (key: string) => void items.delete(key),
    clear: () => items.clear(),
    key: () => null,
    length: 0,
  };
}

test("ac147 the switches are all off the first time and come back as stored", () => {
  const storage = memory();
  assert.deepEqual(loadSvgExportOptions(storage), DEFAULT_SVG_EXPORT_OPTIONS);
  saveSvgExportOptions(storage, { ...DEFAULT_SVG_EXPORT_OPTIONS, textAsPaths: true, activePageOnly: true });
  assert.deepEqual(loadSvgExportOptions(storage), {
    textAsPaths: true,
    strokesAsPaths: false,
    selectionOnly: false,
    activePageOnly: true,
  });
});

test("ac147 unreadable storage gives the defaults", () => {
  const broken = { getItem: () => { throw new Error("denied"); } };
  assert.deepEqual(loadSvgExportOptions(broken), DEFAULT_SVG_EXPORT_OPTIONS);
  const garbage = { getItem: () => "{not json" };
  assert.deepEqual(loadSvgExportOptions(garbage), DEFAULT_SVG_EXPORT_OPTIONS);
  assert.doesNotThrow(() => saveSvgExportOptions({ setItem: () => { throw new Error("full"); } }, DEFAULT_SVG_EXPORT_OPTIONS));
});

test("ac4 the session's answers are read", () => {
  assert.deepEqual(parseSvgExportAnswer("svg:3:1:<svg a:b/>"), { kind: "svg", objects: 3, kept: 1, text: "<svg a:b/>" });
  assert.deepEqual(parseSvgExportAnswer("nothing-selected"), { kind: "nothing-selected" });
  assert.deepEqual(parseTimedNotice("6000:Exported a.svg. 2 texts: kept"), { ms: 6000, text: "Exported a.svg. 2 texts: kept" });
});

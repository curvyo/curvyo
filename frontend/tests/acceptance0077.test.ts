// Tester acceptance cases for `specs/0077-font-preview`, written from the specification and the
// exported names before the implementation was read, kept where they add to
// `fontPreview.test.ts`: stored state that parses but is not an object (criterion 6), a selected
// text with an empty first line (criteria 8, 11), and the grid filling its width (criterion 13).

import assert from "node:assert/strict";
import { test } from "node:test";

import { DEFAULT_SAMPLE, DEFAULT_SIZE, FONT_PREVIEW_KEY, readPreviewPrefs } from "../src/lib/fontPreviewState.ts";
import { openingSample } from "../src/lib/fontPreviewSample.ts";
import { gridColumns } from "../src/lib/fontPreviewLayout.ts";

test("AC6: stored JSON that is not an object gives the defaults", () => {
  const defaults = { sample: DEFAULT_SAMPLE, size: DEFAULT_SIZE, full: false };
  for (const junk of ["null", "[]", "42", '"text"', "true"]) {
    const storage = { getItem: (k: string) => (k === FONT_PREVIEW_KEY ? junk : null), setItem: () => {} };
    assert.deepEqual(readPreviewPrefs(storage), defaults, junk);
  }
});

test("AC8/AC11: a selected text with an empty first line opens with an empty sample, not the kept one", () => {
  assert.equal(openingSample({ node: "7", sample: "" }, "Kept"), "");
});

test("AC13: equal columns of at least max(240, 5 x size) that fit the grid width", () => {
  for (const grid of [436, 460, 944, 1576, 3000]) {
    for (const size of [16, 48, 100, 160]) {
      const { count, width } = gridColumns(grid, size);
      assert.ok(count >= 1, `${grid}/${size}`);
      if (grid >= Math.max(240, 5 * size)) {
        assert.ok(width >= Math.max(240, 5 * size) - 1e-9, `${grid}/${size}: ${width}`);
        assert.ok(count * width + (count - 1) * 12 <= grid + 1e-6, `${grid}/${size}: ${count} x ${width}`);
        const more = count + 1;
        assert.ok(more * Math.max(240, 5 * size) + (more - 1) * 12 > grid, `${grid}/${size}: room for ${more}`);
      }
    }
  }
});

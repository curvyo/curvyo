import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

test("the editor does not render the removed bottom status bar", () => {
  const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
  assert.doesNotMatch(app, /<CursorStatus\b/);
});

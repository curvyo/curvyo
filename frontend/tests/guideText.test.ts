// The words and outcome mapping of the Guide and Template buttons (`specs/0046-document-shapes`
// criteria 9, 10, 19, 22 and 23).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  GUIDE_NAME,
  TEMPLATE_LOADED_NOTICE,
  TEMPLATE_NAME,
  readGuideState,
  templateOutcome,
} from "../src/lib/guideText.ts";

test("the names are the tooltips: no hint text", () => {
  assert.equal(GUIDE_NAME, "Guide");
  assert.equal(TEMPLATE_NAME, "Template");
});

test("the guide state reads the session's three words and greys anything else", () => {
  assert.equal(readGuideState("off"), "off");
  assert.equal(readGuideState("pressed"), "pressed");
  assert.equal(readGuideState("disabled"), "disabled");
  assert.equal(readGuideState(""), "disabled");
});

test("a load shows the notice, a refusal the session's sentence", () => {
  assert.deepEqual(templateOutcome("loaded:3"), { notice: TEMPLATE_LOADED_NOTICE });
  assert.deepEqual(templateOutcome("loaded:0"), { notice: "Template loaded." });
  assert.deepEqual(templateOutcome("error:This file is damaged and can't be read."), {
    error: "This file is damaged and can't be read.",
  });
  assert.deepEqual(templateOutcome("error:"), {
    error: "This file couldn't be loaded as a template.",
  });
});

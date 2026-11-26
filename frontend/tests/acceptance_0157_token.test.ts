// `specs/0157-library-git-token` criterion 8 on the page's side: the token field is uncontrolled and
// its text never reaches state, a store, `localStorage` or a `title`. The DOM check (`value`
// attribute null after typing and after Add) runs in the real app; this guards the source.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

const SRC = path.resolve(import.meta.dirname, "..", "src");
/** The file without its comments, so a comment that names a rule does not trip the rule. */
const read = (file: string) =>
  readFileSync(path.join(SRC, file), "utf8")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");

test("the token field is an uncontrolled password input with no value attribute", () => {
  const field = read("components/panel/TokenField.tsx");
  assert.match(field, /type="password"/);
  assert.match(field, /autoComplete="off"/);
  assert.match(field, /data-1p-ignore/);
  assert.match(field, /data-lpignore="true"/);
  assert.doesNotMatch(field, /\bvalue=/, "a controlled input would put the text into the value attribute");
  assert.doesNotMatch(field, /onChange/);
  assert.doesNotMatch(field, /useState/);
  assert.doesNotMatch(field, /localStorage|sessionStorage/);
});

test("the token text is only read at Add and Save and cleared after", () => {
  for (const file of ["components/panel/AddSourceForm.tsx", "components/panel/TokenForm.tsx"]) {
    const text = read(file);
    assert.match(text, /token\.current\?\.read\(\)/, file);
    assert.match(text, /token\.current\?\.clear\(\)/, file);
    assert.doesNotMatch(text, /useState<string>\(""\).*[Tt]oken/, file);
    assert.doesNotMatch(text, /title=\{[^}]*token/i, file);
  }
});

test("the page no longer says the repository must be public", () => {
  for (const file of ["components/panel/AddSourceForm.tsx", "components/libraryBrowser/GuideView.tsx"]) {
    assert.doesNotMatch(read(file), /must be public|publicOnly/, file);
  }
});

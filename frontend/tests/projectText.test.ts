// The words of the project in the page (`specs/0051-sidebar-and-no-menu/`).

import assert from "node:assert/strict";
import { test } from "node:test";

import { detectPlatform } from "../src/lib/platformKind.ts";
import {
  baseName,
  checkSentence,
  projectName,
  projectStatus,
  projectTitle,
  savedNotice,
} from "../src/lib/projectText.ts";

test("the platform: the browser build is the browser, whatever the system", () => {
  assert.equal(detectPlatform("MacIntel", false), "browser");
  assert.equal(detectPlatform("Linux x86_64", false), "browser");
});

test("the platform of the desktop build follows navigator.platform", () => {
  assert.equal(detectPlatform("MacIntel", true), "macos");
  assert.equal(detectPlatform("Win32", true), "windows");
  assert.equal(detectPlatform("Linux x86_64", true), "linux");
  assert.equal(detectPlatform("FreeBSD amd64", true), "linux");
});

test("the file name of a path, with either separator", () => {
  assert.equal(baseName("/home/maker/Poster.curvyo"), "Poster.curvyo");
  assert.equal(baseName("C:\\Users\\maker\\Poster.curvyo"), "Poster.curvyo");
  assert.equal(baseName("Poster.curvyo"), "Poster.curvyo");
});

test("an untitled project is called Untitled", () => {
  assert.equal(projectName(null), "Untitled");
  assert.equal(projectName("a.curvyo"), "a.curvyo");
});

test("the status line (criterion 16)", () => {
  assert.equal(projectStatus("a.curvyo", true), "Unsaved changes");
  assert.equal(projectStatus(null, true), "Unsaved changes");
  assert.equal(projectStatus("a.curvyo", false), "No changes since the last save");
  assert.equal(projectStatus(null, false), "Not saved yet");
});

test("the sentence of the check names the project and the action (criterion 25)", () => {
  assert.equal(
    checkSentence("Poster.curvyo", "open"),
    "Poster.curvyo has unsaved changes. Save them before you open another project? Discarded changes cannot be brought back.",
  );
  assert.match(checkSentence(null, "new"), /^This project has unsaved changes\. Save them before you start a new project\?/);
  assert.match(checkSentence("a.curvyo", "close"), /before you close it\?/);
  assert.match(checkSentence("a.curvyo", "quit"), /before you quit\?/);
  assert.match(checkSentence("a.curvyo", "handover"), /before you open another project\?/);
});

test("the notice after a save (criterion 19)", () => {
  assert.equal(savedNotice("Poster.curvyo", false), "Saved Poster.curvyo.");
  assert.equal(savedNotice("poster.curvyo", true), "Downloaded poster.curvyo.");
});

test("the title (criterion 30)", () => {
  assert.equal(projectTitle(null, false), "Curvyo");
  assert.equal(projectTitle(null, true), "*Curvyo");
  assert.equal(projectTitle("a.curvyo", false), "a.curvyo — Curvyo");
  assert.equal(projectTitle("a.curvyo", true), "*a.curvyo — Curvyo");
});

test("a long file name is cut in the middle in the sentence of the check and keeps its end", () => {
  const long = `${"x".repeat(70)}.curvyo`;
  const sentence = checkSentence(long, "new");
  assert.match(sentence, /^x+…x*\.curvyo has unsaved changes\./);
  assert.ok(sentence.indexOf(" has unsaved") <= 41);
});

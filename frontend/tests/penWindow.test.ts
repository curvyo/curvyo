// The pen window's page side (`specs/0158-pen-display` slice 2a criteria 1, 21, 22; ADR 0096 A5,
// A8): the role from the query, the host guard's allow-list (the same as the app's), the keys
// that reach the canvas, and the per-frame shuttle of bundles.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { PEN_ALLOWED, roleOf, windowAllows } from "../src/lib/windowRole.ts";
import { isPeerGone, linkFrame, type LinkSession } from "../src/lib/windowLink.ts";
import { penKeyAllowed } from "../src/pen/penKeys.ts";

test("the pen role comes from ?window=pen in the desktop build only", () => {
  assert.equal(roleOf("?window=pen", true), "pen");
  assert.equal(roleOf("", true), "main");
  assert.equal(roleOf("?window=main", true), "main");
  assert.equal(roleOf("?window=pen", false), "main", "the browser build has no pen window");
});

test("the page's allow-list is the app's allow-list (criterion 21)", () => {
  const rust = readFileSync(new URL("../../curvyo-app/src/window_commands.rs", import.meta.url), "utf8");
  const list = rust.slice(rust.indexOf("PEN_ALLOWED"), rust.indexOf("];", rust.indexOf("PEN_ALLOWED")));
  const names = [...list.matchAll(/"([a-z_]+)"/g)].map((match) => match[1]);
  assert.deepEqual([...PEN_ALLOWED].sort(), names.sort());
});

test("the pen page may call its list only; the main page everything", () => {
  for (const command of ["pick_project", "save_project_bytes", "reset_project", "set_dirty", "quit_app", "recovery_write_copy", "pen_window_open"]) {
    assert.equal(windowAllows("pen", command), false, command);
    assert.equal(windowAllows("main", command), true, command);
  }
  for (const command of PEN_ALLOWED) assert.equal(windowAllows("pen", command), true, command);
});

test("keys of tools the pen window lacks and file chords do nothing (criterion 22)", () => {
  for (const key of ["p", "S", "n", "Delete", "Escape", " "]) assert.equal(penKeyAllowed(key, false, false), true, key);
  for (const key of ["b", "r", "e", "t", "i", "o", "m"]) assert.equal(penKeyAllowed(key, false, false), false, key);
  for (const key of ["z", "y"]) assert.equal(penKeyAllowed(key, true, false), true, key);
  for (const key of ["o", "n", "i", "e"]) assert.equal(penKeyAllowed(key, true, false), false, key);
});

test("one frame sends the session's bundle and reports what it left", () => {
  const sent: number[][] = [];
  const project = new Uint8Array([9, 9]);
  const session: LinkSession = {
    link_frame: () => new Uint8Array([1, 1, 0]),
    link_take_project: () => project,
    link_take_changed: () => true,
    link_take_save: () => false,
    link_take_notice: () => "Reloaded from the main window.",
    link_state: () => "live",
  };
  const frame = linkFrame(session, (bytes) => sent.push([...bytes]));
  assert.deepEqual(sent, [[1, 1, 0]]);
  assert.equal(frame.project, project);
  assert.equal(frame.changed, true);
  assert.equal(frame.notice, "Reloaded from the main window.");
  const idle = linkFrame({ ...session, link_frame: () => undefined, link_take_project: () => undefined }, (bytes) => sent.push([...bytes]));
  assert.equal(sent.length, 1, "nothing to send sends nothing");
  assert.equal(idle.project, null);
});

test("the hub's PEER_GONE is recognised and nothing else is", () => {
  assert.equal(isPeerGone(new Uint8Array([1, 2])), true);
  assert.equal(isPeerGone(new Uint8Array([1, 1])), false);
  assert.equal(isPeerGone(new Uint8Array([1, 2, 0])), false);
});

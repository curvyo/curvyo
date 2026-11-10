// Run with `npm test`. The window-wide Ctrl store of the palette swatches (`src/lib/heldCtrl.ts`,
// `specs/0089` UX notes §27.12), driven by events as WebKitGTK sends them: the Control key's own
// key-up still says `ctrlKey: true`, and no pointer event follows.

import assert from "node:assert/strict";
import { test } from "node:test";

import { heldCtrlStore } from "../src/lib/heldCtrl.ts";

function page() {
  const window = new EventTarget();
  const document = Object.assign(new EventTarget(), { visibilityState: "visible" });
  return { window, document };
}

const flags = { shiftKey: false, ctrlKey: false, metaKey: false, altKey: false };
const key = (type: "keydown" | "keyup", name: string, code: string, ctrlKey: boolean) => Object.assign(new Event(type), { ...flags, key: name, code, ctrlKey });
const pointer = (ctrlKey: boolean) => Object.assign(new Event("pointermove"), { ...flags, ctrlKey });

test("a Control key-up whose own flag is stale (true) releases Ctrl without a pointer event", () => {
  const p = page();
  const store = heldCtrlStore(() => p);
  let heard = 0;
  const stop = store.subscribe(() => (heard += 1));
  p.window.dispatchEvent(key("keydown", "Control", "ControlLeft", true));
  assert.equal(store.get(), true);
  p.window.dispatchEvent(key("keyup", "Control", "ControlLeft", true));
  assert.equal(store.get(), false, "released by the key, not by its flag");
  assert.equal(heard, 2);
  stop();
});

test("a Control key-down whose own flag is stale (false) holds Ctrl, a repeat keeps it, Meta counts as Ctrl", () => {
  const p = page();
  const store = heldCtrlStore(() => p);
  const stop = store.subscribe(() => undefined);
  p.window.dispatchEvent(key("keydown", "Control", "ControlLeft", false));
  p.window.dispatchEvent(key("keydown", "Control", "ControlLeft", true));
  assert.equal(store.get(), true);
  p.window.dispatchEvent(key("keyup", "Control", "ControlLeft", false));
  assert.equal(store.get(), false);
  p.window.dispatchEvent(key("keydown", "Meta", "MetaLeft", false));
  assert.equal(store.get(), true);
  p.window.dispatchEvent(key("keyup", "Meta", "MetaLeft", true));
  assert.equal(store.get(), false);
  stop();
});

test("pointer flags correct a lost key-up; a blur or a hidden page forgets Ctrl; no subscriber, no listening", () => {
  const p = page();
  const store = heldCtrlStore(() => p);
  const stop = store.subscribe(() => undefined);
  p.window.dispatchEvent(key("keydown", "Control", "ControlLeft", true));
  p.window.dispatchEvent(pointer(false));
  assert.equal(store.get(), false);
  p.window.dispatchEvent(pointer(true));
  assert.equal(store.get(), true);
  p.window.dispatchEvent(new Event("blur"));
  assert.equal(store.get(), false);
  p.window.dispatchEvent(key("keydown", "Control", "ControlLeft", true));
  p.document.visibilityState = "hidden";
  p.document.dispatchEvent(new Event("visibilitychange"));
  assert.equal(store.get(), false);
  stop();
  p.window.dispatchEvent(key("keydown", "Control", "ControlLeft", true));
  assert.equal(store.get(), false, "detached");
});

// The dialog gate (`specs/0051-sidebar-and-no-menu/` criterion 26): while a save or open dialog
// runs, an action that ends the project does not start.

import assert from "node:assert/strict";
import { test } from "node:test";

import { createBusyGate } from "../src/lib/busyGate.ts";

test("a second dialog does not start while one runs", () => {
  const gate = createBusyGate();
  assert.equal(gate.enter(), true);
  assert.equal(gate.enter(), false);
  assert.equal(gate.isBusy(), true);
  gate.leave();
  assert.equal(gate.isBusy(), false);
  assert.equal(gate.enter(), true);
});

/** Save dialog up on an untitled project, then New and Discard: the sequence of review B1. */
test("New is refused while a save is pending and runs after the save ended", () => {
  const gate = createBusyGate();
  const events: string[] = [];
  const requestNew = () => {
    if (gate.isBusy()) {
      events.push("new refused");
      return;
    }
    events.push("new ran");
  };
  assert.equal(gate.enter(), true); // Ctrl+S: the dialog opens
  requestNew(); // the maker clicks New and Discard meanwhile
  gate.leave(); // the dialog is confirmed or cancelled
  requestNew();
  assert.deepEqual(events, ["new refused", "new ran"]);
});

// The workspaces Sync these could not switch on (PR #281 feedback): one alert per outage, gone
// once the server answers or the group's list names them, none without a server address.
import assert from "node:assert/strict";
import { test } from "node:test";

import { RETRY_MS, UnreachableWorkspaces } from "../src/lib/unreachableWorkspaces.ts";

const names = (failed: UnreachableWorkspaces<string>, server: string) => failed.alerts(server).flatMap((a) => a.names);

test("only an unreachable server keeps a workspace, and the names come together", () => {
  const failed = new UnreachableWorkspaces<string>();
  failed.record("a", "Box job", "A", "offline", false);
  failed.record("b", "Poster", "B", "offline", false);
  failed.record("c", "Lid", "C", "member-only", true);
  failed.record("d", "Tray", "D", "refused", true);
  assert.deepEqual(names(failed, "wss://relay.example"), ["Box job", "Poster"]);
  failed.record("a", "Box job", "A", "ok", true);
  assert.deepEqual(names(failed, "wss://relay.example"), ["Poster"]);
});

test("no alert while the server address is unset", () => {
  const failed = new UnreachableWorkspaces<string>();
  failed.record("a", "Box job", "A", "offline", false);
  assert.deepEqual(names(failed, ""), []);
});

test("a retry starts when the group comes online again, not while it is offline", () => {
  const failed = new UnreachableWorkspaces<string>();
  failed.record("a", "Box job", "A", "offline", false);
  failed.settled(0);
  assert.deepEqual(failed.due(false, 1000), []);
  assert.deepEqual(failed.due(true, 2000), ["A"]);
  // One retry at a time.
  assert.deepEqual(failed.due(true, 3000), []);
  failed.settled(3000);
  assert.deepEqual(failed.due(true, 3000 + RETRY_MS - 1), []);
  assert.deepEqual(failed.due(true, 3000 + RETRY_MS), ["A"]);
});

test("a room that did not answer is retried only when that is cheap", () => {
  const failed = new UnreachableWorkspaces<string>();
  failed.record("a", "Box job", "A", "unreachable", false);
  failed.record("b", "Poster", "B", "unreachable", true);
  failed.settled(0);
  assert.deepEqual(names(failed, "wss://relay.example"), ["Box job", "Poster"]);
  assert.deepEqual(failed.due(true, RETRY_MS), ["B"]);
});

test("the network coming back retries at once", () => {
  const failed = new UnreachableWorkspaces<string>();
  failed.record("a", "Box job", "A", "offline", false);
  failed.settled(0);
  assert.deepEqual(failed.due(true, 10), []);
  failed.wake();
  assert.deepEqual(failed.due(true, 20), ["A"]);
});

test("a workspace the group's list names, or a group that ended, clears the alert", () => {
  const failed = new UnreachableWorkspaces<string>();
  failed.record("a", "Box job", "A", "offline", false);
  failed.record("b", "Poster", "B", "offline", false);
  assert.equal(failed.forget(["a", "x"]), true);
  assert.equal(failed.forget(["x"]), false);
  assert.deepEqual(names(failed, "wss://relay.example"), ["Poster"]);
  failed.clear();
  assert.deepEqual(names(failed, "wss://relay.example"), []);
  assert.deepEqual(failed.due(true, 0), []);
});

test("the alert names why: unreachable, refused by the server or failed here, one line each", () => {
  const failed = new UnreachableWorkspaces<string>();
  failed.record("a", "Box job", "A", "offline", false);
  failed.record("b", "Poster", "B", "unreachable", true);
  failed.record("c", "Lid", "C", "server-refused", true);
  failed.record("d", "Tray", "D", "share-failed", true);
  assert.deepEqual(failed.alerts("wss://relay.example"), [
    { problem: "unreachable", names: ["Box job", "Poster"] },
    { problem: "refused", names: ["Lid"] },
    { problem: "local", names: ["Tray"] },
  ]);
  failed.settled(0);
  assert.deepEqual(failed.due(true, RETRY_MS), ["A", "B"], "a refusal and a local failure wait for the maker");
});

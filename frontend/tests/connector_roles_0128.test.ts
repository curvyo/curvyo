// `specs/0128` A26, A27 (UX notes sections 8 and 9): the owner and member words of a start that
// waits, the Start slot that confirms it, the trust block's rows and the new link lines.
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  type Session,
  type StatusMessage,
  confirmKind,
  errorLine,
  jobButtons,
  jobStateLine,
  pairRowShown,
  testLine,
  trustLabel,
  waitingText,
} from "../src/lib/connectorText.ts";
import { type LinkHost, createLinkStore, discardedName } from "../src/lib/connectorLink.ts";
import { controlsLine } from "../src/lib/machineControls.ts";

const waiting = (requestedBy: string | null, line = 0): StatusMessage => ({
  type: "Status",
  state: { name: "AwaitingLocalStart" },
  job: { job_id: "j1", line, lines: 412, elapsed_ms: 0 },
  position: null,
  alarm: null,
  requested_by: requestedBy,
  job_name: "sign.gcode",
  sent_by: requestedBy,
});

const owner: Session = { role: "Owner", clientName: "Laptop", ownerLabel: "Marc" };
const phone: Session = { role: "Member", clientName: "Phone", ownerLabel: "Marc" };
const tablet: Session = { role: "Member", clientName: "Tablet", ownerLabel: null };

test("a26 the waiting line names the member for an owner and the owner for a member", () => {
  assert.equal(waitingText(waiting("Phone"), owner), "Start requested by Phone");
  assert.equal(waitingText(waiting("Phone", 120), owner), "Resume requested by Phone at line 120");
  assert.equal(waitingText(waiting("Phone"), phone), "Waiting for Marc to confirm");
  assert.equal(waitingText(waiting("Phone"), tablet), "Phone waits for an owner to confirm");
  assert.equal(waitingText(waiting("Tablet"), tablet), "Waiting for an owner to confirm");
  // Foreground: no requester, the words as built.
  assert.equal(waitingText(waiting(null), owner), "Waiting for the start at the machine");
  assert.equal(jobStateLine(waiting("Phone"), null, owner)?.text, "Start requested by Phone");
  assert.equal(jobStateLine(waiting(null), null)?.text, "Waiting for the start at the machine");
});

test("a26 long names are cut so the line stays within 70 characters", () => {
  const long = "A".repeat(80);
  const text = waitingText(waiting(long), owner);
  assert.ok(text.length <= 70, text);
  assert.ok(text.startsWith("Start requested by AAAA") && text.endsWith("…"), text);
});

test("a26 an owner's Start slot confirms, a member's stays off, Stop works for both", () => {
  assert.equal(confirmKind(waiting("Phone"), owner), "Confirm start");
  assert.equal(confirmKind(waiting("Phone", 120), owner), "Confirm resume");
  assert.equal(confirmKind(waiting("Phone"), phone), null);
  assert.equal(confirmKind(waiting(null), owner), null);
  const state = { name: "AwaitingLocalStart" } as const;
  assert.deepEqual(jobButtons(state, false, "Confirm start"), {
    send: true,
    start: true,
    startName: "Confirm start",
    pause: false,
    stop: true,
  });
  const member = jobButtons(state, false, null);
  assert.equal(member.start, false);
  assert.equal(member.startName, "Start");
  assert.equal(member.stop, true);
});

test("a26 a request someone else discarded reads Not started for its member", () => {
  const idle: StatusMessage = { type: "Status", state: { name: "Idle" }, job: null, position: null, alarm: null };
  assert.equal(discardedName(waiting("Phone"), idle, phone, false), "sign.gcode");
  assert.equal(discardedName(waiting("Phone"), idle, phone, true), null, "its own Stop gives no line");
  assert.equal(discardedName(waiting("Phone"), idle, owner, false), null);
  assert.equal(discardedName(waiting("Tablet"), idle, phone, false), null);
});

test("a26 a27 the trust block's text and the new link lines", () => {
  const paired = { connectorName: "workshop-laser", fingerprint: "3F2A 9C10" };
  assert.equal(
    trustLabel(paired, phone),
    "Paired with workshop-laser, belongs to Marc, this app is a member, key 3F2A 9C10",
  );
  assert.equal(trustLabel(paired, null), "Paired with workshop-laser, key 3F2A 9C10");
  assert.deepEqual(errorLine({ kind: "inMaintenance" }), { text: "Connector in maintenance mode", failure: true });
  assert.deepEqual(errorLine({ kind: "memberOnly" }), { text: "This app is a member", failure: true });
});

const host = (answers: Record<string, unknown> = {}): LinkHost => ({
  invoke: async (command: string) => answers[command],
  listen: async () => () => undefined,
});

test("a26 a refusal 'This app is a member' turns the role to Member and takes the Confirm slot away", () => {
  const store = createLinkStore(host());
  store.patch("p", { session: owner, status: waiting("Phone") });
  assert.equal(confirmKind(store.get("p").status, store.get("p").session), "Confirm start");
  store.linkFailure("p", { kind: "memberOnly" });
  assert.equal(store.get("p").session?.role, "Member");
  assert.equal(store.get("p").session?.clientName, "Laptop");
  assert.equal(confirmKind(store.get("p").status, store.get("p").session), null);
  assert.equal(jobButtons({ name: "AwaitingLocalStart" }).start, false);
});

test("8.4 a maintenance connector: Connected line, no Pair row", async () => {
  const answer = { kind: "maintenance", protocol: 0, version: "0.2.0" };
  assert.deepEqual(testLine(answer as never), {
    text: "Connected, maintenance mode, protocol 0, connector 0.2.0",
    failure: false,
  });
  const store = createLinkStore(host({ connector_test: answer }));
  await store.read("p", "a");
  assert.equal(store.get("p").maintenance, true);
  assert.equal(store.get("p").readError, null);
  assert.equal(pairRowShown({ paired: false, noStore: false, maintenance: true }), false);
  assert.equal(pairRowShown({ paired: false, noStore: false, maintenance: false }), true);
});

test("8.3 'Not started: <job>' of a discarded request is muted, not a failure", () => {
  const idle: StatusMessage = { type: "Status", state: { name: "Idle" }, job: null, position: null, alarm: null };
  const entry = { ...createLinkStore(host()).get("x"), status: idle, discarded: "sign.gcode" };
  const line = controlsLine(entry);
  assert.equal(line?.text, "Not started: sign.gcode");
  assert.equal(line?.failure, false);
  assert.equal(line?.muted, true);
});

// `specs/0064-machine-connector` second slice, the pure UI logic of the Connector group (UX notes
// §3 to §9): the texts of every answer, the button table, the number formats.
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  type JobState,
  type StatusMessage,
  codeComplete,
  errorLine,
  fingerprintLines,
  formatCount,
  formatElapsed,
  formatSize,
  hasProgress,
  jobButtons,
  jobIsActive,
  jobStateLine,
  pairLine,
  refusalText,
  testLine,
  typedCode,
} from "../src/lib/connectorText.ts";

const status = (state: JobState, job: StatusMessage["job"] = null, alarm: string | null = null): StatusMessage => ({
  type: "Status",
  state,
  job,
  position: null,
  alarm,
});
const progress = (line: number, lines: number) => ({ job_id: "x", line, lines, elapsed_ms: 0 });

test("q11: the test line names the controller, protocol and connector version", () => {
  assert.equal(
    testLine({
      kind: "connected",
      controller: "Fake controller",
      protocol: 0,
      version: "0.1.0",
      connectorName: "PC",
      status: status({ name: "Idle" }),
    }).text,
    "Connected, controller answered: Fake controller, protocol 0, connector 0.1.0",
  );
});

test("q11: every failure has exactly its words", () => {
  const words = (error: Parameters<typeof errorLine>[0]) => errorLine(error).text;
  assert.equal(words({ kind: "unreachable", address: "10.0.0.2:7447" }), "Nothing answers at 10.0.0.2:7447");
  assert.equal(words({ kind: "badAddress" }), "Not a valid address");
  assert.equal(words({ kind: "notPaired" }), "Not paired");
  assert.equal(words({ kind: "noLongerPaired" }), "No longer paired");
  assert.equal(words({ kind: "keyChanged", stored: "a", presented: "b" }), "This connector's key changed");
  assert.equal(words({ kind: "protocolMismatch", connector: 3 }), "Protocol mismatch: app 0, connector 3");
  assert.equal(words({ kind: "timedOut" }), "Timed out");
  assert.equal(errorLine({ kind: "keyChanged", stored: "a", presented: "b" }).failure, true);
  assert.equal(errorLine({ kind: "noLongerPaired" }).failure, false);
});

test("q6: pairing refusals say how many attempts are left and no hint follows", () => {
  assert.equal(pairLine({ kind: "wrongCode", attemptsLeft: 2 })?.text, "Wrong code, 2 attempts left");
  assert.equal(pairLine({ kind: "wrongCode", attemptsLeft: 1 })?.text, "Wrong code, 1 attempt left");
  assert.equal(pairLine({ kind: "codeSpent" })?.text, "Code spent");
  assert.equal(pairLine({ kind: "codeExpired" })?.text, "Code expired");
  assert.equal(pairLine({ kind: "codeNotValid" })?.text, "Code not valid");
  assert.equal(pairLine({ kind: "paired", connectorName: "x", fingerprint: "y" }), null);
});

test("q13: a refusal is said in words, and an alarm explains a busy connector", () => {
  assert.equal(refusalText("NotAscii", null), "Not plain ASCII text");
  assert.equal(refusalText("TooLarge", null), "Larger than 64 MiB");
  assert.equal(refusalText("HashMismatch", null), "Hash mismatch");
  assert.equal(refusalText("Busy", null), "Connector busy");
  assert.equal(refusalText("NoController", null), "No controller");
  assert.equal(refusalText("Busy", "Fake alarm at line 3"), "Machine in alarm: Fake alarm at line 3");
});

test("q15: the buttons apply per state as the UX table says", () => {
  const row = (state: JobState | null) => {
    const b = jobButtons(state);
    return [b.send, b.start, b.pause, b.stop].map(Number).join("");
  };
  assert.equal(row(null), "1000");
  assert.equal(row({ name: "Idle" }), "1000");
  // `specs/0132` UX notes section 5: Send job... is on while a job is held, the job goes to the list.
  assert.equal(row({ name: "Receiving" }), "1000");
  assert.equal(row({ name: "Ready" }), "1101");
  assert.equal(row({ name: "AwaitingLocalStart" }), "1001");
  assert.equal(row({ name: "Running" }), "1011");
  assert.equal(row({ name: "Paused" }), "1101");
  assert.equal(row({ name: "Done", result: "Stopped" }), "1000");
  assert.equal(row({ name: "Alarm" }), "0000");
  assert.equal(row({ name: "NoController" }), "0000");
  assert.equal(jobButtons({ name: "Paused" }).startName, "Resume");
  assert.equal(jobButtons({ name: "Ready" }).startName, "Start");
});

test("q14 q15 q19: the state line says what the maker needs", () => {
  const held = { name: "coaster.gcode", lines: 5000, bytes: 123_456 };
  assert.equal(jobStateLine(status({ name: "Ready" }), held)?.text, "Ready: coaster.gcode, 5,000 lines, 121 KiB");
  assert.equal(jobStateLine(status({ name: "Receiving" }), held)?.text, "Sending coaster.gcode");
  assert.equal(jobStateLine(status({ name: "Running" }, progress(3, 9)), held)?.text, "Running");
  assert.equal(jobStateLine(status({ name: "Done", result: "Stopped" }, progress(3, 9)), held)?.text, "Stopped at line 3");
  assert.equal(jobStateLine(status({ name: "Done", result: "Completed" }, progress(9, 9)), held)?.text, "Completed, 9 lines");
  const failed = jobStateLine(status({ name: "Done", result: "Failed" }, progress(4, 9)), held);
  assert.equal(failed?.text, "Failed at line 4");
  assert.equal(failed?.failure, true);
  assert.equal(jobStateLine(status({ name: "Alarm" }, null, "Fake alarm at line 3"), held)?.text, "Alarm: Fake alarm at line 3");
  assert.equal(jobStateLine(status({ name: "AwaitingLocalStart" }, progress(0, 9)), held)?.text, "Waiting for the start at the machine");
  assert.equal(jobStateLine(status({ name: "Idle" }), held), null);
});

test("the progress bar shows in the states with a line count", () => {
  assert.equal(hasProgress(status({ name: "Running" }, progress(1, 2))), true);
  assert.equal(hasProgress(status({ name: "Ready" })), false);
  assert.equal(hasProgress(status({ name: "Alarm" }, progress(1, 2), "x")), true);
  assert.equal(hasProgress(status({ name: "Alarm" }, null, "x")), false);
});

test("q16: a job in flight is active, a finished one is not", () => {
  for (const name of ["Receiving", "Ready", "AwaitingLocalStart", "Running", "Paused"] as const) {
    assert.equal(jobIsActive({ name }), true, name);
  }
  assert.equal(jobIsActive({ name: "Idle" }), false);
  assert.equal(jobIsActive({ name: "Done", result: "Completed" }), false);
  assert.equal(jobIsActive({ name: "Alarm" }), false);
});

test("the number formats", () => {
  assert.equal(formatSize(812), "812 B");
  assert.equal(formatSize(4300), "4.2 KiB");
  assert.equal(formatSize(120 * 1024), "120 KiB");
  assert.equal(formatSize(3.5 * 1024 * 1024), "3.5 MiB");
  assert.equal(formatElapsed(7000), "0:07");
  assert.equal(formatElapsed(754_000), "12:34");
  assert.equal(formatElapsed(3_723_000), "1:02:03");
  assert.equal(formatCount(5000), "5,000");
  assert.equal(formatCount(12), "12");
});

test("the code field: cut as typed, complete at 8 symbols of the alphabet", () => {
  assert.equal(typedCode("k7qm-4xtr-extra"), "K7QM-4XTR");
  assert.equal(codeComplete("k7qm-4xtr"), true);
  assert.equal(codeComplete("K7QM 4XTR"), true);
  assert.equal(codeComplete("K7QM-4XT"), false);
  assert.equal(codeComplete("K7QM-4XTU"), false);
});

test("the fingerprint is two lines of four groups", () => {
  assert.deepEqual(fingerprintLines("3F2A 9C10 77B4 E2D1 0A5C 61F8 9B3E 4D27"), [
    "3F2A 9C10 77B4 E2D1",
    "0A5C 61F8 9B3E 4D27",
  ]);
});

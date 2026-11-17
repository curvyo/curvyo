// The texts of the connector row that the host's answers map to (0064 Q13, Q15).
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  type StatusMessage,
  UNKNOWN_JOB_TEXT,
  FORGET_FAILED_TEXT,
  UNREADABLE_TEXT,
  clearsRefusal,
  errorLine,
  pairRowShown,
  refusalText,
} from "../src/lib/connectorText.ts";

const status = (state: StatusMessage["state"], line = 0): StatusMessage => ({
  type: "Status",
  state,
  job: { job_id: "j1", line, lines: 10, elapsed_ms: 0 },
  position: null,
  alarm: null,
});

test("a refusal leaves when the job moves on, not at a progress push or a refused send's Idle (UX notes §8)", () => {
  // "The connector holds another job" in Ready, then the job starts at the machine.
  assert.equal(clearsRefusal(status({ name: "Ready" }), status({ name: "Running" })), true);
  // "Connector busy" while another job runs, then it completes.
  assert.equal(
    clearsRefusal(status({ name: "Running" }, 4), status({ name: "Done", result: "Completed" }, 10)),
    true,
  );
  assert.equal(clearsRefusal(status({ name: "Running" }, 1), status({ name: "Running" }, 2)), false);
  // A refused send goes Idle, Receiving, Idle; the late pushes keep "Not plain ASCII text".
  assert.equal(clearsRefusal(status({ name: "Idle" }), status({ name: "Receiving" })), false);
  assert.equal(clearsRefusal(status({ name: "Receiving" }), status({ name: "Idle" })), false);
});

test("an unreadable file and an unknown job have their own words", () => {
  assert.equal(UNREADABLE_TEXT, "Could not read the file");
  assert.equal(UNKNOWN_JOB_TEXT, "The connector holds another job");
});

test("a refused job keeps the words of Q13", () => {
  assert.equal(refusalText("NotAscii", null), "Not plain ASCII text");
});

test("a failed forget and an unmapped error have fixed short words; no store hides the Pair row", () => {
  assert.equal(FORGET_FAILED_TEXT, "Could not forget the pairing");
  assert.equal(errorLine({ kind: "other", message: "Connector error" }).text, "Connector error");
  assert.equal(errorLine({ kind: "noCredentialStore" }).text, "No credential store on this computer");
  assert.equal(pairRowShown({ paired: false, noStore: false }), true);
  assert.equal(pairRowShown({ paired: true, noStore: false }), false);
  assert.equal(pairRowShown({ paired: false, noStore: true }), false);
});

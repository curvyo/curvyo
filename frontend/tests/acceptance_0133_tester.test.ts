// Independent acceptance tests for `specs/0133-fix-pause-while-homing`, criterion 8 and
// `ux-notes.md` sections 2 to 6 (H1 to H8), written from the spec and notes before reading the
// implementation. The suite runs under `node --test` without a DOM: rule-shaped behaviour goes
// through the pure modules, the wiring of the refusal is checked on the sources, and the rendered
// result is checked in the real desktop app.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import type { LinkEntry } from "../src/lib/connectorLink.ts";
import { SETTLES, controlsLine, pauseButton, homeButton } from "../src/lib/machineControls.ts";
import { type JobState, type StatusMessage, jobButtons, jobStateLine } from "../src/lib/connectorText.ts";

const JOB = "11111111-1111-4111-8111-111111111111";

function entry(over: Partial<LinkEntry> = {}): LinkEntry {
  return {
    link: "paired",
    paired: { connectorName: "navi", fingerprint: "AA" },
    noStore: false,
    keyChanged: null,
    status: null,
    controller: null,
    lost: false,
    held: null,
    pageJobId: null,
    sendingName: null,
    pageSend: null,
    sendFailure: null,
    busy: null,
    reading: false,
    readError: null,
    ...over,
  } as LinkEntry;
}

const st = (state: JobState, extra: { pending?: boolean; line?: number; detail?: string | null } = {}): StatusMessage =>
  ({
    type: "Status",
    state,
    job: { job_id: JOB, line: extra.line ?? 0, lines: 412, elapsed_ms: 1500 },
    position: null,
    alarm: null,
    detail: extra.detail ?? null,
    ...(extra.pending ? { pause_pending: true } : {}),
  }) as StatusMessage;

const src = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");

test("PH8: the state line reads 'Pause after homing' while the status carries pause_pending, neutral", () => {
  const line = controlsLine(entry({ status: st({ name: "Running" }, { pending: true, detail: "Controller state: Home" }) }));
  assert.equal(line?.text, "Pause after homing");
  assert.equal(line?.failure, false);
  assert.equal(line?.alarm, false);
  assert.equal(jobStateLine(st({ name: "Running" }, { pending: true }), null)?.text, "Pause after homing");
  // elapsed, bar and count follow the report unchanged
  assert.deepEqual(line?.bar, { line: 0, lines: 412, failure: false });
});

test("PH8: without pause_pending the same status reads 'Running'; pause_pending false too", () => {
  assert.equal(controlsLine(entry({ status: st({ name: "Running" }) }))?.text, "Running");
  const off = { ...st({ name: "Running" }), pause_pending: false } as StatusMessage;
  assert.equal(controlsLine(entry({ status: off }))?.text, "Running");
});

test("PH8: when the connector reports Paused the line reads 'Paused', no line number in it", () => {
  const line = controlsLine(entry({ status: st({ name: "Paused" }, { line: 1 }) }));
  assert.equal(line?.text, "Paused");
});

test("PH8: the state line follows the status only (a second app, no local memory)", () => {
  // the same status, however the entry's own bookkeeping differs, gives the same line and button
  const status = st({ name: "Running" }, { pending: true });
  for (const busy of [null, "pause", "resume"] as const) {
    const e = entry({ status, busy: busy as never });
    assert.equal(controlsLine(e)?.text, "Pause after homing", String(busy));
    assert.equal(pauseButton(e).busy, true, String(busy));
  }
  // a pending flag on any other state is not shown
  for (const state of [{ name: "Idle" }, { name: "Homing" }, { name: "Ready" }] as JobState[]) {
    assert.notEqual(controlsLine(entry({ status: st(state, { pending: true }) }))?.text, "Pause after homing", state.name);
  }
});

test("H3 / H5: Pause stays busy while pending, not enabled, no time limit, second line 'Pause after homing'", () => {
  const b = pauseButton(entry({ status: st({ name: "Running" }, { pending: true }) }));
  assert.equal(b.busy, true);
  assert.equal(b.enabled, false);
  assert.equal(b.cause, "Pause after homing");
  // no time limit: the press may time out, the pending phase is the status's
  assert.equal(SETTLES.pause.done(st({ name: "Running" }, { pending: true })), true);
});

test("H6: while pending Resume and Stop are enabled, Send and Pause are not (Resume is named Resume)", () => {
  const j = jobButtons({ name: "Running" }, true);
  assert.equal(j.start, true);
  assert.equal(j.startName, "Resume");
  assert.equal(j.stop, true);
  assert.equal(j.send, false);
  assert.equal(j.pause, false);
  // even while a command's press waits, the pending phase keeps Stop enabled
});

test("H7: Pause is enabled in a normal Running state and while a job's own `$H` runs without pending", () => {
  const b = pauseButton(entry({ status: st({ name: "Running" }, { detail: "Controller state: Home" }) }));
  assert.equal(b.enabled, true);
  assert.equal(b.busy, false);
  assert.equal(b.cause, null);
});

test("H7 / PH3: Pause is off during the Home button's Homing, with or without a job, and Home says why", () => {
  for (const job of [false, true]) {
    const status = job ? st({ name: "Homing" }) : ({ ...st({ name: "Homing" }), job: null } as StatusMessage);
    const b = pauseButton(entry({ status }));
    assert.equal(b.enabled, false, `job=${job}`);
    assert.equal(b.busy, false, `job=${job}`);
    assert.notEqual(b.cause, "Pause after homing");
    assert.equal(homeButton(entry({ status })).enabled, false);
  }
});

test("H8: 'No job to pause' reads in the failure look as an alert, and is wired to the refusal of Pause", () => {
  const e = entry({ status: { ...st({ name: "Homing" }), job: null } as StatusMessage });
  const line = controlsLine(e, { sending: false, failure: { text: "No job to pause", failure: true } as never });
  assert.equal(line?.text, "No job to pause");
  assert.equal(line?.failure, true);
  assert.equal(line?.alerts, true);
  const hook = src("../src/hooks/useMachineControls.ts");
  assert.match(hook, /wrongState"\s*&&\s*kind === "pause"/);
});

test("H4: no aria-pressed on Pause and the toolbar names the pending description", () => {
  const bar = src("../src/components/panel/MachineControls.tsx");
  assert.ok(!bar.includes("aria-pressed"), "Pause is not a toggle");
  const toolbarButton = src("../src/components/workbench/SymbolButton.tsx");
  assert.ok(/aria-busy/.test(toolbarButton) && /aria-disabled/.test(toolbarButton));
  assert.ok(!toolbarButton.includes("aria-pressed"));
});

test("H2: no text of its own for a dropped pending pause, no hint labels", () => {
  const all = src("../src/lib/connectorText.ts") + src("../src/lib/machineControls.ts");
  assert.ok(!/Pause cancelled|Pause dropped|Please wait|cannot be paused|Use Stop/i.test(all));
});

test("PH8: after the pending phase ends nothing stays busy (Resume cancels: line back to Running, Pause enabled)", () => {
  const e = entry({ status: st({ name: "Running" }) });
  assert.equal(controlsLine(e)?.text, "Running");
  assert.equal(pauseButton(e).enabled, true);
  // Resume waits for the status to drop the pending flag (UX notes section 4)
  assert.equal(SETTLES.resume.done(st({ name: "Running" }, { pending: true })), false);
  assert.equal(SETTLES.resume.done(st({ name: "Running" })), true);
  assert.equal(SETTLES.resume.done(st({ name: "Paused" })), false);
});

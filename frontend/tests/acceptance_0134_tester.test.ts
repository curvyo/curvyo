// `specs/0134-send-and-start` SS8, SS9, SS13 and UX notes P4, P9, P10, P12 to P14, from the
// specification. The pure rules run with injected time; what lives in the React hook and the
// component (Escape, a press elsewhere, focus leaving, the glyph, the look) has no DOM here, so it
// is checked on the source as a guard and in the real app (see the report).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { ARM_GUARD_MS, ARM_TIMEOUT_MS, IDLE, focusKeepsArm, pressTrash } from "../src/lib/deleteArm.ts";
import { armEnds, ignoredKey, sendPress } from "../src/lib/sendArm.ts";
import { SEND_ARMED_NAME, SEND_NAME, SEND_START_NAME, notStartedText, sendName } from "../src/lib/sendText.ts";

const src = (path: string) => readFileSync(new URL(`../src/${path}`, import.meta.url), "utf8");

test("SS13 names: the three strings are exactly the spec's", () => {
  assert.equal(SEND_NAME, "Send to machine");
  assert.equal(SEND_START_NAME, "Send to machine and start");
  assert.equal(SEND_ARMED_NAME, "Press again to send to machine and start");
  assert.equal(sendName(true, false), SEND_START_NAME);
  assert.equal(sendName(false, false), SEND_NAME);
  assert.equal(sendName(true, true), SEND_ARMED_NAME);
});

test("SS13 timings: guard 400 ms, timeout 4 s", () => {
  assert.equal(ARM_GUARD_MS, 400);
  assert.equal(ARM_TIMEOUT_MS, 4000);
});

test("SS13 injected time: first press arms, a press inside the guard is ignored, then it confirms once", () => {
  const t0 = 1_000_000;
  let r = pressTrash(IDLE, "send-to-machine", t0);
  assert.equal(r.remove, false);
  assert.equal(r.state.armed, "send-to-machine");
  for (const dt of [0, 1, 200, 399]) {
    const again = pressTrash(r.state, "send-to-machine", t0 + dt);
    assert.equal(again.remove, false, `+${dt} ms`);
    assert.equal(again.state.armed, "send-to-machine", `+${dt} ms stays armed`);
  }
  const confirm = pressTrash(r.state, "send-to-machine", t0 + 400);
  assert.equal(confirm.remove, true);
  assert.equal(confirm.state.armed, null);
  // a press after the confirm arms again, it never confirms twice
  assert.equal(pressTrash(confirm.state, "send-to-machine", t0 + 401).remove, false);
});

test("SS13 injected time: after 4 s the arm is gone and a press only arms again", () => {
  const t0 = 5_000;
  const armed = pressTrash(IDLE, "send-to-machine", t0).state;
  assert.equal(pressTrash(armed, "send-to-machine", t0 + ARM_TIMEOUT_MS - 1).remove, true);
  const late = pressTrash(armed, "send-to-machine", t0 + ARM_TIMEOUT_MS);
  assert.equal(late.remove, false);
  assert.equal(late.state.at, t0 + ARM_TIMEOUT_MS);
});

test("SS13 one armed action at a time: arming another target replaces it and never confirms", () => {
  const a = pressTrash(IDLE, "trash-1", 0).state;
  const b = pressTrash(a, "send-to-machine", 1_000);
  assert.equal(b.remove, false);
  assert.equal(b.state.armed, "send-to-machine");
  assert.equal(pressTrash(b.state, "trash-1", 2_000).remove, false);
});

test("SS13 focus leaving: only the button's own zone keeps the arm", () => {
  assert.equal(focusKeepsArm("send-to-machine", "send-to-machine"), true);
  assert.equal(focusKeepsArm("send-to-machine", "other"), false);
  assert.equal(focusKeepsArm("send-to-machine", null), false);
  assert.equal(focusKeepsArm("send-to-machine", undefined), false);
});

test("SS9 P4 P12 P13: the press table over name x armed x fresh status x pressable", () => {
  for (const pressable of [true, false])
    for (const starts of [true, false])
      for (const armed of [true, false])
        for (const fresh of [true, false]) {
          const got = sendPress({ pressable, starts, armed, fresh: () => fresh });
          const want = !pressable
            ? "ignore"
            : !starts
              ? "send" // a name that only sends never arms and never starts
              : !armed
                ? fresh
                  ? "arm"
                  : "ignore" // P12: nothing sent
                : fresh
                  ? "sendAndStart"
                  : "send"; // P13: the confirm sends only
          assert.equal(got, want, JSON.stringify({ pressable, starts, armed, fresh }));
        }
});

test("SS9 P4: a name that only sends never starts, whatever the fresh status says", () => {
  for (const armed of [true, false]) {
    assert.notEqual(sendPress({ pressable: true, starts: false, armed, fresh: () => true }), "sendAndStart");
    assert.notEqual(sendPress({ pressable: true, starts: false, armed, fresh: () => true }), "arm");
  }
});

test("SS9 P4: the fresh status is read at the arming press and again at the confirming press", () => {
  let reads = 0;
  const fresh = () => {
    reads += 1;
    return true;
  };
  sendPress({ pressable: true, starts: true, armed: false, fresh });
  sendPress({ pressable: true, starts: true, armed: true, fresh });
  assert.equal(reads, 2);
});

test("SS13 disarm: status/checks/busy/job changes end the arm, an unrelated render does not", () => {
  assert.equal(armEnds(true, false, true, 1, 1), true, "name no longer start (status or checks)");
  assert.equal(armEnds(true, true, false, 1, 1), true, "disabled or busy");
  assert.equal(armEnds(true, true, true, 2, 1), true, "job changed (P15)");
  assert.equal(armEnds(true, true, true, 1, 1), false);
  assert.equal(armEnds(false, false, false, 9, 1), false, "nothing armed, nothing to end");
});

test("SS13 P14: a repeated Enter or Space never clicks; the first one does", () => {
  assert.equal(ignoredKey("Enter", true), true);
  assert.equal(ignoredKey(" ", true), true);
  assert.equal(ignoredKey("Enter", false), false);
  assert.equal(ignoredKey(" ", false), false);
  assert.equal(ignoredKey("a", true), false);
});

test("a held Enter: simulated repeats after the first click are all ignored and nothing confirms", () => {
  // first keydown clicks (arms); repeats at 500 ms and later are prevented, so no second click comes
  let state = IDLE;
  let removed = 0;
  const events = [{ repeat: false, at: 0 }, ...[500, 550, 600, 1000, 1500, 2000].map((at) => ({ repeat: true, at }))];
  for (const e of events) {
    if (ignoredKey("Enter", e.repeat)) continue;
    const r = pressTrash(state, "send-to-machine", e.at);
    state = r.state;
    if (r.remove) removed += 1;
  }
  assert.equal(removed, 0);
  assert.equal(state.armed, "send-to-machine");
});

test("SS6 'Not started: <reason>' wording", () => {
  assert.ok(notStartedText("WrongState").startsWith("Not started: "));
  assert.equal(notStartedText("Something odd"), "Not started: Something odd");
});

// ---- source guards (no DOM available) -------------------------------------------------------

test("SS12/P1 glyph: the page button is CirclePlay, the footer Start stays Play", () => {
  const button = src("components/panel/SendToMachine.tsx");
  assert.match(button, /CirclePlay/);
  assert.doesNotMatch(button, /\bUpload\b/);
  assert.match(button, /LoaderCircle/);
  const footer = src("components/panel/MachineControls.tsx");
  assert.match(footer, /icon: <Play \{\.\.\.GLYPH\} \/>/);
  assert.doesNotMatch(footer, /CirclePlay/);
});

test("SS13 armed look: accent fill and border, white glyph; hover does not change it", () => {
  const b = src("components/panel/SendToMachine.tsx");
  const armedClass = /armed\s*\?\s*"([^"]*)"/.exec(b)?.[1] ?? "";
  assert.match(armedClass, /bg-\[var\(--armed-accent\)\]/);
  assert.match(armedClass, /border-\[var\(--armed-accent\)\]/);
  assert.match(armedClass, /text-\[var\(--toolbar-icon-active-fg\)\]/);
  assert.doesNotMatch(armedClass, /hover:|destructive/);
});

test("SS13/P9: the arming press keeps focus (no onReturnFocus on the arm branch)", () => {
  const b = src("components/panel/SendToMachine.tsx");
  const arm = /action === "arm"\) \{([\s\S]*?)\n      \}/.exec(b)?.[1] ?? "";
  assert.ok(arm.length > 0, "arm branch found");
  assert.doesNotMatch(arm, /onReturnFocus/);
});

test("SS13/P10: no shortcut or hotkey reaches the send button", () => {
  for (const f of ["App.tsx", "components/panel/SendToMachine.tsx", "components/panel/OutputGroup.tsx", "hooks/useSendJob.ts"]) {
    assert.doesNotMatch(src(f), /(?:send-job|sendJob|Send to machine)[^\n]*(?:shortcut|hotkey|accelerator)/i, f);
  }
  assert.doesNotMatch(src("hooks/useSendJob.ts"), /addEventListener\(\s*["']key/);
});

test("SS13 the hook disarms on Escape, a press elsewhere, focus leaving and the 4 s timeout", () => {
  const h = src("hooks/useDeleteArm.ts");
  assert.match(h, /pointerdown/);
  assert.match(h, /Escape/);
  assert.match(h, /focusin/);
  assert.match(h, /ARM_TIMEOUT_MS/);
});

test("SS9 the button's name, tooltip and accessible name come from one string", () => {
  const b = src("components/panel/SendToMachine.tsx");
  assert.match(b, /aria-label=\{name\}/);
  assert.match(b, /content=\{tip\}/);
  assert.match(b, /const tip = pressable \? \(\s*name/);
});

test("SS1 the page press never goes through the machine-control 'start' command", () => {
  for (const f of ["hooks/useSendJob.ts", "components/panel/SendToMachine.tsx", "lib/connectorLink.ts"]) {
    assert.doesNotMatch(src(f), /connector_command["'`]?,?\s*[^)]{0,40}["']start["']/, f);
  }
});

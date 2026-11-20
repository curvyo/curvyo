# Fix: a Pause pressed while the machine homes is forgotten

Status: Done (PR #420)
Kind: Bug (branch `fix/pause-while-homing`)
Priority: Must (safety: a Pause the maker pressed must never be lost)
Origin: Customer, test of PR #399 on the real laser, 2026-11-19. Translated: "When you home and press pause, it forgets the pause. Expectation: it should go directly into pause."

## Reproduction (as reported)

1. Laser page, Start pick **Home, then machine origin** (`0029` criterion 38; the job's first line is `$H`), Send to machine (`0125`), Start. The connector homes with `$H` (GRBL does not answer `ok` until homing ends, `0127` G9).
2. While the machine homes (the state line reads "Homing" or "Running" at line 1) the maker presses **Pause**.
3. Observed: nothing seems to happen, and when homing ends the job runs on. The press is ignored or forgotten. Expected: see criteria.

Possible causes (not known, the implementer finds out): the connector answers `Error { WrongState }` for `Pause` in the state `Homing` (`0118` Home) or while the `$H` line is current; `0127` G29 drops a `Pause` that GRBL does not confirm with `Hold:0` within 2 s (GRBL ignores `!` while homing), so the press is dropped by design; the app shows no answer to the refusal.

## The rule (Proposal, from the customer's expectation)

A `Pause` is never dropped silently. While `$H` runs, GRBL cannot hold, so the connector **remembers the pause** and applies it at the first safe moment: when GRBL ends homing with `ok`, the connector streams **no further line**, the laser stays off, and the job is `Paused` (before any motion line of the job). Resume continues with the next line as after any pause.

## Acceptance criteria

Prefix `PH`. Tests use the simulator of `0127` G22 with the silent homing it has since `0129` (`quiet_homing`: no `ok` and no `?` answer for `homing_steps` simulator steps; time is stepped, never wall-clock sleeps, `CLAUDE.md` §5; architect `adrs.md` P5).

1. Given a job whose first line is `$H` and the connector waiting for GRBL's `ok` for it, when the connector receives `Pause`, then it answers `Status` with the new additive field `pause_pending: true` (protocol stays 0, golden file), the state stays `Running` (controller word `Home`, line count unchanged; a job's `$H` never enters `Homing`, `0118` M4), and nothing is sent to GRBL (no `!`).
2. Given criterion 1 and GRBL then answers `ok` for `$H`, then the connector sends **no further line** (it writes the real-time feed hold `!`, not a line; GRBL is idle and holds at once), the state is `Paused` when GRBL reports `Hold:0` (`0127` G10), with `pause_pending` false and `Status.job.line` the line number of `$H` (lines acknowledged, `0127` G8), so Resume continues with the next line, and the simulator has received no motion line, no `S` above 0 and no `M3`/`M4` since the `$H`. Test: the simulator's received lines end with `$H`.
3. Given the state `Homing` from the Home button without a job (`0118`), when `Pause` arrives, then it is answered `Error { WrongState }` with the text "No job to pause" (nothing to pause, nothing is remembered). Given a Ready job and the Home button, `Pause` is the same refusal.
4. Given `pause_pending`, when `Resume` arrives, then the pending pause is cancelled and the job continues after homing; a second `Pause` is idempotent (`0064` P17).
5. Given `pause_pending`, when `Stop` arrives, then it works as in `0127` G9 (soft reset without a feed hold, `Done { Stopped }`) and the pending pause is cleared; Stop is never held back by a pending pause.
6. Given `pause_pending` and homing fails (`ALARM:8`/`ALARM:9`, `error:5`, a reset, a lost controller, the homing cap of `0142`), then the pending pause is dropped, the job ends as `0129` criterion 2 says (`Failed` or `Alarm` with the cause) and the state is never `Paused`.
7. Given a pause pressed in a normal run (not homing), then it works as before (`!`, `Hold:0`, `0127` G12); the 2 s hold limit of `0127` G29 applies only to lines other than `$H`: a `Pause` is never dropped for homing.
8. Given the app, then after the press the Pause button shows the pending state (UX notes) and the footer's state line (Machine controls group; the Output state line was removed by Frage 262 A) reads "Pause after homing"; when the connector reports `Paused` it reads "Paused" (the line number is in the counts line); Resume is enabled in both. The same holds for a second app watching the status (it reads `pause_pending` from `Status`, no local memory).
9. Given the connector is restarted or the controller connection is lost while a pause is pending, then the pending pause is gone with the job's run (`0127` G15).
10. Given the cause, then the PR names it in one sentence and the test of criterion 2 fails on the code before the fix. Cause unknown: the implementer reproduces with the simulator first; a fix that cannot be reproduced first is not accepted.
11. Given the quality gate of `CLAUDE.md` §7, then it is green. 🟢 is claimed only after the customer repeats the run on the real laser; until then 🟡.

## Changes accepted behaviour

- `0127` G29 ("a `Pause` that GRBL does not confirm with `Hold:0` within the hold limit is dropped and the state stays `Running`"): no longer for `$H`; a pause during homing is kept pending (criterion 7). `0064` P17 and Q15 gain "Pause while homing is remembered".
- `0118` Home: "disabled with its cause otherwise" stays; `Pause` in `Homing` without a job answers "No job to pause" (criterion 3).

## Out of scope

- Aborting homing on Pause (GRBL cannot; Stop does it, `0127` G9), pausing in any other state, changing `$H` itself or its Locked bug (`0129`), the wording and look of the pending state (ux-engineer), a Pause the controller refuses.

## Needed before Ready

Done: architect addendum (`adrs.md`: PH1, PH2 and the test preamble fixed) and UX notes (`ux-notes.md`, H1 to H9, vetoable at the demo; they follow Frage 262 A: the footer's line is the state line). No open question.

## UX notes must change

None for this spec's criteria. Open items outside it: the UX notes of `0118` (branch `story/machine-control`) gain the three amendments listed in `ux-notes.md` section 9, and the design-system row "Machine controls (Operations tab) (`0118`)" lands with the build.

## Links

Requirements: R-MFG-LASER-002
Related: `specs/0127-connector-grbl-serial/` (G9, G12, G15, G22, G29), `specs/0129-fix-home-first-locked/` (same `$H` path, same simulator), `specs/0118-machine-control/`, `specs/0064-machine-connector/` (P17, Q15), `specs/0125-send-to-machine/`
PR: -

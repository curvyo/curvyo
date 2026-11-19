# Fix: "Home, then machine origin" homes, then the machine answers "Locked"

Status: Draft
Kind: Bug (branch `fix/home-first-locked`)
Priority: Must
Origin: Customer, first test on a real laser, 2026-11-18 (GRBL 1.1f, 400 x 850 mm), PR #386. Translated: "Send job with Home: it homes the machine origin first and then goes into an error 'locked'." Start, Pause and Stop work.

## Reproduction (as reported)

1. Laser page, output group, Start pick = **Home, then machine origin** (`0029` criterion 38; the file's first command is `$H`).
2. Send to machine (`0125`), then Start in the Connector row (`0127`).
3. The machine homes (the head travels to its switches), then the job ends in an error that names "Locked" (GRBL `error:9`, "G-code locked out during alarm or jog state", or the alarm text of `0127` G13/G14).

Expected: after `$H` GRBL reports Idle and the job runs from the machine origin to `Completed`. Start pick **Machine origin** (no `$H`) is reported to work.

## Cause: unknown

Nothing is known yet. The implementer finds it out, in this order, and records the answer in the PR:

1. Read the real log of the laser run (the connector's terminal output and `jobs.log` of the run; ask the customer for them if the connector was not logging, "nichts" if the log exists on the branch of PR #386).
2. Read what GRBL answered to `$H` and to the first lines after it: `ok`, `error:5` (homing not enabled, `$22=0`), `ALARM:8` or `ALARM:9` (homing failed or limit switch not found), `error:9` (Locked). Note `$22`, `$23`, `$24` to `$27` and `$20` from the `$$` the connector reads (`0127` G2, G19).
3. Check the connector's own steps around `$H`: the soft reset of `0127` G17 and its alarm; the wait for `ok` of `0127` G9 (no time limit) versus lines sent before GRBL finished; the loss rule of G15 judged on `?` answers during homing; the first line after `$H` (`G21`, `G90`, ...) reaching GRBL while it is still in `Alarm` or `Home`; a `?` state read mis-parsed as `Alarm` after homing.
4. Reproduce with the pty simulator of `0127` G22 (`curvyo-connector-io/tests/common/`): extend it to the observed GRBL behaviour (homing cycle, `$22`, the exact answers of step 2) until the failure shows, then fix. A fix that cannot be reproduced first is not accepted.

## Acceptance criteria

1. Given the simulator set to GRBL 1.1 with `$22=1` that behaves like the real run (the reproduction of step 4), when the golden job `home_first.gcode` runs, then it ends `Completed`, every line after `$H` is accepted, and no `Locked` text appears. The test fails on the code before the fix.
2. Given homing fails on the machine (`ALARM:8`/`ALARM:9`, or `$22=0` which answers `error:5`), then the job ends as `Failed` or `Alarm` at line 1 with a text that names the real cause in words ("Homing did not find the switch (alarm 9)", "Homing is not enabled (error 5)", the texts built in `curvyo-connector-io/src/grbl/texts.rs`) and not "Locked"; the machine is left safe (laser off, `0127` G13). Table-driven test of the answers.
3. Given the job starts while GRBL is already locked (`Alarm` at connect), then Start stays impossible with the alarm text and the **Unlock** of `0127` G33 is the way on; a job whose first line is `$H` is the only job that may start from `Alarm` after Unlock, because `$H` itself leaves the alarm (decide from the cause; if GRBL needs `$X` first, the cause goes into the PR and into `0118` Home).
4. Given the fix, then the exported file and the sent job are byte-identical to before unless the cause is the file (`$H` placement); a changed golden file is named in the PR.
5. Given the quality gate of `CLAUDE.md` §7, then it is green. 🟢 is claimed only after the customer repeats the run on the real laser (or sends the log); until then 🟡.

## Changes accepted behaviour

None intended. If the cause is in `0029` criterion 38 (what the file writes after `$H`), the change is listed in the PR and the customer is asked.

## Out of scope

- A live Home button (`0118`), changing GRBL settings (`0127` G5), other controllers.

## Links

Requirements: R-MFG-LASER-002, R-MFG-LASER-003
Related: `specs/0029-laser-job-preview-and-output/` (criterion 38), `specs/0127-connector-grbl-serial/` (G9, G13 to G15, G17, G22, G33), `specs/0125-send-to-machine/`, `specs/0118-machine-control/`
PR: -

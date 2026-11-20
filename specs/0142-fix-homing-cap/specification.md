# Fix: a homing that never answers ends after a cap (120 s by default)

Status: Ready (no UX notes and no architect work needed: one footer text, one flag, one constant; the build waits for PR #399 and for `0133`, which touch the same `$H` code)
Kind: Bug (branch `fix/homing-cap`)
Priority: Must (safety: a pulled cable must never leave the app reading "Running")
Origin: Customer. Test on the real laser, 2026-11-20: pulling the USB cable while the connector waits for the silent `$H` `ok` is never noticed (the status stays Running for 90 s); in a normal job the same unplug gives "Lost the controller" after about 3 s. **Frage 265 (HOMING-CAP), B decided by the customer 2026-11-20:** the connector gives up after a cap when `$H` returns no `ok`, 120 s by default, adjustable with `--homing-timeout <seconds>`.

## Cause

`0127` G9 waits for the `ok` of `$H` "with no time limit" and judges the loss by the answers to `?`; `0129` paused the 3 s loss clock while `$H` is in flight (PR #392) because GRBL answers no `?` while it homes; `0118` (Home button) and `0133` (pending pause) rely on the same wait. A cable pulled during homing therefore looks exactly like homing.

## Acceptance criteria

Prefix `HC`. Tests use the pty simulator of `0127` G22 with its silent homing (`quiet_homing`) and a time source injected into the connector, stepped by the test, never wall-clock sleeps (`CLAUDE.md` §5).

1. Given a `$H` in flight, from the job's first line (`0029` criterion 38) or from the Home button (`0118`), when GRBL has not answered `ok` (or `error:<n>`, `ALARM:<n>`) for the cap, then the controller counts as lost. One constant, one clock start (the moment `$H` is written), one code path for both cases.
2. Given no flag, then the cap is 120 s. Given `--homing-timeout <seconds>`, then the cap is that value; whole seconds from 10 to 3600. **0 does not mean "no cap" and is refused, as is any value outside the range or not a number:** the customer wants a cap (Frage 265 B), and a cap that can be switched off is the bug again; 3600 s covers the slowest big machine. The refusal is a start error naming the flag, the range and the value ("--homing-timeout must be 10 to 3600 seconds, got 0"), exit code 2, before the port is opened.
3. Given the cap expires, then the connector (a) stops sending, (b) writes the reset byte `0x18` first and then `M5` and `M9`, tries it even if the port refuses (as `0127` G15 b; the 2 s and 3 s limits of G12 do not apply, the port is probably gone), (c) goes to `NoController` and retries by G3, (d) drops a Ready job (as after every alarm or reset), (e) ends a job by going straight to `NoController` with the text **"Lost the controller: homing took longer than 120 s"** in `Status.detail` and no `Done` frame, as every loss of `0127` G15 (**Proposal**; the configured value in place of 120; at most 70 characters at the maximum, 3600; amended 2026-11-20, the draft said `Done { Failed }`), (f) writes `Failed: controller lost, homing took longer than <cap> s` to `jobs.log` for a job. The Home button (no job) shows the same text in the state line and writes nothing to `jobs.log`.
4. Given a pending pause (`0133`, `pause_pending`) when the cap expires, then it is dropped and the state is never `Paused` (`0133` criterion 6 gains "the cap").
5. Given a homing that takes longer than 3 s but ends with `ok` before the cap, then nothing changes against today: state `Running` (job) or `Homing` (button), `?` polling continues, no loss text. Given the `ok` arrives in the same simulator step in which the cap would expire, then the `ok` wins and the job continues (the cap is judged after the step's input is read).
6. Given `Stop` during homing, then it keeps its rules (`0127` G9: reset without a feed hold, `Done { Stopped }` after the banner); the cap is cleared by the Stop and cannot turn it into a loss. Given the controller falls silent while the stop waits, G12's 3 s rule applies as before.
7. Given a lost controller and the port coming back, then the usual reconnect and fresh identify happen (G3, G2); the dropped job is not resumed; a new `Start` needs a new `Send`. The fresh identify reads `$$` again even when the banner of the reset the connector gave up with is still in the port (two banners, so two `$I` and two `ok`): an `ok` before any `$<n>=` line is not taken as the answer to `$$`, `$32` is known only from a `$$` that was really read, and a new `Send` after the reconnect runs (the same after a G15 loss; found by the tester, also present on `main`).
8. Given the wire, then nothing changes: no new message, field or command, protocol stays 0, no golden file changes; the app reads the text from `Status` like every loss text.
9. Given `curvyo-connector-server --help`, then it lists `--homing-timeout <seconds>` with the default and range; `docs/guides/connector-serial-linux.md` gets a short "Homing" paragraph (the cap, the flag, what the app reads, that a long machine needs a higher value).
10. Given configured mode (ADR 0071), then the cap is a field `homing_timeout_s` in `[controller]` of `connector.toml` with the same default and range, added by `0128`'s follow-up (**Proposal**, no ADR change: an additive optional field, golden files for both spellings); this story builds the flag and the constant only.
11. Given the quality gate of `CLAUDE.md` §7, then it is green. 🟢 is claimed only after the customer repeats the unplug during homing on the real laser; until then 🟡.

Tests: the simulator with silent homing longer than a short test cap (for example 5 s injected) ends as criterion 3 says, for the job and for the Home button, each asserting the bytes written (`0x18` before `M5` before `M9`); one that answers `ok` just before and one in the step of the cap; a pulled port (closed pty) during homing; a pending pause dropped; table-driven flag validation (0, 9, 10, 3600, 3601, "abc", "-5").

## Changes accepted behaviour

- `0127` G9: "waits for GRBL's `ok` for that line with no time limit (homing takes minutes)" becomes: no limit except the cap of this story (120 s by default, adjustable); homing that takes longer than the cap on the maker's machine needs `--homing-timeout`. The loss rule of G15 is still judged by the answers to `?`, and the cap is the addition for the time `?` is silent.
- `0129` note (PR #392): "the 3 s loss clock is paused while `$H` is in flight" stays; it is bounded by the cap now instead of running indefinitely.
- `0118` Home: "until GRBL answers `ok`" gains "or the cap ends it as a lost controller"; the Home button's failure texts gain the cap text. No change to enabling, Stop or `--require-local-start`.
- `0133`: criterion 6 and `pause_pending`'s drop list gain the cap (criterion 4 above).
- `0127` G2 (identify): an `ok` that arrives in the settings step before any `$<n>=` line answers an earlier `$I`, not `$$`, and is skipped (criterion 7 above). Before, a stale banner after any loss could end identify with no settings, and every Send was then refused with `LaserModeUnknown` until the connector restarted.

## Out of scope

- Detecting a pulled cable faster than the cap during homing (GRBL answers no `?` while it homes, so a cable pull is only separable from a silent homing by the port closing, which G15 already catches as a read or write error; the cap is for a port that stays open), a progress estimate for homing, a cap for other long lines, changing `$H` itself.

## Links

Requirements: R-MFG-LASER-002
Related: `specs/0127-connector-grbl-serial/` (G3, G9, G12, G15, G22), `specs/0129-fix-home-first-locked/`, `specs/0118-machine-control/`, `specs/0133-fix-pause-while-homing/`, `docs/adr/0071-*` (flags, configuration), `docs/guides/connector-serial-linux.md`
PR: -

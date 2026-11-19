# Send and start: the page's button starts the job when nothing else is on the machine

Status: Done (PR #428)
Priority: Should
Origin: Customer, test of PR #399 on the real laser, 2026-11-19; decisions of 2026-11-20 applied (Frage 263 B, Frage 264 A, glyph accepted). Translated: "I'd like it so that if you are the owner yourself and send the job first, with no other job in the list, it goes directly to play; I would also show the Send to machine button for the page as a play symbol." Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As the owner of my own laser I want one press on the page to send the job and start it when the machine is free, so that I do not send and then walk to a second button (`R-MFG-LASER-002`, `R-MFG-LASER-003`).

**One-line meaning (customer decision 2026-11-19):** *Send to machine* on the page sends the job and, when the machine holds nothing else, starts it, after an armed press (first press arms, second confirms; customer 2026-11-20). With anything held or waiting it only sends.

## What changes in the demo

- The Output group's **Send to machine** button shows a play glyph (Lucide `CirclePlay`; the footer's Start keeps `Play`; accepted by the customer 2026-11-20) and its tooltip and accessible name say what it does: "Send to machine and start" when the next press will start, "Send to machine" when it will only send. The Machine controls group keeps its own Start (`0118`).
- When the next press will start, it is an **armed press**: the first press arms the button (it changes look, name and tooltip to say what the next press does), the second press within the arming time sends and starts in one action; Escape, a click elsewhere, focus leaving or the timeout disarms without sending. Pressed with a held or waiting job nothing is armed and it only sends.
- After the second press the job is sent, accepted, then started in one action; the state line goes "Sending <name>", "Running ...". Under `--require-local-start` or as a member, it reads "Waiting for the start at the machine" / "Waiting for <owner> to confirm" (`0064`, `0128`).
- Pressed with a held or waiting job it behaves as today.
- Try it: `curvyo-connector-server --fake-controller`, pair, press the play button twice.

## Boundary (Proposal)

- **In:** the enabling and "will it start" rule as a pure function in an existing `*-core` crate; the second step (`Start` after `JobAccepted`) in the app; glyph, tooltip, state texts.
- **Reuses, does not change:** the protocol (no new message), the connector, the job writer, `Start` rules of `0064`/`0127`, the Lasered commit of `0125`.
- **No new crate, dependency, protocol or `format_version` change.**

## Acceptance criteria

Prefix `SS`.

1. Given an app that is an owner (until slice `0128` exists **every paired app counts as owner**; afterwards the role from `Authenticated`), and a connector that holds no job and nothing waits (state `Idle` or `Done`, no `Ready`/`Receiving`/`AwaitingLocalStart`/`Running`/`Paused` job, job list empty once `0132` exists), when the maker confirms the armed press (SS13), then the app sends the job as `0125` S6 and, on `JobAccepted` with `Ready` and the sent job id, sends `Start` for that id in the same action, without a further press.
2. Given a connector that holds or waits for a job (`Ready`, `Receiving`, `AwaitingLocalStart`, `Running`, `Paused`, or a non-empty list; Frage 264 A, the customer's rule "no other job", also for this app's own held `Ready` job), then nothing is armed and the press sends only: today's refusal "Connector busy" until `0132`, then the job is added to the list and nothing starts (the list's own Start, `0132` criterion 4).
3. Given `--require-local-start` (`0064` criterion 9, foreground mode) or a member app (`0128` A13), then the press is armed and confirmed as in SS13, the app still sends `Start` and the connector's rule applies: the state is `AwaitingLocalStart` and the line reads "Waiting for the start at the machine" / "Waiting for <owner> to confirm"; the job runs only when the press or confirmation comes. Nothing is bypassed.
4. Given a job whose Start pick is **Home, then machine origin**, then the `Start` runs the job text as written: `$H` first (`0029` criterion 38); the press does not add or skip homing. A pause pressed during it follows `0133`.
5. Given the connector reports `Alarm`, `Door`, `Check` or `Sleep`, or `NoController`, then nothing is armed, no `Start` is sent and the job is not held either: refused as `0125` S7 says for `Alarm` and `NoController` ("Machine in alarm: <text>"), and as `Busy` in `Door`, `Check` and `Sleep` (`0127` G4); for any state that blocks the start, the footer shows the cause, never a started job. Given `$32` unknown or 0 (laser mode off), then the send is refused as today (`0127`).
6. Given the job is accepted but `Start` is refused or not delivered (a state change in between, another client's command, the connection drops), then the job **stays `Ready`**, the Output state line reads "Ready: <name>, <n> lines, <size>" and one inline text "Not started: <connector's reason>" (Proposal), the Lasered commit has happened (criterion 7), and the Start of the Machine controls group is enabled; the line shows only while the connector still reports this job `Ready` (architect `adrs.md` S3). Given the controller's second guard fails the start before any byte (`0127` G4 `Door`/`Check`/`Sleep`, G34 `$32`), then the job is `Done { Failed }` with the connector's words, as for a Start from the Machine controls group. No dialog, no retry by itself.
7. Given `JobAccepted`, then the Lasered commit of `0125` S10 to S12 happens on `Ready` exactly as today; it does not wait for the start and does not depend on it.
8. Given the page's checks block, no connector is paired, the browser build or a send in progress, then the button is disabled with the same causes as `0125` S2 to S5, S8; the glyph stays Play, the cause is the tooltip.
9. Given the tooltip and accessible name, then they read "Send to machine and start" when criterion 1's machine conditions hold at that moment, in every role (a member's or a foreground connector's start then waits, criterion 3) (from the last status the app has; it is re-read at the press) and "Send to machine" otherwise; the press re-evaluates from a fresh status, never from the label: a press starts the job only if the name the maker saw and a fresh status both say it will start (UX P4); this check holds at the arming press and again at the confirming press. The armed name and tooltip read "Press again to send to machine and start" (Proposal).
10. Given an owner on a free machine, when the press sends and starts, then it is two presses, armed (SS13, Frage 263 B, customer 2026-11-20): no popup, no dialog, no sentence. 
11. Given the decision functions (will it start, which cause), then they are pure with a table test over the connector's state (x list once `0132` exists; role and `--require-local-start` do not change what the press sends and the flag is not on the wire, so they are covered by the integration test); the integration test runs the fake connector: first press arms and sends nothing, second press -> started; free -> started, held -> sent only, member -> waiting, `$32` refused, start refused -> stays Ready, without wall-clock sleeps.
12. Given the quality gate of `CLAUDE.md` §7, then it is green; `docs/design-system.md` row "Send to machine button" gets the `CirclePlay` glyph, the two names and the armed look.
13. (SS-arm, Frage 263 B) Given the will-it-start conditions of SS1 and SS9, then the first press **arms** the button and sends nothing: name, tooltip and accessible name change to "Press again to send to machine and start", the look changes (UX notes to decide; Proposal: filled `--editor-accent` ground, white glyph, not `--destructive` because starting is not destructive and Stop stays red), and a polite status region repeats the name. The second press within the arming time sends and starts in one action (SS1), and only if the will-it-start check holds at that second press from a fresh status (SS9); if it no longer holds the button disarms and only the send rule of SS2/SS5 applies, nothing starts. A press within 400 ms of arming is ignored. It disarms, without notice, after 4 s, on Escape, on any other pointer press, when the focus leaves the button, and when the connector status or the page's checks change so that the name is no longer "start". The keyboard works: `Enter` or `Space` on the focused button arms and confirms, focus stays put. One armed action in the app at a time (arming another disarms the first; `docs/design-system.md` rows "Inline delete confirm" and "Armed action", same hook and timings). No shortcut (UX P10). When the name is not "start" the button is not armed and a single press sends only.

## Changes accepted behaviour

Customer decision of 2026-11-20 (Frage 263 B, armed press):

- `0125` decision **CONN-SEND-MEANING A** ("Send job is a copy only, Start is separate"), amended, decided by the customer 2026-11-20 with B: becomes "Send to machine on the page also starts the job when the machine is free and the maker confirms an armed press (first press arms, second sends and starts; for a member or under `--require-local-start` the start then waits, criterion 3); otherwise it is still a copy only".
- `0125` one-line meaning and the "Out of scope" line "a way to start the job from the send (Send to machine never starts a job)"; `0125` S9 ("Given the connector accepted the job ... Start is enabled, the maker presses Start") gains the start-by-press as the other way; `0125` S15 and its Output group rule are unchanged (the group has no Start button, only the page's play button); S1 (glyph chosen by UX) and the `0125` UX notes for the button glyph and tooltip; S8 is as before.
- `0064` Q14 "Start is a separate press" stays for the Machine controls group and for anything not sent from the page.
- `0118` criteria where the Output group's button is mentioned: the Machine controls group keeps Start/Pause/Stop/Home/Unlock and the Output group state line; the `0118` change "S15 names the Machine controls group" stands, with the added exception that the page button starts on a free machine.

## Questions

Decided by the customer, 2026-11-20:

- Frage 263 (SEND-START-CONFIRM): **B**, an armed press: first press arms, second sends and starts (SS13).
- Frage 264 (SEND-START-HELD): **A**, today's rule: with a held `Ready` job the press sends only ("Connector busy" until `0132`, then the list); nothing starts, nothing is armed (SS2).
- The glyph change (`CirclePlay`) is accepted.

No open question.

## Out of scope

- Reordering, list, auto-chain (`0132`); the owner/member roles and confirmation themselves (`0128`); a Start in the Output group beyond this button; starting from the browser build; changing what Export does.

## Needed before Ready

Done: architect addendum (`adrs.md`: SS5, SS6, SS9, SS11 fixed) and UX notes (`ux-notes.md`, P1 to P10). SS2 does not list `Homing`: the connector refuses a send there as `Busy` (`0127` G4). SS6 names the footer's state line (Frage 262 A removed the Output line). No open question.

## UX notes must change

`ux-notes.md` predates Frage 263 B (the press was one press). The ux-engineer adds, as the next step:

- The **armed look** of the page button: choose and write down (Proposal: filled `--editor-accent`, white `CirclePlay`; `--destructive` is for destructive acts, and a start is not one; check contrast, 28px size and position unchanged, no transition over 120 ms, none under `prefers-reduced-motion`).
- The **states**: idle ("Send to machine and start"), armed ("Press again to send to machine and start"), the 400 ms guard, busy "Sending" and "Starting" after the second press, disarmed; name, tooltip, `aria-label` per state; the polite status region text and that disarm is silent.
- **Disarm rules**: 4 s, Escape, another pointer press, focus leaving, status or checks change; keyboard `Enter`/`Space`, focus stays; one armed action in the app at a time.
- Section 4 (P4) to say the will-it-start check holds at both presses; P3 (name for a member / `--require-local-start`) stays; P10 stays (no shortcut); sections 5, 8, 9 and the design-system rows "Send to machine button" and "Armed action" (new user) get the armed state.

## Links

Requirements: R-MFG-LASER-002, R-MFG-LASER-003
Amends: `specs/0125-send-to-machine/` (CONN-SEND-MEANING A, S9, Out of scope)
Related: `specs/0118-machine-control/`, `specs/0128-connector-api-and-ctl/` (owner and member, branch `story/connector-api`), `specs/0132-connector-job-queue/`, `specs/0133-fix-pause-while-homing/`, `specs/0064-machine-connector/`, `specs/0127-connector-grbl-serial/`
PR: -

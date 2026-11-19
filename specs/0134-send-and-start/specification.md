# Send and start: the page's button starts the job when nothing else is on the machine

Status: Draft
Priority: Should
Origin: Customer, test of PR #399 on the real laser, 2026-11-19. Translated: "I'd like it so that if you are the owner yourself and send the job first, with no other job in the list, it goes directly to play; I would also show the Send to machine button for the page as a play symbol." Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As the owner of my own laser I want one press on the page to send the job and start it when the machine is free, so that I do not send and then walk to a second button (`R-MFG-LASER-002`, `R-MFG-LASER-003`).

**One-line meaning (customer decision 2026-11-19):** *Send to machine* on the page sends the job and, when the machine holds nothing else, starts it. With anything held or waiting it only sends.

## What changes in the demo

- The Output group's **Send to machine** button shows a play glyph (Lucide `Play`) and its tooltip and accessible name say what it does: "Send to machine and start" when the next press will start, "Send to machine" when it will only send. The Machine controls group keeps its own Start (`0118`).
- Pressed with a free machine, the job is sent, accepted, then started in one action; the state line goes "Sending <name>", "Running ...". Under `--require-local-start` or as a member, it reads "Waiting for the start at the machine" / "Waiting for <owner> to confirm" (`0064`, `0128`).
- Pressed with a held or waiting job it behaves as today.
- Try it: `curvyo-connector-server --fake-controller`, pair, press the play button.

## Boundary (Proposal)

- **In:** the enabling and "will it start" rule as a pure function in an existing `*-core` crate; the second step (`Start` after `JobAccepted`) in the app; glyph, tooltip, state texts.
- **Reuses, does not change:** the protocol (no new message), the connector, the job writer, `Start` rules of `0064`/`0127`, the Lasered commit of `0125`.
- **No new crate, dependency, protocol or `format_version` change.**

## Acceptance criteria

Prefix `SS`.

1. Given an app that is an owner (until slice `0128` exists **every paired app counts as owner**; afterwards the role from `Authenticated`), and a connector that holds no job and nothing waits (state `Idle` or `Done`, no `Ready`/`Receiving`/`AwaitingLocalStart`/`Running`/`Paused` job, job list empty once `0132` exists), when the maker presses Send to machine, then the app sends the job as `0125` S6 and, on `JobAccepted` with `Ready` and the sent job id, sends `Start` for that id in the same action, without a second press.
2. Given a connector that holds or waits for a job (`Ready`, `Receiving`, `AwaitingLocalStart`, `Running`, `Paused`, or a non-empty list), then the press sends only: today's refusal "Connector busy" until `0132`, then the job is added to the list and nothing starts (the list's own Start, `0132` criterion 4).
3. Given `--require-local-start` (`0064` criterion 9, foreground mode) or a member app (`0128` A13), then the app still sends `Start` and the connector's rule applies: the state is `AwaitingLocalStart` and the line reads "Waiting for the start at the machine" / "Waiting for <owner> to confirm"; the job runs only when the press or confirmation comes. Nothing is bypassed.
4. Given a job whose Start pick is **Home, then machine origin**, then the `Start` runs the job text as written: `$H` first (`0029` criterion 38); the press does not add or skip homing. A pause pressed during it follows `0133`.
5. Given the connector reports `Alarm`, `Door`, `Check` or `Sleep`, or `NoController`, then no `Start` is sent: the job is not sent either for `Alarm` and `NoController` (refused as `0125` S7 says, "Machine in alarm: <text>"); for any state that blocks the start, the footer shows the cause, never a started job. Given `$32` unknown or 0 (laser mode off), then the send is refused as today (`0127`).
6. Given the job is accepted but `Start` is refused or fails (a state change in between, the controller left Idle, the connection drops), then the job **stays `Ready`**, the Output state line reads "Ready: <name>, <n> lines, <size>" and one inline text "Not started: <connector's reason>" (Proposal), the Lasered commit has happened (criterion 7), and the Start of the Machine controls group is enabled. No dialog, no retry by itself.
7. Given `JobAccepted`, then the Lasered commit of `0125` S10 to S12 happens on `Ready` exactly as today; it does not wait for the start and does not depend on it.
8. Given the page's checks block, no connector is paired, the browser build or a send in progress, then the button is disabled with the same causes as `0125` S2 to S5, S8; the glyph stays Play, the cause is the tooltip.
9. Given the tooltip and accessible name, then they read "Send to machine and start" when criterion 1's conditions hold at that moment (from the last status the app has; it is re-read at the press) and "Send to machine" otherwise; the press re-evaluates from a fresh status, never from the label.
10. Given an owner on a free machine, when the press sends and starts, then it is one press: no confirmation, no popup (Frage 261 A, default).
11. Given the decision functions (will it start, which cause), then they are pure with a table test over state x role x `--require-local-start` x list; the integration test runs the fake connector: free -> started, held -> sent only, member -> waiting, `$32` refused, start refused -> stays Ready, without wall-clock sleeps.
12. Given the quality gate of `CLAUDE.md` §7, then it is green; `docs/design-system.md` row "Send to machine button" gets the Play glyph and the two names.

## Changes accepted behaviour

Customer decision of 2026-11-20:

- `0125` decision **CONN-SEND-MEANING A** ("Send job is a copy only, Start is separate"): becomes "Send to machine on the page also starts the job when the machine is free and the app is an owner; otherwise it is still a copy only".
- `0125` one-line meaning and the "Out of scope" line "a way to start the job from the send (Send to machine never starts a job)"; `0125` S9 ("Given the connector accepted the job ... Start is enabled, the maker presses Start") gains the start-by-press as the other way; `0125` S15 and its Output group rule are unchanged (the group has no Start button, only the page's play button); S1 (glyph chosen by UX) and the `0125` UX notes for the button glyph and tooltip; S8 is as before.
- `0064` Q14 "Start is a separate press" stays for the Machine controls group and for anything not sent from the page.
- `0118` criteria where the Output group's button is mentioned: the Machine controls group keeps Start/Pause/Stop/Home/Unlock and the Output group state line; the `0118` change "S15 names the Machine controls group" stands, with the added exception that the page button starts on a free machine.

## Questions

Frage 261 (SEND-START-CONFIRM): Does the play press start the laser without a second confirmation?
- A: Yes, one press (the customer asked for "directly"); the safety rests on the checks, the connector's rules and Stop.
- B: An armed press (`0081`): first press arms and names the job, second sends and starts.
- C: A setting per machine profile, default as A.
Recommendation A. Default if unanswered: A.

Frage 262 (SEND-START-HELD): When the connector holds this app's own `Ready` job and the maker presses the play button on the same page, what happens?
- A: Today's rule: refused "Connector busy" (until `0132`), then added to the list; nothing starts.
- B: The press starts the held job if it came from this page and is unchanged.
- C: As B, for any held job of this app.
Recommendation A (the customer said "no other job"). Default if unanswered: A.

## Out of scope

- Reordering, list, auto-chain (`0132`); the owner/member roles and confirmation themselves (`0128`); a Start in the Output group beyond this button; starting from the browser build; changing what Export does.

## Needed before Ready

- `ux-engineer`: ux-notes for the Play glyph on the Output button, the two names and tooltips (criterion 9), the "Not started" line (criterion 6), the waiting states, focus after the press; update the `0125` UX notes rows.
- `architect`: an addendum on where the "will it start" function lives (existing `*-core` crate), the one-action flow `JobAccepted` -> `Start` (and its race with another client), confirm no protocol change, the interplay with `0128` roles and `0132` list.

## Links

Requirements: R-MFG-LASER-002, R-MFG-LASER-003
Amends: `specs/0125-send-to-machine/` (CONN-SEND-MEANING A, S9, Out of scope)
Related: `specs/0118-machine-control/`, `specs/0128-connector-api-and-ctl/` (owner and member, branch `story/connector-api`), `specs/0132-connector-job-queue/`, `specs/0133-fix-pause-while-homing/`, `specs/0064-machine-connector/`, `specs/0127-connector-grbl-serial/`
PR: -

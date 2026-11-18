# Bed size from the controller: take the work area from the connected GRBL machine

Status: Draft
Priority: Could
Origin: Customer, first laser test, 2026-11-18. Translated: "It would be practical to take the bed size from the laser cutter when we get it." The laser reports 400 x 850 mm; the profile of the test said 400 x 300.

## User value

As a maker with a laser on a connector I want the app to offer the bed size the machine itself reports, so that the profile's work area is right without measuring or typing it (`R-MFG-LASER-001`, `R-MFG-LASER-003`).

## What other tools do

LightBurn asks for the bed size in its device wizard and can read GRBL `$130`/`$131` when it adds a device; LaserGRBL shows them only. We do better: the maker is offered the numbers on the profile they already use, with one press and one undo step, and nothing changes without that press.

## Boundary (Proposal)

- GRBL's `$130` (X travel) and `$131` (Y travel) in mm, which the connector already reads after identify (`0127` G2, G19, shown in the Controller line). The connector exposes them as numbers in what the app already receives (additive field in `Welcome.controller` or `Status`; protocol stays 0; the architect decides). The connector writes no setting (`0127` G5).
- Only the work area. Origin corner (`$23`), max feed (`$110`, `$111`), max power (`$30`) and laser mode (`$32`) are possible later offers, not here.
- Travel is the machine's range, not always the usable bed; it is an offer, never silent.

## Questions

Frage 255 (BED-APPLY): What happens when the controller reports a size? **Decided B by the customer (2026-11-18).**
- A: The profile's work area is overwritten on every connect.
- B: The app offers it: while the connected machine's size differs from the work area of the manufacturing's copy, one line in the Machine block reads "Machine reports 400 x 850 mm" with a symbol button **Use machine size**; nothing changes until it is pressed, never on connect; no memory of a refusal.
- C: Offer only when the work area still has the default (400 x 300).

Frage 256 (BED-TARGET): What does the press change? **Decided A by the customer (2026-11-18).**
- A: The manufacturing's copy and the library profile it points at (one press, the copy and the page size in one undo step, the library write as with Update profile without undo); for the built-in Generic laser (not editable) only the copy.
- B: Only the manufacturing's copy (the maker then presses Update profile).
- C: Only the library profile.

## Acceptance criteria (Fragen 255 B and 256 A, decided)

1. Given a paired connector whose controller reports `$130=400.0`, `$131=850.0` and a copy with 400 x 300, then the Machine block shows "Machine reports 400 x 850 mm" and **Use machine size**; given a copy of equal size (tolerance 0.01 mm) the line is absent; given no connector, no controller or no reported travel, it is absent.
2. Given the press, then the copy's work area and the output page become 400 x 850 mm in one undo step with the page behaviour of `0093` criterion 3 (content keeps its place from the page's top-left corner), and the library profile takes the new work area; the line goes away. For the built-in profile only the copy changes and "Differs from profile" shows as for any edit.
3. Given a reported value outside 1 to 10,000 mm or not finite, then no offer is shown.
4. Given a reconnect to the same machine after the maker edited the work area by hand, then the offer returns (no refusal memory).
5. Given the browser build, the line is absent. Given a second paired app, each shows its own offer.
6. Tests: pure comparison and range check in `*-core` (table); connector test with the simulator's `$$`; one browser check. No `format_version` change (a work area is stored already).

## Changes accepted behaviour

None: `0026` criterion 1 says a work area is typed; a typed value stays possible. `0127` G19 ("shows travel") gains the offer, no change to it.

## Out of scope

Writing `$130`/`$131` to the machine, origin corner and limits from the controller, several machines, Ruida and others.

## Links

Requirements: R-MFG-001, R-MFG-LASER-003
Depends on: `specs/0127-connector-grbl-serial/` (G2, G19), `specs/0026-machine-profile/`
Related: `specs/0118-machine-control/`
PR: -

# 0131 Bed size from the controller: UX notes

Revised 2026-11-28 by `docs/design-drafts/2026-11-machine-families/ux-unify.md` section 4.3: the offer row moves with the work area into the machine's body in the Machines tab; a press is one undo step on the library record; the pages using the machine follow through the inline ask of ux-unify 4.5.

ux-engineer, 2026-11-21. Covers criteria 1 to 5 of `specification.md` (Fragen 255 B, 256 A). Checked against the built `frontend/src/components/workbench/MachineBlock.tsx` (state row, Work area line), `ConnectorGroup.tsx` (Controller block), `frontend/src/components/panel/MachineControls.tsx`, `MachineProfileList.tsx`, `specs/0029` P31 (nothing in the Machine block moves under the pointer) and `docs/design-system.md` rows "Machine block (profile picker)", "Laser page Document tab (`0098`)", "Controller block (`0127`)", "Armed action", "No notice after a success". No hint or tip text. (Proposal) marks are vetoable at the demo (V1 to V5).

## 1. What changes where

- **Machine block** (laser page Document tab, and a manufacturing's tab wherever the block renders): one new 28px **offer row** directly under the "Work area" line, above Origin corner (section 2). Nothing else in the block changes.
- **Controller block** (`0127`): unchanged; its Travel row stays text only, no control.
- **Machine controls footer** (Operations tab, `MachineControls.tsx`): unchanged; no offer there, no warning line (the work area is no settings warning of `controller_warnings`).
- **Profile list** (`MachineProfileList.tsx`, Edit work area sub-row): unchanged; after a press the row's 12px work area text shows the new size because the library record changed.

## 2. Offer row

- Layout: label at the left, 12px `--toolbar-icon`, `role="status"` (polite), text **"Machine reports 400 × 850 mm"**; at the right edge the symbol button **Use machine size** (`ICON_BUTTON`, 28 x 28, 36 coarse, Lucide `ArrowUpToLine` 16px at 1.5px stroke: "into the line above", V1 (Proposal)). Tooltip `side="left"` "Use machine size"; accessible name "Use machine size"; `aria-describedby` on the label.
- Numbers: in the document's display unit, with `formatWorkArea` as the Work area line above it ("Machine reports 40 × 85 cm" in cm), so the two lines compare at a glance, V2 (Proposal). The `×` sign as in Work area and Travel (the spec's "x" is the plain-text spelling).
- The place (P31): the row's 28px place is kept, empty and `aria-hidden`, while the connector reports a valid travel for this page's connector (criterion 3 range), whether or not it differs. It appears or goes only when a controller with travel arrives or leaves, never because of a press: after **Use machine size** the text and button leave, the place stays, so a second press at the same point hits nothing. Without a connector, controller or valid travel, and in the browser build, there is no row and no place (criteria 1, 3, 5), V3 (Proposal).
- Not armed: one press acts (the copy part is undoable; the library write is the same write as Update profile, but the maker asked for exactly these numbers, Frage 256 A). Not greyed during a job: the work area is editable then as well.

## 3. States

| State | Offer row | State row (above Work area) |
|---|---|---|
| Unknown (no connector, not paired, no controller, `NoController`, no or invalid `$130`/`$131`, browser) | absent, no place | as today |
| Equal (within 0.01 mm) | empty kept place | as today |
| Differs from the copy | "Machine reports W × H unit" + **Use machine size** | as today (may also read "Differs from profile" when the copy was edited) |
| Applied, user profile | empty kept place; Work area reads the new size | empty (copy equals the library record) |
| Applied, built-in Generic laser | empty kept place | "Differs from profile" with Reset from profile (Update hidden), as for any edit |
| After Undo | offer back (copy is old again) | "Differs from profile" (the library kept the new size) |
| Library write fails | empty kept place | "Differs from profile"; alert at the save anchor (section 4) |

A "Not in your profiles" copy has no connection (`0064` Q11), so it never gets the offer. A reconnect after a hand edit shows the offer again (criterion 4); a refusal is not remembered. Each paired app decides its own offer (criterion 5).

## 4. Texts (exact)

- "Machine reports <W> × <H> <unit>" (label), "Use machine size" (tooltip and name).
- Screen-reader only, polite, once after the press: "Work area <W> × <H> <unit>" (state, not a notice; no visible notice after a success).
- Library write failure for this press: "Could not save the profile." (alert, `--field-invalid`, the Machine block's existing write-failure notice at the "save" anchor). The existing "Could not save the profile. Nothing was changed." would be false here, because the copy and page did change (one undo step), V4 (Proposal).
- History step name: "Use machine size", V5 (Proposal).

## 5. Keyboard and focus

- Tab order: profile button, Save as profile, state row buttons, **Use machine size**, Origin corner, ... (the Work area line is no Tab stop). Enter or Space presses. No shortcut.
- After the press focus goes to the profile button, as after Reset from profile and Update profile. If the offer leaves while the button has focus (controller lost, an undo elsewhere), focus goes to the profile button too.
- The label's polite status is read once when the offer appears; the Controller block's rows stay `aria-live="off"`.

## 6. For the implementer and tester

- The comparison, range check (1 to 10,000 mm, finite) and 0.01 mm tolerance in Rust (`*-core`, next to `controller_warnings`), returning the offered size or none; TS only formats.
- Browser check: paired fake or simulator with `$130=400`, `$131=850`, copy 400 x 300: the row reads "Machine reports 400 × 850 mm"; press it twice at one screen point: Work area "400 × 850 mm", Origin corner unchanged, one History step; Ctrl+Z brings the offer back.

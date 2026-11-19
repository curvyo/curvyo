# Fix: Update profile and Reset from profile look swapped after a rename

Status: Draft
Kind: Bug (branch `fix/machine-profile-rename`)
Priority: Must
Origin: Customer, first laser test, 2026-11-18. First report, translated: "A machine cannot be renamed. I rename 'Generic Laser Copy', then 'differs from profile' appears, I press update profile twice and it is back to how it was. But if I press 'reset from profile' it gets saved... somehow confusing. Instead of red arming, green arming would probably be better. Save as profile works great." Answer to Frage 253 (2026-11-18): "C, it is already right, but the buttons seem to be swapped": the rename and the "Differs from profile" rule stay as built; the two buttons are the bug.

## Reproduction

1. Machine block, profile list: rename the user profile "Generic laser copy" to "Kitchen laser" (`0026` criterion 3).
2. The manufacturing that uses it shows "Differs from profile" (criterion 7 compares the name; stays, Frage 253 C).
3. Press **Update profile** twice (arm, then confirm): the old state is back (the library profile is called "Generic laser copy" again). The customer expected this from **Reset from profile**.
4. Press **Reset from profile**: the page takes the new name and the notice goes. The customer expected this from **Update profile** ("it gets saved").

## Expected behaviour (from `0026` criterion 7, unchanged)

- **Update profile** copies page to library: the library profile takes the copy's values, name and work area. Armed press (`0081`), no undo, not for the built-in.
- **Reset from profile** copies library to page: the copy and the output page take the library record's values and name. One undo step, asks nothing.

## Which side is wrong (finding of the PO, 2026-11-18, static reading, to be confirmed by the reproduction)

- Handlers: right. `MachineBlock.tsx` calls `api.update()` on the armed button and `api.reset()` on the other; `useWorkbenches.ts` maps them to `update_machine_profile` and `reset_machine_profile`; `machine.rs` and `MachineProfiles::update` copy copy-to-library and the reset copies library-to-copy as `0026` says.
- Armed text: right in meaning ("Update profile. Overwrites the saved profile.").
- Suspect: the signs. The glyphs are `BookmarkCheck` (reads "saved") for Update and `RotateCcw` (reads "undo") for Reset, and neither the glyph nor the tooltip says which side is copied onto which. After a rename the stale side is the page, so Update (page to library) undoes the rename while Reset (library to page) "keeps" it; the maker, who sees the rename as the new truth, reads the pair the other way round.
- The implementer first reproduces the steps in the running app (browser check) and records, per button, the value of name and work area in copy and library before and after. If any button does the other's job, the handler is the bug and is fixed; if both do as specified, the words and glyphs are the bug and Frage 260 (decided B) swaps position and glyphs.

## Reproduction result (ux-engineer, 2026-11-19, headless Chromium on the browser build)

Laser page, Save as profile ("Generic laser copy", 400 x 300 mm), rename the library row to "Kitchen laser": the page keeps "Generic laser copy" and shows "Differs from profile".

| Press | Copy (page) after | Library after | State row |
|---|---|---|---|
| Update profile, twice (600 ms apart) | Generic laser copy, 400 x 300 | Generic laser copy, 400 x 300 (rename gone) | none |
| Reset from profile (after renaming again) | Kitchen laser, 400 x 300 | Kitchen laser, 400 x 300 | none |

Both handlers do what `0026` criterion 7 says (Update copies page to library, Reset library to page); criterion 2 has no handler defect, so the fix is the swap of criterion 2a and the arming colour of criterion 3. The full table of criterion 1 (name plus value, value only, built-in) stays with the tester.

## Decisions

Frage 253 (PROFILE-RENAME): decided C by the customer (2026-11-18): keep as built. The name stays part of "Differs from profile"; renaming a library profile makes the copies that use it show Differs; Reset takes the new name, Update keeps the old one in the library. The earlier proposal that the name leaves the comparison is dropped.

Frage 254 (ARM-COLOUR): decided A by the customer (2026-11-18): only Update profile arms in the accent colour (the "green arming"); every other armed action (delete, Wipe, Forget pairing, the profile list's delete) stays red.

Frage 260 (BUTTON-WORDS): decided B by the customer (2026-11-18): swap the positions and the glyphs of the two buttons only. No new words (tooltips, accessible names and the armed text stay as built), no status line naming the fields. Frage 260 A and C are dropped.

## Acceptance criteria

1. Reproduction test (browser, then kept as a regression): after the rename of step 1 and a copy that equals the old library values, **Update profile** (armed, second press after 400 ms) leaves the library with the copy's name and values and the copy unchanged; **Reset from profile** leaves the copy with the library's name and values and the library unchanged. A table of both buttons across name only, name plus value, value only, built-in, each asserting the sides before and after.
2. Given a defect in the handler (a button doing the other's job) found by criterion 1, then it is fixed there and criterion 1's table passes; the swap of criterion 2a follows in any case (Frage 260 B).
2a. Given the "Differs from profile" row, then the two 28 x 28 buttons stand in this order, left to right: **Reset from profile** first with `BookmarkCheck`, **Update profile** second (rightmost) with `RotateCcw`. This swaps both position and glyph of `0026` ux-notes (Update `BookmarkCheck` first, Reset `RotateCcw` second); everything else on the buttons stays: names, tooltips, armed text "Update profile. Overwrites the saved profile.", hidden Update for the built-in (Reset is then the only button, at the rightmost slot), Tab order follows the visual order. A test reads both accessible names, the glyph ids and the DOM order and asserts exactly this. Reading of the glyphs (Proposal, veto at the UX review): the save mark sits on the button that takes the library's saved state onto the page, the counter-clockwise arrow on the button that winds the saved profile back to this page's values; the direction is carried by the unchanged tooltip words and the order, not by new glyphs, because B allows no new ones.
3. Given Update profile, then the first press arms it in the accent colour (`--editor-accent` filled, white glyph, passes 4.5:1) with the armed text; the second press after 400 ms overwrites; it disarms after 4 s, on Escape and on focus leaving (`0081` rules unchanged). Given any other armed action (delete a profile, Wipe, Forget pairing, delete in the list), then it is armed in `--destructive` as before; a test asserts both colours.
4. Given the comparison, then it includes the name exactly as before (`0026` criterion 7 and 12): a unit table "name only, name plus value, value only, none" shows Differs for the first three and none for the last. No change to what renaming does to copies, no file rewrite on read, `format_version` unchanged.
5. Given a profile list row, then renaming works by the means that exist (press the name, edit, Enter), is checked for 1 to 60 characters, and the new name is on disk before the next start; a browser test renames, reloads and finds the new name.
6. Quality gate green.

## Changes accepted behaviour

- `docs/design-system.md`: the armed colour of Update profile becomes the accent colour; all other armed actions stay red (Frage 254 A). Reason: customer feedback, 2026-11-18.
- `0026` criterion 7 text is unchanged (Frage 253 C).
- `specs/0026-machine-profile/ux-notes.md` (line 16, "Differs from profile" row) and `docs/design-system.md` (Machine block row): order and glyphs of the two buttons swap as in criterion 2a. Reason: customer feedback, 2026-11-18 (Frage 260 B).

## Out of scope

- Any change to the rename and Differs rule, renaming the built-in Generic laser, merging profiles, changing Save as profile (works, customer).

## Links

Requirements: R-MFG-001
Related: `specs/0026-machine-profile/` (criteria 3, 7, 12; ux-notes), `specs/0081-inline-delete-confirm/`
PR: -

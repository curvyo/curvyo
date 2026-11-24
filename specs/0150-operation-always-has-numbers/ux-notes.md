# 0150 Operation always has numbers: UX notes

ux-engineer, 2026-11-24. Covers `specification.md` (criteria 1 to 29, Draft) with the defaults of Fragen 337 to 340 (all A: one pair per profile, 10 % and 1,000 mm/min; old files derived on read; fixture jobs unchanged; the default is copied, not live). Checked against the built `frontend/src/components/workbench/MachineBlock.tsx`, `components/panel/OperationSettingsBlock.tsx`, `OperationRow.tsx`, `FixtureOutputPart.tsx`, `lib/operationSettingsRows.ts`, `lib/operationsView.ts`, `curvyo-laser-core/src/job_gate_text.rs`, `curvyo-editor-wasm/src/session/output/frame.rs`, and the UX notes of `0026` (sections 2, 7, 9), `0028` (8.3, 8.4), `0029` (6.1 to 6.4, 10) and `0118` (slice 2b). No hint or tip text anywhere (customer rule 2026-10-16): the field labels, the values and the source pick are the whole display.

## 1. What changes where

- **Machine block** (laser page's Document tab and the manufacturing's tab, `0026`): two copy fields, **Default power** and **Default speed**, directly after Max power (section 2).
- **Operation Settings block** (unfolded row, `0028` 8.3): the plain state line "No settings" and the empty fields are gone; a new operation opens with filled fields and the pick reads "Own numbers" (section 3).
- **Operation row** (`0029` 6.1): line 3 always holds numbers; line 2 of a running row always holds a time, never "–".
- **Output group checks** (`0029` 6.4, 10): "<op> has no power and speed.", "has no power." and "has no speed." are gone. Export and Send are no longer blocked by missing numbers.
- **Frame** (`0118` 2b): nothing visible changes; it already frames such a page (PR #460).
- **Fixture jobs section** (`0103`, `0029` 6.2): unchanged (Frage 339 A). Its run line b keeps "No settings" without a pick, its time keeps "–", its two checks stay. This is the one place the words "No settings" remain (W9).
- No new token, glyph, component or shortcut.

## 2. Machine block: Default power and Default speed (criteria 1 to 3, 22)

**Place (W1).** Field order becomes: Work area line, (Use machine size offer), Origin corner, Max feed, Max power, **Default power**, **Default speed**, Laser mode, Motorised Z axis, Air assist, Output line, then the Connection group. The limits and the defaults of the same quantities sit together, and Max feed and Default speed, which pull on each other (criterion 2), are both in view at once. No sub-heading (W2): the labels name the fields; a heading such as "New operations" would explain, which the hint rule forbids.

**Fields.** The built `NumberField` of Max feed, the column's width by 28 (256 x 28 at the default panel, as Max feed), label inside the left edge, unit inside the right (12px `--panel-muted-fg`), value right-aligned, tabular:

| Field | Label | Unit | Range | Accessible name | Built-in |
|---|---|---|---|---|---|
| Default power | "Default power" | "%" | 1 to 100, one decimal | "Default power (percent)" | 10 |
| Default speed | "Default speed" | "mm/min" | whole number, 1 to the copy's Max feed | "Default speed (mm/min)" | 1000 |

- The field shows the value without thousands separator, as Max feed does ("1000", "12.5"); notices use separators (0026 section 7).
- Default power is a percent of the laser's power; Max power is the controller's S-max. They do not limit each other (W3). The units "%" and none tell them apart.
- **Commit** as every copy field (`0026` section 2): Enter or blur commits, Escape reverts, empty or non-numeric reverts silently, one undo step per commit, History labels **"Edit default power"** and **"Edit default speed"**. No success notice.
- **Limits** (Action notice under the field, status, 3 s, the `0026` section 7 look, at most 70 characters): "Default power limited to 100 %." / "Default power limited to 1 %." / "Default power rounded to 12.3 %." / "Default speed limited to 6,000 mm/min." (the copy's current Max feed) / "Default speed limited to 1 mm/min." / "Default speed rounded to 1,250 mm/min.". The field shows the value that was set.
- **Max feed lowered under Default speed** (criterion 2, W4): one commit "Edit max feed" changes both fields in the same frame; the Default speed field shows the new maximum. The notice sits under **Max feed** (the field the maker just left, where focus and eyes are): "Default speed limited to 4,000 mm/min.". When Max feed itself was limited, its own notice wins (one notice at a time, as built). Ctrl+Z restores both values. Operations keep their speeds; one above the new maximum shows its existing "Speed 7000 is above this machine's 6000 mm/min." line (`0028` 8.4), as today.
- **How a change reads** (criteria 8, 22; Frage 340 A): the field shows the new number; nothing else on the page moves. No operation row, field or line changes; the next operation added on this page takes the new pair. "Differs from profile" appears or goes by the existing rule, since the pair is a copied field (criterion 3).
- **Pick profile, Reset from profile, Save as profile, Update profile** carry the pair like every other copied field; a pick or reset changes the two fields and no operation.
- **A copy or record without the pair** (made before this story, criterion 3): the fields show 10 and 1000, no notice, no "Differs" from that alone, nothing written. If that copy's Max feed is under 1000, the field shows the Max feed (the default never reads above the limit, W5), still without notice.

## 3. Numbers that are never empty (criteria 4 to 6, 23 to 27)

**New operation** (a kind button, `0101`): the row appears with line 3 filled from the page's copy: "10 %, 1,000 mm/min, 1 ×" (Engrave kinds add their default interval as built; Engrave + Outline adds line 4 "Outline 10 %, 1,000 mm/min, 1 ×"; Image adds its dither as built). Unfolded, the Settings block shows the pick with "Own numbers", no state line, and the fields Power 10, Speed 1000, Passes 1. Focus goes where `0101` puts it today. One step "Add operation".

**What replaces "No settings" (W6):** nothing. The line is removed, not reworded; an own operation's block is pick, fields, and (when it applies) the over-feed line. Exact removals:

| Where | Today | After |
|---|---|---|
| Settings block (`OperationSettingsBlock`) | "No settings" line over empty fields | gone; fields always hold numbers |
| Block summary (`blockSummary`, sr-only) | "No settings" | always "Power 10 %, speed 1000 mm/min, 1 pass" (with "from <material>", "differs" as built) |
| Row line 3 (`NumbersLines`) | "No settings" | always the numbers |
| Row line 2 time (`RowRun.time`) | "–" | always a time |
| Output group checks | "Cut 2 has no power and speed." / "has no power." / "has no speed." (block) | gone; the job is not blocked by numbers |
| Frame row | framed at stand-in numbers | unchanged look; the stand-in stays only for fixture jobs (Frage 339 A, criterion 25) |
| Fixture jobs run line | "No settings", "–" | unchanged (W9) |

- **Damaged settings** stay exactly as built (W7): the "Damaged settings" line with `TriangleAlert`, no fields' numbers on line 3, no run, and the check "2 operations have damaged settings and do not run." (a warning in the build, rank 25; see section 9). Unknown-kind rows and test-grid rows are unchanged.
- **No mark for "came from the default"** (W8, Frage 340 A): a default-filled operation is an own operation; its pick reads "Own numbers", the same as one whose maker typed those numbers. The row line, the job file and the time are identical (criterion 26).
- A typed number on an own operation replaces it ("Edit settings"); range texts as built ("Power is 1 to 100 %.", "Speed is 1 to 6000 mm/min."), and "Enter a number." for text that is no number. Clearing a field and pressing Enter, Tab or leaving it reverts it silently: a field can never be left empty (the Machine block's fields do the same).

## 4. Material pick, Own numbers, undo (criteria 9 to 14, 20)

- **Pick a material** (own operation): pick value becomes the material's name, the fields show its numbers in the same frame, line 3 follows, one step "Use material". No notice (as built).
- **Pick another material**: same, one step; Ctrl+Z gives back the previous material and its numbers.
- **Pick "Own numbers"** (removing the material): the numbers that ran stay in the fields, typed ones included; the pick reads "Own numbers"; Differs line, "Material 60" lines and both buttons go. It never falls back to the Default pair (W10): what the maker last saw running keeps running.
- **Material deleted or damaged in the library**: state (b) "Material not found, using stored numbers" as built; the defaults are not involved.
- **Ctrl+Z after "Use material"** on a default-filled operation: the fields show 10 and 1000 again and the pick "Own numbers" (the step's own numbers, never an empty state).
- **Ctrl+Z after "Add operation"** removes the row. **Redo** (Ctrl+Shift+Z, Ctrl+Y) brings it back with the numbers of that step, also when the Default pair was edited in between (W11): a redo replays, it does not re-derive.
- **Several operations selected**: only the pick shows (as built); "Own numbers" over several keeps each one's numbers.

## 5. Old files (criteria 15 to 19, 21; Frage 338 A)

- An operation saved without numbers opens as an own operation with its page's Default pair: line 3 "10 %, 1,000 mm/min, 1 ×" (or the copy's own pair), pick "Own numbers", a time on line 2. Two laser pages with different profiles show different numbers.
- **Nothing says the project changed:** no Dirty dot on the File button, no History row, no notice, and closing or opening another project raises no unsaved-changes confirm (the three-button Inline confirm of `0051` does not appear) while the maker only looked.
- **The numbers shown at open are the numbers that stay** (W12): editing Default power or Default speed afterwards does not change those rows, even before the first save, the same as for any other operation (Frage 340 A). This needs the architect's read rule (section 9).
- Restoring an older History step whose operation had no numbers shows the derived pair (criterion 21), with no extra step.
- Damaged `op_settings`, an unknown kind, or a material link without readable numbers: unchanged ("Damaged settings", the Unknown row), never filled.

## 6. Keyboard and focus

- Tab order in the Machine block: ..., Origin corner (one stop), Max feed, Max power, **Default power**, **Default speed**, Laser mode, Motorised Z axis, Air assist, Connection fields. Enter or blur commits, Escape reverts, focus stays in the field after Enter (as built); no arrow stepping (none of the block's number fields has it).
- After the criterion-2 commit the focus moves on by Tab as usual; the Default speed field is not focused or flashed (W4).
- Settings block keys unchanged; with the "No settings" line gone, the first stop after the pick is Power.
- Undo and Redo as everywhere; no new shortcut.

## 7. 244 px column, 800 x 600

- The Machine block grows by two 28px rows plus two 8px gaps (72px). The Document tab scrolls as one column today; at 800 x 600 the Machine block already scrolls, nothing is pinned, no footer changes.
- "Default speed" (13px label), "100000" and "mm/min" fit 244px with the field's 8px inner padding (about 200px measured by width estimate; the implementer measures the widest case "Default speed 100000 mm/min" and, if it does not fit, the label drops to 12px as the Connection fields do).
- The Settings block gets 16px shorter for a new operation (no state line). Operation rows keep their heights (`0029` 6.1: 56px, 70px for Engrave + Outline); line 3 already existed.
- Notices stay within 70 characters; the longest is "Default speed limited to 100,000 mm/min." (40).

## 8. Accessibility

- Notices are polite statuses (as built). The criterion-2 notice names the field it changed, so a screen-reader user learns it without moving focus.
- The fields' accessible names carry the unit (section 2). The Settings block summary always reads numbers, never "No settings".
- No colour-only signals were added; contrast as the built fields.

## 9. For the product owner and the architect

- **Read rule (architect, W12):** the derived numbers of an old file are taken from the copy's pair as it was at open; an edit of the pair before the first save must not change them (write them before applying the pair edit, or record them at open). Otherwise a row changes under the maker without a step.
- **Damaged profile copy (architect):** a new operation on a page whose copy is damaged takes the built-in pair (10 %, 1,000 mm/min) (Proposal); the page's existing block "The machine profile of this page is damaged." still stands.
- **Spec wording (PO), criterion 24:** "the check that blocks ... still reports it" does not match the build: `Cause::DamagedSettings` is a warning (rank 25, "2 operations have damaged settings and do not run."), the operation simply does not run. These notes keep the built warning; the criterion should say "the warning" or the PO decides otherwise.
- **`0029` section 10 and `0028` 8.3:** the "has no power and speed" texts and the "No settings" bullet are superseded by this story; the fixture texts stay.

## 10. Decisions for veto (Proposal)

- **W1** Default power and Default speed follow Max power, before Laser mode.
- **W2** No sub-heading over the two fields.
- **W3** Default power (percent) and Max power (S-max) do not limit each other.
- **W4** Lowering Max feed under Default speed: one step, notice under Max feed naming Default speed; no focus move, no flash.
- **W5** A pair read from an old copy never shows above that copy's Max feed, without a notice.
- **W6** "No settings" is removed, not replaced by another word or mark.
- **W7** Damaged settings unchanged (state line, warning, no run).
- **W8** No mark for a default-filled operation; it reads "Own numbers".
- **W9** Fixture jobs keep "No settings", "–" and their checks (Frage 339 A).
- **W10** "Own numbers" after a material keeps the numbers that ran; it never returns to the Default pair.
- **W11** Redo of "Add operation" brings back the step's numbers, not the current pair.
- **W12** An old file's derived numbers are fixed at open; a later edit of the pair does not change them.

## 11. Design system

`docs/design-system.md`, row "Machine block (profile picker)": field order amended with Default power and Default speed and the linked-limit notice (W1, W4). No new row: the fields and the notice reuse the existing number-field and Action-notice patterns.

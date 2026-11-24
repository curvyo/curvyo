# Operation always has numbers: every laser operation has a power and a speed, from the material or from the machine profile

Status: Ready (2026-11-24; Fragen 337 to 340 applied by default A; `adrs.md` and `ux-notes.md` W1 to W12 done)
Priority: Must
Origin: Customer (2026-11-24, Frage 336 OP-NO-EMPTY-VALUES **A**, translated: every laser operation always has power and speed; when none is set they come from the chosen material if there is one, else from default values in the machine profile; old files get these values when opened). Companion decision Frage 335 MC-FRAME-NO-POWER **B**: a Cut without power or speed frames anyway; that is built in PR #460 for `0118` and is **not** part of this story. Details marked **(Proposal)** are the product owner's and vetoable at the demo.

Revises `0101` (a new operation has no numbers, "No settings"), `0028` criteria 24 and 26 (the state "No settings"), `0111` criterion 18 (the "No settings" line) and `0029` criterion 4 (a) (the check "has no power and speed"), and extends `0026` (the Machine block). Nothing here is built.

## User value

As a maker who adds an operation I want it to have a power and a speed from the first moment, so that I never face an empty field, the job is never blocked by "has no power and speed", and I only change the numbers that are wrong for my material.

**What other tools do.** LightBurn gives every layer a speed and a power (a fixed 20 mm/s and 20 %) and never an empty one; a maker burns a wrong default by accident. xTool and Glowforge take the numbers from the chosen material and refuse to start without one. **We do better:** the numbers come from the material when there is one and from the maker's own machine profile when there is not, the default is the maker's own (low and safe in the built-in profile), and the row always says where a number comes from.

## What exists today (reading of the code, 2026-11-24)

- An operation stores `op_settings` (`curvyo-laser-core/src/operation_settings.rs`): `main` (and `outline` for Engrave + Outline) with `power_pct`, `speed_mm_min`, `passes`; an optional material link; typed overrides. Without `op_settings` the operation reads as `SettingsState::NoSettings` ("No settings"), with a broken `op_settings` as `Damaged`.
- A new operation is created with no `op_settings` (`0101`: "no cut or engrave settings in this step"; `0028` criterion 24: "An operation without either shows ..."). Only a pick of a material, or a typed number, gives it numbers.
- The job check reads that state: `NumbersRead::NoSettings` becomes the cause `NoSettings`, shown as "<op> has no power and speed." and it blocks (`0029` criterion 4 a; `curvyo-laser-core/src/job_gate.rs`, `job_gate_text.rs`). The frame (`0118`) traces such a job at stand-in numbers (PR #460).
- The machine profile (`0026`, `machine_profile.rs`, copy in the project `mfg_profile`, ADR 0039) holds the maximum feed rate and the maximum power value, but **no default power or speed**.
- A material that is gone, damaged or without a set of the needed kind leaves the stored numbers in place ("stored", `0028` criterion 26 b). Unchanged here.

## Words (shown in the UI)

- **Default numbers:** the power and the speed of the chosen machine profile that an operation takes when no material gives it numbers. Shown in the Machine block as **Default power** and **Default speed**.
- **Numbers of an operation:** power, speed and passes of the set (`0028`). This story changes power and speed only; passes already default to 1 when a set exists.

## Acceptance criteria

### The machine profile

1. Given the Machine block (`0026` criterion 1), then it has two more fields, **Default power** (percent, 1 to 100, one decimal, built-in default **10 %**) and **Default speed** (mm/min, a whole number from 1 to the profile's max feed rate, built-in default **1,000 mm/min**) (Proposal; Frage 337 A). They follow the rules of `0026` (typed outside the range: set to the nearest limit with a notice of at most 70 characters; edits change the manufacturing's copy only, one undo step per edit). Generic laser (built-in) has exactly these defaults.
2. Given a profile whose max feed rate is lowered under its Default speed, when the maker edits the max feed rate, then the Default speed is set to the new maximum in the same undo step, with the notice of criterion 1 (the default never exceeds the limit).
3. Given the manufacturing's profile copy (`mfg_profile`) and the library record, then both carry the two defaults; "Differs from profile" compares them like every other field (`0026` criterion 7). A copy or a record without them (made before this story) reads each as the built-in default of criterion 1, is shown, and is never rewritten on read (ADR 0033 §3, ADR 0039).

### A new operation

4. Given a laser page and no material for the new operation, when the maker presses a kind button (`0101` criterion 2), then the operation has numbers at once: power and speed of the profile's default numbers, passes 1 (Engrave + Outline: the outline set equal to `main`, as `StoredSettings::own` does), its source state is **own** (`0028` criterion 26 c), and its row shows "<power> %, <speed> mm/min, 1 x" and no "No settings". One undo step "Add operation" (as `0101`; the numbers are part of that step, there is no second step).
5. Given a new operation whose kind has an interval (Engrave, Engrave + Outline, Image), then the interval, the options of `0111` and the Image settings keep their defaults (`0111` criterion 20, `0123`); this story adds nothing to `op_options`.
6. Given the Image kind, then it takes power and speed from the same default numbers (`0123`'s Image settings, e.g. lowest power, are not touched).
7. Given an operation that is created already linked to a material (a route another story may add; today the pick comes after creation), then its numbers are the material's, as `0028` criterion 25 says; the profile defaults fill only a field the material's set does not give.
8. Given an existing operation row, when the maker edits the defaults in the Machine block, then operations that already have numbers keep theirs (Frage 340 A: the defaults are copied into an operation when it is created or opened, they are not a live source).

### Material

9. Given an operation with own numbers (criterion 4), when the maker picks a material that has a set of the operation's kind, then the operation links to it, the numbers become the material's, any typed overrides are cleared, in one undo step "Use material" (`0028` criterion 25, unchanged); Ctrl+Z gives back the profile default numbers the row had before.
10. Given an operation linked to a material, when the maker picks another material, then the numbers become the new material's in one undo step; Ctrl+Z restores the previous material and its numbers.
11. Given an operation linked to a material, when the maker picks "Own numbers" (the pick's no-material entry), then the link ends and the numbers that ran stay as own numbers (`0028` criterion 24), so the operation still has power and speed; it does **not** fall back to the profile defaults.
12. Given a material whose set for the kind has power and speed, then those numbers are always whole numbers within range (`0028` criterion 29), so a linked operation never has an empty value; a material without a set for the needed kind is disabled in the pick as before (`0028` criterion 25).
13. Given an operation linked to a material that is missing, damaged, newer, of another machine or without the needed set, then it keeps showing and running the stored numbers with the reason (`0028` criterion 26 b), unchanged; the profile defaults are not used here, because stored numbers exist.
14. Given a typed number on a linked operation (`0028` criterion 24), then it is an override as before; "Reset to material" removes it and the material's number shows again. On an own operation a typed number replaces the stored one (one undo step "Edit settings").

### Old files

15. Given a project file saved before this story that holds an operation without `op_settings`, when it is opened, then the operation reads as own with the profile copy's default numbers (the copy of its manufacturing; criterion 3 gives the built-in default when the copy has none), shows "<power> %, <speed> mm/min, 1 x" and runs with them. Opening adds no history step, shows no modified mark and no notice (Frage 338 A: the numbers are derived on read, and written with the first save that changes the project).
16. Given the same file, when it is saved again without any other change, then it is byte-identical to the original apart from what the format rules of `0001` always rewrite (no write-back on a plain open; ADR 0033 §3). Given the file with any change saved afterwards, then the derived numbers are stored in the operation's `op_settings`, so a later edit of the profile does not change them (criterion 8).
17. Given an old file whose operation has a damaged `op_settings` (`SettingsDamage`, "Damaged settings"), then it is **not** repaired: it keeps its state, runs nothing and keeps its own message (`0028` criterion 29's rule, ADR 0033 §3). Only a missing `op_settings` is filled. Given a readable `op_settings` of an operation, then it is untouched.
18. Given an old file whose operation of an unknown kind has no `op_settings`, then it is kept untouched and not filled (R-MFG-010).
19. Given an old file with an operation that has a material link but no readable numbers in `main`, then it is "Damaged settings" as before (criterion 17), never filled from the defaults.

### Undo and history

20. Given a new operation (criterion 4), then the numbers come into being with the operation: there is no step "add operation, then add numbers", so Ctrl+Z after "Add operation" removes the whole operation and Ctrl+Z after an "Edit settings" returns to the previous numbers, never to an empty state.
21. Given a history that already holds steps from before this story (the History list of `0043`), when a project saved earlier is opened, then no step is added by the fill of criterion 15; restoring an old step from the History list shows the operation with the numbers of that step, or, when that step's operation had no numbers, with the derived defaults of criterion 15.
22. Given an edit of the Default power or Default speed in the Machine block, then it is one undo step on the manufacturing's copy and changes no operation (criterion 8); Ctrl+Z puts the previous default back.

### The check

23. Given a running operation with Output on (any kind), then the check "has no power and speed" (`Cause::NoSettings`, "<op> has no power and speed.") no longer occurs for it and no longer blocks a job; the cause and its text are removed from the checks of `0029` (criterion 4 a) in the same story, together with the state `SettingsState::NoSettings` and the "No settings" text of `0111` criterion 18 and `0028` criterion 24.
24. Given an operation with "Damaged settings", then the check still reports it as the warning that is built ("<n> operations have damaged settings and do not run.", `Cause::DamagedSettings`, `0029` criterion 33) and the operation runs nothing; this is the only numbers-related check left for an operation. Its severity is unchanged (corrected 2026-11-24 by the architect to the built behaviour: a warning, not a block).
25. Given the frame (`0118`) of a job, then it no longer needs its stand-in numbers for an operation (PR #460 keeps them for a fixture job, Frage 339 A); a frame of a job with only operations traces the same outline as before.
26. Given the job file export (`0029`), then an operation run with default numbers writes the same G-code as one whose maker typed those numbers (no marker, no warning); the row in the Operations tab shows the source "own".

### The row

27. Given an unfolded Cut, Score, Engrave, Engrave + Outline, Cut-out or Image row, then Power and Speed fields are never empty, never show a dash and never read "No settings"; the three source states of `0028` criterion 26 are the only ones, plus "Damaged settings" (criterion 24). No hint or tip text is added to explain the default (the field values and the state line are the whole display).

### Format and tests

28. Given the whole story, then the profile copy and record are extended with the two defaults (`format_version` and the profile record version per the architect), the golden project file of `0028` criterion 28 and a new golden `operation_defaults_v<next free>.curvyo` (an operation with no `op_settings`, one with own numbers, one linked, one damaged, one of an unknown kind) pass, and an old-file test checks criterion 16 byte for byte.
29. Given the whole story, then there is no network access, no device I/O and no telemetry.

## Out of scope

- A default for passes, interval or the operation-only settings; they keep their rules (`0111`, `0123`).
- A default per operation kind or per material; one pair per profile (Frage 337, B asks for per kind).
- Fixture jobs (`0103`): they take their numbers only from the fixture's material pick and keep the checks "no material" and "no set" (Frage 339 A; B asks to cover them too).
- Repairing damaged `op_settings`, material records or profile records.
- Changing the framing without power (`0118`, Frage 335 B, PR #460).
- Showing a power or speed on the canvas, a table of defaults per material, or a "Reset to default" button (no speculative button; the numbers are typed or picked).
- A machine-specific limit check of the typed numbers beyond the existing ranges (`0029` limits stay).

## Questions for the customer

Nothing blocks the build: each question has a default.

**Frage 337 (OP-DEFAULT-VALUES): What are the default numbers, and how many?**
- A: one pair per profile, Default power 10 %, Default speed 1,000 mm/min, both editable in the Machine block, never above the profile's max feed.
- B: one pair per operation kind (Cut, Score, Engrave, Engrave + Outline, Cut-out, Image), six rows in the Machine block.
- C: no built-in numbers; the Machine block asks for them when the profile is created.
Recommendation: A (the smallest model; B can come when a maker asks). Default if unanswered: A.

**Frage 338 (OP-OLD-FILES): When are the numbers of an old file written?**
- A: derived when the file is opened (no history step, no modified mark) and written with the first save that changes the project (criteria 15, 16).
- B: written at open as one commit that the History list shows ("Filled power and speed"), the project counts as modified.
- C: written at open without a history step, the project counts as modified.
Recommendation: A (the file of a maker who only looks stays untouched, ADR 0033 §3; Frage 214 B of `0112` is the precedent of no conversion on read). Default if unanswered: A.

**Frage 339 (OP-FIXTURE-JOBS): Do fixture jobs also always have numbers?**
- A: no; a fixture job takes its numbers only from its material pick and keeps the checks "no material" and "no set" (`0103`).
- B: yes; a fixture job without a pick takes the profile defaults.
Recommendation: A (a fixture job runs a once-engraved board outline; a default burn there would be a guess on the bed's surface). Default if unanswered: A.

**Frage 340 (OP-DEFAULT-LIVE): Is the default copied or live?**
- A: copied into the operation when it is created or filled on open; a later edit of the defaults changes no existing operation (criteria 8, 22).
- B: live: an operation that never had a number typed or picked follows the profile defaults, and shows "Machine default" as a fourth state.
Recommendation: A (what a maker sees in the row is what runs; B adds a state to `0028`'s three). Default if unanswered: A.

## UX notes

(filled in by ux-engineer before Ready)

## Links

Requirements: R-MFG-001 (the default numbers live in the machine profile), R-MFG-002 (operations), R-MFG-LASER-001 (output with power and speed), R-MAT-001 (material numbers), R-MFG-010 (an unknown kind is kept untouched, criterion 18) (checked by the architect 2026-11-24: R-MFG-005 is Show and Output, not numbers, and is dropped)
Specs: `0026` machine profile, `0028` material library, `0029` job preview and output, `0101` laser operations, `0103` fixture jobs, `0111` operation tools, `0118` machine control (PR #460), `0123` raster engraving
ADRs: 0033 (§3 never rewritten on read), 0038 (profile record), 0039 (profile copy), 0054 (`op_settings`), 0056, 0058, 0064 (gate); feature notes `adrs.md`
PR: none yet

# Fixture jobs: a fixture carries its own operations, listed below the work with Output off, stored with the machine setup

Status: Done (PR #330; ADR 0052 accepted with the PR; Fragen 176 A, 177 B, 178 B built as defaults, the Output turn-off after an output is a rule for 0029)
Priority: Should
Origin: Customer (concept `specs/0063-manufacturing-workbenches/concept-output-pages.md` §6 "Fixture jobs", accepted 2026-11-05 with feedback 11: "fixtures may need cutting or engraving themselves (a slide-in board with engraved coaster outlines)"; request of 2026-10-17 in `R-MFG-007`: "a fixture job cuts or engraves them before the parts"). Slice 5b of `specs/0063-manufacturing-workbenches/`, the last slice of the epic's fixtures. The customer's example: the coaster outlines of a slide-in board are engraved once. Details marked **(Proposal)** are the product owner's and vetoable at the demo.

**Lessons of the reviews of PR #305 and PR #313 (customer, 2026-11-10) applied here:** few clear things; no invented colours (symbols, ink, the red of the cut lines and the pale orange of the hatch only); in-place rename by double-click; delete as an armed trash button; order by arrows plus drag and drop; no machine settings in a step.

## User value

As a maker with a slide-in board I want to engrave the coaster outlines of the board once, with the board's own setup, so that every later job runs only its parts and the board is never engraved by accident.

**What other tools do.** LightBurn has no fixtures and no per-fixture job; a bed engraving is a separate job the maker starts by hand, and nothing keeps it with the bed. Fusion 360 keeps a fixture as a body of its setup, but its operations are dialogs and a fixture is not switched off per job. **We do better:** the job lives with the fixture, travels with the machine setup (`0026`, Frage 175 A), starts with Output off, and is listed in the same tab as the work, so nothing in the bed runs unless the maker turned it on.

## Words (shown in the UI)

- **Fixture job:** an operation of one fixture of the laser page: a kind, a name, Show and Output, and its ticked parts. It is listed in the Fixture jobs section. It takes fixtures only (criterion 10).
- **Fixture part:** a part of a fixture that can be ticked: the board of a holder, one slot of a holder, the lines and the tapes of a grid, the outline of a clamp, the L of a stop, the head and the thread of a screw (Frage 177 B; the list is in criterion 8).
- **Part key:** the stored form of a tick: `target.<node id>` on the job, with the suffix `.lines`, `.tapes`, `.head` or `.thread` for the grid and the screw parts (ADR 0052 §2). Only a key of a part of the job's fixture counts.
- **Parts list:** the list under a selected job that shows its fixture parts with a tick each. It was called "Targets list" in the first draft of this spec; the UI says "Parts". "Target" stays the word for a ticked part in this spec.
- **Fixture jobs section:** the collapsed section below the work list of the Operations tab (criterion 1).
- **Runs first:** the label of the fixture jobs' run block under Frage 176 A.
- **Once:** the behaviour of Frage 178 B: after the page is output, a fixture job's Output goes off by itself. The rule belongs to `0029` (criterion 6); `0103` builds the Output switch only.

## Acceptance criteria

"Laser page" and "active page" as `0101` and `0102` use them. A fixture job exists only on a laser page, and only on a fixture of that page. The kinds are the five of `0101` (Cut, Score, Engrave, Engrave + Outline, Cut-out), with the same geometry rules per kind (`0101` criteria 19 to 23), applied to the parts' outlines. Proposal: the kinds are not limited for fixtures; the maker decides what to cut.

### The Fixture jobs section

1. Given the Operations tab of a laser page, then below the work list a section "Fixture jobs" is shown, above the summary. It is collapsed by default; its header reads "Fixture jobs (N)" with the count of its jobs, unknown ones included. Its expanded state is per viewer, kept for the session and not stored in the file. With no fixture job, the expanded section shows "No fixture jobs. Select a fixture to add one.".
2. Given exactly one fixture selected (on the canvas, in the Fixtures block or in the Layers row, `0102` criterion 12), when the maker presses one of the five kind buttons in the section's add toolbar, then one fixture job of that kind is added under that fixture, named "<Kind> <N>" (N the lowest number not used by a fixture job of that kind on this page), **Show on, Output off**, selected, with its Parts list open, and one undo step "Add fixture job". The five buttons are shown only while exactly one fixture is selected; with no fixture or with several they are hidden, not disabled, and no text explains it (lead decision J3, `ux-notes.md` 3.3). A selection of work items does not show them.
3. Given the section, then the jobs are grouped by their fixture: each group has the fixture's name and glyph as its heading, and the groups follow the creation order of the Fixtures block (`0102` criterion 1). The jobs of one fixture run in their order in the group. The order is changed by the up and down arrows and by drag and drop, inside the group only (`0101` criterion 5 without its sections). A job cannot move to another fixture (Proposal).
4. Given a job row, then it shows the kind's glyph and word, its name, a Show switch, an Output switch, the armed trash, and the up and down arrows. The name is renamed by a double-click, as `0101` criterion 3 says; the trash is the armed button of `0101` criterion 4 ("Press again to delete <name>", 400 ms, disarms by itself). Each change is one undo step with the fixed label names of `adrs.md` §7 (`0101` labels, not the job name): "Rename fixture job", "Delete fixture job", "Show on" or "Show off", "Output on" or "Output off", "Move fixture job". "Add fixture job" is the undo step of criterion 2. The undo names carry no job name (lead decision J9, ADR 0052 §8).
5. Given a new fixture job, then its Output is off, whatever the kind. Output is the switch that decides whether the job runs (criterion 13). The switches are stored with the job and kept on save. Absent Output reads **off** for a job, unlike `0101`'s operations, where absent reads on (ADR 0052 §1): a job whose Output is missing never runs.
6. Given the page is output (`0029`), then every fixture job whose Output was on is switched off in the same step, and its row shows "Output off". **This criterion is `0029`'s acceptance, not `0103`'s.** Nothing in this slice outputs a page: no output command, no G-code, no rule that writes this step exists yet (`adrs.md` §5). `0103` implements the Output switch of a job only (criterion 5); under Frage 178 A the switch stays as set. The contract for `0029` (one command, one commit, the page output and the switch-off together) is in `adrs.md` §5.
7. Given a fixture that is removed (`0102` criterion 4), then its fixture jobs and their part keys are removed in the same step, and nothing else changes. One undo step "Remove fixture" restores the fixture, its jobs and their keys exactly.

### Parts and ticks

8. Given a selected fixture job, when its Parts list is shown (directly below its row; only one job's list is open at a time), then it lists the fixture parts of its fixture, one row each, with a check box:
   - **Grid:** "Grid lines" (key suffix `.lines`) and "Tapes" (suffix `.tapes`; the edges switched on in `0102` criterion 15 are the parts that run).
   - **Holder:** "Board" (the holder's node id, no suffix) and one row per readable slot in creation order, named as the slot (the slot's node id, no suffix).
   - **Clamp:** "Clamp outline" (the clamp's node id). **Stop:** "Stop" (both legs as one part; the stop's node id).
   - **Screw:** "Head" (suffix `.head`) and "Thread" (suffix `.thread`).
   Frage 177 B: the maker ticks each part. A new job has no tick, so it runs nothing. The list's header shows the count of ticked parts of the readable parts, "2 of 5".
9. Given a part row, when the maker ticks or unticks it, then the job gets or loses its part key (`target.<node id>`, `true`); each is one undo step, "Assign to fixture job" or "Remove from fixture job" (the fixed labels of `adrs.md` §7). Ticks are made in the Parts list only. Nothing on the canvas and nothing in the Fixtures block ticks a part (lead decision J4); a press on a fixture on the canvas selects the fixture, not a part. "Select" on a row selects the part on the canvas, is not an undo step, and keeps the job selected.
10. Given a fixture job takes fixtures only: a work item (an own item, a group, a linked clone, a clone member, a placed page, or a member row of a placed page) is not a target. Dropping a work item onto a fixture job row is refused, the row does not take it and a notice of at most 70 characters reads "Work items do not run in fixture jobs." (Proposal). A fixture dropped on a job row is refused silently (lead decision J8). Symmetrically, no work operation (`0101`) takes a fixture part: fixture parts get no tick in the work list (`0101` criterion 10 stays). The two sets never share a key or a row: the work reader reads only `member.` keys and the fixture jobs read only `target.` keys (ADR 0052 §2).
11. Given a part, its tick is kept by the part's node id and its suffix (criterion 8). A slot that is moved, resized or renamed keeps its tick. A slot that is deleted (`0102` criterion 19) drops its keys in the same step, silently, as `0101` criterion 27 and Frage 169 A say; undo restores them. Deleting a job drops its keys with it.
12. Given a slot that cannot be read (`0102` criterion 33: a damaged slot is skipped), then it is not a row of the Parts list, and its key stays in the file, unchanged, until the slot is readable again. A key that names a node which is gone, or that is not a part of the job's fixture, is kept in the file and is not damage; it is not a target and runs nothing.

### Running

13. Given a fixture job, then it runs its ticked parts only when all of these hold: (a) its fixture is Active (`0102` criterion 5); (b) its Output is on; (c) its kind is one this build knows; (d) at least one of its parts is ticked and readable. The fixture's eye does not matter (Proposal: the eye is view only, as `0101` criterion 15). Each ticked part runs once, with the geometry of its kind (criteria 19 to 23 of `0101`, applied to the part), in the order of the Parts list.
14. Given Frage 176 A (default), then all running fixture jobs run before the work of the page: the fixture jobs run in the order of criterion 3, and inside that block, with Cuts last on, Marking before Cutting as on the work (Proposal); with Cuts last off, the list order. "Runs first" is the label of that block, and the section stays below the work list. Under Frage 176 B the block runs after the work, in list order; under C each job carries a choice "Before the work" or "After the work", default before, and the label goes. The rule is stated in the summary (criterion 17).
15. Given a fixture that is not Active, then its jobs do not run whatever their Output. Their rows show "not on the bed" (the word of `0102`, lead decision J12; priority in the row: not on the bed, then Output off, then the part count). Their Output and ticks stay as they are, and switching the fixture on again restores the run (`0102` criterion 5). Switching the fixture off is one undo step, as in `0102`.
16. Given a fixture job with Output on and the work list with its own operations, then the two are independent: the fixture job runs, the work runs, and no switch of one changes the other.

### Summary and look

17. Given the Operations tab, then the summary (`0101` criterion 17) has a fixture line when at least one fixture job runs: "1 fixture job runs." for one, "<F> fixture jobs run." for more. A job counts when its Output is on, its kind is known, it has a readable ticked part and its fixture is Active; its Show does not count (criterion 13). The count is a text button that selects the fixtures of those jobs on the canvas and in the Layers tree. The work line "Runs <N> operations, <M> items." is shown only when at least one work operation runs (lead decision J7). "Nothing runs. Assign items to an operation." shows only when neither a work operation nor a fixture job runs. The summary is updated in the same frame as any change.
18. Given View: Operations (`0101` criterion 18), then the look of a fixture part follows its state, checked in this order, the first that applies deciding: (a) Show off on its job, or the job's kind unknown: the fixture ink of `0102`, with no kind look; (b) ticked in a job with Show on and **Output off**: the Not assigned look (a dashed outline in the line colour, no hatch, no line of the kind; its symbol muted), so that a part of a job that does not run is not drawn as if it did (lead decision J5; this replaces the earlier "never dimmed" rule); (c) ticked in a job with Show on and Output on, on an Active fixture: the kind look of `0101` (red line for Cut and Cut-out, pale orange hatch for Engrave and Engrave + Outline, the symbol of the kind on the part); (d) not ticked in any shown job: the fixture ink of `0102`. A part ticked in two jobs takes the look of each job in list order, the later on top. Where a board area is engraved, the ground under the hatch is the canvas ground, not the toolbar ground, so the hatch keeps its 3:1 contrast (lead decision J6, `ux-notes.md` 6.2). Grid lines with Engrave or Engrave + Outline show only the symbol, as they have no closed area (J14). Under View: Drawn, fixture parts keep the fixture ink as `0102`.
19. Given a Cut-out fixture job, then each ticked part runs its outer contour as one closed line with **margin 0**: a fixture job has no margin field and no margin register, since a fixture job row has no settings field (customer, 2026-11-10; ADR 0052 §1). Show is view only: it changes the drawing in View: Operations, as criterion 18 (b) and (c) say, and it never changes what runs (`0101` criterion 15).

### Files, failures, budgets

20. Given a laser page with a holder that carries an Engrave + Outline job on its board and on two of its slots (Output on), a Cut job on a grid (Output off), a grid with no job, an inactive holder with one job, and one part key whose node is gone, when it is saved and reopened, then the job order, names, Show and Output states and ticks are identical, the dead key is kept and is not a target, and the golden test `fixture_jobs_v41.curvyo` (`curvyo-document-core/tests/fixtures/`) passes. Format 41 (see "Format").
21. Given a fixture job of a kind this build does not know, or a job whose kind is missing or not a text, then it is kept with its keys and listed as "Unknown fixture job" with a neutral glyph (`CircleHelp`); it has no Show, no Output, no arrows and no Parts list, runs nothing, is counted in no summary line, and can only be renamed or deleted (lead decision J10). Saved back unchanged. Golden `fixture_jobs_unknown_kind_v41.curvyo`. A kind that is missing or not a text is also one damaged value (ADR 0052 §3).
22. Given a damaged value of a job, then: a `show` that is not a boolean reads **on** and counts as one damaged value; an `output` that is not a boolean reads **off** (never on) and counts as one damaged value; a part key whose value is not `true`, or whose suffix is not a part of the fixture's kind, is kept, counts as one damaged value and is not a target. One notice of at most 70 characters counts the damaged values of the fixture ("1 value damaged on Holder 1 job."). The file is never rewritten on read. Golden `fixture_jobs_damaged_v41.curvyo` (a text `show`, a text `output`, a part key with value `false`, a `.tapes` suffix on a holder).
23. Given the machine profile's fixture set (`0102` criteria 28 to 30, ADR 0050 §1 and ADR 0052 §7), then each fixture entry holds its jobs in creation order: kind, name, Show and the part tokens, `board`, `clamp`, `stop`, `grid.lines`, `grid.tapes`, `screw.head`, `screw.thread` and `slot:<n>` (the 0-based index among the entry's slots). The entry holds **no Output**: a profile never runs a job. **Save as profile** and **Update profile** write them. **Pick profile** and **Reset from profile** copy them with the fixtures, in the same undo step as today, each job with Output off and one part key per token mapped to the new fixture's slot nodes by index. A token that names no part of the entry's fixture (a `grid.tapes` on a holder, a slot index out of range, a duplicate) makes the record damaged, as criterion 24 says: nothing is dropped and nothing is defaulted. "Differs from profile" compares the jobs in order by kind, name, Show and the set of tokens; Output is not compared (ADR 0052 §7).
24. Given the profile record (`0026`, ADR 0038 and ADR 0050): a format 1 record has no fixtures (ADR 0050, unchanged); a format 2 record has fixtures without a `jobs` key, and a `jobs` key in a format 2 entry damages the record; a format 3 record requires `jobs` in each entry, and an `output` key in any entry damages the record (the entry has no Output, criterion 23). Reads never write. Any write of a profile record writes format 3 with `"jobs"` in each entry, possibly `[]`. A damaged job makes the whole record damaged and skipped with its notice, as `0102` criterion 30 says for the set. A set holds at most 100 jobs and 200 part tokens (criterion 23); a record over these limits is damaged.
25. Given the whole slice, then there is no network access, no device I/O, no G-code, no output file and no telemetry (`R-SYS-002`, `R-SYS-007`). Fixture jobs' parts are not in any SVG export or print, as `0102` criterion 7 says. The demo shows what would run; `0029` makes the output.
26. Budgets, thread CPU time in `curvyo-editor-wasm` (tests in `curvyo-editor-wasm/tests/fixture_jobs_budget.rs`, the shape of `operations_budget.rs`, never wall time; the figures are confirmed in `adrs.md` §8): (a) one tick, that is one `assign` command and the rebuilt Operations view, on a page with 50 fixture jobs and 200 ticked parts, takes within 16 ms; (b) the summary recomputed after any change, on a page with 50 fixture jobs and 1,000 work items, takes within 16 ms, and a job command touches no drawing node; (c) the Fixture jobs section with 50 jobs shows its first rows within 100 ms.

## Changes to accepted specs

This slice changes accepted text; the old text is superseded by the criterion named here.

| Spec | Criterion | Was | Now |
|---|---|---|---|
| `0101` | 1 (the Operations tab) | the list, the Cuts last switch and the summary | plus the collapsed Fixture jobs section below the work list (1 to 6) |
| `0101` | 10 (a fixture is not a target) | a fixture is never a target of an operation | unchanged for work operations; a fixture part is a target of a fixture job only (8 to 10) |
| `0101` | 17 (summary) | "Runs <N> operations, <M> items." and its further lines | plus the fixture line "1 fixture job runs." or "<F> fixture jobs run." (17); the work line is shown only when a work operation runs; "Nothing runs." only when no fixture job runs either |
| `0101` | 18 (look) | the look applies to work items | a fixture part ticked in a running fixture job takes the look of its kind; a ticked part of an Output-off job takes the Not assigned look (18) |
| `0101` | Out of scope ("Fixture jobs and the Fixture jobs section (`0103`)") | out of scope | done in this slice |
| `0102` | 7 (no operation can be assigned to a fixture) | no operation takes a fixture | a fixture job takes fixture parts (8 to 11); work operations still take none; a fixture is never output by a work operation, and never output at all (25) |
| `0102` | 15 (tapes) | the edges switched on | the switched-on tapes are the parts "Tapes" can tick (8); no change to the switches |
| `0102` | 33 (damage counts) | the damaged values of a fixture | plus the damaged values of its fixture jobs (22) |
| `0102` | Out of scope ("Fixture jobs (`0103`)") | out of scope | done in this slice |
| `0102` | 28 to 30 (profile set) | the set holds the fixtures | the set holds their fixture jobs too, without Output (23, 24) |
| `0026` / ADR 0050 | record format 2 (`fixtures` entries) | an entry has no jobs | record format 3: each entry gains `jobs` (23, 24); format 2 reads with no jobs, and a `jobs` key in it damages the record (24) |
| ADR 0049 | §2 (fixture node), §3 (children), §7 (`format_version` 40) | a fixture has slot children | a fixture may hold job children beside its slots; format 41 (see "Format"); amended on acceptance by ADR 0052 |
| ADR 0046 | §1, §2 (operation nodes under the manufacturing; `member.` keys) | operations are children of the manufacturing node; `member.` keys | not changed for work. A fixture job is a child of its fixture node (ADR 0052 §1) and its keys are `target.` (11), not `member.` (ADR 0052 §2) |
| ADR 0052 (new) | whole | none | the job node, the part keys, format 41, the profile record format 3 (`adrs.md`) |
| `R-MFG-007` | text | "a fixture job cuts or engraves them before the parts" | unchanged; the order is Frage 176 (14); `docs/requirements.md` is updated when it is answered |
| `0063` concept | §6 "Fixture jobs" (listed below the work, run order not stated) | listed below Cutting with Output off | listed below the work; runs first or last is Frage 176 (14) |

## Out of scope

- Cut and engrave settings per job: power, speed, passes, material, the Score surface power (`0028`), and a margin for a Cut-out job (criterion 19). No settings field appears in a fixture job row (customer, 2026-11-10).
- The page output and its effects: the Output switch of a job turns off by itself after the page is output (criterion 6, Frage 178 B), the output notice "Output turned off on N fixture jobs." (`0029`). `0103` builds the Output switch of a job and nothing that outputs a page.
- Image optimisation (dithering, resolution, contrast) and Offset fill (noted as later in `0101`).
- Preview, time estimate, the gate, G-code, the order inside one cutting operation and the travel order of parts (`0029`).
- Freehand slot outlines: a slot drawn with the tools and assigned as a slot, and node-editing a holder's outline (Frage 172 B of `0102`, later). The coaster outlines of the example are typed slots in this slice (`0102` criterion 19).
- Fixture jobs on drawing, sketch or non-laser pages; a fixture job moved to another fixture or another laser page (only through the profile, `0102` Frage 175 A).
- Work items as targets of a fixture job, and fixture parts as targets of a work operation (criterion 10).
- Operations on placed pages, linked clones and clone members of fixtures (fixtures are not clone sources).
- Collision and crossing checks (the clamp count of `0102` and concept §6), automatic arrangement of parts into slots (nesting, R-MFG-004).
- Fixture jobs for other machine families (CNC table, plotter, embroidery).
- Undo of profile library changes (`0026` out of scope stands).
- A count of the fixture's damaged parts in the profile row, and a job count in the "Fixtures in profile: N" row (the row keeps counting fixtures, `ux-notes.md` J16).

## Open customer questions

Defaults apply until the customer answers; none blocks the build. The defaults are A, B and B and are the ones `adrs.md` and `ux-notes.md` are written for. Numbers 176 to 178 are the next free ones after `0102`'s 175.

**Frage 176 (JOB-ORDER): When do fixture jobs run relative to the work?**
- A: before the work of the page, as `R-MFG-007` says ("before the parts"). The section stays below the work in the list, its run block labelled "Runs first".
- B: after the work, in list order, as the position of the section in concept §6 suggests.
- C: each fixture job has a choice "Before the work" or "After the work", default before.
- Recommendation: A. A bed engraved after the parts have dropped out of the board would be the mistake the concept warns against (§5 point 6, inner shapes first).
- Default if unanswered: A.

**Frage 177 (JOB-TARGET): What does a fixture job take?**
- A: the whole fixture. A job runs all of its fixture's parts with one kind (a holder's Engrave + Outline also engraves the board outline).
- B: the maker ticks the parts one by one (the board, each slot, the grid's lines and tapes, the screw's circles), nothing is ticked by default.
- C: a holder's job takes its slots only; the board is never a target.
- Recommendation: B. The customer's example engraves the coaster outlines and leaves the board alone; a whole-fixture rule (A) would engrave the board too.
- Default if unanswered: B, with no tick, so nothing runs until the maker ticks.

**Frage 178 (JOB-ONCE): Does a fixture job run once, or with every job?**
- A: its Output stays on until the maker switches it off, so it runs with every job.
- B: after the page is output, the Output of its fixture jobs goes off by itself (criterion 6, built with `0029`), so the job runs once, as "engrave the board once" says.
- C: a "Once" switch on each job row, default off, which does the same for that job only.
- Recommendation: B. The customer's word is "once", and a bed engraved again by accident is the failure this slice exists to prevent.
- Default if unanswered: B. Until `0029` is built, B changes nothing: the switch stays as set.

## UX notes

`specs/0103-fixture-jobs/ux-notes.md` (ux-engineer, 2026-11-11) covers the section and its add toolbar, the job rows and their armed trash, the Parts list with its ticks and Select, the run block label under Frage 176, the look of the parts (the `0101` symbols, the red and the pale orange only, and the Not assigned look for an Output-off job), the summary line and the empty states. Lead decisions on it are applied here: J2 (no inner scroll area; the section grows with its rows) is rejected; J3 (hidden buttons), J5, J6, J7, J8, J9, J10, J12 and J14 are accepted as in criteria 2, 17, 18, 4, 21 and 15.

## Links

Requirements: R-MFG-007 (fixtures and the work area; a fixture job cuts or engraves them, Should), R-MFG-005 (what is output is decided by the maker; `0101` decided it for work, this slice for fixtures), R-MFG-010 (unknown kinds round-trip), R-SYS-002, R-SYS-007
Parent: `specs/0063-manufacturing-workbenches/` (slice 5b; concept `concept-output-pages.md` §6 "Fixture jobs" and the "Never output by accident" rule, sketch g in `concept-sketches.html`, §5 point 6)
Depends on: `0101` (the operation model, rows, look, summary, armed trash), `0102` (fixtures, Active, the fixture set in the profile, damage rules), `0026` and ADR 0050 (the profile record), `0081` (armed trash), `0093` (the eye)
Next: `0028` `material-test-library` (settings per operation), `0029` (preview, output and the run order in code; criterion 6 is its acceptance)
Changes: `specs/0101-laser-operations/`, `specs/0102-laser-fixtures/`, `specs/0026-machine-profile/`, `docs/adr/0049-laser-fixtures-in-the-project-file.md`, `docs/adr/0050-laser-fixture-set-in-machine-profile.md`, `docs/adr/0046-laser-operations-record.md`, `docs/requirements.md` (R-MFG-007 after Frage 176)
ADRs: [ADR 0052](../../docs/adr/0052-laser-fixture-jobs-in-the-project-file.md) (Proposed, accepted with the PR: job nodes under their fixture, the `target.` keys, format 41, profile record format 3) and `adrs.md` (architect: the commands, the run function, the view contract, the labels, the budgets, and the "output" contract for `0029`)
Format: `format_version` **41**. A `fixture_job` child is a new node shape under a fixture. Build 40 refuses a fixture with a non-slot child, so it would refuse a file with jobs as damage; 41 makes it a version refusal instead (ADR 0033 §7, the rule that bumped 37 to 40). Main is at 40 (`0102`). Migration: none. A format 40 file has no job node, opens unchanged, and nothing is written on open. Goldens in `curvyo-document-core/tests/fixtures/`: `fixture_jobs_v41.curvyo` (criterion 20), `fixture_jobs_unknown_kind_v41.curvyo` (criterion 21), `fixture_jobs_damaged_v41.curvyo` (criterion 22); the v40 and v39 goldens stay and must open unchanged.
Profile record: format **3** (ADR 0050 amended by ADR 0052 §7): `format_version` 3 and a `jobs` key in each fixture entry, always written. Limits: 100 jobs and 200 part tokens per set (`MAX_SET_JOBS`, `MAX_SET_TARGETS`), names at most 40 characters, and the record text limit rises from 64 KiB to **128 KiB** (`MAX_RECORD_BYTES`), since a maximal set in the pretty form is above 64 KiB. Goldens in `curvyo-laser-core/tests/fixtures/machine_profiles/`: `format3_fixture_jobs.json` (reads, writes back equal), `format3_max_set.json` (the maximum, reads, fits 128 KiB), `format3_damaged_token.json` (`grid.tapes` on a holder), `format3_output_key.json` (an `output` key), `format2_jobs_key.json` (a format 2 entry with `jobs`). `format2_fixtures.json` still reads; a write of it is format 3 with `"jobs": []`. `newer_format.json` moves to format 4.
PR: -

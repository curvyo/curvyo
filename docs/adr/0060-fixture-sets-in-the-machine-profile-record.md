# ADR 0060: Fixture sets in the machine profile record: named `fixture_sets`, pins and grids, role jobs, record format 5

**Status:** Proposed 2026-11-12, `needs-customer` (persistence: machine profile record format). Built on Frage 207 B (named sets of the machine; the page keeps its own copy and its own On this page choices) and the decisions listed in [ADR 0059](0059-fixture-sets-job-fixtures-and-pins-in-the-project-file.md). **Amends** [ADR 0050](0050-laser-fixture-set-in-machine-profile.md) §1 to §5 and §8 and [ADR 0052](0052-laser-fixture-jobs-in-the-project-file.md) §7 (record format, copy, Differs, goldens); notes go on both and on [ADR 0039](0039-machine-profile-copy-in-the-project-file.md) §4 and §5 on acceptance. Builds on record format 4 of [ADR 0056](0056-laser-operation-tools-in-the-project-file.md) §7 (`motorised_z`). Spec: [`0113`](../../specs/0113-fixture-sets-and-pins/specification.md) criteria 9, 10, 14, 39 to 41. Feature notes: [`0113` adrs.md](../../specs/0113-fixture-sets-and-pins/adrs.md).

## Context

The record holds one unnamed `fixtures` array (formats 2 to 4). The page now holds named sets, and the profile must carry them by name and order, never the page's On this page choices and never its job fixtures.

## Options

- **A. Replace `fixtures` by `fixture_sets`, an array of `{ name, fixtures }`, in record format 5 (chosen).** One record per profile (ADR 0038 option A); the entry shape of format 3 is reused inside each set.
- **B. Keep `fixtures` and add a `set` name per entry.** Order of sets is implicit, an empty set cannot exist. Rejected.
- **C. One file per set.** Several files per profile to keep consistent (ADR 0050 option B). Rejected.

## Decision

1. **Record format 5.** `"fixture_sets": [ { "name": "Bed", "fixtures": [ ... ] } ]` replaces `fixtures`, in the same place (after `connection`), always written (possibly `[]`). Each fixture entry is the format-3 entry (ADR 0052 §7) with:
   - kind `pin` (`params`: `diameter_mm`, `columns`, `rows`, `column_spacing_mm`, `row_spacing_mm`, `mark_mm`, no slots); a screw's `params` gain `columns`, `rows`, the two spacings and `mark_mm`; ranges as ADR 0059 §4; the matrix of a pin or a screw is a translation;
   - a job entry may hold `"role": "centre_mark" | "diameters"`; a pin or a screw holds exactly one job of each role, with the fixed tokens `pin.marks` / `screw.marks` and `pin.circle` / `screw.head` + `screw.thread`; a role on another kind, a missing or duplicate role job, or other tokens on a role job are damage. Further jobs on a pin or a screw are allowed only as the ones a format-3 screw had (kept and copied).
   - New tokens: `pin.circle`, `pin.marks`, `screw.marks`.
   - No `on_page` key and no job fixtures, ever: the writer builds the record from the page's sets only.
2. **Limits:** 16 sets, 64 fixtures in all sets together, 200 slots, 200 jobs, 400 targets, set and fixture names of 1 to 40 characters; the record text limit rises from 128 KiB to **256 KiB** (a maximal golden must read and fit). Damage anywhere in a set makes the whole record damaged (ADR 0050 §2), as do an empty set name and a key not listed here.
3. **Reading.** Format 1: no sets. Formats 2 to 4: the flat `fixtures` array reads as one set "Set 1", or as no set when the array is empty; a format-2 to -4 screw reads as a 1 x 1 grid without role jobs. In formats 1 to 4 a `fixture_sets` key is an unknown key and damage; in format 5 a `fixtures` key is. A record above 5 is skipped with its notice. Reads never write.
4. **Writing.** Always format 5. Any write (rename, a machine value, Update profile, Save as profile) upgrades a record of format 1 to 4.
5. **Copy into a page** (amends ADR 0050 §4). Pick profile and Reset from profile delete the page's set nodes and write one set node per record set, in order, `set_on_page` on (`0113` criterion 9, Proposal), with fresh node ids for sets, fixtures, slots and jobs, `op_output` = `false`, and the target keys mapped by place (ADR 0052 §7); a screw or pin read without role jobs gets them (ADR 0059 §5). Job fixtures are not touched. Generic laser has no sets: picking it removes the sets and keeps the job fixtures; the pick notice says "Fixture sets removed (undo restores them)." One commit with the existing labels, one undo step.
6. **Save as profile / Update profile** write the page's sets (names, order, fixtures, jobs with their `show`, roles and tokens) and never On this page, Output, locks or job fixtures.
7. **Differs from profile** (amends ADR 0050 §5, ADR 0052 §7): the page's sets compared in order by name and by their fixtures as ADR 0050 §5 and ADR 0052 §7 compare them, plus the new parameters and roles (lengths within `PRESET_MATCH_TOLERANCE`). On this page, Output, `fixture_locked` and job fixtures are ignored.
8. **Module.** `curvyo-laser-core/src/machine_profile_fixtures.rs` keeps the entry codec; the set layer and the format-2-to-4 read path go into `machine_profile_sets.rs` ("reads and writes the named fixture sets of a profile record") so neither module passes 500 lines.
9. **Goldens** in `curvyo-laser-core/tests/fixtures/machine_profiles/`: `format5_fixture_sets.json` (two sets, one empty; a holder with slots and a job, a 4 x 1 screw grid, a 2 x 2 pin grid with role jobs; reads, writes back byte for byte), `format5_max_sets.json` (the limits of decision 2; reads, fits 256 KiB), `format5_damaged_role.json` (a third role job), `format5_damaged_pin.json` (a pin with text in `diameter_mm`), `format5_fixtures_key.json` (a format-5 record with `fixtures`: damaged), `format4_fixture_sets_key.json` (damaged), `newer_format.json` moves to format 6. `format3_fixture_jobs.json`, `format2_fixtures.json` and `format4_motorised_z.json` read as one set "Set 1"; a write of them is format 5.

## Consequences

- A format-5 record is skipped by a build that reads up to format 4 ("newer version"), as ADR 0038 §3 intends.
- Records stay whole-record writes: two computers editing one profile's sets offline make a conflict copy, as before (ADR 0038).
- The page's On this page choices never travel through the profile; a second project on the same machine starts with every set on.
- Rejected and why: a set name per entry (B, no order, no empty set), a file per set (C).

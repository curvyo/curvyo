# ADR 0065: Laser output settings in the project file and Air assist in the machine profile: three page registers, `mfg_profile.air_assist`, format 48, profile record format 6

**Status:** Proposed 2026-11-16 (architect), `needs-customer` (document model and file format; persistence: machine profile record). Accepted with the customer's merge of the `0029` story PR, as ADR 0056 and 0060 were. Built on Frage 221 B (one fixture material pick on the page), 222 B (Air assist in the profile, off by default) and 224 B with the homing addition (Start from and Home first stored on the page), all 2026-11-16. Amends [ADR 0038](0038-machine-profile-records.md) §2 and [ADR 0039](0039-machine-profile-copy-in-the-project-file.md) §1, §5 (`air_assist`); notes go on both on acceptance. Uses the process registers of [ADR 0033](0033-manufacturing-data-in-the-project-file.md) §3 as `cuts_last` does ([ADR 0046](0046-laser-operations-record.md) §3). The job itself is derived and stored nowhere ([ADR 0064](0064-laser-job-builder-and-grbl-writer.md)). Spec: [`0029`](../../specs/0029-laser-job-preview-and-output/specification.md) criteria 2, 25, 32, 38.

## Context

The job, the toolpath, the gate and the summary are derived from the page (criterion 32), so the job needs no format. Three answers add stored values: which material the page's fixture jobs run with, how the job starts (Machine origin or Head position) and whether it homes first, and whether the machine has air assist. What must stay cheap to change: the stored shapes of the page and of the profile record (both published, both synced by ADR 0062).

## Options

**The page's three choices**

- **A. Three process registers on the manufacturing node (chosen).** `x.curvyo.laser.fixture_material`, `.start_from`, `.home_first`, like `cuts_last`. Two peers flipping two switches keep both (ADR 0009 §3); document-core stays opaque (ADR 0033 §3).
- **B. One atomic map `x.curvyo.laser.output`.** One value per page, but a concurrent pick and switch keep one whole map and drop the other. Rejected.
- **C. In the machine profile.** Start from and Home first are per job (the customer: switches on the Output tab), the fixture pick is per page (Frage 221 B). Rejected.

**The fixture pick's shape**

- **A. The material id only (chosen, Proposal).** A material not loaded here blocks the fixture jobs (ADR 0064 §3); with library sync (ADR 0062) the record follows the maker.
- **B. Id plus stored numbers, as an operation's link (ADR 0054 §1).** A second refresh path for a one-line pick. Rejected for now; it is additive later.

**Air assist:** one more boolean in `ProfileValues`, the `motorised_z` precedent (ADR 0056 §7). No other option kept.

## Decision

1. **Page registers** on the manufacturing node, written in history steps of their own (labels `set_fixture_material` "Set fixture jobs material", `set_start_from` "Set start from", `set_home_first_on` / `set_home_first_off` "Home first on" / "Home first off"; names in `curvyo-ui-core/src/history_names.rs`):
   - `x.curvyo.laser.fixture_material`: a lowercase UUID text naming a material record; absent = no pick.
   - `x.curvyo.laser.start_from`: `"machine_origin"` or `"head_position"`; absent = `machine_origin`.
   - `x.curvyo.laser.home_first`: boolean; absent = false.
   Read in `curvyo-laser-core/src/output_settings.rs`; a value of the wrong type or spelling reads as its default, counts one damaged value and is never rewritten on read. A damaged pick is no pick (it blocks running fixture jobs).
2. **Home first rule.** With `home_first` true the effective start is Machine origin whatever `start_from` holds; the stored `start_from` is not rewritten, so switching Home first off shows the earlier choice again (Proposal). The rule is one pure function, read by the Output tab and the job.
3. **Air assist** (amends ADR 0038 §2, ADR 0039 §1, §5). `ProfileValues` gains `air_assist: bool` (default false; Generic laser false).
   - **Profile record format 6:** `"air_assist": false` after `motorised_z`, always written. Format 5 records read it as false and are not rewritten on read; the key in a format 5 record is an unknown key and damages it; formats 1 to 4 stay skipped as older (ADR 0060 §3); above 6 is skipped as newer with its notice. Any write upgrades to 6. A device on an older build skips a synced format 6 record as newer and leaves it untouched (ADR 0062).
   - **Page copy:** `mfg_profile.air_assist` (boolean, absent = false, a non-boolean reads false and shows as damaged like the other fields). Pick, Reset and field edits write it with the map; "Differs from profile" compares it exactly. The job reads it from the copy, never from the library.
4. **`format_version` 48.** A build of 47 rewrites the whole `mfg_profile` map from its own type on a field edit and would drop `air_assist` silently (ADR 0056 §8 reasoning); ADR 0033 §7 refuses that by a bump. The three page registers alone would not need one (a 47 build keeps unknown process registers), they ride on it. **Migration: none.** A format-47 file opens with the defaults and is not rewritten on open.
5. **Not stored:** the job, its moves, the gate, the summary, the Toolpath view toggle, the slider position, the export folder and name. An export stores only its effects (ADR 0064 §11): fixture job Output, refreshed stored numbers, `test_grid_lasered` (existing key of ADR 0057).
6. **Goldens.** `curvyo-document-core/tests/fixtures/`: `laser_output_v48.curvyo` (a fixture pick, `start_from` `head_position` with `home_first` true, a copy with `air_assist` true, a second page with no registers); `laser_output_damaged_v48.curvyo` (pick a number, `start_from` `"middle"`, `home_first` `"yes"`, `air_assist` `"on"`: each reads its default, counts one damage and saves back unchanged); the version 47 goldens still open. `curvyo-laser-core/tests/fixtures/machine_profiles/`: `format6_air_assist.json` (reads, writes back byte for byte), `format5_air_assist_key.json` (damaged), `newer_format.json` moves to format 7, every format 5 golden still reads with `air_assist` false.

## Consequences

- Published keys: `x.curvyo.laser.fixture_material`, `x.curvyo.laser.start_from` and its two spellings, `x.curvyo.laser.home_first`, `mfg_profile.air_assist`, the record key `air_assist`. Renaming one is a format change.
- The `specs/README.md` format table gains row 48 for `0029` (the lead, with the story PR; the number is taken at merge).
- A live Home button (`0064`) needs no stored value; it reuses nothing of this ADR.

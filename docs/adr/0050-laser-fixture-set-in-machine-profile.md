# ADR 0050: Laser fixture set in the machine profile record: record format 2, copied into each page

**Status:** Proposed (architect, 2026-11-10; `needs-customer`: persistence, a record format). Drafted on Frage 175 A (the profile holds a set, each page holds a copy); answered by the customer 2026-11-10 as A. Revision 2 of the same day adds the kind `screw` and its two parameters, and the rigid-matrix and relation rules of ADR 0049 §6. The grid's origin corner is not part of the set or the copy (ADR 0049 §4). Extends [ADR 0038](0038-machine-profile-records.md) §2 and §3 (record format 2, the size limit) and [ADR 0039](0039-machine-profile-copy-in-the-project-file.md) §4 and §5 (Pick, Reset and "Differs" include the set); amendment notes go on those ADRs on acceptance. Node form: [ADR 0049](0049-laser-fixtures-in-the-project-file.md). Spec: [`0102`](../../specs/0102-laser-fixtures/specification.md) criteria 28 to 30, [`0026`](../../specs/0026-machine-profile/specification.md) criteria 5, 7 and the record. Feature notes: [`0102` adrs.md](../../specs/0102-laser-fixtures/adrs.md).

## Context

A machine profile is the maker's bed (ADR 0038). With Frage 175 A the bed also includes its fixtures, so a new project starts with the holders, grid, screws and stops the maker installed once. A project file must still show its bed without the library (ADR 0039), so the page keeps a copy of the used values, as it already does for the limits. The set is therefore a library record and a page copy, and the copy must be the page's own fixture nodes (ADR 0049), not a live link.

### Options considered

- **A. A `fixtures` array inside the profile record, format 2 (chosen).** One record per profile (ADR 0038 option A) with its set; one whole-record write per edit, as the record already has.
- **B. A second file per profile, `<uuid>.fixtures.json`.** Two files to sync and to keep consistent, and two damage paths. Rejected: ADR 0038 chose one record per profile for exactly this reason.
- **C. A live link, the page follows the profile's set (Frage 175 C).** A change to a profile would move the fixtures of every project at once, and a file opened without the library would show no bed. Rejected by ADR 0039's copy rule.

## Decision

1. **Record format 2.** `format_version` 2 adds one key after `connection`: `"fixtures": [ ... ]`, in creation order. An entry has no id, no eye and no selection:
   ```json
   { "kind": "holder", "name": "Holder 1", "active": true,
     "params": { "width_mm": 100.0, "height_mm": 60.0 },
     "matrix": [1.0, 0.0, 0.0, 1.0, 40.0, 25.0],
     "slots": [ { "name": "Slot 1", "shape": "circle", "cx_mm": 30.0, "cy_mm": 30.0,
                  "w_mm": 0.0, "h_mm": 0.0, "d_mm": 12.0 } ] }
   ```
   `kind` is one of `engraved_grid`, `holder`, `clamp`, `stop`, `screw` (no process prefix; the record is laser-only). `params` holds the kind's parameters of ADR 0049 §2 and no matrix for the grid. A screw's `params` are `thread_diameter_mm` and `head_diameter_mm`, and it has no slots. `slots` is present for a holder and empty otherwise. Limits: at most 32 fixtures and 200 slots in the whole set, a name of 40 characters; the record's text limit rises from 16 KiB to 64 KiB.
2. **Reading.** A `format_version` 1 record has no `fixtures` key and reads as the empty set. It is never rewritten on read. A format 2 record reads the set. A higher version is skipped with its notice, as ADR 0038 §3 says. In a format 1 record a `fixtures` key is an unknown key and so damage. **Damage in a format 2 set makes the whole record damaged:** it is skipped with its notice and never rewritten. This is stricter than ADR 0049's per-field defaults on purpose: a profile is a bed the maker chose, and a silently defaulted holder would be placed on every new page. An unknown kind, a set over the limits, a slot without a size, a matrix that is not finite or not rigid (ADR 0049 §6), a value outside its range (for example a screw diameter outside 0.5 to 1,000 mm, `adrs.md` §6a.6), and a screw whose head is below its thread are damage.
3. **Writing.** The writer always writes format 2 with the `fixtures` array, possibly `[]`. Any write of a record (rename, Update profile, Save as profile) upgrades a format 1 record. Reads never write.
4. **Copy into a page.**
   - **Pick profile** (`0026` criterion 5) is one commit, label `pick_profile`: it writes `mfg_profile` and the frame (ADR 0039 §4) and replaces the page's fixture nodes with one new node per entry, with fresh node ids and the stored values, in array order. Old fixture nodes and their slots are removed in the same commit. One undo step restores the old nodes with their ids (ADR 0014).
   - **Reset from profile** writes the same fixture part in one commit, label `reset_profile`.
   - Fresh ids are safe: no membership key (ADR 0046) and no other record refers to a fixture node id. Fixtures take no operations (`0101` criterion 10).
   - **Save as profile** writes the page's fixtures into a new record, in page order. **Update profile** writes them into the existing record. Both write the library, not history, and Update is armed as `0026` criterion 7 says.
5. **"Differs from profile"** is derived on every read. The page differs when its fixture list differs from the set in kind, name, `active`, `params`, `matrix` and slots, in order. Lengths compare within `PRESET_MATCH_TOLERANCE` (0.01 mm), matrix entries within 1e-6. Node ids, the eye and the selection are ignored.
6. **Generic laser** holds an empty set. Picking it on a page with fixtures removes them in one undoable commit; the notice of the pick says "Fixtures removed (undo restores them)." This is a consequence the customer should see at the demo.
7. **An old record opens.** A format 1 profile reads as an empty set, shows "Fixtures in profile: 0", and a page with fixtures installed before it reads "Differs from profile" until the first Update profile. Nothing is written on open.
8. **Goldens** (`curvyo-laser-core/tests/fixtures/machine_profiles/`, the implementer writes or regenerates them):
   - `format2_fixtures.json`: one grid, one holder with a circle slot and a rectangle slot, one stop, **one screw** (thread 6 mm, head 10 mm); reads, writes back byte for byte in the canonical form;
   - `format2_damaged_fixture.json`: one slot with a string `d_mm`; the record is damaged;
   - `format2_damaged_screw.json` (new, revision 2): a screw whose head is 4 mm and thread 6 mm; the record is damaged;
   - `format2_unknown_kind.json`: kind `"turntable"`; the record is damaged;
   - `format2_over_limit.json`: 33 fixtures; the record is damaged;
   - `format1_full_record.json`: the existing `full_record.json`, unchanged, reads as the empty set and is not rewritten;
   - `newer_format.json`: changed from format 2 to format 3, since format 2 is now current;
   - `damaged_unknown_key.json`: check that its unknown key is not `fixtures`; if it is, change the key.
   - `curvyo-document-core`'s `machine_profile_v36.curvyo` is unchanged: the page copy holds no set.

## Consequences

- Records are larger and are still whole-record writes: two computers editing the same profile's set at once produce a conflict copy, as they already do for its limits (ADR 0038 consequences).
- The copy into a page is the existing undo step plus new nodes; a 32-fixture set costs one tree delete and one tree insert per fixture in one commit.
- A future format 3 record (for example a fixture kind added later) is skipped by a format 2 build with its notice. That is the intended behaviour of ADR 0038 §3.
- Rejected: a second file (B), and a live link (C). Both are above.

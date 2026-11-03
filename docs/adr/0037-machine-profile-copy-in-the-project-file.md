# ADR 0037: Machine profile copy in the project file

**Status:** Proposed, `needs-customer` (persistence and file format; Frage 122 PROFILE-HOME and
Frage 123 PROFILE-CONN, written for answer A on both). Amends [ADR 0033](0033-manufacturing-data-in-the-project-file.md)
§1 (`mfg_machine_name` is replaced) and §7 (a new `format_version`). Uses the records of
[ADR 0036](0036-machine-profile-records.md). Spec: `specs/0026-machine-profile` criteria 5 to 8,
12, 15.

## Context

ADR 0033 §1 gave the manufacturing a placeholder register `mfg_machine_name` and said it would be
replaced by "a profile reference and snapshot in `0026`". The file must show the bed and the
limits it was made for on a computer that has none of the maker's profiles, so it carries a copy
of the used values plus the id of the library record they came from (ADR 0036 §5). The work area
is already the owned page's `page.<id>.frame` (ADR 0031, format 34) and stays the only number for
it.

### Options considered

- **A. One atomic register `mfg_profile`, a map holding the id and the copied values (chosen).**
  A pick, a reset and an edit write it whole, so a merge never pairs one peer's id with another
  peer's values, and the reader validates one key.
- **B. `mfg_profile_id` plus an `mfg_profile` map (the spec's Format line).** Two registers
  written in one commit can still interleave under concurrent picks (last writer wins per key),
  leaving an id beside foreign values. Harmless (it shows "Differs"), but it is a second key to
  validate and migrate for no gain; nothing reads the id without the values.
- **C. One register per field under `x.curvyo.laser.machine.*`** (ADR 0033 §3). Loses because a
  profile is one choice: two peers picking two profiles at once would end with a mix of both,
  and Reset would be seven writes to keep consistent.

## Decision

1. **The register.** On the manufacturing node, owned by `curvyo-document-core` as a generic
   register next to `mfg_process` and `mfg_page`; its content is opaque there and typed by the
   process's extension, exactly as `fixture_params` (ADR 0033 §2, §3):
   ```text
   mfg_profile = { id: "6f1c...e5f" | "generic-laser",
                   name: "K40 in the garage",
                   origin: "bottom_left",
                   max_feed_mm_per_min: 6000,
                   max_power: 1000,
                   laser_mode: true }
   ```
   Numbers are written as Loro `I64`; the reader also accepts an integral `F64`. The work area is
   not in it.
2. **Not in it, ever:** the connection fields and anything else from a library record that the
   copy does not list (criteria 6, 15). The writer builds the map from the copy type, which has
   no connection field, so there is no code path that could add one.
3. **Reading** (`curvyo-laser-core::machine_profile_copy`, "reads and writes the machine profile
   copy of a manufacturing", over `StoredParams`): every field is read on its own; a missing or
   damaged field (wrong type, non-finite, out of range, name empty or over 60 characters, unknown
   origin) reads as that field's default from criterion 1 (name: "Generic laser") and is shown;
   nothing is rewritten on read (ADR 0033 §3). A missing or non-string `id` reads as "no
   library record", which shows "Not in your profiles". Document-core refuses the file as
   damaged only when `mfg_profile` is present and not a map of plain values.
4. **Writing.** Pick (criterion 5) is one commit that writes `mfg_profile` and the owned page's
   frame through the existing resize of `0093` criterion 3 (content keeps its place relative to
   the page's top-left corner); a pick whose result equals the stored map and frame writes
   nothing. A field edit and Reset from profile are one commit each that write the whole map.
   Every one of these commits also deletes `mfg_machine_name` if present. Update profile and
   Save as profile write the library (ADR 0036), not history; Save as profile additionally sets
   the map's `id` in one commit (criterion 3: the manufacturing now points at it).
5. **"Differs from profile"** is derived on every read, never stored: the copy differs when the
   library record with its id exists and any copied field differs or the page size differs from
   the record's work area by more than `PRESET_MATCH_TOLERANCE` (0.01 mm). Integers, strings and
   the corner compare exactly. So criterion 12's "the Differs state is identical after reopening"
   holds whenever the library is unchanged.
6. **Migration from format 35** (a file with `mfg_machine_name`, no `mfg_profile`): it opens and
   reads as `id = "generic-laser"` with the built-in's values, name taken from
   `mfg_machine_name` when that is a valid name. Opening writes nothing; the first commit of
   decision 4 writes `mfg_profile` and removes the old key. The validator keeps accepting
   `mfg_machine_name` (string or absent). New manufacturings never write it.
7. **`format_version`: next free number at merge** (36 today; main is at 35). Reason: a
   version-35 build would ignore `mfg_profile`, show the manufacturing without its machine, and
   Add manufacturing would write the old shape into the same (possibly shared, ADR 0031)
   document, so it must refuse the file. Damage: decision 3's one case. Goldens in
   `curvyo-document-core/tests/fixtures/`: `machine_profile_v<N>.curvyo` (a user profile id, an
   edited copy, a 600 x 400 mm output page; criterion 12) and `machine_profile_damaged_v<N>.curvyo`
   (a NaN feed, a string power value, a 61-character name: opens, reads the defaults, saves the
   registers back unchanged), `mfg_profile_not_a_map_v<N>.curvyo` (refused as damaged).
   `laser_workbench_v35.curvyo` and `manufacturing_unknown_process_v35.curvyo` stay and must
   open with decision 6's reading and no write.
8. **API.** `ManufacturingSnapshot::machine_name` becomes `profile: Option<FixtureParams>`
   (`None`: the format-35 case); `NewManufacturing` takes the initial map instead of a name; one
   new command `set_manufacturing_profile(id, map, work_area)`. `curvyo-extension-core`'s
   `Manifest::machine_name` and `HostKey::MachineName` are removed (the extension interface is
   `0.x`, ADR 0005 §3; noted in its changelog); the laser extension supplies the built-in.

## Consequences

- An unknown process keeps its `mfg_profile` untouched like every other register (ADR 0033 §4).
- Two peers editing two different fields of the same copy at the same time: the later whole map
  wins and the other edit is lost, the trade ADR 0033 §2 already made for `fixture_params`.
- Adding a copied field later (a value `0027`/`0028` needs) is additive inside the map and needs
  no bump if old readers may ignore it; changing a field's meaning is a bump.
- `document.json` (ADR 0033 §6) lists the map as is.

### If the customer answers B

- **Frage 122 B** (library only): the map shrinks to `{ id, name }`; decisions 3 and 5 drop the
  limits, Update and Reset disappear, and on a computer without the record the limits are
  unknown, so `0029` must refuse to produce a job (fail closed). The bump and migration stay.
- **Frage 122 C** (project only): the map drops `id`; nothing is derived; decisions 5 and the
  "Not in your profiles" state disappear.
- **Frage 123 B** (connection travels): the map gains `connection: { connector_address,
  controller_port, baud_rate }`; decision 2 is withdrawn; a "Not in your profiles" copy gets
  editable connection fields; criterion 15's "no network address in a shared file" no longer
  holds. Same bump if decided before merge.

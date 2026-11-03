# ADRs for "Machine profile"

Architect, 2026-11-03, against `story/0026-machine-profile` on top of `main` at `903ba362`
(format 35). Two new ADRs, **no new crate, no new external dependency** (`serde` joins
`curvyo-laser-core`, already a workspace dependency in `Cargo.lock`), one new file shape we
write (the machine profile record) and one project `format_version` bump. Both ADRs are
`needs-customer` and written for Frage 122 A and Frage 123 A; each says what changes on B.

## Depends on or extends

- [ADR 0036](../../docs/adr/0036-machine-profile-records.md) (new): the per-user library is one
  JSON file per profile, `machines/<uuid>.json` in the data directory (ADR 0004 §6), one
  `localStorage` entry per profile in the browser; model and codec in `curvyo-laser-core`
  (`machine_profile`, `machine_profile_file`, `machine_profiles`); the host only moves text.
- [ADR 0037](../../docs/adr/0037-machine-profile-copy-in-the-project-file.md) (new, amends
  ADR 0033 §1 and §7): one atomic `mfg_profile` map (id plus copied values, never the
  connection); per-field defaults on damage; "Differs" derived; format-35 files read as Generic
  laser; next `format_version` (36 today).
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §6, §7, §9: per-record
  files with UUID, `format_version`, `modified`; data directory, atomic writes, no lock files;
  a newer record is never partly read.
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) §2 to §4: the
  `fixture_params` pattern (atomic opaque map, typed by the extension, never rewritten on read),
  unknown processes kept untouched.
- [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md): laser data belongs to
  `curvyo-laser-core`; `ui-core/src/extensions.rs` stays the only dispatch, no trait.
- [ADR 0031](../../docs/adr/0031-sync-engine-relay-protocol-and-transport.md) decision 9a: the
  work area is the owned page's `frame`; a pick resizes it through `0093` criterion 3.
- [`0045` adrs.md](../0045-document-formats-library/adrs.md) and
  [`0075` adrs.md](../0075-font-library/adrs.md): the host-moves-text pattern (`curvyo-app`
  commands over `curvyo-storage-io`, `localStorage` in `frontend/src/platform/host.ts`, parsing
  in wasm). Their single-TOML-file shape is deliberately not copied (ADR 0036 option B).

## Spec issues (for the product owner before Ready)

1. **Criteria 4, 10, 13 say "one file" / "the library file".** ADR 0036 uses one file per
   profile (ADR 0004 §6) and keeps the last pick in the config directory. Reword: "the
   profiles are kept per user ... in the data directory"; criterion 10 "remembered on this
   computer"; criterion 13 becomes per record: a damaged record is skipped and untouched, a
   record with a newer format is skipped and never overwritten, and the rest of the library
   stays writable (no whole-library read-only state).
2. **Criterion 13 misses a file that is not JSON at all.** Same rule as a damaged record
   (skipped, counted in the one notice). No set-aside is needed: nothing ever rewrites another
   record's file.
3. **Criterion 9 has no range for Baud rate and no length for Controller port.** Proposal:
   baud rate an integer 1 to 4,000,000; port up to 200 characters, like the address.
4. **Criterion 1: feed and power are integers** (ADR 0036 §2); a typed fraction is rounded with
   the range notice. Proposal.
5. **Criterion 12: "Differs" is derived, not stored** (ADR 0037 §5). The golden file test in
   `curvyo-document-core` checks id, copy and work area; the "Differs" part is a session test
   in `curvyo-editor-wasm` with a fixture record in the library.
6. **Format line: `mfg_profile_id` + `mfg_profile`** becomes one register, the id inside the map
   (ADR 0037 option B lost). Behaviour is unchanged.
7. **Criterion 12, damaged name:** the default is "Generic laser" (ADR 0037 §3). A long name
   reading as "Generic laser" may surprise; truncation would be the alternative. Proposal kept
   because it matches ADR 0033 §3 (default, never repaired).
8. **Criterion 3, Save as profile then Undo:** undo restores the old `id` in the copy; the new
   library record stays (library has no undo, Out of scope). Worth one sentence in the spec.

## Feature-local notes (2026-11-03)

1. **Budgets of criterion 14 confirmed** (200 records are about 70 KiB of JSON); one test under
   `curvyo-editor-wasm/tests/cpu_time/` loads 200 fixture records and builds the list view.
2. **Origin marker (criterion 11)** is a view item of the Laser frame in
   `curvyo-editor-wasm/src/session/manufacturing/`, drawn from the copy's `origin`; no document
   write, no geometry change.
3. **No machine abstraction:** `kind = "laser"` is a stored value only; no `MachineKind` enum
   with one variant, no trait (`CLAUDE.md` §5, §6).
4. **The Machine block is a host-drawn block kind, not new parameter kinds** (ux-notes §10
   "Schema"). ADR 0034 is not amended: its §4 ("only the kinds a story uses exist, added by the
   story that needs them") and its Consequences ("a new block kind in the application") already
   cover this; the contract is `0.x` (§5), so the change goes into
   `curvyo-extension-core/CHANGELOG.md`.
   - **Chosen: one new variant `Block::MachineProfile { label }`** in
     `curvyo-extension-core/src/manifest.rs`, drawn by the shell like `FixtureList`. The laser
     manifest replaces its `Params { label: "Machine" }` block with it; the extension only says
     where the block sits. Its content (copy, library rows, state, connection, field ranges)
     comes from the session's manufacturing view, built from `curvyo-laser-core`'s
     `machine_profile*` modules through the dispatch in `ui-core/src/extensions.rs` (ADR 0034
     §3, one more arm, no trait). Width and Height move into it (they are part of the copy's
     compare and of Save as profile); their range comes from the profile model, not a `Param`.
   - **Rejected: `ParamKind::Text`, `Count`, `Switch`, `Choice` plus a picker block.** Four
     kinds and a block for one user; the fields are not `x.<process>.*` registers but the
     `mfg_profile` map (ADR 0037) and, for Connection, the library record with no undo
     (ADR 0036), so a `Params` block's "store in registers, one undo step" meaning would break
     or need a host-key per field. Layout needs (side-by-side work area, 2 x 2 corner, unit
     suffix, sub-group) would have become schema options with one user (`CLAUDE.md` §5).
     Generic kinds are added when an extension's own registers need them (`0027`, `0028`).
   - **Deleted with it** (no user left, `CLAUDE.md` §5 dead code): `HostKey` (all three keys),
     `ParamKind::Display`, `ParamValue::Text`, the `host.*` values of the view JSON, and
     `host_param` in `curvyo-editor-wasm/src/session/manufacturing/commands.rs`. The 0093
     acceptance tests that read `values["host.*"]` move to the Machine block's view keys.

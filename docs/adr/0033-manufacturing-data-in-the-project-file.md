# ADR 0033: Manufacturing data in the project file

**Status:** Accepted (customer, 2026-11-02: Frage 118 MFG-MODEL A); §1 `mfg_machine_name` and §7 amended by [ADR 0039](0039-machine-profile-copy-in-the-project-file.md) (proposed 2026-11-03). `needs-customer` (persistence: what a `.curvyo` file holds and
how it survives builds without an extension). Follows [ADR 0032](0032-workbench-model-and-registry.md);
extends ADR 0004 §1 and §9, ADR 0009 (merge per register), ADR 0014 (history). Specs: `0063`
criterion 9, `0093` criteria 12 and 13.
Note 2026-11-05: §1 and §5 amended by [ADR 0042](0042-page-kinds.md) (proposed): the manufacturing is created and deleted with its output page; its `name` is no longer written.

## Context

A manufacturing (ADR 0032 §4) stores generic data (process, name, output page, placements) and
process data that only its extension understands (laser settings, fixture parameters). The file
must keep process data a build does not understand: a project with a CNC record must survive a
build that only knows lasers, and an extension switched off (R-EXT-002) must not cost the maker
data. Old-file compatibility is waived for this development build (customer, 2026-10-29), so the
reader refuses any file with a higher `format_version`; the round-trip rule is about a build of the
same format that lacks or disables the extension.

### Options considered

- **A. Inside `document.loro`, as registers on the manufacturing's nodes (chosen).** One source of
  truth, undo and merge per value, the container stays as ADR 0004 §1 defines it.
- **B. A container member per manufacturing (`manufacturing/<id>.json`).** Easy to skip when
  unknown, but outside history and merge, and a second truth for ids (ADR 0032 option B).
- **C. One opaque blob register per manufacturing.** Round-trips trivially, but two peers editing
  two different settings overwrite each other (ADR 0024 rejected the same shape for pages).

## Decision

1. **Generic registers on the manufacturing node** (owned by `curvyo-document-core`):
   `manufacturing` (marker, `true`), `name` (existing register), `mfg_process` (string: the
   process id, equal to the extension's workbench id, e.g. `curvyo.laser`), `mfg_page` (the owned
   page's id; the page carries `page.<id>.owner` back, ADR 0032 §5; the page wins on disagreement),
   `mfg_machine_name` (placeholder setup name, "Generic laser" in `0093`; replaced by a profile
   reference and snapshot in `0026`). The work area is the owned page's size registers, never a copy.
2. **Placement and fixture nodes** (ADR 0032 §6, §8): `placement_link = { page, matrix }` (one
   atomic value); `fixture_kind` (string `<process id>/<kind>`, e.g. `curvyo.laser/engraved_grid`)
   and `fixture_params` (one atomic map of the kind's parameter values, written whole so a merge
   never pairs one peer's spacing with another peer's origin). Hidden uses `visible`.
3. **Process data: one register per declared parameter**, key `x.<process id>.<param key>`
   (for the laser `x.curvyo.laser.<key>`), value the parameter's typed value as the schema of
   ADR 0034 §4 encodes it (a length in mm as a number tagged with its unit). Document-core treats
   them as opaque values: it validates only that they are plain Loro values. The extension
   validates on read; an invalid value reads as the parameter's default and is shown, never
   rewritten on read.
4. **Unknown or disabled process: keep everything, write nothing.** A build that does not know or
   has disabled `mfg_process` lists the manufacturing disabled with its name, does not open its
   workbench, and never writes, repairs or drops its registers or children (the orphan repair of
   ADR 0015 §5 and every other repair skip manufacturing subtrees). Placements still resolve
   (they are generic); fixtures of an unknown kind draw nothing. Deleting the whole manufacturing
   stays allowed (one undoable step). Save writes the Loro snapshot as is, so the data round-trips
   byte-for-byte in meaning.
5. **History and merge.** Every write above is an ordinary register write: Add manufacturing
   (node, owned page, first placement) is one history step; deleting it removes node and owned
   page in one step and undo restores both (`0093` criteria 2, 13). Concurrent edits merge per
   register (ADR 0009 §3). Two peers adding a manufacturing at once get two.
6. **`document.json`** (ADR 0004 §1, non-authoritative export) includes manufacturings with their
   generic fields and the raw `x.*` values.
7. **`format_version`: next free number at merge** (31 today on this branch; other branches may
   take 32 first). Reason: a build of the previous version would read a placement or fixture as an
   unknown primitive and could draw or output it, so it must refuse the file. Migration: none (no
   older record changes shape; a file without manufacturings opens unchanged). Damage: a
   `placement_link` that is not `{ id string, six finite numbers within ±1e7 }`, a
   `page.<id>.owner` naming a node that is not a manufacturing, a `fixture_params` that is not a
   map. Goldens in `curvyo-document-core/tests/fixtures/`: `laser_workbench_v<N>.curvyo` (two
   fixtures, one hidden, a 600 x 400 mm page, one placement) and
   `manufacturing_unknown_process_v<N>.curvyo` (opens, shows disabled, saves back with equal
   content).
8. **No container member is added.** `jobs/` (ADR 0004 §1) stays reserved for generated job
   snapshots (`0029`), not for the manufacturing record.

## Consequences

- An extension can add settings without a `format_version` bump: a new `x.*` key is additive and
  ignored by builds that do not know it. Changing the meaning or encoding of an existing key is a
  bump and needs an extension-side migration (`CHANGELOG` of the extension interface, ADR 0005 §3).
- The `x.*` namespace makes process data a stable on-disk surface the moment a file is shared;
  ADR 0034 §5 therefore treats parameter keys as part of the extension's published contract.
- Every tree consumer must honour the "skip manufacturing subtrees" rule (ADR 0032 consequences);
  a missed one would draw a fixture in the Vector canvas or repair an unknown process away. The
  unknown-process golden guards the second.

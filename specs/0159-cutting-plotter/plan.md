# Plan for the cutting plotter

## Groundwork (ADR 0095 §11 steps G1 to G3, done once for `0159`, `0160` and `0161`)

Invisible commits on `story/machine-families`; no criterion of slice 1 is met by them alone, they
are what M1 to M3 build on. The shared path order (`adrs.md` PL4) is a feature-local decision of
slice 1, not groundwork, and comes with M1.

### Affected crates/modules

- new: `curvyo-cutter-core`, `curvyo-embroidery-core`, `curvyo-cnc-core` (AGPL-3.0-or-later),
  `curvyo-inkstitch-core` (GPL-3.0-or-later, `LICENSE`, `NOTICE.md`, SPDX headers, no
  dependencies yet); workspace `Cargo.toml`, `deny.toml`.
- `curvyo-ui-core`: `extensions.rs` (registry), `history_names.rs`, `step_scope.rs`.
- `curvyo-editor-wasm`: `session/manufacturing/commands.rs` (Add page per family).
- `curvyo-library-core`: new `record_envelope`, `record_merge`, `record_kind`, `connection`.
- `curvyo-laser-core`: `record_envelope` and `library_merge` leave; `machine_profile_file` and
  `material_record_file` sort by `kind` first; `Connection` and `BaudRate` re-exported.

### Tasks

- [x] G1. Four crate skeletons with `#![forbid(unsafe_code)]`, each family with `manifest.rs`
  (`PROCESS_ID` `curvyo.cutter`, `curvyo.embroidery`, `curvyo.cnc`; words "Plotter",
  "Embroidery", "CNC" per the adrs.md files) and a compiled-in `Extension` in `EXTENSIONS`;
  `deny.toml` exceptions; wasm32 build of each (ADR 0095 §2, §7, §10; prepares AC 1).
- [x] G2. Built-in profiles in code per family (Cricut Maker 3 and Generic HPGL cutter; W6 EU7;
  Generic GRBL router) with id, name and page size; Add page of a family process takes its
  default built-in's size, writes no laser copy **(Proposal: the family copy is M1 to M3's)**, and
  commits as "Add plotter page", "Add embroidery page", "Add CNC page" (ADR 0095 §1, §11;
  prepares AC 1, 2).
- [x] G3. Record envelope, merge (taking read outcomes), `kind` dispatch and the `connection`
  value move into `curvyo-library-core`; the laser readers skip another family's record silently
  and an unknown kind as newer; laser goldens byte-identical (ADR 0095 §3, §4; prepares AC 2, 18).
  The family record readers and writers (`profile_file`, format 9) come with M1 to M3.

### Validation

- `git diff --exit-code -- curvyo-laser-core/tests/fixtures` empty after G3; the laser merge tests
  pass unchanged.
- `cargo build --target wasm32-unknown-unknown -p` each new crate; `cargo deny check licenses`.
- Full `CLAUDE.md` §7 gate once at the end.

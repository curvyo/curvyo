# ADR 0034: Extension contract, compiled-in route

**Status:** Accepted (customer, 2026-11-02: Frage 119 EXT-LASER-UI A). `needs-customer` (plugin model). Amends
[ADR 0005](0005-extension-and-plugin-model.md) §9 for compiled-in extensions (a registry before
the wasm host story); ADR 0005 §1 to §8 and §2 (wasm host in the MVP) stay as they are. Follow-up
of [ADR 0006](0006-license.md) for extensions. Adds crates to
[ADR 0011](0011-workspace-and-crate-layout.md) §2 and §3. Specs: `0065`, `0093` criterion 11.

## Context

The customer decided on 2026-10-17 (spec `0065`, EXT-ROUTE): extensions may be compiled in as
dependencies, no dynamic loading is required, the front end gets extension points (a registry), and
every extension is AGPL-3.0-or-later (EXT-LICENCE). On 2026-11-02 he named the first extension: the
laser workbench. ADR 0005 §9 forbade a registry before the host story; that rule was written for
the wasm route and is now overtaken by the customer's route, so this ADR amends it for compiled-in
crates only.

What `CLAUDE.md` §5 allows today: one extension (laser) plus the built-in Vector and Sketch
workbenches. That is enough for a **data** contract (a manifest the front end renders, with three
workbench users) but not for a Rust **trait** over extensions (one implementation).

### Options considered

- **A. Extension = an ordinary workspace crate with plain functions and a data manifest; one
  registration module in `curvyo-ui-core` (chosen).** No ABI, no runtime, testable as any crate,
  builds for wasm32 so the browser gets the same extensions.
- **B. An `Extension` trait object list from day one.** One implementation; a trait whose shape is
  guessed from the laser alone. Rejected (`CLAUDE.md` §5); extracted with the second extension.
- **C. Wasm components now (ADR 0005 §1).** The interface would have to be designed and frozen
  around one user; large dependency. Not rejected, deferred: ADR 0005 §2 stays, and the data
  contract below is what a WIT interface will mirror.
- **D. Manifest as a TOML file parsed at start.** Needed when code and manifest arrive separately
  (wasm); for a compiled-in crate a typed Rust value is simpler and checked by the compiler.
  Deferred to the wasm route.

## Decision

1. **Crates.** An extension is one crate `curvyo-<name>-core` (pure logic, `#![forbid(unsafe_code)]`,
   builds for `wasm32-unknown-unknown`, in CI's wasm list); device I/O, if any, in
   `curvyo-<name>-io`. Flat at the repository root (ADR 0011 §1). The first is
   **`curvyo-laser-core`**: the laser workbench contribution, fixture outlines (Engraved grid), and
   later roles, material tests, job generation to GRBL G-code (`0027` to `0029`) and the simulation
   timeline. Shared contract types live in a new crate **`curvyo-extension-core`**: manifest,
   contribution, parameter schema and typed parameter values. It is needed because extensions may
   not depend on `ui-core` (which dispatches into them) and the contract is not the document model.
2. **Dependency direction** (adds to ADR 0011 §3):
   ```text
   curvyo-extension-core → document-core (units only)
   curvyo-laser-core     → extension-core, document-core, geometry-core
   curvyo-ui-core        → extension-core, laser-core   (in addition to its edges)
   curvyo-editor-wasm    → extension-core               (manifest JSON to the front end)
   ```
   An extension crate never depends on `ui-core`, `render-core`, `editor-wasm`, any `-io` crate,
   `-app` or the front end; it receives document data as arguments and returns values or a list of
   edits that the host applies through the command journal (ADR 0005 §5). It never changes the
   document model or the format: its data lives in the `x.<process id>.*` registers and the fixture
   registers of ADR 0033.
3. **Registration: one module, `curvyo-ui-core/src/extensions.rs`.** It holds the compiled-in list
   (`[curvyo_laser_core::manifest()]`) and the dispatch by extension id (`match id`, one arm per
   extension) for the few calls the contract has: `manifest()`, `fixture_outlines(kind, params,
   work_area) -> Vec<Path>`, and the validation of parameter values against the schema. Adding an
   extension is a Cargo dependency plus one list entry and one arm, nothing else in the shell. **No
   trait now.** Trigger for the `Extension` trait: the second extension crate.
4. **Manifest and parameter schema (Rust values, `curvyo-extension-core`).**
   - `Manifest { id, name, version, licence, contributions }`: `id` is a dotted string
     (`curvyo.laser`; third parties use `<author>.<name>`), `version` the crate version,
     `licence` the SPDX id.
   - `Contribution::Workbench { id, label, icon, tabs, toolbox_cards, fixture_kinds }`: what
     `0093` uses. A tab is a list of blocks; the shell's block kinds are `Params` (a schema),
     `FixtureList` (the installed fixtures with add, hide, remove) and `Notice` (a short text the
     extension computes, e.g. the overhang of `0093` criterion 8). A toolbox card is a list of
     command buttons (id, label, icon). The front end's `ManifestWorkbench` draws them with the
     design system; the extension ships no TypeScript (spec `0065` EXT-UI, decided A as Frage 119 EXT-LASER-UI).
   - `Param { key, label, tooltip, kind, default }` with `kind` one of `Length { min, max }`,
     `Count { min, max }`, `Switch`, `Choice(options)`. Values are `ParamValue` with the unit
     newtypes of `document-core` (never bare `f64`, `CLAUDE.md` §5).
   - **Only the kinds a story uses exist.** Speed, power, percent, tables, canvas tools and the
     vectorizer contribution are added by the story that needs them (`0027` to `0029`, `0025`).
5. **Stability.** The contract is `0.x`, unstable, with a `CHANGELOG.md` in
   `curvyo-extension-core` (ADR 0005 §3 applies unchanged). An extension's parameter keys and
   fixture kinds are written into files (ADR 0033 §3), so renaming one is a format change with a
   migration in the extension.
6. **Enable and disable (R-EXT-002).** The list of disabled extension ids is a per-user app
   setting in a small text file `extensions.toml` (`disabled = ["curvyo.laser"]`), moved by the
   host as `document-formats.toml` is (Tauri command on the desktop, `localStorage` in the browser)
   and parsed in `ui-core`. Default: everything enabled (permit). The Properties panel lists every
   compiled-in extension with name, version and an enable switch, marked "trusted code, compiled
   in" (spec `0065` criterion 8). Disabled: its contributions disappear; its manufacturings stay in
   files and show disabled (ADR 0033 §4). No restart.
7. **Licence (follow-up of ADR 0006).** A compiled-in extension is linked into the program and is
   part of the AGPL-3.0-or-later work: its crate uses `license.workspace = true`, `cargo deny`'s
   allow-list applies to its dependencies unchanged, and an extension kept in another repository
   and linked into a Curvyo build must be AGPL-3.0-or-later as well (customer, EXT-LICENCE).
8. **What waits:** the wasm host and WIT (ADR 0005 §2, unchanged), a manifest file (option D), a
   generic display list for extension canvas drawing (the first extension that draws more than
   fixture outlines), extension canvas tools and pointer input, capability grants for network or
   files (ADR 0005 §7), the `Extension` trait (§3 trigger). `curvyo-plugin` (ADR 0011) stays the
   reserved name of the wasm guest SDK.

## Consequences

- The laser is built as an extension from its first line, so the contract is tested by a real,
  large user before any third party sees it (ADR 0005 §2's spirit).
- Two new crates and a new edge `ui-core → laser-core`: every laser change rebuilds `ui-core`, as a
  tracer change does today. Accepted; compiled-in means linked.
- Laser UI must fit the schema. When it does not, the answer is a new block kind in the
  application (spec `0065` EXT-UI A), which costs front-end work per kind but keeps the design
  system whole.
- There is no sandbox: a compiled-in extension is trusted code. The extension list says so.
- A third-party extension needs a Curvyo build that includes it; there is no install from the app
  (customer, no marketplace).

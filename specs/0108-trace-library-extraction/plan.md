# 0108 Trace library extraction: plan

Status: **Approved by the customer, 2026-11-10**, with these answers: Frage 1 = B, the crate is
named `vectrace` (directory `vectrace/`, no Curvyo name); Frage 2 = A, OCR is a module of the same
crate; Frage 3 = A, licence MIT OR Apache-2.0 (weights stay Apache-2.0). Branch
`story/trace-library-extraction`.

## Goal
All tracing algorithms live in one crate that can be moved to its own repository (own CI, own
releases) with a git mv and a path-to-version change. The Trace UI stays in Curvyo. A standalone
app and a SaaS will use the same crate. Glue between the crate and Curvyo is as small as possible.

## What is in scope today (measured, 2026-11-10)
| Part | Where | Lines | Depends on Curvyo document model? |
|---|---|---|---|
| Raster tracer (pipeline, colour, analysis) | `curvyo-vectorize-core` | 9,030 | only `Affine`, `Length` from `document-core` |
| Text reader (PaddleOCR on `rten`) | `curvyo-ocr-core` | 1,189 | no |
| Fitting and recognition on top of a trace: fit, shapes (rect, ellipse, polygon, star), words, inpaint, regions, parts, OCR crops | `curvyo-ui-core/src/trace_*.rs` (21 of 25 files) | about 6,500 | mostly numeric types (`Point`, `Length`, `Angle`, `Affine`, `Shape`, `StarFrame`); `trace_plan` (16 refs), `trace_text` (6) and `trace_transfer` build document objects |
| Glyph matching to fonts | `curvyo-text-core::recognise`, `ui-core/text_recognise_*` | about 800 | needs the font library, **stays in Curvyo** |
| Wasm glue | `curvyo-editor-wasm` `trace_*_glue`, `wasm_trace_*`, `wasm_recognise`, `ocr_pool_glue`, `session/trace_*` | about 2,200 | thin API wrappers |
| UI | `frontend/` Trace bar, panel, hooks, `traceWorker.ts` | about 5,000 TS | stays |

## Target shape
```text
vectrace                     (new; later its own repo)
  raster, pipeline, colour, analysis      <- vectorize-core
  ocr                                     <- ocr-core (module, weights still passed in as bytes)
  fit, shapes, words, inpaint, regions    <- ui-core trace_* (pure parts)
  api: Job -> Result, wire format (TRACE_WIRE_HEADER), progress, cancel poll
  deps: kurbo, thiserror, rten (+ rten-*), sha2; NO curvyo-* crate

curvyo-ui-core / editor-wasm (stay)
  trace_adapter.rs   Result -> document objects (Shape, text objects, layers, styles, notices)
```
Rules kept: `forbid(unsafe_code)`, builds for `wasm32-unknown-unknown`, native and wasm results
identical, no filesystem, clock or threads (the ADR 0028 scoped `rayon` exception moves with it).

## How the glue stays small
1. The crate owns its geometry in `kurbo` plus its own mm newtypes (`Mm`, `Px`) and a plain-data
   shape result (`Rect`, `Ellipse`, `Polygon`, `Star` as fields, no document type). Today's
   `Length`, `Angle`, `Affine`, `Shape` imports are replaced; the adapter converts once at the
   boundary (about 15 lines per shape kind).
2. One entry point per task (`trace`, `trace_colours`, `analyse`, `read_text`, `fit`), each takes
   plain data and returns plain data. The wire format the worker already uses becomes the crate's
   public format; the wasm glue only forwards bytes.
3. UI strings and notices (`trace_text_notice`, `trace_plan` names and styles) stay in Curvyo;
   the crate returns reasons as enums.
4. The Web Worker (`traceWorker.ts`) keeps loading the editor wasm; a later standalone package
   gets its own wasm build from the same crate with a 40-line wrapper.

## Slices
Each slice is a commit series on one branch `story/trace-library-extraction`; one PR when the
whole move is testable (CLAUDE.md §4): the trace in the demo behaves exactly as today.

0. **Inventory and bloat pass (refactorer + lead, no behaviour change).** Run `ponytail-audit` on
   the three source areas; list dead code, duplicated helpers, one-impl traits, options nobody
   uses, over-long functions (>60 lines) and modules (>500: `outline.rs` 1,037, `palette.rs`
   733, `trace_transfer.rs` 715, `trace_region.rs` 620, `trace_shapes.rs` 594, `trace_plan.rs`
   534). Delete first, then move. Output: `inventory.md` with each file marked *move*, *stay*,
   *delete* or *split*. Gate: tests unchanged and green.
1. **Cut the document-model edges inside `vectorize-core`.** Replace `Affine`/`Length` with the
   crate's own types; `document-core` leaves its `Cargo.toml`. Goldens must stay byte-identical.
2. **Make `ui-core` trace modules pure.** Split each `doc:`-coupled file (`trace_plan`,
   `trace_text`, `trace_transfer`, `trace_shapes`) into a pure part (moves) and an adapter part
   (stays). Goldens and `ui-core` tests unchanged.
3. **Create `vectrace`.** `git mv` vectorize-core, ocr-core and the pure `trace_*`
   files; history stays with `git log --follow`. Re-export nothing from the old crates; fix the
   callers. Remove the two empty old crates. ADR (new) amends 0011, 0023, 0028 by note.
4. **Shrink the glue.** One adapter module in `ui-core`, one thin forwarding layer in
   `editor-wasm`; delete the per-feature `*_glue` files that only rename calls. Target: wasm
   glue below 1,000 lines (from 2,200), `trace_*` in `ui-core` below 1,500.
5. **Extraction rehearsal (no new repo yet).** In a scratch directory, copy the crate out with
   `git subtree split`, build and test it alone on native and `wasm32`, and run the dependency
   and licence check (`cargo deny`). Output: the exact steps and a CI file for the new repo
   (fmt, clippy, nextest, wasm32 build, deny, doc, release-plz or tags), kept in
   `specs/0108.../extraction.md`. The actual repo creation is the next task.

## Verification
- Every slice: the affected crates' tests; trace goldens in `tests/fixtures/trace/` unchanged.
- Before the PR: full gate (CLAUDE.md §7) and the CPU-time and memory tests of
  `vectorize-core` (criteria 25 and 26 of 0089) at the same budgets.
- Tester (Haiku, own VM) runs the acceptance tests of 0025, 0089, 0091, 0094 against the branch;
  Chromium check of Trace in the browser: Photo and Logo presets, Multicolour, Recognise text.
- Metric to report: editor wasm size before and after (`metrics/wasm-size`); no hard limit.

## Who does what
Lead: slices 0 and 5, integration, PR. Architect (Opus): the ADR and the public API of the crate
(slice 3 gate). Refactorer (Haiku): the mechanical moves of slices 1 to 4 against a named
target. Implementer (Sonnet): the adapter split of slice 2. Tester: as above.

## Risks
- **Identical results.** Replacing `Affine`/`Length` must not change a float. Mitigation: goldens
  and the native-vs-wasm check run after slice 1.
- **Wasm size.** Merging OCR and tracer in one crate changes nothing in the bundle, but a
  moved function that is no longer inlined can. Report only.
- **Hidden coupling** in `trace_plan`/`trace_transfer` may need more than the adapter split.
  Slice 0 measures this first; if it exceeds 3 days the lead brings it back before slice 2.

## Questions for the customer
Answered 2026-11-10, see Status. The licence goes into the new crate's `Cargo.toml` and a `LICENSE-*` pair at the extraction (slice 5).

**Not asked (decided, vetoable at the demo) (Proposal):** font matching stays in Curvyo; shapes
are returned as plain data; no behaviour change in the demo; no hard size limits.

## Skills used per slice (ponytail and pstack, from `~/.claude/skills`)
- Slice 0: `ponytail-audit` for the inventory; pstack `principle-subtract-before-you-add` (delete
  before moving), `principle-laziness-protocol`, `principle-minimize-reader-load` and
  `principle-type-system-discipline` for what to split or simplify.
- Slice 1: pstack `blast-radius` to prove the one fact the change is safe because of (the goldens
  stay byte-identical, native and wasm), as a script that runs the real code.
- Slices 2 and 3: pstack `architect` to sketch the crate's types and signatures first, with
  `principle-boundary-discipline` (guards only at the crate's entry points) and
  `principle-model-the-domain` (plain-data shape results); `principle-migrate-callers-then-delete-legacy-apis`
  (no re-exports, old crates deleted in the same series).
- Slice 4: `ponytail-review` on the diff; pstack `unslop` and `no-comments` on the moved
  doc comments.
- Before the PR: pstack `interrogate` (adversarial multi-model review) on the crate's public API
  and the adapter, and `principle-prove-it-works` in Chromium.

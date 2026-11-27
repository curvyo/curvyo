# Plan for QR codes, barcodes and Data Matrix, slice 1 (QR)

Branch `story/qr-codes`. Criteria 1 to 10 and 13 to 19 of `specification.md` (QR only, Frage 321 B,
CODE-PAINT default B: one fill for the whole code, no finder colour, no background). ADR 0091, UX notes
revision 1. No `format_version` change.

## Affected crates/modules
- `curvyo-extension-core`: `ParamKind::Text` / `Percent`, `ParamValue::Text` / `Percent`,
  `ShownWhen::Full`, `Bound::DependentFor` (ADR 0091 §3), `CHANGELOG.md`.
- `curvyo-codes-core` (new, `curvyo.codes`, kind `qr`): `grid.rs`, `qr/encode.rs` (the only file naming
  `qrcode`), `trace.rs` (Merged), `shapes.rs`, `style.rs` (Separate and Inverted), `finder.rs`,
  `clear_area.rs`, `qr/mod.rs` and `qr/schema.rs`, `values.rs`, `manifest.rs`.
- `curvyo-ui-core`: register `CODES`; the refusal rule in the record builder (no parts = keep the stored
  parts, write the params); Text and Percent in the value conversions; clamp and warning sentences; the
  panel view's lasting problem line.
- `curvyo-editor-wasm`: the No solution outline on kept geometry, text values through the session, the
  CPU-time test `codes_cpu_time.rs`.
- `frontend`: the Generators bar glyph, the Generator block (multi-line Text field, Percent rows, labelled
  choice rows, the error line, the contrast line).
- Dependencies: `qrcode` 0.15 (MIT OR Apache-2.0, product), `rqrr` 0.11 (`(MIT OR Apache-2.0) AND ISC`, test
  only; brings `g2p`, `lru`). `deny.toml` gets the AGPL exception of the new crate.

## Decisions fixed here (all Proposal)
- **Content edit vs. level edit.** A Content edit (or a regeneration) that does not fit at the stored level is
  refused: no parts, a warning, the host keeps the last parts and stores the params. Only an edit of Error
  correction itself lowers the level to the highest the content fits (clamp "the most this content allows").
  Turning the logo on raises the level to the highest level up to H the content fits, without a notice.
- The clamp text of the clear area is the house form ("Logo size set to 24 %, the most error correction M
  allows.") through `Bound::DependentFor(subject)`.
- A refusal's bytes limit is the byte-mode capacity of version 40 at the level; the sentence says bytes.

- **Finder Outer has no Circle in slice 1** (lead decision): `rqrr` cannot read it; Later, after a phone check or a second decoder.

## Tasks
- [x] 1. Contract: Text, Percent, Full, DependentFor with tests and changelog (AC 5, 6, 8, 13)
- [x] 2. Encoder, module grid, capacity numbers (AC 1, 2)
- [x] 3. Merged trace, Separate shapes, finders, Inverted (AC 6, 7, 10)
- [x] 4. Clear area and its codeword budget (AC 8)
- [x] 5. Generator: params, Width and Module size, scale, refusal, clamps (AC 2, 5, 13, 14, 16)
- [x] 6. Decoder round trip with `rqrr`, goldens (AC 4, 19)
- [x] 7. Host: registration, refusal rule, value kinds, sentences, panel view (AC 5, 13, 14, 16)
- [x] 8. Session: No solution outline, text values, CPU-time budgets (AC 13, 17)
- [x] 9. Frontend: bar entry, Generator block fields (AC 5, 6, 8, 13)
- [x] 10. Docs: ADR 0091 Accepted, spec status, design system rows, index, tech debt if needed
- [ ] 11. Full gate once, into a file

## Validation
- `curvyo-codes-core/tests/decoder.rs`: rasterised styled output (10 px per module) read by `rqrr`: versions
  1 to 10 at L, M, Q, H in five module styles, Diamond, finder styles, Inverted (light ink on dark ground),
  the largest clear area per level at versions 3, 5, 10 and the default one at versions 1 to 10.
- `tests/golden.rs`: four SVG fixtures within 0.001 mm.
- `tests/acceptance_0144.rs`: the numbers of criteria 2, 5 to 8, 10, 13, 14, 16, 18.
- `cargo build -p curvyo-codes-core --target wasm32-unknown-unknown` (CI's core-wasm32 job does the same).
- CPU time (thread time) in `curvyo-editor-wasm/tests/codes_cpu_time.rs`, asserted in release.

---

# Plan for slice 2 (1D barcodes)

Branch `story/barcodes` (the same branch as slice 1's follow-up docs; no PR of its own for the invisible
steps). Criteria 3, 4, 11, 14, 16 and 19 to 22 of `specification.md`, ADR 0092, UX notes revision 2
(section 11), Frage 407 default B (no font embedding; answered A on 2026-11-27, built in the slice 3 follow-up, no `format_version` change; a change found
unavoidable stops the work). Slice 1 task 11 (the full gate) was run on the PR #518 tree, not on this one.

## Affected crates/modules
- `curvyo-extension-core`: `Warning.param` / `subject`, `GeneratorKind.scanned` / `family` / `resize`,
  `Family`, `Resize`, `ParamKind::Font` / `Ratio`, `ParamValue::Font` / `Ratio`, `Percent` as `i32`,
  `GlyphTable`, `Glyph`, `GenerateRequest.glyphs`, `Edit::Stretch`, `CHANGELOG.md`.
- `curvyo-codes-core`: `linear/{mod,code128,code39,ean,style,schema}.rs`, manifest (five kinds), tests
  (`tests/support/glyphs.rs`, `linear_golden.rs`, `linear_decoder.rs`, `acceptance_0144_slice2.rs`).
- `curvyo-ui-core`: `instance_switch.rs` (`switch_kind`), `code_glyphs.rs` (table from text-core), the glyph
  input, `plan_resize` with two factors, the handle layout for `Free` kinds, sentences.
- `curvyo-editor-wasm`: the face cache and "Loading font..." gate, the family segment, `switch_generated_kind`,
  `session/generators/{panel,source}.rs` (the split), CPU-time test for a 1D code with text.
- `frontend`: the Code segment, the Symbology strip, Ratio, signed Percent and Font rows.

## Decisions fixed here (all Proposal)
- **Lasting line vs. notice.** A warning that names a row (`Warning.param`) is the lasting line under that row
  only while the generator answers no parts (the stored outline is kept); with parts it is an ordinary 3 s notice
  (`check_digit_corrected`, `glyph_missing`). The host no longer lists ids.
- **Sentence inputs.** `Warning.subject` carries the cause words ("error correction M", "A to Z, 0 to 9 ..."),
  the kind's label is the host's. QR keeps "Too long for a QR code at error correction M: n bytes, at most m."
  byte for byte (id `content_too_long`, unit bytes); the 1D kinds use `too_many_characters`
  ("Too long for Code 128: 92 characters, at most 80."), `wrong_length`, `digits_only`, `not_encodable`.
  `noun()` keeps a label that holds a digit ("Code 128") like an acronym.
- **Glyph input.** The session sets the face tables on the thread before every write, as it sets the active
  extension settings (`set_active_extension_settings`, the same precedent), so `instance::generate` stays the one
  funnel and no signature of the 70 callers changes.
- **Code 39 at 2.5 : 1 has fractional widths**: run widths are `f64` modules.
- **Code 128 switching** is a shortest-path search over the three sets; a tie keeps the current set.
- **UPC-A quiet zones 9 / 9 are unverified** (from memory, spec criterion 11): flagged in the report.

## Tasks
- [x] 1. Refactor: `Warning.param` / `subject`, `GeneratorKind.scanned`; `instance.rs` split by arm;
  `session/generators.rs` split (panel, source); QR sentences byte-identical (criteria 13, 16)
- [x] 2. Contract: Ratio, Font, signed Percent, glyph table, Stretch, family, resize, changelog (11, 14, 21)
- [x] 3. Encoders with goldens from the standards: Code 128, Code 39, EAN-13, EAN-8, UPC-A (3, 19)
- [x] 4. Run list to outlines, bar width adjust, text layout, schema, defaults, normalisation, warnings (11, 21, 22)
- [x] 5. Decoder round trip with `rxing` (dev only), every symbol value of Code 128 and every Code 39 character (4)
- [x] 6. Host: `switch_kind`, family segment, glyph table builder and cache, Loading font gate (20, 21)
- [x] 7. Stretch: handle layout, `plan_resize` with two factors, Size chip (14)
- [x] 8. Session: CPU-time budget for a 1D code with text (17); `curvyo-codes-core` builds for `wasm32-unknown-unknown`, there is no wasm run of the determinism test (1)
- [x] 9. Frontend: Symbology strip, Ratio, signed Percent, Font rows, bar glyph (5, 11, 20)
- [x] 10. Docs: ADR 0092 Accepted, spec status, index, changelog, technical debt entry removed
- [ ] 11. Full gate once into `gate-0144b.txt`

## Validation
- `curvyo-codes-core/tests/linear_golden.rs` (module counts and the standards' tables; five SVG fixtures
  within 0.001 mm), `linear_decoder.rs` (10 px per module, `rxing`, all of criterion 4's symbologies),
  `acceptance_0144_slice2.rs` (criteria 3, 11, 21, 22 numbers).
- `cargo deny check`; `cargo tree -e normal -p curvyo-codes-core` shows no `rxing`.
- CPU time (thread time) in `curvyo-editor-wasm/tests/`, asserted in release.

---

# Plan for slice 3 (Data Matrix)

Same branch `story/barcodes`, same PR #523. Criteria 3, 4, 12, 17 and 19 of `specification.md`, UX notes 11.14,
ADR 0092 "Implementation notes, slice 3". No `format_version` change (a new kind is additive).

## Affected crates/modules
- `curvyo-codes-core`: `datamatrix/{mod,encode,schema,style}.rs`, manifest (kind `data-matrix` after QR), the QR
  schema rows made shareable (`content`, `size`, `modules`, `colours`, `advanced(quiet)`), tests `datamatrix.rs`.
- `curvyo-ui-core`: `noun("Data Matrix")`; `curvyo-editor-wasm`: session and CPU-time tests; `frontend`: `Grid3x3`
  glyph (bar and Layers), the Symbology strip in rows of 2, 3 and 2 (`ToggleGroup` `rows`).
- Dependency: `datamatrix` 0.3.3, **Apache-2.0 OR MIT** (permissive, passes `cargo deny`; `no_std`; brings
  `arrayvec` MIT OR Apache-2.0 and `flagset` Apache-2.0); `rxing` gets its `datamatrix` feature, dev only.

## Tasks
- [x] 1. Encoder: smallest square 10 x 10 to 144 x 144, function cells, limit (AC 3, 12)
- [x] 2. Drawing: module styles on data modules, solid L finder, clock and alignment patterns, Merged, Inverted (AC 12)
- [x] 3. Decoder round trip with `rxing`, sizes 10 to 32 in four styles, Inverted (AC 4)
- [x] 4. Golden `datamatrix_10.svg` (AC 19)
- [x] 5. Manifest, host wording, Symbology strip, glyphs (AC 12, 20)
- [x] 6. CPU time of a 144 x 144 code: 19 ms Merged, 43 ms Dot, budget 100 ms (AC 17)
- [x] 7. Docs: spec Done, ADR notes
- [x] 8. Full gate once into `gate-0144c.txt` (nextest 18914 passed; one clippy lint in the new test fixed and clippy rerun green)

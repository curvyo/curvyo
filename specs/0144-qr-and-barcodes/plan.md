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

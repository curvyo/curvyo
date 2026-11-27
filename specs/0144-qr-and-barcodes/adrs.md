# ADRs for QR codes, barcodes and Data Matrix

Status: Proposed 2026-11-26 (architect), written against the Draft `specification.md` and `main` at
`2a2526899` (`curvyo-extension-core/src/extension.rs`, `param.rs`; `curvyo-document-core/src/
generated_codec.rs`, `generated_outline.rs`; `curvyo-ui-core/src/extensions.rs`, `instance.rs`;
`curvyo-generators-core/src/lib.rs`, `manifest.rs`). Follows Frage 319 A, 321 B, 320/322/323 at
their defaults. Decisions marked **(Proposal)** are the architect's and vetoable at the demo.
Customer questions carry a KURZ-ID only; the PO assigns the number.

- [ADR 0091](../../docs/adr/0091-qr-and-barcode-codes-extension.md) (new, Proposed 2026-11-26):
  decides this feature. New crate `curvyo-codes-core` (extension `curvyo.codes`), one kind per
  symbology (`qr` in slice 1), `qrcode` encoder and `rqrr` test decoder, `ParamKind::Text` and
  `Percent`, the refusal rule (no parts: stored parts kept, params written), traced Merged outlines and Separate
  modules in one part `code`, the codeword budget of the clear area, colours through the ordinary
  fill, budgets, SVG goldens, no `format_version` change. §8 (per-part paint) is
  `needs-customer`: CODE-PAINT.
- [ADR 0080](../../docs/adr/0080-generated-objects-extension-trait-and-parametric-instances.md):
  the generated object, its stored record and frame, unknown kinds kept and drawn (§4, §5), the
  instance mechanism, scale and Convert to paths (§6), history labels (§7). Used unchanged;
  extended by two parameter kinds and the refusal host rule.
- [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md) §1, §3, §5, §6: an extension
  is a crate and one registry entry; parameter keys and option ids are file format;
  `extensions.toml` can switch codes off. Unchanged.
- [ADR 0082](../../docs/adr/0082-generator-plate-joints-corner-rule-and-rounded-box.md) §10: part
  keys and per-part operations. The code has one part `code`, so Detach part is inert. Unchanged.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §2: gains `curvyo-codes-core`
  through ADR 0091.

## Spec changes the PO should take (from ADR 0091, all Proposal)

1. **Criterion 8:** the 4 / 9 / 15 / 20 % budget fails its own decoder test at small versions
   (measured largest readable share at version 3: 4.3 / 7.6 / 11.9 / 14.4 %, and L reads only
   2 % at version 1). Replace it with ADR 0091 §5: cleared non-function modules at most
   0.4 x 8 x the correctable codewords of the version and level.
2. **Criterion 15:** a Separate code converts to one compound path (one part); Break apart gives
   one path per module.
3. **Criterion 5:** no Symbology field in slice 1 (the kind is QR); switching in place comes with
   slice 2.
4. **Criterion 9:** Finder colour and Background wait for CODE-PAINT (default B); Fill and
   gradient and the contrast chip stay in slice 1.
5. **Criterion 1:** "a style change never re-encodes" becomes "encode and style are separate
   functions"; the generator runs both on each edit (no cache in a core crate; 7 ms at version 40).
6. **Criterion 16:** no `format_version` change in slice 1; an unknown symbology is an unknown kind.
7. **Criterion 2 (size):** the object's box is the symbol; the 29.0 mm is the Width parameter (it
   includes the quiet zone). The box with the quiet zone (UX notes §12.10) needs a model change.
8. **Criterion 13 (invalid content):** stored with the last valid paths (UX notes §12.3, adopted),
   drawn in the No solution look, as the sketch clone's fallback.

## Conflicts with UX notes revision 1 (for the lead)

- §9: Separate converts to a group of module paths; ADR 0091 §4: one compound path, Break apart.
- §7, §12.9: Finder colour and Background as own parts with own fills; ADR 0091 §8: CODE-PAINT,
  default B (not in slice 1), so the Colours group holds Inverted and the contrast line only.
- §12.10: box with the quiet zone; ADR 0091 §4: not in slice 1 (model change).

## Notes

- 2026-11-26: Spike results (scratch crate, not committed) are recorded in ADR 0091 Context and
  §5. `cargo deny check` passes for every candidate with this repo's `deny.toml`.
- 2026-11-26: Slice 2's human-readable text needs glyph outlines passed into a pure generator;
  ADR 0091 §9 defers that to a revision with slice 2.

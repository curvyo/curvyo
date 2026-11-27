# ADR 0091: Codes extension: QR codes as generated objects, encoder and decoder crates, traced outlines and the clear-area budget

**Status:** Accepted 2026-11-26 (architect) for slice 1; §2 (switching in place), §3 (contract) and §9
(1D encoders, human-readable text) extended for slice 2 by [ADR 0092](0092-codes-slice-2-1d-barcodes-symbology-switch-and-human-readable-text.md); with the deviations recorded in the
implementation notes at the end (they amend §1, §3 and §7 as written); proposed the same day. Not
`needs-customer` as a whole: it adds an extension crate through the published route of ADR 0034 and
ADR 0080 and changes no file format.
**§8 (per-part paint for the Finder colour and the Background) stays `needs-customer`** (document
model, file format): Frage 406 CODE-PAINT, default B taken (not in slice 1). Spec:
[`0144`](../../specs/0144-qr-and-barcodes/specification.md) (Frage 319 A: a Code is a generator;
Frage 321 B: slice 1 QR, slice 2 1D, slice 3 Data Matrix). Adds a crate to ADR 0011 §2; extends
ADR 0080 §2 (two parameter kinds, one host rule) without amending its decisions.

**In plain words (for the customer).** A QR code is one more generator, like the gear and the
boxes: one object that remembers its text and style, rebuilt when you change a number, saved with
its outline so every Curvyo opens it as drawn. It lives in its own small module so the code
libraries never touch the box code. Colour and gradient work as on any shape; a separate colour
for the three corner squares and a background need a change to the file format, so they are your
call (CODE-PAINT).

## Context

Read against `main` at `2a2526899`: `curvyo-extension-core/src/extension.rs` (`Part` is a key and
`Vec<ClosedPath>`, `Generated` has no error case), `curvyo-document-core/src/generated_codec.rs`
and `generated_outline.rs` (a generated leaf reads as **one** `PathSnapshot` with **one** style,
`params` opaque), `curvyo-ui-core/src/extensions.rs` (registry `[&LASER, &GENERATORS]`;
`instance::enabled_kinds` lists the generator kinds of every manifest), `render-core/src/fill.rs`
(`FillRule::NonZero`) and `svg-core/src/write_leaf.rs` (`fill-rule="nonzero"`).
`CURRENT_FORMAT_VERSION` is 65.

**Spike (scratch crate, not committed, 2026-11-26).** `qrcode 0.15` (no default features, no
dependencies) builds for `wasm32-unknown-unknown`; "Hello" at M is 21 x 21; 2953 bytes at L give
177 x 177 and 2954 fail; encode takes 0.25 ms for a short URL and 7.2 ms at version 40 (native
release, all eight masks). `rqrr 0.11` (no default features, no dependencies) decodes the grid at
10 px per module. `cargo deny check` with this repo's `deny.toml` passes advisories, bans, licences
and sources for qrcode, rqrr, `datamatrix 0.3` and `rxing 0.9` (features `encoding_rs`,
`decoders`, `qrcode`, `datamatrix`, `oned`; it does not compile without `encoding_rs`).
Clear-area measurement in §5.

### Options: where the code generator lives

- **A. Its own extension crate `curvyo-codes-core`, extension id `curvyo.codes` (chosen).** ADR
  0034 §1's route (an extension is a crate `curvyo-<name>-core`): one Cargo dependency of ui-core
  and one entry in `EXTENSIONS`; the Generators entry already lists every manifest's kinds. The
  encoder and decoder crates stay out of the box crate's tree, criterion 18's `cargo tree` check is
  about one crate, and `extensions.toml` can switch codes off alone.
- **B. A kind inside `curvyo-generators-core`.** No new crate, but the plate-and-gear crate
  gains encoders, styling and test decoders (a second responsibility), and the stored kind
  `curvyo.generators/qr` ties codes to it in every file forever. Rejected.
- **C. A pure encoder crate plus the kind in generators-core.** Two crates to touch per
  symbology and the same coupling as B. Rejected.

### Options: the QR encoder

- **A. `qrcode` 0.15 (MIT OR Apache-2.0, chosen).** Optimal mode segmentation, all 40 versions,
  `is_functional(x, y)` and `max_allowed_errors()` public (both needed by §5); no dependencies
  without default features.
- **B. `fast_qr` 0.14 (MIT).** Faster, but no public function-pattern map; we would rebuild it.
  Rejected; speed is not the bottleneck (7 ms at version 40).
- **C. `rxing` as encoder for every symbology.** One crate for all, but Apache-2.0 only, pulls
  `chrono`, `regex`, a second `syn` into the product, and is a large port. Rejected for the
  product; kept as a test decoder candidate (§7).
- **D. Our own QR encoder.** Reed-Solomon, masks and segmentation are about 1,500 lines with
  their own test burden. Rejected; 1D codes are the exception (§9).

## Decision

1. **Crate `curvyo-codes-core`** (ADR 0011 §2): `#![forbid(unsafe_code)]`, AGPL-3.0-or-later
   (a `[[licenses.exceptions]]` entry in `deny.toml`), in CI's wasm32 list by its suffix. Edges:
   `→ curvyo-extension-core, curvyo-document-core, qrcode, thiserror`; `curvyo-ui-core →
   curvyo-codes-core`. No `libm` needed (no trigonometry: circles use the cubic constant), no
   geometry-core (§4 traces, no booleans). Modules, one responsibility each: `lib.rs` (the
   `Extension` impl, `CODES`), `manifest.rs`, `grid.rs` (`ModuleGrid { width, dark: Vec<bool>,
   function: Vec<bool> }`), `qr/encode.rs` (the only file that names `qrcode`: content and level
   to a `ModuleGrid` or `EncodeError`, `thiserror`), `trace.rs` (§4 Merged), `module_shape.rs`
   (§4 Separate), `finder.rs`, `clear_area.rs` (§5), `invert.rs`, `qr/mod.rs` (params to
   `Generated`). `encode` and `style` are separate public functions (criterion 1); the generator
   calls both on every generation and keeps no cache (no hidden state in a `*-core` crate, ADR
   0082 §10 note).
2. **Kinds, one per symbology (Proposal).** Slice 1 adds `curvyo.codes/qr`, label "QR code";
   slices 2 and 3 add `code-128`, `code-39`, `ean-13`, `ean-8`, `upc-a`, `data-matrix`. A file
   with a symbology this build does not know is then an unknown kind and opens with its stored
   outline and the notice of ADR 0080 §5 (criterion 16) with no new code; a Symbology *choice*
   inside one kind would instead read an unknown option as the default and silently rebuild a
   Data Matrix as a QR on the next edit. Consequence: slice 1 has no Symbology field. Switching
   symbology in place is a kind change; it is designed in slice 2, when a second kind exists
   (CLAUDE.md §5, nothing speculative), as `GeneratorKind.family` plus a host command that keeps parameters
   of equal key.
3. **Contract additions** (extension-core 0.x, `CHANGELOG.md`):
   - `ParamKind::Text { multiline: bool }`, `ParamValue::Text(String)`, stored `{"text": "..."}`;
     `ParamKind::Percent { min: u32, max: u32 }`, `ParamValue::Percent(u32)`, stored
     `{"pct": n}` (Rounding 0 to 50, Module fill 50 to 100, Logo size 5 to 35). Both are opaque to
     document-core like every value (ADR 0080 §4), so neither changes the file format.
   - **A refusal is a result without parts** (host rule, criterion 13 "keeps its last valid
     geometry"). Content that cannot be encoded (empty, too long for version 40 at this level) is
     not clampable without changing its meaning, so the generator returns `parts: []` and a
     `Warning` (`content_empty`; `content_too_long` with `limit` the most bytes at this level). The
     rule sits in the one funnel `instance::generate`, so create, the typing and drag preview,
     change and scale all inherit it: the host **keeps the stored parts** and, on a commit, writes
     the new `params` with them (one "Change parameter" step), as the sketch clone's last solved
     geometry does (`0099`, "No solution for these values"); the canvas draws the kept outline in
     the No solution look and the field shows the sentence, also after a reopen (UX notes §5, §12.3,
     adopted: refusing the commit would lose typed text on a selection change). The record then
     holds params its parts do not match; that is the accepted state, never damage (parts stay
     non-empty). Create uses defaults, which always encode. Everything else is permit over refuse
     as ADR 0080 §2: an error-correction level the content does not fit is lowered to the highest
     that fits (`Clamp`, `Dependent`); a clear area over budget is shrunk (§5).
   - **The content is never logged**: it appears in no `Warning`, notice, commit field
     (`;key=value` carries the key only, ADR 0080 §7) or panic message; a test greps the crate for
     `log`/`println`/`dbg`.
4. **Module grid to closed paths** (frame-local mm, **origin at the code's centre**, so a change
   of version, Module size or Quiet zone keeps the centre in place with no host code, UX notes §9):
   - **Size.** Module size is the stored truth; Width is a derived stored length (the Pitch
     diameter rule): `Edit::Param("width")` writes Module size = Width / (modules + 2 x quiet zone),
     content and quiet-zone edits keep Module size (the code grows), `Edit::Scale` multiplies Module
     size, clamped to 0.1 to 20 mm.
   - **The quiet zone is not in the object's box (Proposal).** ADR 0080 reads a generated object's
     bounds from its stored outline, and nothing in the contract gives empty extent; an invisible
     rectangle would be filled or cut. So the selection box, W readout, Align and the gradient
     frame cover the symbol (21 mm for "Hello", M, 1 mm), Width (29 mm) includes the quiet zone, and
     Inverted (and a Background under §8 A) brings the quiet zone into the box. UX notes §12.10 asks
     for the box with the quiet zone; that is a document-model change (an `extent` in the record
     that every bounds consumer reads) and is not taken in slice 1. Criterion 2 then tests Width.
   - **Merged** (default for Square and Rounded square at Module fill 100 %): `trace.rs` walks the
     cell edges between ink and no-ink cells with ink on the left, so outers and holes come out
     with opposite winding, as the NonZero fill of render-core and the SVG writer need; finder
     rings become an outer and a hole by the same walk. **4-connectivity:** at a vertex where two
     ink cells touch only diagonally the walk turns so they stay separate loops, never a pinch
     vertex (kerf and offset stay sane). Collinear edges merge, so a straight run is one segment;
     a line is a cubic with `c1 = start`, `c2 = to` (ADR 0080 §2). Rounded square rounds **convex**
     corners by Rounding x module edge (at most 50 %, so two arcs on one module edge meet but never
     cross; one cubic per quarter, k = 0.5522847498); concave corners stay sharp (liquid fillets are
     out of scope). Criterion 7's area test holds exactly for Square.
   - **Separate** (Dot, Diamond, Module fill below 100 %, or chosen): one subpath per dark module,
     the shape centred in its cell and scaled by Module fill; a dot is four cubics.
   - **Finders** are excluded from module styling and drawn by `finder.rs`: an outer 7 x 7 shape
     with a 5 x 5 hole of the same family (Square, Rounded, Circle) and a 3 x 3 centre; radii are
     **(Proposal)** fixed fractions, the UX notes may tune them.
   - **Inverted**: one rectangle of symbol plus quiet zone, and every ink subpath of the chosen
     style reversed inside it. Reversal flips each nesting level, so finder centres and the
     inside of an "O" come out as islands (criterion 10); works for Merged and Separate.
   - **One part**, key `code`, holding every subpath. Convert to paths therefore gives **one
     compound path** for Merged *and* Separate (ADR 0080 §6, one part, one path); one path per
     module is then Break apart (`0035`). One part per module (thousands of part keys) is
     rejected. **This changes spec criterion 15 for Separate (PO, Proposal).**
5. **Logo clear area and its budget.** The area is the set of modules whose centre lies inside
   the chosen shape (Square, Rounded, Circle) of side Size x symbol width (quiet zone excluded),
   grown by Margin, centred on the middle module, symmetric. It never covers finders, separators,
   timing, format or version areas (the size is clamped first); alignment patterns inside it are
   kept and drawn (from version 7 one sits in the centre; UX notes). Every other module inside is
   cleared, dark or light, because an opaque logo covers both.
   **Budget (Proposal, replaces criterion 8's percentages):** N cleared non-function modules
   <= 0.4 x 8 x E, with E = the correctable codewords of the version and level
   (`qrcode::QrCode::max_allowed_errors`). Over budget, the size shrinks to the largest allowed
   (`Clamp { key: "logo_size", bound: Dependent }`, chip "Clear area limited by error correction
   M"). Turning the logo on raises the level to H when the content fits (a knock-on value of
   `Edit::Param("logo")`). Measured with rqrr, centred squares, function patterns kept, versions
   1 to 40: the smallest area that failed was N = 0.56 x 8E (H, version 15), the largest per-level
   shares that still read at version 3 were 4.3 % (L), 7.6 % (M), 11.9 % (Q), 14.4 % (H) of the
   symbol, below the spec's 4/9/15/20 %; a flat percentage fails at versions 1 to 3 at every level,
   the codeword rule holds at every version with a 30 % margin for engraving blur. Criterion 8's
   decoder test stays the guard.
6. **Colours in slice 1 need no contract change.** A generated leaf already carries the path style
   keys (ADR 0080 §4), so Fill as a colour or a `0124` gradient over the code's box works as on a
   path, and so do strokes and laser operations. The contrast chip (criterion 9) compares the
   fill, for a gradient its lightest stop, to the document background (shell side, Proposal).
7. **Decoders are dev-dependencies only.** Slice 1: `rqrr` 0.11 (`(MIT OR Apache-2.0) AND ISC`,
   no default features, no dependencies). The criterion 4 test rasterises the **styled output**,
   not the grid: a test-support scanline filler (`tests/support/raster.rs`, NonZero, cubics
   flattened at 0.01 module) at 10 px per module, then decodes; it covers versions 1 to 10 at
   L, M, Q, H, each module shape, Inverted, and the largest clear area per level at versions
   3, 5, 10. Slices 2 and 3: `rxing` (Apache-2.0, features above) as the decoder for 1D and Data
   Matrix, decided there with its `cargo deny` and `cargo tree` output in the PR.
8. **Per-part paint (`needs-customer`, CODE-PAINT; not in slice 1 by default B).** A Finder
   colour and a Background cannot be expressed today: `generated_outline::outline` reads every
   part as one compound path with the node's one style, so a Background part would be filled in
   the code's colour and, under NonZero, cover the modules. Built as option A it would be:
   `Part.paint: Paint::{Object, Own(Rgb)}` and `ParamKind::Colour`; the record gains `part_paints`
   parallel to `parts` (the next `format_version`; an older build would otherwise draw a solid
   square); the read pass yields one path per paint group, so render, hit test, SVG export, clones
   and the job each change; the `background` part never reaches the laser job (it is print colour,
   not material). Options: **A** build it in slice 1; **B** slice 1 ships Fill and gradient only,
   Finder colour and Background become a later slice with this design (recommended: the format
   change is the riskiest part and the code is pretty and scannable without it); **C** never, the
   maker uses Convert to paths and Break apart. Default B.
9. **Slices 2 and 3, recorded now so slice 1 does not block them.** 1D encoders are ours
   (`linear/code128.rs`, `code39.rs`, `ean.rs`; each well under 200 lines, tables from the
   standards, check digits), output a run list in modules, bars as rectangles in one part. Data
   Matrix via `datamatrix` 0.3 (Apache-2.0 OR MIT; `arrayvec`, `flagset` Apache-2.0; spiked on
   wasm32), the L finder and clock edge excluded from styling as the QR finders are. The
   human-readable text needs glyph outlines, which a pure generator cannot fetch: slice 2 revises
   this ADR for how the host passes them (`GenerateRequest` has no font input today).
10. **Budgets (criterion 17).** `curvyo-editor-wasm/tests/codes_cpu_time.rs` (uses
    `tests/cpu_time/`, asserted in release as `generators_cpu_time.rs`): the session's parameter
    edit (encode, style, the one commit, the canvas view) **20 ms** for version 10 and **100 ms**
    for version 40, each Square Merged and Dot Separate. The spike leaves the encoder 7 ms of the
    100; the rest is the commit of up to some 15,000 subpaths. The test also prints the stored
    record's bytes for version 10 and 40 Dot Separate; the PR states them (estimate: about
    0.4 MB and 3.5 MB per commit), and a `docs/technical-debt.md` entry follows if version 10
    exceeds 0.5 MB (ADR 0080's compact encoding).
11. **Golden fixtures** (CLAUDE.md §5): `curvyo-codes-core/tests/fixtures/*.svg`, one `<path>`
    with `fill-rule="nonzero"` and three decimals, written by a test helper (no svg-core edge) and
    compared number by number within 0.001 mm. SVG, not generators-core's `.txt` dumps, because
    criterion 19 asks for it and a code fixture can be checked by eye and by a phone. Slice 1:
    `qr_v1_square_merged.svg`, `qr_v3_rounded_logo_circle_h.svg`, `qr_dot_separate.svg`,
    `qr_inverted_merged.svg`; slices 2 and 3 add `ean13.svg`, `code128.svg`, `datamatrix_10.svg`.
    document-core gets no new golden: the record is unchanged; a ui-core test opens, draws and
    saves back a QR object, and the generic unknown-kind golden covers criterion 16.
12. **No `format_version` change in slice 1** (ADR 0080 §5: a new kind is additive; §3 values are
    opaque to document-core). Option A of §8 would be the only bump.

## Consequences

- One more crate and two dependencies (`qrcode` in the product, `rqrr` in tests), both tiny.
- Every committed edit stores the whole outline (ADR 0080's known cost); large dot codes make
  this visible first, measured by §10.
- The extension contract grows two parameter kinds and one host rule (no parts = refusal),
  usable by any later generator with input that cannot be clamped.
- The spec changes for the PO (Proposals): criterion 8's percentages become §5's rule; criterion
  15 for Separate converts to one compound path; criterion 5's Symbology field arrives with
  slice 2; criterion 9's Finder colour and Background wait for CODE-PAINT; criterion 2's 29 mm is
  Width, the box is the symbol (§4).
- UX notes revision 1 differs in three places, decided here: §9 (Separate converts to a group of
  module paths) and §12.9/§7 (Finder colour and Background as own parts) follow CODE-PAINT and §4;
  §12.10 (box with the quiet zone) is not taken in slice 1. §12.3, §12.4 a, §12.5 and §12.6 are
  adopted. Schema elements of §12.9: Text and Percent are §3, the derived length exists, Colour
  comes with §8 A, the labelled choice and the two-row strip are presentation only.

## Implementation notes (slice 1, 2026-11-26)

- **Contract additions beyond §3:** `ShownWhen::Full { key }` (the Output row shows only at a Module fill of
  100 %) and `Bound::DependentFor(subject)` ("the most error correction M allows", the house clamp form of
  criterion 8; the subject is a static string per level). Source-breaking for an exhaustive `match`; recorded
  in `curvyo-extension-core/CHANGELOG.md`.
- **Content edit versus level edit (§3, made precise):** a Content edit or a plain regeneration that does not
  fit at the stored level is refused (no parts, `content_empty` or `content_too_long`, the host keeps the
  stored parts and stores the params); only an edit of Error correction lowers the level to the highest the
  content fits (clamp "the most this content allows"); turning the logo on raises it silently. A resize of a
  refused code is refused. The sentence says bytes always (the limit is the byte-mode capacity of version 40).
- **§7 dependencies:** `rqrr` 0.11 is not dependency-free; its tree adds `g2p`, `g2gen` (a proc-macro on the
  already locked `syn` 2), `g2poly` (all three 1.2.2, MIT/Apache-2.0) and `lru` 0.18.5 (MIT, on the already locked
  `hashbrown` 0.17). All pass `cargo deny check` (architect, 2026-11-26); they stay test-only (`cargo tree -e
  normal` of the crate shows only extension-core, document-core, `qrcode` and `thiserror`). `rqrr` never reads a circular outer finder, so Finder Outer has no Circle in slice 1 (lead decision;
  Later after a phone check or a second decoder; a stored `circle` reads the default). The rounded finder
  uses radii 1.5 (ring) and 0.75 (centre): the first try of 2.0 failed the decoder.
- **§10 budget:** held; measured (release, thread CPU time, the session's edit with commit and canvas
  view): version 10 3.8 ms Merged, 8.0 ms Dot; version 40 42 ms Merged, 94 ms Dot (the generator is 11 ms
  of it; 6 % headroom at version 40 Dot). The No solution overlay costs nothing per frame: it reads the
  parameters without the outline (`Document::generated_params`, additive) and keeps the answer per
  parameter set, a commit of the session fills it. Saved file: version 10 40 KB Merged, 167 KB Dot;
  version 40 518 KB Merged, 2.2 MB Dot (below the 3.5 MB estimate; no `technical-debt.md` entry, version
  10 is under 0.5 MB).
- **§1 modules as built:** `lib.rs`, `manifest.rs`, `grid.rs`, `qr/mod.rs` (params to `Generated`), `qr/schema.rs`
  (the parameter groups), `qr/encode.rs` (the only file naming `qrcode`), `values.rs` (schema-checked reads and the
  clamp list), `style.rs` (Merged or Separate, finders, clear area and Inverted put together), `shapes.rs` (the
  module and finder shapes, which replaces `module_shape.rs`), `trace.rs`, `finder.rs`, `clear_area.rs`. There is no
  `invert.rs`: Inverted is a few lines of `style.rs`.
- **§3 where the refusal rule lives:** not in `instance::generate` but at the two host writers of an edit:
  `instance::record_keeping` (used by `plan_value`, so the typing and drag preview and the commit inherit it) and
  `plan_resize`, which answers `Refused` for a code without parts. Create and the ghost use defaults, which always
  encode, and the box-from-source path places boxes only, so no writer can store `parts: []` (`read_record` would
  then read the object as damaged). A later writer of a generated record (paste re-seed, a shared edit of several
  objects, a regenerate on a settings change) must go through `record_keeping`.
- **§3 which generators can refuse:** the host asks only a kind with a `ParamKind::Text` parameter whether it
  refuses (`instance::can_refuse`), so the No solution overlay costs nothing for boxes and gears. This narrows the
  contract rule: a later generator that can answer no parts without a text parameter needs that predicate widened
  (for example a manifest flag) in its own ADR.
- **§3 text limit:** `Param::accept` cuts a text over 8,192 bytes at a character boundary. That is the stored-size
  guard, not a clamp of meaning: any text that long is past version 40 at every level and is refused anyway.
- **`Document::generated_params`** (document-core, additive public read): the kind, seed and parameters of a
  generated object without decoding its outline; it does not run the record checks `read_record` runs. Used by the
  editor's No solution memo.
- **The bar label** is "QR code" (the kind's label, as the lead asked), not "Code" (UX notes §1); the bar
  sorts the segments by their text, so it reads Box, Box from path, Gear, QR code, Rounded box.
- **Not built in slice 1:** the clear-area outline on the canvas while the code is selected (UX notes §6).
  The ghost is the symbol square and the three outer finder edges (`Edit::Draft`).

## Customer question

**CODE-PAINT: own colour for the corner squares and a background colour.** A: in the first QR
slice (a file-format change, every view and export learns colours per piece). B: first slice
with one fill colour or gradient for the whole code; corner colour and background as their own
later slice (recommended). C: never; recolour by hand after Convert to paths. Default: B.

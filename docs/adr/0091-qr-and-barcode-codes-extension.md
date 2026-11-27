# ADR 0091: Codes extension: QR codes as generated objects, encoder and decoder crates, traced outlines and the clear-area budget

**Status:** Accepted 2026-11-26 (architect) for slice 1; §2 (switching in place), §3 (contract) and §9
(1D encoders, human-readable text) extended for slice 2 by [ADR 0092](0092-codes-slice-2-1d-barcodes-symbology-switch-and-human-readable-text.md); with the deviations recorded in the
implementation notes at the end (they amend §1, §3 and §7 as written); proposed the same day. Not
`needs-customer` as a whole: it adds an extension crate through the published route of ADR 0034 and
ADR 0080 and changes no file format.
**§8 (per-part paint for the Finder colour and the Background) is decided A** (document model, file
format; Frage 406 CODE-PAINT answered A by the customer on 2026-11-27, superseding the default B taken
in slice 1): designed in the section "Per-part paint" below, Proposed 2026-11-27 (architect). Spec:
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
8. **Per-part paint (CODE-PAINT; not in slice 1 by default B; answered A on 2026-11-27, built as
   the section "Per-part paint" below, which replaces the sketch in this item).** A Finder
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

## Per-part paint: Finder colour and Background (§8 option A, 2026-11-27)

**Status:** Accepted 2026-11-27, built (implementation notes at the end of this section); was Proposed (architect), on Frage 406 CODE-PAINT answered **A** by the customer
on 2026-11-27 (supersedes default B; built as its own slice, one PR, `story/codes-paint`). Written
against `main` at `f439d6143` (`CURRENT_FORMAT_VERSION` 68). Everything not tied to a Frage is a
**(Proposal)**.

**In plain words (for the customer).** A QR code can have its three corner squares in their own
colour and a coloured rectangle behind it. The file stores which piece of the code has which colour,
so every Curvyo that opens it draws the same thing; an older Curvyo refuses the file instead of drawing
a solid square. On a laser, cutter or other machine the background is never cut or engraved; the
corner squares run with the code. Barcodes and Data Matrix get the Background too.

### Options

- **A. A paint per part key in the generated record (chosen).** The generator gives a part an own
  solid colour and says whether it goes to machines; the host stores it with the parts. Reuses the
  part keys of ADR 0082 §10 (grammar, damage checks, Detach part), merges with the parts as one
  register, and a build without the codes extension still draws it from the record (ADR 0080 §5).
- **B. Colours only in `params`.** No record change, but only the extension can read them: an unknown
  kind, a clone in a build without codes, and every exporter would need the extension. Rejected.
- **C. A style register per part key on the node (`fill.finder`, edited in the Style panel).** Needs a
  part picker in the Style panel and a second style read path in every build, for two colours that
  belong to the code's look like Finder shape. Rejected; ADR 0082 §10's keys stay the hook if a box
  ever wants a colour per plate.
- **D. A `backdrop` field beside `parts`.** The job would ignore it for free, but bounds, hit test,
  Convert to paths and clones would each need it as a special case. Rejected.
- **E. A list parallel to `parts`** (§8's sketch). Works, but writes an entry for every part of every
  painted record and says nothing a key map does not. Rejected for the map below.

### Decision

P1. **Contract** (extension-core 0.x, `CHANGELOG.md`, source-breaking for struct literals of `Part`):
   `Part` gains `paint: Option<PartPaint>` (`None` = the object's style, every box, gear and an unpainted
   code). `PartPaint { fill: Color, job: bool }` is a document-core type (`Color` as in the style), used
   unchanged by the contract, the record and the read pass. **Part order is draw order** (first part
   bottom); under one compound path it never mattered, now a generator puts a `job: false` part first.
   New parameter kind `ParamKind::Colour`, `ParamValue::Colour(Color)`, stored `{"rgb": [r, g, b]}`,
   opaque to document-core (no bump of its own). New `ShownWhen::Off { key }` (the Finder colour rows
   hide while Inverted is on).

P2. **Record** (document-core, `generated_codec.rs`): `GeneratedRecord.part_paints`, stored as
   `part_paints: { "<part key>": { "fill": [r, g, b], "job": false } }` inside the one `generated` map
   value, written whole with `parts` and `part_keys` (so a paint never merges apart from its geometry).
   Absent = every part in the object's style; written only when a part has a paint, so boxes, gears and
   unpainted codes store exactly what they store today. `job` absent = true. Damage (refused at open,
   as the other record checks): not a map; a key that is no entry of `part_keys`; `part_paints` without
   `part_keys`; an entry without `fill` or with a colour that is not three integers 0 to 255; `job` not
   a bool. Document-core knows no word "background" or "finder": only a colour and a machine flag.

P3. **The codes generator** emits the parts only when switched on, so with both off the output is the
   slice-1/2/3 output byte for byte and every existing golden stays:
   - `background` (Switch, default off) + `background_colour` (Colour, default white, `ShownWhen::On`):
     a part `background`, paint `{ fill, job: false }`, first in the list: the rectangle every kind
     already builds as its Inverted ground (QR `style.rs`, Data Matrix `datamatrix/style.rs`, 1D
     `linear/style.rs` with the text band). **All seven kinds** (cheap: the rectangle exists); the keys
     are shared by the family, so the Symbology switch carries them (same meaning in every kind, unlike
     `quiet_zone` in ADR 0092).
   - `finder_colour_on` (Switch) + `finder_colour` (Colour; `ShownWhen::All[On, Off { inverted }]`):
     **QR only (Proposal)**. A part `finder`, paint `{ fill, job: true }`, last in the list, holding the
     three finders `finder.rs` already draws apart; the `code` part keeps the rest. While Inverted is on
     the switch is ignored and the finders stay in `code`: inverted, they are holes of the ground
     rectangle under NonZero and cannot be filled apart. Data Matrix draws its L edge inside the traced
     modules, so a finder colour there needs a split first: later if asked.
   - A part that comes and goes follows ADR 0082 §10 Lifecycle: an `N#finder` membership with the
     switch off is kept and ignored. The schema default of `finder_colour` is black; the shell writes
     the fill's colour (first gradient stop) in the same step as the first switch-on (UX notes §3).

P4. **Read pass** (one place, `generated_outline::outline`): the outline stays the union of every part
   as one compound path, so bounds, hit test, transforms, snapping, Align and `generated_map` need no
   change. The box therefore **includes the quiet zone while Background is on** (amends §4 for that
   case; UX notes §12.10 is then met). `PathSnapshot` gains `paint_runs: Vec<PaintRun>`
   (`PaintRun { first: u32, count: u32, paint: PartPaint }`, subpath indices with the first outline as
   0; empty and skipped in serde for every other path, as `width_points`). Indices survive every
   transform, so clones, placed pages and previews carry them for free. A command that changes a
   snapshot's subpaths clears them; `path_codec` never stores them. Cost: one `is_empty` per path when
   off (the read is cached per document version, never per frame).

P5. **Painting** goes through one pure function in document-core, `PathSnapshot::paint_groups()`: a
   path with runs becomes several fills in part order, each with its style: an unpainted run the node's
   style (a clone's fill override applies here only), a painted run the node's style with a solid fill
   of its colour (fill opacity kept, fill on), a `job: false` run its fill only, no stroke. It is called
   **inside the two painters**, render-core's path draw (`artwork.rs` `draw_path`) and svg-core's path
   leaf writer, not by their callers: render-core has at least six artwork callers (canvas, greyed
   views, placed pages, history preview, fixture looks, the laser gradient preview), and any caller that
   skipped the split would draw a solid square. So every view, the SVG export and its PNG neighbour
   inherit it; a path without runs takes today's code path. **Gradient box:** a node gradient keeps the
   object's box of `gradient_frame.rs` (the union, the box the Select tool and the gradient handles
   show), computed on the whole path before the split, so editor and painter agree; with Background on
   that box includes the quiet zone (Proposal).

P6. **Machine jobs: a `job: false` part never reaches any job** ("printed and exported, never burnt",
   as a registration mark, ADR 0081 §1). One function `PathSnapshot::job_subpaths()` (document-core)
   is the funnel for `output/leaf_geometry.rs` `leaf_geometry` (own items, clone members, placed
   members) and the leaf counts of `operations/index.rs`; `part_geometry` and `split_parts` drop
   `job: false` parts from their masks. So the Operations view never draws a look on it, Cut-out never
   cuts around it, and switching Background on or off leaves the job unchanged. Every later machine
   family (`story/machine-families`) builds its job through the same function. The `finder` part is ink:
   it runs with the code's operations; Detach part works on it unchanged (an own engrave depth for the
   finders). `Document::detach_part` refuses a `job: false` part (`PartError::NoSuchPart`; the UI lists
   job parts only). The outline references of `tabs.<key>` and `entry.<key>` (formats 67, 68) index the
   job geometry after `job_subpaths()`, so a background that comes and goes shifts none of them.

P7. **Convert to paths** (ADR 0080 §6, one path per part): the parts other than `job: false` convert
   exactly as today (one part: the node becomes that path; several: a group under the node's id), each
   path with its `paint_groups` style, the node's memberships, its `tabs.`/`path_own.`/`entry.`
   records and `rewrite_part_keys` untouched. The `background` part becomes a **new sibling path
   directly below the node** in the same parent, filled, no stroke, named "Background", with no
   membership, selected with the node. So **Convert never changes the job** for the node's own
   operations. Under an ancestor group that has an operation the background path is an ordinary path
   and runs with it, as any path would; the test states it. Rejected: the background as a child of the
   group (it would inherit the code's operations and be engraved) and moving `member.N` to the children
   (it orphans the per-item records of formats 67 and 68, which are keyed by the membership key).

P8. **`format_version`: the next free number at merge** (69 is taken by `story/machine-families`, so
   expect 70). A build that stops below it ignores `part_paints`, reads every part as one NonZero
   compound path in the object's fill and draws (and engraves, on a laser page) a solid square over the
   code, so this build's files are refused there by version through the existing container check; a
   file of a lower version opens unchanged (no paints) and nothing is written on open. Goldens in
   `curvyo-document-core/tests/fixtures/`: `generated_part_paint_v<N>.curvyo` (a QR with Background
   and Finder colour, the finder detached into its own operation) and
   `generated_part_paint_damaged_v<N>.curvyo` (a paint key with no part). Codes goldens
   (`curvyo-codes-core/tests/fixtures/`, one `<path>` per part in draw order with its `fill`):
   `qr_background_finder.svg`, `ean13_background.svg`, `datamatrix_10_background.svg`.

P9. **Tests and budget.** The decoder test (§7) rasterises ink parts (unpainted and `job: true`) on the
   `job: false` ground: QR with both on, QR Inverted with Background, EAN-13 and Data Matrix with
   Background. `codes_cpu_time.rs` gains version 40 Dot Separate with both on, inside the same 100 ms
   (§10 left 6 % headroom: one extra rectangle and a split of three finders cost no tracing).
   A paint change while the content is refused is refused with it (§3: parts and paints are kept
   together), so the colour shows once the content encodes again.

P10. **Contrast line** (criterion 9, shell side): each dark colour (fill stops, Finder colour) against
   the Background colour when it is on, otherwise the document background; hidden on laser pages.

### Consequences

- One record key, one snapshot field, one contract field, two parameter kinds' worth of contract
  (`Colour`, `ShownWhen::Off`) and one `format_version`; render-core and svg-core change only in their path painter (one call each).
- Every `PathSnapshot { .. }` literal gains `paint_runs: Vec::new()` (mechanical, as `width_points`).
- The `background` part makes the code's box the visible rectangle while on; without it nothing about
  the box changes.
- Booleans, Offset and Stroke to path on a painted code see the union outline (the rectangle included),
  as they would after Convert to paths.

### Implementation notes (slice 4, 2026-11-27)

- `format_version` **70** (69 is `story/machine-families`). `PartPaint` lives in document-core; the record
  stores `part_paints` as written in P2 (`job` only when false).
- P5, SVG: svg-core has no path knowledge of document-core, so the split is made where the export leaves are
  built (`curvyo-ui-core/src/svg_export_parts.rs`, `painted_shape`: a group of one path per paint group, the
  gradient box of the whole object); render-core's `draw_path` is as written. With "Strokes as paths" a
  painted code keeps its plain stroke.
- P6, the Operations view: `restyle_path` in render-core reads `PathSnapshot::job_path()` (no `job: false`
  outline, no runs), so no look is drawn on a Background.
- P7: `convert_generated` returns the Background path's id; the session selects it with the converted object.
- Finder colour start (UX notes §3): switching `finder_colour_on` on while `finder_colour` still holds the
  schema's black writes the fill's first colour in the same step (`instance_value::plan_value`); a maker who
  chose black on purpose gets the fill's colour back (fill defaults to black).
- Step names "Change finder colour" and "Change background" are the session's `set_generated_value` names.
- Not built, left open: the shared colour picker under the active colour row and the eyedropper of UX
  notes §3 (the block's Colour row is a swatch and a hex field), the check "Background and Finder colour
  both below 3 : 1" shows through the one contrast line of P10.

## Customer question

**CODE-PAINT: own colour for the corner squares and a background colour.** A: in the first QR
slice (a file-format change, every view and export learns colours per piece). B: first slice
with one fill colour or gradient for the whole code; corner colour and background as their own
later slice (recommended). C: never; recolour by hand after Convert to paths. Default: B.
**Answered A by the customer on 2026-11-27** (after slices 1 to 3 shipped with B): built as its own
slice, section "Per-part paint".

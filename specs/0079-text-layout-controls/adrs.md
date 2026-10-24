# ADRs for the text layout controls

Architect draft 2026-10-24, written against `main` at `84b1143` (`0074` to `0078` merged,
`CURRENT_FORMAT_VERSION = 18`) without running code. Slice 1 (Parts A, B, G) is decided in detail
below; slices 2 to 4 are decided as far as their build needs today, with what still needs an ADR
or a spike.

**Slice 1: no new ADR, no new crate, no new external dependency, `format_version` 19.** It adds
five optional registers to the text node of ADR 0020 §1; the build PR adds a dated note there, the
same way `0076` added `text_font.kind`. **Slice 4 needs a new ADR (0021, `needs-customer`:
document model, a per-character register).** Slices 2 and 3 need no ADR; slice 3 needs a spike
first.

## Depends on

- [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) §1: the text node,
  "no measured layout is stored", uniform scale goes into `text_size`. §2: `curvyo-text-core`
  owns the line layout (today left-aligned only), `harfrust` `=0.14.0` and `skrifa` `=0.48.0`
  pinned, wasm budget 2 MiB (dated note of 2026-10-20).
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §3: merge per field; `text_string`
  merges per character. §5: drawing never writes back.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: a new on-disk key that
  an older build would misread takes the next `format_version`; older builds refuse the file.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one interaction is one commit, one
  step, with the label the spec names.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md): a linked clone
  of a text reads the original's `TextSnapshot` (`clone_read.rs`, `RawLeaf::Text`), so it shows the
  new values with no new code.
- [`0074` adrs.md](../0074-text-objects/adrs.md) decision 2 (the edit session commits once) and
  decision 3 (caret positions are shaper cluster boundaries).
- [`0076` adrs.md](../0076-single-line-fonts/adrs.md) decision 1: the pattern for an optional key
  with a default and a format bump.
- [`0078` adrs.md](../0078-font-style-buttons/adrs.md) decision 3: one document method over
  several texts, one commit.

## Slice 1: feature-local decisions (2026-10-24)

1. **Where the values live (criteria 7, 13, 34, 36, 37): five flat optional registers on the text
   node, absent = default, a default is never written.**

   ```text
   text_align         : "centre" | "right" | "justify"   absent: left
   text_letter_space  : f64, factor of text_size          absent: 0
   text_word_space    : f64, factor of text_size          absent: 0
   text_line_space    : f64, factor of text_size, >= 0    absent: Auto (natural line spacing)
   text_rotation      : f64, degrees in (-180, 180]       absent: 0
   ```

   Setting a value to its default (the reset slot, typing 0, Auto) **deletes** the key, so a
   text whose values are all back at default is stored exactly like a version-18 text. Read
   rules in `text_codec`: an unknown `text_align` string reads as left, a non-finite number reads
   as default, a factor outside the typed range of criterion 7 is clamped (merge residue, never
   refused: the text still opens). `text_rotation` is stored in degrees because that is what the
   maker types and the wrap at 180 is exact in degrees; it becomes the existing `Angle` newtype on
   read.
   Model: `TextSnapshot` gains `layout: TextLayoutValues { align: TextAlign, letter_spacing: f64,
   word_spacing: f64, line_spacing: Option<f64>, rotation: Angle }` (factors are plain `f64`
   because they are ratios, not lengths; `TextLayoutValues::DEFAULT`). The name keeps it apart
   from `curvyo_text_core::TextLayout` (the engine's output).
   **Relative storage needs nothing in the transform code**: a uniform resize changes `text_size`
   only (ADR 0020 §1), so every mm readout scales with it (criterion 34); stretch and skew stay in
   the placement, as today.

   | Option | Why it lost or won |
   |---|---|
   | **A. Five flat registers, absent = default (chosen)** | Merge per value (ADR 0009 §3): peer 1 changes letter spacing, peer 2 line spacing, both survive. Same shape as `text_size`; a pre-19 file needs no migration. |
   | B. One atomic map value `text_layout` (as `text_font`) | One register, but concurrent changes of two different fields lose one side; and a default would have to be filtered inside the map. |
   | C. A nested `LoroMap` container `text_layout` | Per-key merge, but two peers that create the container concurrently each get their own and one container (with its keys) is lost; needs a "create at node birth" rule that pre-19 texts break. |
   | D. Store mm instead of factors | Customer decided relative (Frage 32 A); mm would also need a rewrite on every uniform resize. |

2. **`format_version` 19, golden file `text_layout_v19.curvyo` (criterion 37).** A version-18
   build would ignore the keys and draw a centred, spaced text left-aligned and tight, without a
   word: hence the bump, so it refuses the file (ADR 0004 §9). Migration from 18: none. The
   golden file holds one text per key (each non-default) and one text with all five, plus one text
   with every key absent, which must read back as `TextLayoutValues::DEFAULT`. Test: a version-18
   fixture (`text_v17.curvyo`, `text_single_line_v18.curvyo`) opens, saves and its text meta maps
   have no `text_align`/`text_*_space`/`text_rotation` key. **`0080` (text on path) also takes a
   number:** whichever merges first takes 19, the other renumbers itself and its fixtures (the rule
   in `document.rs` above `CURRENT_FORMAT_VERSION`).

3. **One document method, one commit, placement passed in (criteria 4, 12, 13, 14).**
   `Document::set_text_layout(&[(NodeId, TextLayoutValues, Affine)], label: TextLayoutLabel)`
   writes only the keys that differ from the stored ones (deleting defaults) and the placement
   only when it differs, in one commit with the spec's label ("Change text alignment", "Change
   letter spacing", "Change word spacing", "Change line spacing", "Change character rotation").
   One method for five fields, not five methods. `document-core` does not know faces, so the
   **anchor shift of criterion 4 is computed in `curvyo-ui-core`**: lay out with the old and the
   new alignment, take the widest line's extent start in both, `placement' = placement *
   translate(old_start - new_start, 0)`. A mm value typed for several texts becomes one factor
   per text (`mm / size`) in `ui-core`; "Mixed" and the mm readouts come from the Text bar /
   block state in `ui-core` (`0078` decision 2 pattern), the frontend decides nothing. A drag is
   a live preview of the session (as the `0047` sliders), one call on release; Escape calls
   nothing. Tool state for the next text (criterion 33) holds the alignment only;
   `create_text` writes `text_align` when it is not left.

4. **The engine takes the values as one argument, in mm (criteria 2, 3, 8 to 11, 38).**
   `curvyo_text_core::layout(face, text, size, &LayoutOptions)` with `LayoutOptions { align:
   Align, letter_spacing: Mm, word_spacing: Mm, line_spacing: Option<Mm>, rotation_rad: f64 }`
   (`LayoutOptions::default()` = today). Text-core stays "bytes, string, size → outlines" and does
   not learn factors or the document. **One mapping function**, `ui-core::text_layout::
   layout_options(&TextSnapshot) -> LayoutOptions` (factor × size), is used by both callers of
   `layout` today: `ui-core::text_layout::text_layout` and the edit session
   (`curvyo-editor-wasm/src/session/text.rs:94`), so drawing, editing, hit test and Object to path
   lay out alike. The order inside `layout`:
   1. **Shape once per line.** With `letter_spacing != 0` the shaper gets the features `liga=0`,
      `clig=0`, `calt=0` through `ShapeOptions` features (harfrust 0.14 supports them); `kern`
      stays on (criterion 8). SVG fonts have no ligatures; their `hkern` stays.
   2. **Place clusters.** Walking the shaped glyphs, the pen gains `letter_spacing` at each
      cluster change (so after every cluster except the line's last) and `word_spacing` on the
      advance of each cluster whose first character is U+0020, U+00A0 or U+3000. `PlacedGlyph`
      gains `centre_x: Mm`, the middle of its **cluster's** advance (marks and ligature parts turn
      with their base, which the spec's "each glyph" means for a one-glyph cluster). `LineLayout`
      gains `extent: Range<Mm>` (criterion "Line extent": trailing spaces excluded) and keeps the
      shaped glyphs of the line, so justify does not shape twice.
   3. **Align.** Per line a shift `dx`: Left 0, Centre `-(extent.start+extent.end)/2`, Right
      `-extent.end`. **Justify** (criterion 3): `W` = widest extent; for each line but the last
      with at least two clusters, the extra `W - width` goes to its spaces inside the extent in
      equal parts, or, with no space, to its `clusters - 1` gaps; then the line is re-placed from
      its kept shaping. Glyph origins, `centre_x`, `extent` and the caret boundaries all take the
      same shift, so caret and drawing never disagree.
   4. **Lines.** Baseline `n` at `n × line_spacing.unwrap_or(natural)`; `TextLayout.line_height`
      reports the used value, `natural_line_height` the face's (the field's Auto readout,
      criterion 10).
   5. **Glyph map.** `glyph_paths` maps each outline em → mm → (slice 2: taper in the glyph frame)
      → rotation about `(centre_x, baseline - size/2)` (y is down in text space, so "raised" is
      minus). The rotation sign is the Select tool's `R` sign; a unit test pins it with "I" at 90°
      (criterion 11).

   Rejected: a separate post-pass in `ui-core` that moves glyph outlines (alignment and spacing
   would not reach the caret boundaries, and criterion 38 puts them in text-core); a factor-based
   API in text-core (units are types: text-core works in `Mm`).

5. **Caret, hit test, Object to path (criteria 5, 15, 35, 36).**
   - **Caret boundaries** sit in the middle of the added letter spacing between two clusters (the
     spec's proposal) and after the widened space for word spacing and justify; the line's first
     boundary at `extent.start + dx`, its last after the last cluster without spacing. The
     functions `caret_point`, `nearest_boundary`, `boundary_on_line` keep their signatures.
   - **Line box** becomes the union over lines of `[extent.start+dx, max(width)+dx]` (today it
     assumes x starts at 0; with Centre and Right lines start at negative x). With `rotation != 0`
     the box is the union of the line box and the bounding boxes of the turned glyph paths
     (`ui-core::text_layout::lay_out` computes both already, `text_layout.rs:34-39`). The hit
     area and the selection box read that box; no second geometry.
   - **Object to path and Weld** read `TextOutline` from the same `lay_out`, so criterion 35 holds
     by construction; one test converts a centred, spaced, turned text and compares with the
     drawn outlines within 0.01 mm. Single-line faces turn their open centre lines the same way.
   - **Missing face** (criterion 36): the values are read and applied to the substitute; drawing
     never writes (ADR 0009 §5 holds, nothing measured is stored).
   - **Typing budget** (criterion 5, 12: 50 ms on 500 characters): shaping stays the cost; place,
     align and rotate are linear passes. The plan adds one benchmark-style test on the 500-char
     fixture with every value set.

6. **Module split (§5, one responsibility, about 500 lines).** `layout.rs` is 234 lines and would
   pass 500 with spacing, justify and the glyph map. New `curvyo-text-core/src/align.rs` ("the
   per-line shift and the justify widths of a laid-out text") and `glyph_map.rs` ("the map from a
   glyph's em outline into text space: scale, taper, rotation"); `layout.rs` keeps shaping into
   lines and the caret queries. In `document-core` the read/write of the five keys goes into
   `text_codec.rs` (327 lines; stays under 500) and the type into `text_model.rs`.

## Slice 2 (taper): feature-local decisions

7. **Storage:** one more flat register `text_taper` (f64 fraction, −0.5 to 0.5, absent: 0), the
   rule of decision 1; next free `format_version` at its merge. No ADR.
8. **The map (criterion 17):** in the glyph frame (`x_c` = `centre_x`, `y` up from the baseline,
   `h` = size), `x' = x_c + (x − x_c)·s(y)`, `s(y) = max(0.05, 1 + t(2y/h − 1))`, `y' = y`, applied
   before the rotation (criterion 19), in `glyph_map.rs`.
9. **Flattening within 0.01 mm (criterion 18, spec question 4):** horizontal and vertical lines
   are mapped exactly as lines (the map is affine on them). Any other line maps to an exact
   quadratic (the map is bilinear), written as an exact cubic. Curves go through
   `kurbo::fit_to_bezpath` (kurbo is already a dependency of text-core, wasm32-clean) on a small
   `TaperedCubic` that implements `ParamCurveFit` (mapped point and derivative by the chain rule).
   Accuracy is 0.01 mm divided by the placement's largest singular value, so a stretched text
   still holds 0.01 mm in the document. A segment that crosses the `s = 0.05` clamp is split at
   that height first. Draw and Object to path use the same paths. Rejected: uniform subdivision
   into many cubics (no error bound) and a degree-6 exact curve (nothing downstream draws it).
10. **The circle value (criterion 19), checked:** at baseline radius `R`, a point at height `y`
    lies on radius `R + y`; the ring widens by `(R + y)/(R + h/2) = 1 + (y − h/2)/(R + h/2)`
    against half height, and `s(y) = 1 + (2t/h)(y − h/2)`, so `t = h / (2(R + h/2))` is exact for
    text **outside** the ring. **For the product owner:** for text **inside** the ring (letters
    pointing at the centre, radius `R − y`), the exact value is `t = −h / (2(R − h/2))`, not the
    negative of the outside formula; it exists only for `R > h/2`. `0080` should use this form.

## Slice 3 (vertical): what needs deciding before the build

11. **No ADR expected, a spike first** (`spike/text-vertical`, never merged), result as a dated
    note on ADR 0020 §2 and here. What the code shows today: harfrust 0.14 accepts
    `Direction::TopToBottom`, enables `vert` itself (`planner.rs`), and takes vertical advances and
    origins from the font's own tables (`ShaperFont::default_glyph_v_advance`/`_v_origin`,
    through read-fonts `vmtx`/`VORG`). `vrt2` is not enabled automatically; we pass it as a
    feature when the face has it. `skrifa` reads `vhea` for the Auto column spacing. Fallback for
    faces without `vmtx`: advance = ascender − descender scaled (criterion 23), for SVG fonts the
    em height (criterion 25). The spike confirms these three on the plan's CJK fixture.
12. **UAX #50 (Auto orientation, criterion 23):** a generated range table in text-core
    (`vertical_orientation.rs`, U/Tu/R/Tr ranges from `VerticalOrientation.txt`, a few KiB,
    generator script under `tools/`), measured against the 2 MiB budget; a crate such as
    `unicode-vo` (MIT OR Apache-2.0) is the alternative if it is current with Unicode 16. Either
    is "decide without asking" (§3); exceeding 2 MiB would be a customer question.
13. **Storage:** `text_writing_mode` ("vertical_rl" | "vertical_lr", absent: horizontal) and
    `text_glyph_orientation` ("upright" | "sideways", absent: auto), rule of decision 1, next
    `format_version`. `LayoutOptions` gains `writing_mode` and `orientation`; the caret queries
    become column-aware (criterion 24). No ADR: the node keeps its shape and no value is
    per-character.

### Slice 3 spike result and build notes (2026-10-24, implementer)

Spike run on `story/text-vertical` itself (no separate branch; the throwaway test was deleted),
against harfrust `=0.14.0`, read-fonts 0.45.1, skrifa `=0.48.0`. **No blocker; no ADR.** One new
dependency of text-core after the tester pass: `unicode-segmentation` (below).

- **Fixtures.** None of the included faces has vertical metrics: Source Sans 3, Source Serif 4
  and Great Vibes have no `vhea`, `vmtx`, `VORG` and no `vert`/`vrt2`. CJK test face:
  `curvyo-text-core/tests/fixtures/vertical/NotoSansJP-Subset.otf`, a 3.5 KB fontTools subset of
  Noto Sans JP Regular (SIL OFL 1.1, `OFL.txt` and `README.txt` beside it): `vhea` (500/-500/0),
  `vmtx`, `VORG`, `vert`, `vrt2`; Latin A B H I x y and 日本語、。「」ー（）.
- **`Direction::TopToBottom` works.** Noto: y advance −1000 (`vmtx`), offsets (−500, −880)
  (half the advance width, `VORG` 880), and `vert` substitutes 、。「ー（ for their vertical
  forms with no feature passed. Source Sans 3 (no `vmtx`): y advance −1326 = hhea ascender −
  descender, x offset −h_advance/2, which is criterion 23's fallback, computed by read-fonts as
  HarfBuzz does. So the engine passes no metrics of its own for OpenType faces.
- **`vrt2` is not applied**, and criterion 23 no longer names it (tester and implementer agree):
  `vrt2` substitutes pre-rotated forms of proportional glyphs, which would lay Latin on its side
  in Upright mode; HarfBuzz enables only `vert` for the same reason. Sideways runs are turned
  by the layout instead. Upright CJK gets its vertical forms from `vert`.
- **UAX #50 source: a generated table in text-core**, not `unicode-vo`: that crate is 0.1.0 of
  2019 (Unicode 12, unmaintained). `tools/unicode-data/vertical_orientation.py` reads
  `VerticalOrientation.txt` of Unicode 17.0.0, checks its SHA-256 (`dcef09c3...cd3cd`) and writes
  `curvyo-text-core/src/vertical_orientation_data.rs`: the U and Tu ranges merged, 99 pairs of
  `u32` (792 bytes). R and Tr are sideways, as criterion 23 says (Tr glyphs such as 「 and ー are
  therefore turned in Auto, not taken from `vert`; Upright gives the `vert` forms).
- **Runs split per grapheme cluster** (tester finding, criterion 23 with `0074` decision 3): in
  Auto a combining mark, variation selector or joiner takes its base's orientation, so base and
  mark stay in one run and one caret cluster. The cluster split comes from
  `unicode-segmentation` 1.13 (MIT OR Apache-2.0, no dependencies, builds for wasm32, already in
  `Cargo.lock` through `tao`).
- **Single-line (SVG) fonts** have no vertical metrics: Upright advances by the em and centres the
  glyph on the column (criterion 25).
- **Wasm size** (release editor wasm after `wasm-bindgen`, no `wasm-opt`): main at `90de814`
  (0082 merged) 12,612,654 bytes; this slice 12,635,923 bytes: **+23,269 bytes (22.7 KiB;
  +5.4 KiB gzip -9)**. Of the about 179 KiB left in the 2 MiB budget, about 156 KiB remain.

Build decisions (feature-local):

- **Line space and a frame.** `layout` keeps laying lines out along x with baselines on y ("line
  space") and `TextLayout::frame()` maps it to text space: identity for horizontal, a quarter
  turn clockwise for columns (columns of Vertical, left to right take negative baselines, so the
  same turn puts them to the right). Alignment, justify, letter and word spacing, caret
  boundaries, `line_of`, Up/Down and the line box are unchanged code; `line_box()` and
  `nearest_boundary()` speak text space, `caret_point()` and `boundary_on_line()` line space.
  A sideways glyph turns with the frame; an upright glyph (`PlacedGlyph::upright`) only has its
  origin mapped. `PlacedGlyph::centre_y` is the turning point of criterion 11 (the column's centre
  line for upright glyphs). Auto line spacing in columns: `vhea`, else natural.
- **Placement on a change** (criterion 20): `ui-core::kept_placement` keeps the first line's
  extent start (on its baseline, through the frame) where it was, for writing mode and
  orientation alike.
- **Keys** (criterion 24): `ui-core::text_edit` maps the arrows in columns (Up/Down along,
  Left/Right across in reading order) before the existing moves.
- **Greyed orientation** (criterion 21): the frontend greys the group when the pressed writing
  mode is Horizontal (with mixed modes it acts); a greyed press does nothing. The session accepts
  the press on a horizontal text (it is stored and kept), so the rule lives in one place.
- **Order in the bars:** Writing mode and orientation follow Alignment (the Taper field of slice 2
  is not built) and come before the Glyphs button of `0082`.
- **JSON mirror:** `writing_mode` and `orientation` are left out of `document.json` at their
  defaults, so a text of format 19 mirrors byte for byte as before.

## Slice 4 (kerning): needs a new ADR, `needs-customer`

14. **ADR 0021 "Per-character values on a text" (`needs-customer`, document model).** Criteria 30
    and 31 ask for a value that travels with its character through local edits and merges per
    character between peers. Options for the ADR:
    - **A. Loro rich-text marks on `text_string`** (`LoroText::mark(range, "kern_x", factor)`,
      style configured `ExpandType::None`, so a character typed next to a kerned one gets no
      kern). Deleting the character deletes the mark's span; concurrent kerns on different
      characters both survive; on the same character the later write wins (as any register).
      **Recommendation.** Risks for the spike: the expand config is per `LoroDoc` instance and
      must be set on every open and import before any edit; `apply_string_diff` must keep marks on
      the common prefix and suffix; undo (ADR 0014, own restore engine on `diff`) must restore
      marks; `document.json` must mirror them; the pinned Loro `=1.16.2` and its revival canary.
    - B. A map `text_kerns` keyed by Loro stable cursors (`get_cursor`): merges per record, but a
      deleted character leaves an orphan record that every reader must skip and someone must
      collect.
    - C. Offsets into the string, remapped on each local edit: breaks under concurrent edits (two
      peers' offsets disagree after a merge), so criterion 31 fails. Rejected now.
    The file format change and the format bump follow the ADR.

## For the product owner

- Criterion 19: the inside-the-ring taper value is `t = −h / (2(R − h/2))` (decision 10).
- Criterion 11: "each glyph about its own middle" is read as each **cluster** about the middle of
  its advance, so accents and ligatures do not come apart (decision 4.2). Add to the spec if
  accepted.

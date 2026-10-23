# ADRs for single-line fonts

Architect draft 2026-10-20, written against `main` at `6bf6540` (with `0074`), `story/font-library`
at `5d5bcd8` (`0075`) and `story/weld-v2` at `310a1cf` (`0068`), without editing the repository.
Written for the customer answers SL-DEF A (editable text, stroke only, SVG fonts first) and
SL-STICK A (stick fonts later); SL-LICENCE works either way (decision 8).

**No new ADR, no new crate.** One external dependency becomes direct: `roxmltree` `=0.20.0`
(MIT OR Apache-2.0), already in `Cargo.lock` through `fontdb`'s `fontconfig-parser`, so nothing new
is downloaded or vetted. **Project `format_version` bump** (next free at merge, 18 if nothing else
lands first) for one optional key in `text_font` (decision 1; Frage 21). **Font library file
`format = 2`** for the new built-in tag (decision 7). **ADR 0020 needs a dated amendment note**
(text below, the lead adds it when the customer has answered Frage 20 and 21). The wasm budget of
ADR 0020 is exceeded by about 58 KiB (decision 10; Frage 20).

## Depends on

- [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) §1: `text_font` is
  the font reference; this slice adds one optional key and makes matching kind-aware (decision 1).
  §2: `curvyo-text-core` gets a second face source (decisions 2 to 4); its responsibility sentence
  ("turns a face's bytes, a string and a size into positioned glyph outlines and line metrics")
  still holds. §3: the host lists the Curvyo folder; `.svg` files are listed beside `fontdb`
  (decision 5). §4: the licence record (decision 8). Dated note: the 1.4 MiB budget (decision 10).
  "Consequences" foresaw this slice: `text_font` reused, open paths in `BezPath`.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §7: glyph geometry is
  paths that may be open; nothing here stores glyphs. §3: lengths are `Mm`/`Length`, comparisons
  with tolerances.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: a bump wherever an older
  reader would misread a file (decision 1); a newer library file is refused, never written over
  (decision 7). §12: fonts stay referenced, not embedded.
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §3, §5, §6: logic in wasm,
  bytes cross once per face, the host lists and copies files and decides nothing about fonts beyond
  "is this a font file" (as `0075` decision 6 already does for TTF).
- [ADR 0006](../../docs/adr/0006-license.md) item 5: a font's own licence governs; unchanged.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md): crates and dependency direction
  unchanged (`text-core` still depends on no workspace crate).
- Proposed ADR 0018 (`specs/0024-svg-import-export/adrs.md`): names `roxmltree` as the XML parser
  of the SVG import; this slice takes the same parser, so the import pays nothing for it later.
- [`0074` adrs.md](../0074-text-objects/adrs.md) decision 5 (hostile-file limits),
  [`0075` adrs.md](../0075-font-library/adrs.md) decisions 1, 5, 6, 7 (library file, cuttable
  check, Add font, browser store), [`0054`](../0054-combine-open-paths/) (open outlines in compound
  paths), `0068` Weld v2 (`weld_plan` refuses through `area_refusal`).

## Feature-local decisions (2026-10-20)

1. **The file records the face kind (criteria 21, 22).** `text_font` gains an optional key
   `kind`: the string `"single_line"`; absent means an outline face. Any other value reads as
   outline (no refusal for a value a later build may add). `curvyo-document-core`'s `FontRef`
   gets `kind: FontKind { Outline, SingleLine }`, written by a maker's pick together with the
   rest of `text_font`, never changed by matching. Drawing, conversion and refusals follow the
   **stored** kind, so every peer agrees even without the face. Matching (ADR 0020 §1) compares
   only faces of the same kind: (a) a hash match of the other kind counts as missing; (b) family
   and style match among faces of the same kind; (c) the substitute is the included **EMS
   Readability** for a single-line text, the included sans face otherwise.
   **Bump:** a format-17 build ignores the key and, in step (b), could match an outline face of
   the same name and draw the letters filled without the "Font missing" mark: a misread under
   ADR 0004 §9. So the next free `format_version` at merge; reading 17 is the identity (no key =
   outline). Golden file `text_single_line_v<N>.curvyo` with the four texts of criterion 21.
   Rejected: deriving the kind from the face (no key, no bump): criterion 22 then draws a missing
   single-line text filled with the sans substitute, and the area refusal (decision 9) cannot be
   decided without the face. Rejected: a separate register `text_kind`: the kind belongs to the
   pick and must change atomically with the face (ADR 0009, one LWW register).
2. **The SVG font reader lives in `curvyo-text-core`**, two new modules: `svg_font` ("reads the
   faces, glyphs, advances and kerning of an SVG font from bytes") and `path_data` ("parses SVG
   path data into a cubic path"). XML: **`roxmltree` `=0.20.0`**, `default-features = false,
   features = ["std"]` (no `positions`), workspace dependency pinned exactly as `harfrust` is
   (hostile input in wasm). It has no dependencies, is `no_std`-capable and
   `#![forbid(unsafe_code)]`, performs no I/O and never fetches anything.
   **Hostile input (criterion 23)**, named constants in `limits`: file at most 16 MiB
   (`MAX_SVG_FONT_BYTES`, checked before parsing), UTF-8 only, `nodes_limit` 262,144, element
   depth at most 256 (one pass over the parsed tree), at most 65,535 glyphs per font, 65,535 points
   per glyph (the existing `MAX_GLYPH_POINTS`), every number finite, `units-per-em` in 16 to
   16,384 as for OpenType faces. **DOCTYPE:** FontForge-written fonts (the EMS and Hershey Text
   files) carry `<!DOCTYPE svg PUBLIC ... svg11.dtd>`; refusing every DTD would refuse every real
   font. So a DOCTYPE **without** an internal subset is accepted (`allow_dtd: true`; the external
   id is never resolved, `roxmltree` has no I/O), and a DOCTYPE **with** an internal subset (`[`
   before its closing `>`, checked on the bytes before parsing) is refused, so no entity
   declaration ever reaches the parser. The plan confirms the DOCTYPE on the shipped files.
   **Path data:** our own reader (about 200 lines, all commands of criterion 16, implicit command
   repetition, packed arc flags), because `kurbo::BezPath::from_svg` converts arcs at a fixed
   tolerance of 0.1 font units (0.28 mm at Size 2,000 mm, criterion 16 asks 0.001 mm). Arcs go
   through `kurbo::SvgArc` / `Arc::to_cubic_beziers` at 1e-7 em (0.0003 mm at the largest em);
   quadratics are elevated exactly as `outline.rs` does; a sub-path is closed only by `Z`.
   Glyph `d` is y up in font units; the reader scales to em, y down, as `GlyphOutline` is. All
   glyph paths are parsed once in `FaceStore::add`.
   Rejected: `quick-xml` (also in `Cargo.lock`, through Tauri): a streaming reader, we would write
   the tree, entity and DOCTYPE handling ourselves, and the SVG import would bring `roxmltree`
   anyway. Rejected: an own XML subset reader (about 30 KiB less wasm now): a hostile-input parser
   we own, duplicated by the import's `roxmltree` later; it does not bring the slice inside the
   budget either (decision 10). Rejected: `svgtypes` for path data (the import's choice in ADR
   0018): the releases known to the architect depend on `kurbo` 0.11 (not checked offline), a
   second `kurbo` in the wasm. When the
   import is built, ADR 0018 decides whether it reuses `path_data` (then moved to a crate both can
   use) or `svgtypes`.
3. **Two face kinds in `FaceStore` (criteria 7, 14, 15).** `Face` stays the one public type;
   inside it an enum `Source { OpenType { bytes, shaper }, Svg(SvgFont) }`, and `Face::kind() ->
   FaceKind { Outline, SingleLine }` (public). Two kinds exist now, so an enum, no trait (§5).
   `FaceStore::add(bytes, index)` keeps its signature and sniffs: the OpenType tags
   (`0x00010000`, `OTTO`, `true`, `ttcf`) go to `skrifa`; bytes whose first non-blank character
   (after a UTF-8 BOM) is `<` go to `svg_font`; anything else is `Unreadable`. For an SVG file,
   `index` is the ordinal of the `<font>` element (0 for the usual one-font file; criterion 21's
   `face_index` 0 is that case), and every `<font>` with at least one `<glyph d>` is a face.
   Every SVG font is `SingleLine` in this slice (spec "Out of scope": filled SVG fonts are drawn as
   lines). `listable_faces` and `family_name`/`style_name` read both kinds (criterion 1).
   **Shaping:** `shape_line` matches on the source. SVG: one glyph per `char` (the `<glyph>` whose
   `unicode` is exactly that character, else `<missing-glyph>` as glyph 0), cluster = the
   character's byte offset, advance = `horiz-adv-x` of the glyph, else of the font; `<hkern>`
   subtracts `k` from the first glyph's advance for pairs named by `u1`/`u2` (single characters,
   comma lists) or `g1`/`g2` (glyph names, comma lists); unicode ranges are ignored. No
   `harfrust` for SVG faces. **Metrics:** ascender `ascent`, descender minus the absolute value
   of `descent` (writers differ in its sign), line gap 0; either missing gives `units-per-em` as
   the line height (criterion 15); cap height as today (measured H, else `cap-height`, else 0.7,
   inside `CAP_HEIGHT_RANGE`). The layout, caret and Object to path code is unchanged.
4. **Drawing and conversion (criteria 7, 8, 12, 17).** The rule sits in one place,
   `curvyo-ui-core::text_layout::lay_out`: for a text whose stored kind is single-line, the
   outline's `PathSnapshot` gets the text's stroke and **fill none**, and `contours_of` keeps a
   contour open unless it ends with `ClosePath` (today it closes every contour; OpenType contours
   all end with `ClosePath`, so outline text is unchanged). The text's own fill register is kept.
   `curvyo-render-core` needs no change: it already draws `outline.glyphs` with that path's style
   and draws open outlines (`0054`). Object to path, Combine, Split, Break apart and Cut at
   crossings already go through `texts_as_paths` / `build_text_conversions`, so they get open,
   unfilled paths (criteria 17, 18) with no change of their own. Criterion 9's stroke switch is a
   rule of the font pick and of "Add text" in `ui-core` (same commit as the pick). Hit box
   unchanged (line box, `0074` criterion 25).
5. **Host listing and Add font on the desktop (criteria 1, 3).** `fontdb` does not read SVG
   fonts. `curvyo-storage-io::fonts` lists `*.svg` in the Curvyo folder itself **through
   `curvyo-text-core`'s SVG reader** (amended 2026-10-23, dated note): `storage-io` depends on
   `text-core` (normal dependency, io → core as ADR 0011 allows) and calls an SVG-only entry point
   that reaches `svg_font` (limits of decision 2, `units-per-em`, finite numbers, path data) but
   never `Face::new`, so release LTO links `roxmltree`, `svg_font`, `path_data` and `kurbo` and no
   `harfrust`/`skrifa` code. One entry per `<font>` the engine reads, `kind: "single_line"`,
   `stamp` as today, `os2` absent. Add font accepts an `.svg` exactly when this listing lists a
   face from it (the `0075` decision 6 rule "Add accepts what the Curvyo block lists"); since the
   host and the engine share one reader, "listed" and "drawable" cannot drift. The browser path
   (IndexedDB, wasm `listable_faces`) needs only the `.svg` filter. Rejected: a second `roxmltree`
   lister in `storage-io` with the same checks (the 2026-10-20 choice): it drifted within one slice
   (criterion 23: listed faces with `units-per-em` 0 and non-finite numbers that the engine
   refuses), and the fixture comparison test did not catch it; every later limit would have to be
   written twice. Rejected: listing names in wasm on the desktop too (bytes of every `.svg` over
   IPC at every start, and Add font would need a round trip through the page before copying);
   calling the general `listable_faces` from the host (its OpenType arm builds `Face`, so LTO keeps
   `harfrust`/`skrifa` in the native binary, the cost `0075` decision 5 rejected).
6. **Fonts-tab rows in strokes (criterion 6).** Browsers draw no SVG fonts, and a stroke font
   cannot become a CSS font (a glyph without area renders nothing). `editor-wasm` exports
   `face_preview_path(face, text) -> { d, box }` (binding only; `text-core` lays out the name at
   1 em and writes the centre lines as path data, em units, y down). The row draws an inline
   `<svg>` with `viewBox` from `box`, `<path fill="none" stroke="currentColor"
   vector-effect="non-scaling-stroke" stroke-width="1">`: 1 screen px, the row's text colour in
   both themes, scaled to the row's name size. Faces load as `0074` rows do; until loaded, the UI
   font. Rejected: a `<canvas>` per row (device-pixel handling and colour read from CSS in
   script, more code for the same picture).
7. **Tag and cuttable check (criteria 4, 5).** Built-in tag id `single_line`, chip order after
   `cuttable`; automatic membership = the family's faces are single-line (listing `kind`, or
   `Face::kind()` in the browser). A new built-in id in `[overrides.*]` makes a `0075` reader
   report the file broken and rename it at the next change, so the font library file goes to
   **`format = 2`** (reads 1 unchanged; a format-1 build refuses 2 and never writes over it,
   `0075` decision 8). `ui-core::cuttable::cuttable_pieces` returns 0 for a single-line face
   without running Union (it has no glyph with area); the cache stores that 0, and
   `CUTTABLE_CHECK_VERSION` does not change (no outline face changes its result). Included
   single-line fonts ship their tags in `font.toml` and `index.json` gets `kind` per face, checked
   by `included_fonts.rs` against the parsed face.
8. **Licence record (criteria 2, 25; Frage 18 either way).** `font.toml` keeps its shape. Answer
   **B**: EMS Readability and EMS Allure with `licence = "OFL-1.1"` and `OFL.txt`; nothing in the
   test changes. Answer **A**: additionally the allowed value `licence = "Hershey"` with
   `licence_file = "NOTICE.txt"` holding the Hershey use notice as distributed with the fonts; the
   test's allowed list gets the arm `"Hershey" => "originally created by Dr. A. V. Hershey"`. Our own
   value, unless the plan finds an SPDX identifier for the notice. ADR 0006 item 5 already lets a font's own
   licence govern; only ADR 0020 §4's list of two licences gets a dated note (below). Files are
   shipped unmodified in both cases. The app's licence list shows the notice text as it shows
   `OFL.txt`.
9. **Area commands refuse (criteria 19, 20, 22).** `ui-core::text_convert` gains
   `area_refusal(objects, ids) -> Option<TextRefusal>`: first `texts_refusal` (missing face or
   still loading: the `0074` notice wins, criterion 22), then every text among `ids` whose stored
   kind is single-line is an offender with the new reason `no_area` (a field of `TextRefusal`).
   The five Boolean operations, Fracture, Flatten, `weld_plan` and `offset_plan` call
   `area_refusal` instead of `texts_refusal`; Combine, Split, Break apart and Cut keep
   `texts_refusal` and convert (decision 4). All or nothing, as today. Paths made by Object to
   path are ordinary open paths (criterion 20): nothing to decide.
10. **Wasm size (Frage 20).** Measured 2026-10-20: `roxmltree` 0.20.0 alone adds **41 KiB**
    (42,219 bytes of code; isolated `cdylib`, release, `lto = true`, `codegen-units = 1`,
    `opt-level = 3`, no `wasm-opt`, toolchain 1.99.0, name-section attribution). Estimated, not
    measured: `svg_font` and `path_data` 12 to 18 KiB, kind plumbing, shaping dispatch, area
    refusal and the preview export 5 to 8 KiB. About **65 KiB** in all, so the text engine lands
    near 1,491 KiB against the 1,433.6 KiB budget (7 KiB left after `0074`; `0075`'s own growth
    is measured first). The plan measures the real figure on the finished slice, as `0074` did.
    The `roxmltree` part is paid once for the SVG import (`0024`, Must) as well. No choice of
    reader brings the slice inside 7 KiB (an own XML reader saves about 30 KiB and costs a second
    parser later), so the budget question goes to the customer.

## ADR 0020 amendment (dated note, added 2026-10-20 after Frage 20 and 21)

> 2026-10-20, single-line fonts (`specs/0076-single-line-fonts/adrs.md`): §1 `text_font` gains the
> optional key `kind = "single_line"` (absent: outline), matching compares faces of the same kind
> and substitutes EMS Readability for a missing single-line face; `format_version` bump. §2
> `curvyo-text-core` also reads SVG fonts (`roxmltree` `=0.20.0`, own path-data reader, limits in
> `limits`), `Face` has two sources and `Face::kind()`. §3 `curvyo-storage-io` lists `.svg` in the
> Curvyo folder through `curvyo-text-core`'s SVG reader (SVG-only entry point; amended 2026-10-23).
> §4 (only with Frage 18 A) the allowed licences include the
> Hershey notice (`licence = "Hershey"`). Budget: 2 MiB (Frage 20).

The index row of ADR 0020 names the amendment.

## For the product owner (spec changes)

- Criterion 23: "a DTD" becomes "a DOCTYPE with an internal subset (entity declarations)"; a
  plain `<!DOCTYPE svg PUBLIC ...>` line is accepted (decision 2), else no real font loads.
- Criterion 21: `face_index` is the ordinal of the `<font>` in the file (0 for one-font files);
  the file stores `kind = "single_line"` in `text_font` and the format is bumped (decision 1).
- Criterion 15: `descent` is taken as a distance below the baseline whatever its sign.
- Criterion 14: `u1`/`u2`/`g1`/`g2` may be comma lists; unicode ranges in `<hkern>` are ignored.
- Criterion 4: the tag adds a font library file `format = 2` (decision 7).

## Open for the customer

None. Decided 2026-10-20: Frage 18 SL-LICENCE A (the Hershey notice is allowed, decision 8 answer A),
Frage 20 SL-WASM (text-engine budget 2 MiB, decision 10), Frage 21 SL-KIND A (decision 1, with the
bump). Frage 23 SL-BACK (`ux-notes.md` §3) is open; its default A needs no decision here.

## Dated notes

- 2026-10-20: draft. No ADR and no crate: everything is reversible in one PR except the
  `text_font` key (versioned by the bump, golden file) and the library file's `format = 2`.
- 2026-10-20, lead: customer answers applied (SL-LICENCE A, SL-WASM 2 MiB, SL-KIND A). The ADR 0020
  dated note is added, the budget is recorded in `docs/technical-debt.md`. The PO change to
  criterion 23 (a DOCTYPE without an internal subset is accepted) is applied in `specification.md`.
- 2026-10-20, implementer: `format_version` 18 is the next free number (`main` and every open
  branch are at 17 or lower). Weld v2 (PR #169) is not on this branch's base (`story/font-library`):
  `area_refusal` is called by the five Boolean operations, Fracture, Flatten and Offset here; the
  Weld arm of decision 9 is one line in `weld_plan` and is added by whichever of the two PRs merges
  second (plan.md, dated notes). Done 2026-10-23 on the rebase onto `main` with Weld v2.
- 2026-10-20, implementer: the font library file goes to `format = 2` as decision 7 says, even
  though `0075` (format 1) is not released yet; reading 1 stays the identity.
- 2026-10-20, implementer: measured on the finished slice (release editor wasm after
  `wasm-bindgen`, no `wasm-opt`): +111,671 bytes (109 KiB, +41 KiB gzip) against
  `story/font-library`, inside the 2 MiB budget (`docs/technical-debt.md`).
- 2026-10-23, architect: **decision 5 amended; the `storage-io` → `text-core` edge stays, narrowed.**
  Commit `0613b8e` replaced the host's own `roxmltree` lister with `text_core::listable_faces` to
  fix the criterion-23 drift. Measured: `cargo tree -p curvyo-app -e normal` grows by 10 crates
  (`curvyo-text-core`, `harfrust`, `skrifa`, `read-fonts`, `font-types`, `kurbo`, `polycool`,
  `bytemuck`, `bytemuck_derive`, `arrayvec`), all already in `Cargo.lock` and vetted for wasm;
  binary size not measured (two LTO builds of the Tauri app). The cost that `0075` decision 5
  rejected is the shaping and OpenType code in the native binary, and that comes only from
  `listable_faces`' OpenType arm (`Face::new`). One reader for host and engine is worth more
  than a second lister kept in step by a fixture test, which already failed once. **Implementer:**
  (1) export from `text-core` an SVG-only function (e.g. `pub fn svg_listable_faces(bytes) ->
  Vec<ListableFace>`, a wrapper over `svg_font::svg_faces` with the `MAX_FACE_BYTES` check),
  have `listable_faces` call it for XML bytes, and have `storage-io::svg_fonts::svg_font_faces`
  call it instead of `listable_faces` + `retain`; (2) on a release build of `curvyo-app`, check
  `nm -C <binary> | grep -c 'harfrust::'` is 0 and give the size delta against `0613b8e` in the
  PR's Review guide; if `harfrust` symbols remain, report before merging; (3) the `[dependencies]`
  comment in `storage-io/Cargo.toml` names this note. `roxmltree` stays out of `storage-io`.

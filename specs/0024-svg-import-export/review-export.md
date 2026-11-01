# Architect review: SVG export (slice 4)

Branch `story/0024-svg-export` at `b3f4c788`, against `adrs.md` "Export (slice 4, architect
2026-11-01)", ADR 0018 point 9 and `CLAUDE.md` §5/§6. Read only (no build, no tests).

**Verdict: CHANGES REQUESTED** (4 blocking findings, all small; the structure is right).

What holds: `curvyo-svg-core` still depends only on `kurbo`, `roxmltree`, `svgtypes`, `simplecss`
(no Curvyo crate, no new dependency, no `Cargo.lock` change); the writer is pure (`&SvgExport ->
String`, no I/O, clock or thread); no `HashMap`/`HashSet` is iterated (writer uses `BTreeMap` and
`Vec`, the mapping only looks up); `curvyo:` values are read only under `curvyo:format="1"`;
numbers are pure Rust formatting with `-0` and non-finite handled; no new `unwrap`/`expect` outside
tests; no `format_version` change; the clone matrix is conjugated by the origin shift
(`svg_export_parts.rs:129`); the mapping lives in `ui-core` as decided; desktop writes through
`storage-io::write_atomic` after the whole text is built.

## Blocking

1. **Pages from an untrusted file are neither bounded nor range-checked.**
   `curvyo-svg-core/src/page.rs:131-157` (`inkscape_pages`) and
   `curvyo-ui-core/src/svg_import.rs:214-235` (`svg_page`). This path is not gated by the profile:
   any file with `inkscape:page` children reaches it. Two problems. First, the corner is a finite
   number times the view matrix, so `x="1e308"` in a scaled view gives an infinite or huge
   `origin` that is written straight into the page registers (`pages.rs::write_drawing_pages`).
   Second, there is no cap on the count, so up to 500,000 elements become 500,000 document pages
   on Open. This breaks the untrusted-input rule of ADR 0018 point 6 ("a coordinate beyond 10 km
   skips its element").
   Fix: in `inkscape_pages`, skip a page whose mapped rect is non-finite or beyond
   `limits::MAX_COORDINATE_MM`. Add `limits::MAX_PAGES` (Proposal: 1,000), stop at it and set
   `truncated`. Add one hostile test for each case.
2. **The `curvyo:face` and `curvyo:size` import lets unchecked file values into the document.**
   `curvyo-ui-core/src/svg_text_layout.rs:110, 123, 140`. The style name comes from the file, has
   no length limit (up to the 16 MiB attribute limit) and is stored as `FontRef::style`. A name
   that does not parse silently counts as Regular (line 110). An agreeing `curvyo:size` becomes
   `Length::from_mm(exact)` without the `clamp_text_size` that the lookup path applies.
   Fix: use the reference only when `FaceStyle::parse(style_name)` is `Some` and the name has at
   most 128 characters (the reader's font-name cut); otherwise fall back to the lookup. Wrap the
   exact size as `clamp_text_size(Length::from_mm(exact))`, or reject it when clamping would change
   it.
3. **A `<rect>` takes `curvyo:radii` without checking it against the SVG.**
   `curvyo-ui-core/src/svg_import_shadow.rs:127-131`. For a plain `<rect>`, `rect_agrees`
   (`:200-216`) compares the centre, size and turn but never the radii, so any `curvyo:radii`
   values (negative or huge) become `CornerRadii`. This breaks criterion 57 and ADR 0018 point 9
   ("only when the standard SVG next to it agrees"). The writer never writes `curvyo:radii` on a
   `<rect>`.
   Fix: for `LeafKind::Rect`, ignore `shadow.radii` and use `rx · √|det|`, or require all four
   radii within `MATCH_MM` of that value.
4. **The frozen profile v1 is not fully documented.** ADR 0018 point 9 lists the attribute names.
   `docs/svg-import.md` "Export" gives most `curvyo:frame` layouts, but these are missing: the
   value grammar of `curvyo:markers` (five keys, fixed order, `none|arrow|dot`, `spaced|nodes`),
   of `curvyo:outlines` (`closed|open` per outline), of `curvyo:face` (`family|style|64 hex`;
   what a `|` inside the family means: the reader then falls back to the lookup), of
   `curvyo:guides` (`"1"`), of `curvyo:dash` (multiples of the width) and of `curvyo:radii`
   (tl tr br bl, mm). Also missing: the unit of every frame angle (radians), the extra frames on a
   turned `<rect>`/`<ellipse>`, on arcs and on Inkscape stars, and the closing-node rule of
   point 9. Once merged, these files exist on users' disks.
   Fix (docs only): add a "Profile v1 reference" table to `docs/svg-import.md` with one row per
   attribute (elements it appears on, grammar, units, agreement rule used on import) and link it
   from ADR 0018 point 9.

## Non-blocking

5. **Angles and ratios in standard attributes use 6 decimals.**
   `curvyo-svg-core/src/write_text.rs:65, 118`. `line-height` (a ratio) and `rotate` (angles) are
   written with `coord`, but ADR 0018 point 9 and criterion 57 say that opacities, ratios and
   angles use the shortest round-trip text. Either switch both to `exact` (goldens change) or
   amend point 9 to say that per-character `rotate` follows criterion 152's 6-decimal rule. The
   implementer and tester choose; record the choice.
6. **New public items have no user outside the crate.** `curvyo-svg-core/src/lib.rs:54-55`
   re-exports `markers_text`, `CURVYO_NS` and `CURVYO_FORMAT`, which are used only inside the
   crate (`editor-wasm` has its own `markers_text`). Make them `pub(crate)` and drop the
   re-exports (§5, smallest public API).
7. **`curvyo:outlines` is read but never applied.** `shadow.rs:86-94` reads it and nothing in
   `ui-core` uses it. The mapping never sets it either, because `Z` always tells the closed state.
   Its parse also has no `MAX_LIST` cap. Remove the field and its read, and keep the name reserved
   in the profile reference (finding 4). Or, if it stays, cap it.
8. **A missing key writes a broken reference.** `write_leaf.rs:133-136` and `write_text.rs:90-97`
   write `href="#"` silently when a key has no id. The mapping guarantees the key, so add
   `debug_assert!` there so a mapping bug fails in tests and not in users' files.
9. **`sodipodi:arc-type` is written without escaping.** `write_leaf.rs:178-179` writes
   `ArcData::arc_type` (a `String`) with the plain `attr`. Today the mapping only passes literals.
   Write it with `attr_name`, or match on the three known values.
10. **Files over the size limits.** `curvyo-ui-core/src/svg_import.rs` is now 560 lines (limit
    ~500, was 489): move `svg_page`'s page list and `merge_closing_nodes` into
    `svg_import_pages.rs`. `svg_export_leaf.rs::primitive_kind` (~190 lines, allowed with a
    comment) would read better as one function per kind (rect, ellipse/arc, polygon/star). Both
    are justified in the PR at the least.
11. **The locale test proves nothing.** `curvyo-ui-core/tests/svg_round_trip.rs:636-643`
    `ac72_the_locale_changes_nothing` exports twice in one process and sets no locale, though its
    comment says the variable is set. Either run that test under `LC_ALL=de_DE.UTF-8` in CI, or
    say in the comment that Rust formatting reads no locale and keep only the no-comma assert.
12. **Exact float comparisons have no named tolerance.** `svg_export_leaf.rs:69` (`last.0 ==
    first.0`) and `svg_import.rs:250` (`first.point == last.point`) compare exactly. That is
    intended (point 9: the writer's own bytes), but §5 asks for an explicit tolerance. Add a
    `// exact: ...` comment at both places, or one named constant `0.0` with that reason.
13. **Leftover temp file on a failed write (existing debt, not in this diff).**
    `curvyo-storage-io/src/filesystem.rs:109-112` leaves the `.tmp` sibling behind when
    `fs::write` fails (disk full), and it does not `sync_all` before the rename. The old file is
    always kept, so criterion 4 holds. Fix it in a `fix/storage-io` PR: remove the temp file on a
    write error and fsync the file before the rename. Track it in `docs/technical-debt.md` until
    then.
14. **`primitive_kind` takes the turn as a bare `f64`** (`svg_export_leaf.rs:227`). Pass
    `Angle` and convert at the writer boundary (§5 units). `svg-core`'s bare `f64` stays as
    ADR 0018 decided.

## Accepted deviations from `adrs.md` "Export"

Recorded as a dated note in `adrs.md`. The specification the customer accepted wins where it is
more specific.

- **`curvyo:guides="1"` instead of `curvyo:role="guides"`** (criteria 143, 144). One boolean
  flag is simpler than an open value space that no second role needs. Accepted.
- **`curvyo:face="family|style|hash"` plus `curvyo:size` instead of `curvyo:font="<hash>"`**
  (criteria 132, 135). With the family and style, a missing face keeps its full reference, which
  a bare hash cannot do. The agreement rule against `font-family`/`font-weight`/`font-style`/
  `font-size` is the one criterion 57 asks for. Accepted, together with blocking finding 2.
- **No `curvyo:origin`** (criteria 52 amended, 129, 149). The export origin is the corner of the
  first page, the union of all pages or the active page. Open places that corner at 0,0 and keeps
  the pages' relative positions. The only loss is the absolute position of the page set on the
  pasteboard, which carries no meaning for the maker. Accepted; this is one attribute fewer in
  the frozen list.
- **`xmlns:xlink` only when a `use`, `textPath` or `image` needs it** (criterion 52 amended).
  Accepted.
- **Exported goldens in `curvyo-svg-core/tests/fixtures/export/real/<source>/`** instead of
  beside each import fixture. The import fixture folders stay inputs only. Accepted.
- **Writer references by key** (`ExportState::key`) instead of by leaf number. Same concept,
  renamed as the ADR allowed. Accepted.
- **Extra `curvyo:frame` on a turned `<rect>`/`<ellipse>`, on arcs and on Inkscape stars** for
  byte stability (criterion 63). Uses the frozen name `frame`; its layouts must be in the profile
  reference (finding 4). Accepted.
- **Text-on-path carriers keep their own element kind** (criterion 134) instead of being forced
  to `<path>`. Accepted, following the spec; the Inkscape hand check (criterion 155) decides the
  fallback.
- **Writer file split** (`write_leaf.rs` instead of `write_style.rs`; `svg_export_parts.rs` and
  `svg_export_notices.rs` added). Module names only. Accepted.

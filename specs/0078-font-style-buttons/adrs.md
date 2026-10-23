# ADRs for the font style buttons

Architect draft 2026-10-23, written against `story/font-preview` at `31e9a54` (`0074` to `0077`)
without running code. **No new ADR, no new crate, no new external dependency, no project
`format_version` change, no change to the font library file, no new field in any host listing.**
The only shipped-file change is four more included font files (decision 4).

## Depends on

- [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) §1: the font
  reference stays `family`, `style`, `face_hash`, `face_index`; matching (b) compares family and
  style **name**, so the style name is what a pick must agree with on reopen (decision 1). §2:
  `curvyo-text-core` reads OS/2 and `post` from hostile bytes with `Result` and the 64 MiB limit.
  §3: `fontdb` in `curvyo-storage-io` lists the Curvyo folder and the system fonts. §4: licence
  record of shipped fonts (`font.toml`, checked by `curvyo-text-core/tests/included_fonts.rs`).
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §3: `ui-core` → `text-core` and
  `storage-io` → `text-core` exist already; `editor-wasm` stays a binding layer.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one press is one commit, one step.
- [`0074` adrs.md](../0074-text-objects/adrs.md): the face pick ("Change font"), the Text bar
  state read after each change, the pending pick while bytes load.
- [`0075` adrs.md](../0075-font-library/adrs.md) decision 3: `family_key`, and the rule that
  text-core and `storage-io` name a face alike (test `listing_and_text_core_agree_on_names...`).
- [`0076` adrs.md](../0076-single-line-fonts/adrs.md) decision 1: only faces of the text's kind
  count; an SVG font face is Regular, Bold, Italic or Bold Italic.
- [`0077` adrs.md](../0077-font-preview/adrs.md) decisions 2 and 5: `preview_tile` draws the face
  a tile is given; the Text bar state carries the preview's inputs.

## Feature-local decisions (2026-10-23)

1. **Where weight, width and slant live (criteria 3, 9, 28): in the style name we already make,
   read back by one value type in `curvyo-text-core`.** New module `face_style` ("the weight
   class, width class and slant of a face and the style name they make"): `FaceStyle { weight:
   u16, width: u16, slant: Slant }` (`Slant::{Upright, Italic, Oblique}`), `FaceStyle::from_bytes
   (bytes, index)` (the OS/2 and `post` reads `style_name` does today), `name(&self) -> String`
   (today's words) and `parse(name) -> Option<FaceStyle>` (the inverse, weight at the band centre,
   ignoring case). `font_names::style_name`, `svg_font::style_of` (700/400, italic) and
   `storage-io::fonts::style_name` (from `fontdb`'s `weight.0`, `stretch.to_number()`, `style`)
   all render through `FaceStyle::name`, so the band table exists once instead of three times.
   Listings, `index.json`, the host's `FontFace` JSON and `ListedFace` stay as they are: the style
   string is the encoding of the three values, made from bytes (OpenType, SVG) or from the same
   three `fontdb` values. Test: `parse(name(s)) == s` for every band centre, width and slant; the
   existing listing-agreement test keeps the two listings equal. A variable font is one listed
   face with its default instance's OS/2 values (both readers read OS/2, not `fvar`).
   Rejected: numeric `weight`/`width`/`slant` fields on every listing (host JSON, `index.json`,
   wasm binding, three producers). They would only add the spec's "nearest weight class to the
   step centre" tie-break between faces whose style names are **equal**, and match (b) on reopen
   cannot tell those apart anyway, so a press could pick a face that reopen does not. Criterion 9
   (missing face) needs `parse` of the stored style name regardless.

2. **Target face and button states (criteria 3 to 9, 22 to 25, 28): pure functions in
   `curvyo-ui-core`**, new module `face_style_pick` ("which face of a family a weight step, the
   italic toggle or a family pick gives a text"). Inputs: `&[ListedFace]`, the text's `FontRef`
   (or the next font), the wanted step or toggle; output an index into the listing or `None`.
   Family = same block and `family_key`, same kind (as `font_match`); ties: a loaded face first,
   then listing order (amends the spec's tie rule, see "For the product owner"). Not in
   `text-core`: blocks, listings and `FontRef` are `ui-core`/`document-core` types, and text-core
   stays "bytes, string, size → outlines". The Text bar state (`text_bar()`) gains `weights:
   [ButtonState; 5]` and `italic: ButtonState` (`pressed`/`mixed`, `greyed` with the missing
   family name), computed there over the subject texts; the frontend draws, decides nothing. The
   family-row press (criterion 22) and the tile face (criterion 24) call the same function, so
   `preview_tile` is handed the shown face's id; the tile path is unchanged. Unit tests: the six
   families of criterion 28 as `ListedFace` lists (no font bytes needed), plus the mixed and
   missing cases.

3. **"Change font" with a face per text (criteria 10 to 13, 16, 23): one document method, one
   commit.** `Document::set_text_font(ids, font)` becomes `set_text_fonts(&[(NodeId, FontRef)])`,
   one commit `change_font`, unchanged texts not rewritten, the single-line stroke rule per text;
   today's callers pass the same font for each id (one method, not two). Session: `press_weight
   (step)`, `press_italic()`, and `pick_font_face` for a family row, all through `recording`. A
   press whose target faces are not all loaded stores the intent (`pending_pick` becomes an enum
   `Face(host_id, index) | Style(press)`), requests the faces and applies once the last one
   arrives, re-evaluated against the then-current subjects. The notice counts (criteria 11, 12)
   come back from the session call. Rejected: `set_text_font` once per target face inside one
   `recording` (one commit each, so one undo step per distinct face); a separate "change style"
   label (the spec asks for "Change font").

4. **Extra Source Sans 3 faces (criterion 27): four files, not six.** Upstream 3.052R has **no
   Extra Bold** (weights 200, 300, 400, 500, 600, 700, 900; checked against the release tree
   2026-10-23). Shipped: `SourceSans3-ExtraLight.otf` (318,280 B), `-ExtraLightIt.otf` (226,468 B),
   `-Light.otf` (327,396 B), `-LightIt.otf` (233,512 B), unmodified, same `3.052R` release, so
   `font.toml` gains four `[[faces]]` and `index.json` four entries; `LICENSE.md` (OFL 1.1) and the
   licence record are unchanged (ADR 0020 §4, ADR 0006 item 5). Size: **+1,105,656 B (1.05 MiB)**
   in `frontend/public/fonts`, so in the desktop bundle and the static site; **0 B in the wasm**
   (the 2 MiB budget is code); a browser maker downloads a face only on first use. The included
   fonts test checks that every face parses and its style name matches `font.toml`.
   Rejected: Black (900) as a stand-in, it has no button. Adding another family with an 800
   weight (e.g. Inter, OFL) is a product choice, see below.

## For the product owner

- Criteria 8 and 27 must be amended: Source Sans 3 has no Extra Bold, so its Extra Bold button
  is greyed ("Not in Source Sans 3"), which also shows the greyed state in the browser. Extra Bold
  is tried on the desktop with a system family that has it. Default: amend that way. Alternative:
  a fourth included family with weights 200 to 800 (scope, about 1.5 to 3 MiB).
- "Target face" ties: by decision 1 two faces in one band have the same style name, so the tie is
  "a loaded face first, then the Fonts tab's order", not "nearest weight class to the centre".

## Open for the customer

None. No `needs-customer` item: no new ADR, no format change, the licence is already accepted.

## Dated notes

- 2026-10-23: draft. No ADR: decision 1 is a value type in an existing crate replacing three
  copies of one table, decisions 2 and 3 are pure functions and one widened document method,
  decision 4 is four upstream files under an accepted licence.

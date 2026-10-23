# Plan for the font style buttons

Implementer, 2026-10-23, on `story/font-style-buttons`, stacked on `story/font-preview` (`0077`,
PR #175, not merged yet). Built for the architect's decisions 1 to 4 of [`adrs.md`](adrs.md); UX
per [`ux-notes.md`](ux-notes.md).

## Affected crates/modules

- `curvyo-text-core`: new module `face_style` (`FaceStyle`, `Slant`, `WeightBand`: weight class,
  width class and slant of a face and the style name they make, with `parse` back); the band and
  width tables exist once. `font_names::style_name` and the SVG reader render through it.
- `curvyo-storage-io`: `fonts::style_name` renders `fontdb`'s weight, stretch and style through
  `FaceStyle::name`.
- `curvyo-ui-core`: new module `face_style_pick` (target face of a weight band, italic
  counterpart, family pick, the six buttons' pressed and greyed states, the press plans with
  their counts), pure over `&[ListedFace]` and `&[FontRef]`.
- `curvyo-document-core`: `Document::set_text_font(ids, font)` becomes
  `set_text_fonts(&[(NodeId, FontRef)])`, one commit `change_font`.
- `curvyo-editor-wasm`: `TextBarView` gains the six button states; `press_weight`,
  `press_italic`, `pick_font_family`, `family_tile_face`; the pending pick becomes an enum
  (face or style press) applied when the last face arrives; bindings.
- `frontend`: `StyleGroup` (weight radiogroup with roving focus, Italic toggle, tooltips,
  notices) in the Text bar and as its own group in the Select bar; Ctrl+B/Ctrl+I in the edit
  sink; family rows and preview tiles through the session's family pick; pure text rules in
  `lib/styleText.ts` with node tests.
- Fonts: four Source Sans 3 faces from upstream 3.052R; `font.toml`, `index.json`; the included
  fonts test compares the recorded style with the listing's style name.
- Docs: `docs/design-system.md` (UX notes §9), `docs/requirements.md` R-FONT-010,
  `specs/README.md` row 0078.

## Tasks

Milestone 0, docs
- [x] 0. This plan; README row 0078 and next free number 0079; R-FONT-010.

Milestone 1, core (test first)
- [x] 1. `FaceStyle`: `name` for every band, width and slant; `parse(name(s)) == s` at every
  band centre; from OpenType bytes and SVG attributes; the three producers through it (AC 28)
- [x] 2. Four Source Sans 3 faces, licence record, index; included-fonts test names them by the
  listing's style name (AC 27)
- [x] 3. `face_style_pick`: target face (all five steps; Semibold and Black only; OTF and TTF
  duplicate; Condensed; Oblique only; single-face SVG), italic counterpart, family pick, button
  states for one, several and missing texts, greyed reasons (AC 3 to 9, 12, 13, 22, 23, 28)
- [x] 4. `Document::set_text_fonts`: a face per text, one commit, unchanged texts not rewritten,
  single-line stroke rule per text; callers moved (AC 10, 11, 26)
- [x] 5. Session: `text_bar()` button states, `press_weight`, `press_italic` with counts,
  pending press until the faces arrive, next-text press without a step, edit mode, family pick
  and tile face; reopen keeps the face (AC 3 to 18, 22 to 26)

Milestone 2, frontend
- [x] 6. `lib/styleText.ts` with node tests: tooltip lines, greyed reasons, notices, key names
  (AC 1, 6, 11, 12, 19)
- [x] 7. `StyleGroup` in the Text bar and the Select bar, glyphs, states, roving focus, focus
  return, notices (AC 1 to 7, 15, 16, 21)
- [x] 8. Ctrl+B / Ctrl+I in the edit sink (AC 19, 20)
- [x] 9. Family row press and preview tiles through the session's family pick; tile name line
  (AC 22 to 25)
- [x] 10. Design-system amendments (UX notes §9) with measured rows at 800 x 600 (AC 2)

Milestone 3, gate
- [x] 11. Full gate (`CLAUDE.md` §7), frontend tsc, npm test, oxlint; one headless check with a
  screenshot of the Text bar.

## Validation

- Unit tests in `face_style` and `face_style_pick` for the families of AC 28 as `ListedFace`
  lists (no font bytes); a round-trip test of the style names.
- Session tests with the included Source Sans 3 faces: pressed and greyed states, one step
  "Change font", Undo, several texts with the notice counts, Bold then Italic then Light, the
  Size kept (capital H height), next text without a step, edit mode, family pick keeping Bold
  Italic, the pending press, save and reopen.
- Node tests for the frontend text rules; a headless Chromium look at the Text bar.

## Dated notes

- 2026-10-23: the font.toml style of a face is compared with the listing's style name
  (`FaceStyle`), not the name table's subfamily: upstream names the faces "ExtraLight", the
  listings "Extra Light" (adrs.md decision 1), and `index.json` must carry the listing's name so
  a pick matches on reopen.
- 2026-10-23: criteria 5 and 6 differ on Italic: criterion 5 says a pressed button is never
  greyed, criterion 6 and UX notes §4 give "Not in Lobster as upright" for Italic while pressed.
  Built: a pressed weight button is never greyed; Italic can be pressed and greyed (no upright
  counterpart). Criterion 9 with no family anywhere: all six greyed "Font missing" and none
  pressed (criterion 5 wins over "pressed states follow the stored style name", which applies to
  the fallback family).
- 2026-10-23: the weight glyphs are drawn from the upstream 3.052R variable font
  (`VF/SourceSans3VF-Upright.otf`) at weight 200, 300, 400, 700 and 800, since the static faces
  have no 800; only the path data is kept (`frontend/src/components/StyleGlyphs.tsx`), the
  variable font is not shipped.
- 2026-10-23: measured at build (headless Chromium): Text bar 537px on one row at 1280 x 800,
  two rows (72px) at 800 x 600 with the panel open; Select bar with one text 5 rows (180px).

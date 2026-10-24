# Single-line fonts: engrave and plot lettering as one line per stroke

Status: Done (PR #171, merged 2026-10-23; Ready 2026-10-20; customer questions 17 to 21 decided 2026-10-20, Frage 23 SL-BACK decided A 2026-10-23)
Priority: Should (R-FONT-002 names "single-line" first among the manufacturing groups; R-MFG-CUT-001 and engraving need it; not in the laser MVP)
Origin: Customer. A story of the fonts epic `0062` (named out of scope in `0074` and `0075` as "a later story"; ADR 0002 §7 and ADR 0020 "Consequences" keep the model open for it). Items marked **(Proposal)** are the product owner's and are vetoable at the demo.
Depends on: `0074-text-objects` (text node, Text tool, Fonts tab, Object to path, missing-font rule), `0075-font-library` (tags, Add font, Remove, the Curvyo folder in the browser), `0054-combine-open-paths` (open outlines in a compound path, stroke and fill of open outlines), `0068` Weld v2 (PR #169, merged 2026-10-23, open operands closed by a straight line, its criterion 13).

## User value

As a maker who engraves names on a laser, writes addresses with a pen plotter or a cutting plotter's pen, or V-engraves house numbers on a CNC, I want to type a text in a single-line font and have each letter drawn as one centre line, so that the machine goes over every stroke once, fast and thin, instead of tracing both sides of an outline letter.

## Terms

- **Single-line font:** a font whose glyphs are open centre lines with no width (Hershey, the EMS fonts), as opposed to an **outline font** whose glyphs are closed areas (TTF, OTF of `0074`).
- **Single-line face, single-line text:** a face from a single-line font; a text object whose face is one.
- **SVG font:** the `<font>` element of SVG 1.1 as written by Inkscape's Hershey Text v3 and the EMS `svg-fonts` repository: `<font-face>` (`font-family`, `units-per-em`, `ascent`, `descent`, `cap-height`), `<missing-glyph>`, `<glyph unicode horiz-adv-x d>`, `<hkern>`. Glyph `d` is in em units with y up from the baseline.
- **Centre line:** the glyph path itself; the stroke is drawn around it. Size, positions and Object to path refer to the centre line, never to the stroke's edges.
- Lengths in mm, tolerance 0.01 mm unless stated. Notices at most 70 characters, in the slots `0074` uses.

## Field reference

| Tool | Single-line formats | Live text? | What it does with them |
|---|---|---|---|
| Inkscape 1.x, Extensions > Text > **Hershey Text** v3 (Evil Mad Scientist) | SVG fonts: 9 Hershey fonts bundled, the EMS fonts with the AxiDraw software ([EMS wiki](https://wiki.evilmadscientist.com/Hershey_Text), [svg-fonts](https://gitlab.com/oskay/svg-fonts)) | No: a dialog generates a group of stroked paths (or replaces existing text); to change a word the maker runs the extension again | Stroke only, fill none. Size is the text's font size (em), so the cap height is not what was typed |
| LightBurn | SHX (AutoCAD), from a folder set in Font Settings; no SVG fonts ([docs](https://docs.lightburnsoftware.com/UI/FontsAndText.html)) | Yes | Drawn as lines; Line mode cuts the centre line. Forum: many "single line" TTFs are outlines that run back on themselves and cut twice ([forum](https://forum.lightburnsoftware.com/t/text-in-single-line/43343)) |
| Fusion | SHX, listed first in the Text dialog ([help](https://help.autodesk.com/cloudhelp/ENU/Fusion-Sketch/files/SKT-REF-TEXT-DIALOG.htm)) | Yes | Engrave or Trace toolpath on the line; Emboss needs an outline font. OneLineFonts' "open curve" OTFs come out with missing segments ([forum](https://forums.autodesk.com/t5/fusion-manufacture/single-line-fonts/m-p/7396535)) |
| EMS / OneLineFonts / Relief SingleLine | SVG fonts (EMS, OFL); OTF "stick" fonts with doubled-back closed outlines; OpenType-SVG with an open-path layer and an outline fallback (Relief SingleLine, Adobe apps only, [Adobe Fonts](https://fonts.adobe.com/fonts/relief-singleline)) | Depends on the app | The doubled outline is the portable fallback; every non-Adobe app cuts it twice |
| Cricut Design Space | Its own "Writing" fonts (filter), drawn with a pen; system fonts draw their outline | Yes | Pen goes along both sides of an outline font |

**What we do better.**
- **Live, editable single-line text.** Retype a name, change the size: Inkscape regenerates paths through a dialog, we keep a text object.
- **Size is exact:** the capital H's centre line is exactly Size high (as `0074`), whatever the font's em box.
- **Never filled, never doubled.** A single-line text draws stroke only, and Object to path gives open paths with fill off, so nothing downstream can close or fill a stroke by accident.
- **The SVG fonts makers already have** (Inkscape's Hershey Text folder, the EMS fonts) work by Add font, on the desktop and in the browser. LightBurn and Fusion read SHX only.
- **One tag** "Single line" finds them in the Fonts tab (LightBurn users ask for exactly that section).

## Scope of this slice (decided by the PO; Frage 19 can widen it)

| Format | This slice | Why |
|---|---|---|
| **SVG fonts** (Hershey Text v3, EMS) | **Yes** | One reader covers every Hershey font and the 28 OFL EMS fonts; Inkscape users already own them; plain XML, open strokes as they are |
| Hershey JHF (raw Hershey data) | No, later | The same glyphs exist as SVG fonts; JHF adds a second parser for no new letters |
| OTF/TTF stick fonts (doubled-back closed outlines) with de-doubling | No, next slice (Frage 19) | Needs a heuristic that finds the centre line of a thin closed contour; wrong results are possible; it gets its own criteria and fixtures |
| SHX, OpenType-SVG open-path layers | No | SHX is an undocumented Autodesk format; OT-SVG single-line fonts are rare |

**Shipped fonts** (Frage 18): A: Hershey Sans 1-stroke, Hershey Script 1-stroke (Hershey notice: free for any use with attribution, not OFL), EMS Readability and EMS Allure (SIL OFL 1.1). B (default if unanswered, within the accepted licence rule of `0074` TEXT-LICENCE): EMS Readability and EMS Allure only. Shipped unmodified, in the format of the `svg-fonts` repository.

## Acceptance criteria

### Part A: listing and adding single-line fonts

1. Given an SVG font file (`.svg`) in the Curvyo folder (desktop) or added in the browser, then each `<font>` in it is a face listed in the Curvyo fonts block like a TTF (`0074` criterion 19): family from `font-face font-family`, style from `font-style` and `font-weight` ("Regular", "Bold", "Italic", "Bold Italic"), sorted with the other families. An `.svg` file with no `<font>` holding at least one `<glyph>` with a `d` is skipped and counted as skipped ("12 · 2 skipped").
2. Given the included single-line fonts of Frage 18, then they are listed in the Included block, each with its licence record (ADR 0020 §4), on the desktop and in the browser, fetched on first use as the other included fonts.
3. Given Add font… (`0075` criteria 20 to 24 and 28), then the file filter also offers `.svg`, and an SVG font is accepted, copied, de-duplicated and reported exactly as a TTF; an SVG that is not a font counts as "not a font".
4. Given a single-line face, then its family carries the built-in automatic tag **Single line** (tooltip of the chip "Detected: single-line font"), and the tag strip has the cell **Single line** after Cuttable: All, Favourites, Cuttable, Single line, Script, Sans, Serif, Initials. The maker can remove and add it as any built-in tag (`0075` criterion 10). The included fonts' other tags come from the shipped tag list: Hershey Sans 1-stroke and EMS Readability in Sans, Hershey Script 1-stroke and EMS Allure in Script.
5. Given a single-line face, then the cuttable check of `0075` is not run on it (Union would close every stroke) and it never gets Cuttable automatically; the maker can add Cuttable by hand.
6. Given the family row of a single-line font, then its name is drawn in its own strokes (1 screen-px lines in the row's text colour) at the row's name size, loaded as `0074` rows are; until loaded, the UI font. (Browsers do not draw SVG fonts; how the row is drawn is the architect's and UX engineer's.)

### Part B: drawing and typing

7. Given a text whose face is single-line, then the canvas draws each glyph's centre line with the text's stroke (colour, width, cap, join, dashes of the Style tab) and **never fills it**, whatever the fill setting holds. Test: a single-line "O" at Size 20 mm, fill red on, stroke 0.5 mm black: a pixel at the centre of the O is not red.
8. Given a text whose face is single-line, then the fill setting is kept unchanged in the document and is not applied; the Style tab shows that Fill does not apply to it (hidden as stroke markers are for a text, or shown disabled: UX notes). With other objects in the selection, Fill changes those only.
9. Given a new text in a single-line face (criterion 14 of `0074`), or a text switched to a single-line face by a pick, when its stroke is off, then the stroke is switched on in the fill colour at 0.25 mm with round caps and joins, in the same history step ("Add text" or "Change font"). A stroke that is on is not changed. (Proposal: the rule of Weld's Frage 14 A, so a single-line text is never invisible.)
10. Given a single-line text switched back to an outline face, then it draws with its kept fill and its stroke as they are; nothing else changes.
11. Given the Text tool, then typing, editing, caret, selection, IME, Escape, undo inside the session, the 10,000-character limit and the 50 ms budget on 500 characters behave exactly as in `0074` criteria 2 to 10 and 31.
12. Given **Size** (cap-H, `0074` criterion 12) and a single-line face, then Size is the height of the H glyph's centre line: a one-line text "H" at 20 mm, converted to paths, has the top of its path bounding box (not the stroke's) 20.00 mm above the baseline within 0.01 mm, for every included single-line face and the fixtures. A face without an H uses `font-face cap-height`, else 0.7 of `units-per-em`.
13. Given the stroke width, then it is independent of Size: changing Size or a uniform resize does not change the stroke width (the stroke behaves as for a path today).
14. Given an SVG font, then the glyph for a character is the `<glyph>` whose `unicode` is that single character; glyphs whose `unicode` has several characters are not used. Advance is the glyph's `horiz-adv-x`, else the font's. `<hkern>` pairs given by single characters (`u1`, `u2`) or glyph names (`g1`, `g2`) reduce the advance by `k`: test "AV" in the kerning fixture within 0.01 mm. A character without a glyph draws `<missing-glyph>` (nothing if it has no `d`) with its advance; nothing is substituted (as `0074` criterion 6).
15. Given several lines, then the line height is `ascent` minus `descent` of `font-face`, scaled as the glyphs; with either missing, `units-per-em`.
16. Given glyph path data, then all SVG path commands (absolute and relative M, L, H, V, C, S, Q, T, A, Z) are read; a sub-path ends open unless it has Z, even if its ends meet (no auto-close). Arcs become cubics within 0.001 mm; quadratics are elevated exactly.

### Part C: Object to path and the path commands

17. Given selected single-line texts, when the maker presses **Object to path**, then each becomes a group as in `0074` criterion 22 (named by the first 24 characters), with one path per drawn glyph; a glyph of several strokes is one compound path of its open (and, where the font closes them with Z, closed) outlines (`0054`). Every resulting path has the text's stroke and **Fill off**, so no closing line is ever filled. Nodes match the font's points scaled to the size within 0.001 mm. One history step "Object to path".
18. Given a selection with a single-line text, when Combine, Split, Break apart or Cut at crossings runs, then the text is converted by criterion 17 first in the same history step and the command applies its own rules to the open outlines (as `0074` criterion 24).
19. Given a selection with a single-line text, when a command that works on areas runs (the five Boolean operations of `0016`, Fracture, Flatten, Weld, Offset), then the command is refused, nothing is changed, the single-line texts are drawn with the refusal outline, and the notice reads "Single-line text has no area. Nothing was changed." ("2 single-line texts have no area. Nothing was changed."); all or nothing as the Boolean refusals. (PO decision: closing each stroke with a straight line, as `0054` criterion 21 and Weld criterion 13 do for open paths, turns letters into nonsense. Vetoable at the demo.)
20. Given a single-line text **after** Object to path, then its paths are ordinary open paths: every command treats them as `0054` and Weld define for open outlines (closing line with its notice). Criterion 19 holds only for the text object.

### Part D: file, missing face, safety

21. Given a project with single-line texts, when saved, closed and opened again, then every text shows the same string, face, size, placement and style (the kept fill included) and is editable; the file stores what `0074` criterion 28 stores, the font reference with the SHA-256 of the SVG file and `face_index` 0, and no glyphs. Whether `text_font` also stores that the face is single-line, and the `format_version` change if any, are the architect's (see below). Golden file with a single-line text, a two-line text, a rotated one and one with a missing single-line face.
22. Given a single-line text whose face is missing, then it is drawn with the included single-line face **EMS Readability**, stroke only (criterion 7), and marked missing exactly as `0074` criterion 29 (Font button, Missing block, canvas tag "Font missing"); Object to path and every command of criteria 18 and 19 are refused with the `0074` notice. (Needs the face kind to be known without the face: architect.)
23. Given a damaged or hostile SVG font (not well-formed XML, a DOCTYPE with an internal subset (entity declarations) or an external entity, nesting deeper than 256, a file over 16 MiB, more than 65,535 glyphs, a glyph with more than 65,535 points, numbers that are not finite, a `units-per-em` of 0), then reading it never crashes or hangs: the file is skipped (criterion 1) or refused on Add font (criterion 3), or a face in use is drawn as missing (criterion 22). No entity is resolved and nothing is fetched; a plain `<!DOCTYPE svg PUBLIC ...>` line without an internal subset is accepted (architect, `adrs.md` decision 2). Fixtures and a fuzz target as ADR 0020 §2.
24. Given the core crates, then they read no file: SVG font parsing, layout and outlines are pure functions on bytes in a `*-core` crate that builds for `wasm32-unknown-unknown`; listing the folder is platform code (`CLAUDE.md` §6).
25. Given the included single-line fonts, then the test of ADR 0020 §4 covers them: each has a `font.toml` with an allowed licence and its licence or notice text, and every face parses.

## Out of scope

- OTF/TTF stick fonts with doubled-back closed outlines and their de-doubling (next slice, Frage 19); until then they are ordinary outline fonts.
- Hershey JHF files, SHX, OpenType-SVG open-path layers; single-line fonts from the This computer block (the operating system lists no SVG fonts).
- Multi-character glyphs (ligatures in SVG fonts), `<vkern>`, `arabic-form`, vertical text; SVG fonts with filled glyphs (every SVG font is drawn as lines in this slice).
- Stroke to path (expanding a stroke to an outline for cutting a thick single-line text) and stroke bands in Weld (Weld v2 out of scope).
- Turning an outline font into a single-line font (centre-line trace), font editing.
- Machine output: the G-code and HPGL of the open strokes come with `0029` and the plotter workbench; pen-up travel order and lead-ins are theirs.
- Embroidery stitch fonts (`0063` embroidery workbench); the preview page of `0062` stage 3.
- Per-glyph stroke width, a "line weight" built into the font.

## What the architect must decide (`adrs.md`)

1. **SVG font reader:** where it lives (a module of `curvyo-text-core`), and the XML parser dependency: shared with the SVG reader of `0024` (ADR 0018) or its own; the SVG path-data parser shared with `0024`. Limits of criterion 23.
2. **Two face kinds in `FaceStore`** (OpenType through `skrifa`/`harfrust`, SVG font through its own layout): an enum is enough, two kinds exist now. Shaping of SVG fonts is per character plus `hkern` (no `harfrust`).
3. **Is the face kind stored?** Criterion 22 needs to know a missing face was single-line. Options: an optional key in `text_font` (e.g. `outline = "single_line"`) with or without a `format_version` bump (ADR 0004 §9), or derive it and accept a filled fallback for missing single-line faces. PO recommendation: store it.
4. **Drawing:** `render-core` strokes open outlines of a text and skips the fill for a single-line face; hit box unchanged (`0074` criterion 25).
5. **Host listing:** `fontdb` does not read SVG fonts; how the `fonts` module of `curvyo-storage-io` lists `.svg` in the Curvyo folder, and the browser storage of `0075` accepts them.
6. **Fonts tab rows** drawn in SVG fonts (criterion 6): browsers do not render SVG fonts in CSS, so the name is drawn from the engine's outlines.
7. **Licence record:** whether `font.toml` and its test allow a `licence = "Hershey"` notice (only if Frage 18 is A); ADR 0006 item 5.
8. **Cuttable check** skips single-line faces (criterion 5); where that rule sits.
9. Wasm size: the added reader stays inside the text engine's budget (1.4 MiB, ADR 0020 dated note) or the plan says by how much it grows.

## UX notes

In [`ux-notes.md`](ux-notes.md) (ux-engineer, 2026-10-20): the Single line cell and chip, family rows drawn in strokes, Fill hidden for a single-line text, the refusal look and wording, the missing single-line face, Add font with `.svg`, fit at 800 x 600.

## Decided by the customer 2026-10-20

- Frage 17 SL-DEF: **A**. The definition holds: an editable text, stroke only, SVG fonts first, the tag "Single line", Size is the cap-H height on the centre line.
- Frage 18 SL-LICENCE: **A**. The allowed licences are extended by the Hershey notice (attribution to Dr. A. V. Hershey and James Hurt shipped with it). Shipped fonts: Hershey Sans 1-stroke, Hershey Script 1-stroke, EMS Readability, EMS Allure.
- Frage 19 SL-STICK: **A**, plus this from the customer: "stick" fonts do NOT go into the normal font selection at all; the customer wants them only in a dedicated workbench later (the 0063 workbenches). Lead note: "Stick" may mean embroidery (German "Sticken"); to be clarified when that workbench is specified.
- Frage 20 SL-WASM: raise the text-engine wasm budget. Even 2 MiB is acceptable for now; record it in `docs/technical-debt.md`. The new budget is 2 MiB.
- Frage 21 SL-KIND: **A**. The project file stores `text_font.kind = "single_line"` (format version bump); a missing single-line font keeps drawing as a line (fallback EMS Readability, marked missing).

## Decided by the customer 2026-10-23

- Frage 23 SL-BACK: **A**. After switching a text back from a single-line font to an outline font, the stroke stays as it is (criterion 10 as written; `ux-notes.md` §3). This is what was built in PR #171; nothing changes.

## Open questions

None. Frage 17 to 21 and 23 are decided (above).

Not put to the customer (safe default, vetoable at the demo): criterion 9 (stroke switched on at 0.25 mm in the fill colour), criterion 19 (area commands refused on a single-line text), criterion 22 (EMS Readability as the fallback).

## Links

Requirements: R-FONT-002 (single-line group), R-FONT-003, R-FONT-004, R-EDIT-004 (text half), R-MFG-CUT-001 and R-MFG-LASER-001 (later output of the strokes)
Depends on: `specs/0074-text-objects/`, `specs/0075-font-library/`, `specs/0054-combine-open-paths/`, `specs/0068-laserable-lettering/` (Weld v2, PR #169), `specs/0016-boolean-operations/`
ADRs: 0002 §7 (open glyph paths), 0020 (text engine; "Consequences": single-line fonts reuse `text_font`), 0004 §9 and §12, 0006 item 5, 0018 (XML parser, if accepted)
PR: https://github.com/curvyo/curvyo/pull/171

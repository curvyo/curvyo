# Font style buttons: Extra Light, Light, Regular, Bold, Extra Bold and Italic in the Text bar

Status: Ready (2026-10-23; adrs.md and UX notes done)
Priority: Should
Origin: Customer. Request of 2026-10-23 (translated): "Next, could you please build buttons for bold, extrabold, light, extralight and italic, the way it makes sense? If something is not available for a font, just grey it out." The customer left the design to us. Every design choice below that the customer did not state is marked **(Proposal)**; they are in the slice as safe defaults and vetoable at the demo.
Depends on: `0074-text-objects` (text node, Text bar, Select bar Text group, faces per family in the Fonts tab, "Change font"), `0075-font-library`, `0076-single-line-fonts` (an SVG font usually has one face), `0077-font-preview` (tiles; builds after `0077` is merged).

## User value

As a maker who lasers names onto boards and plots signs, I want to switch the selected text between Extra Light, Light, Regular, Bold and Extra Bold, and in and out of Italic, with one click each, and see at a glance which of these my font has, so that I can make lettering thicker for a clean cut or lighter for an engraving without hunting through the font list for the right face.

## Field reference

| Tool | Weight and italic | When the font lacks the style |
|---|---|---|
| Inkscape 1.4 | No B or I buttons; a Style dropdown in the Text toolbar lists the family's faces; variable axes in the Text and Font dialog | Not listed |
| Affinity Designer 2 | B and I buttons in the context toolbar beside a style dropdown; they pick the family's bold and italic faces | Style list shows only what exists |
| LightBurn 2.1 | Bold and Italic check boxes in the text options; documented as working "if available for the selected font" ([docs](https://docs.lightburnsoftware.com/2.1/Reference/Text/)) | The box can be set and nothing changes; makers report it per OS ([forum](https://forum.lightburnsoftware.com/t/bold-font-not-working-on-macos/168320), [forum](https://forum.lightburnsoftware.com/t/font-style-selection-not-possible/104773)) |
| Cricut Design Space | A Style dropdown per font (Regular, Bold, Italic, Bold Italic) | Only existing styles listed |

**What we do better.**
- **Five weights and Italic one click away**, not two (B and I) and not a dropdown. Light and Extra Light matter for engraving fine text, Extra Bold for cut letters that must not break.
- **Unavailable is visible before the click**: a greyed button whose tooltip says which font lacks the style. LightBurn lets the maker tick Bold and then nothing happens.
- **Only real faces, never a synthetic bold or slant.** A faked bold changes outlines by an offset the type designer never drew; a cut follows what the canvas shows, so we use the face the font has or nothing.
- **Switching the family keeps the style** (Proposal, criterion 22), and the font preview shows every family in that style (criterion 24), so "show me my name in bold in every cuttable font" works.

## Terms

- **Family, face, block:** as in `0074` and `0075`. A family's faces are the face rows the Fonts tab lists under that family **in the same block** (Curvyo fonts, Included, This computer).
- **Weight class of a face:** the OS/2 `usWeightClass` of an OpenType face; for an SVG font face (`0076`), 700 if its `font-weight` is `bold`, `bolder` or 600 or more, else 400. These are the values the style names of `0074` and `0076` are already made from; never the file name.
- **Weight step:** the band a weight class falls in, the same bands that name the styles today: **Extra Light** 150 to 249, **Light** 250 to 349, **Regular** 350 to 449, **Bold** 650 to 749, **Extra Bold** 750 to 849. Thin (under 150), Medium (450 to 549), Semibold (550 to 649) and Black (850 and over) have no button.
- **Italic face:** a face whose OS/2 `fsSelection` has the italic or the oblique bit, or, without OS/2, a non-zero `post` italic angle; for an SVG font face, `font-style` italic or oblique. Same rule as the style names today.
- **Width:** the OS/2 width class (Condensed, Expanded, ...); normal without OS/2.
- **Target face** of a text for a weight step: the face of the text's family (same block) in that weight step, with the text's width and with the text's italic state. If more than one face qualifies (the same face installed as OTF and TTF, or two classes in one band), a face whose bytes are already loaded wins, then the first in the Fonts tab's order (`adrs.md` decisions 1 and 2: faces in one band have the same style name, so weight class is not a tie-breaker).
- **Italic counterpart** of a text: the face of the text's family (same block) with the text's width, the same weight class band as the text's face (including Thin, Medium, Semibold, Black), and the opposite italic state; an Italic face preferred over an Oblique one, then ties as above (a loaded face first, then the Fonts tab's order).
- **Subject texts:** the selected text objects, or the text in edit mode, or, with neither, the next new text (the tool state of `0074` criterion 13).
- Notices at most 70 characters, in the slots `0074` uses.

## Acceptance criteria

### Part A: the buttons

1. Given the Text bar (Text tool) and the Select bar's Text group (Select tool, a selection that holds a text), then after the Size field there is a **Style** group with, in this order, a weight control of five buttons **Extra Light**, **Light**, **Regular**, **Bold**, **Extra Bold** (one choice at a time, one Tab stop, arrows move focus between them without pressing) and, 12px after it, an **Italic** toggle button. Their accessible names are exactly these words; each tooltip is the name and, for a greyed button, the reason of criterion 6. In edit mode only, the Bold and Italic tooltips name their key: "Bold (Ctrl+B)", "Italic (Ctrl+I)" (Cmd on macOS); outside edit mode the plain name, since the keys work only there (criterion 20). Glyphs and sizes per `ux-notes.md` §1 and §2 (each weight button shows a lowercase "a" drawn from the Source Sans 3 outlines at that weight as SVG paths, not a loaded web font; Italic shows Lucide `Italic`).
2. Given the window at 800 x 600 with the panel open, then nothing of the Text bar or the Text group is cut off and nothing scrolls sideways: the Style group wraps to the next row as a whole group (design system, "Overlay row": bars wrap by whole groups).
3. Given one subject text whose face is in a weight step, then that step's button is shown pressed (`aria-checked`); given a face in Thin, Medium, Semibold or Black, then no weight button is pressed. Given an italic face, then Italic is pressed (`aria-pressed="true"`).
4. Given several subject texts, then a weight button is pressed only when every subject text is in that step; Italic is `aria-pressed="true"` when all are italic, `"false"` when none is, `"mixed"` otherwise.

### Part B: greyed when not available

5. Given one subject text, then a weight button is **greyed** when the text has no target face for that step, and Italic is greyed when the text has no counterpart for the other slant (no italic for an upright face, no upright for an italic one, as criterion 6 names it). A pressed weight button is never greyed; a pressed Italic is greyed when the family has no upright face (it shows the truth: the text is italic and cannot be made upright).
6. Given a greyed button, then it is `aria-disabled` (not `disabled`), drawn at 40 %, has no hover, keeps its Tab stop, and its tooltip has a second line naming what is missing, for example "Not in Great Vibes" or, for a text that is italic, "Not in Source Serif 4 as italic", for Italic while pressed "Not in Lobster as upright"; with subject texts in several families, "Not in any of the 3 fonts" (with the count). A press or Enter on it changes nothing, writes no history step and shows no notice. This is the fourth recorded exception of the design-system row "Hidden, not disabled" (customer request of 2026-10-23); the row is amended in the build PR.
7. Given several subject texts, then a weight button is greyed only when **none** of them has a target face for that step; Italic is greyed only when none of the texts it would change (criterion 12) has an italic counterpart (permit over refuse).
8. Given a subject text in a single-line font with one face (Hershey Sans 1-stroke), then Regular is pressed and the other four weight buttons and Italic are greyed. Given Source Sans 3 Regular from the Included block with the faces of criterion 27, then Extra Light, Light, Regular, Bold and Italic are available and Extra Bold is greyed with the tooltip line "Not in Source Sans 3" (upstream 3.052R has no Extra Bold; this is the greyed state the browser build shows). Given a system family on the desktop that has an Extra Bold face (weight class 750 to 849), then its Extra Bold button is available and a press applies that face.
9. Given a subject text whose face is missing on this computer (`0074` criterion 29), then the buttons use the faces of the stored family name in the first block that lists that family (Curvyo fonts, Included, This computer); the pressed states follow the stored style name; with no such family, all six are greyed with the tooltip line "Font missing". A press replaces the missing reference by the chosen face (the maker's choice, as a pick in the Fonts tab).

### Part C: what a press does

10. Given one subject text and an available weight button, when the maker presses it, then the text's face becomes its target face for that step, applied exactly like a face pick in the Fonts tab (`0074` criteria 13 and 18): the whole text changes, the history gets **one** step "Change font", and Undo brings the previous face back in one step.
11. Given several selected texts, when the maker presses an available weight button, then every text that has a target face for that step gets it, the others keep their face, all in one step "Change font". When at least one text is left as it was, the notice "Bold set on 2 of 3 texts; the others have no Bold." (with the step's name and the counts) shows under the button (status, 3 s); with exactly one text left, the second half reads "1 has no Bold." ("Bold set on 2 of 3 texts; 1 has no Bold.").
12. Given Italic, when the maker presses it, then: if Italic is pressed (all italic), every subject text gets its upright counterpart; otherwise (none or mixed) every non-italic subject text gets its italic counterpart and italic texts stay; in both cases texts without a counterpart keep their face, with the notice "Italic set on 2 of 3 texts; the others have no italic." (or "Upright set on ...; the others have no upright face.") when any is left; with exactly one left, "...; 1 has no italic." or "...; 1 has no upright face.". One step "Change font".
13. Given a weight press on an italic text, then the result is italic (the target face keeps the italic state); given Italic on a Bold text, then the result is Bold Italic. Weight and Italic never undo each other (Bold, then Italic, then Light on Source Sans 3 Regular with the faces of criterion 27 gives Source Sans 3 Light Italic).
14. Given a face change by these buttons, then the text keeps its string, anchor, transform, Size and style (fill, stroke): with Size 20 mm the capital H of the new face is 20.00 mm high within 0.01 mm (`0074` criterion 12), and the text's line box is re-measured for the new face.
15. Given no text selected and no edit mode (Text tool), when the maker presses a weight button or Italic, then the face for the next new text changes by the same rules, and no history step is written (`0074` criterion 13). The buttons' greyed and pressed states follow that next face.
16. Given edit mode, when the maker presses a button with the mouse, then edit mode stays, the face changes for the whole text and the focus returns to the edit sink with the caret where it was, so typing continues; the edit session and the face change are separate history steps ("Change font" is committed as `0074` commits a Font pick in edit mode).
17. Given a press, then the canvas shows the text in its new face within 200 ms for a 500-character text whose new face's bytes are already loaded; a face not loaded yet is loaded first, as for a pick in the Fonts tab, without blocking the canvas.
18. Given the Fonts tab is open, then after a press its current mark moves to the new face row (`0074` criterion 18), and the Font button names the new face ("Source Sans 3 Bold Italic").

### Part D: keys

19. Given edit mode, then **Ctrl+B** (Cmd+B on macOS) acts as a press on Bold, and on Regular when the text is already Bold; **Ctrl+I** (Cmd+I) acts as a press on Italic. A greyed target does nothing. Both are taken by the edit sink before the browser (Firefox's bookmarks sidebar and page info do not open). (Proposal)
20. Given no edit mode, then Ctrl+B and Ctrl+I do nothing new in this slice: no weight or italic key outside edit mode (Ctrl+I stays free for the Import image Proposal of `0058`). The other three weights have no key. (Proposal)
21. Given the keyboard in the bar, then the weight control is one Tab stop with roving focus over all five buttons (greyed ones included, so their tooltip is reachable); Left/Right and Up/Down move focus only and never press (no face load, no history step per arrow), Home and End go to Extra Light and Extra Bold, no wrap; only Enter or Space presses; Escape returns focus to the canvas (to the edit sink in edit mode), as the Size field does.

### Part E: the style follows the font (Proposal; changes accepted text of `0074` and `0077`)

22. Given a press on a **family** row in the Fonts tab, then each subject text gets the face of that family that matches its current weight step (or weight band) and italic state and width by the target-face rule; when the family has no such face, its Regular (or first) face as today. A press on a **face** row still picks exactly that face. (Amends `0074` criterion 18, which always picks the Regular face; Inkscape behaves this way when the family changes.)
23. Given several subject texts with different styles, when a family row is pressed, then each text keeps its own style where the family has it, in one step "Change font".
24. Given the font preview (`0077`), then each tile shows the family in the face a press on that tile would apply by criterion 22 for the subject texts: Bold Italic for a Bold Italic text where the family has it, else the family's Regular (or first) face; with several subject texts of different styles, the Regular face. A press on a tile applies by criterion 22, and the missing-glyph marker of `0077` criterion 18 checks the shown face. (Amends `0077` criteria 14 and 19 and its out-of-scope line "Tiles per face".)
25. Given a tile that shows a face other than the family's Regular, then its family name line names the face ("Lobster" stays "Lobster" for a one-face family; "Source Serif 4 Bold Italic" for a styled one), so the maker sees which face a press applies.

### Part F: file and engine

26. Given a project whose texts were changed by these buttons, when it is saved, closed and opened again, then every text has the same face; nothing new is stored (the font reference of `0074` criterion 28), and there is no `format_version` change.
27. Given the Included block, then Source Sans 3 ships four more faces, Extra Light, Extra Light Italic, Light and Light Italic, in addition to Regular, Bold, Italic and Bold Italic, unmodified from the same upstream release 3.052R and licence (SIL OFL 1.1, already recorded), loaded on first use like the other included faces, so every button except Extra Bold can be tried in the browser build and Extra Bold shows the greyed state (criterion 8). Upstream has no Extra Bold; it is tried on the desktop with a system font. (Proposal; Source Serif 4 and Great Vibes stay as they are.)
28. Given the core crates, then the weight class, width and italic state of a face come from bytes in `curvyo-text-core` (OpenType and SVG fonts, the same reads that make the style names today), and the system listing passes the same three values from the platform crate; the choice of a target face is a pure function with unit tests for: a family with all five steps, a family with Semibold and Black only (no step pressed, Bold and Extra Bold greyed), a family with the same face as OTF and TTF, a Condensed family, an Oblique-only family, a single-face SVG font. Nothing reads a file or the clock in the core.

## Out of scope

- Synthetic (faked) bold, slant or outline thickening for families without the face; a "nearest weight" fallback (Black for a missing Extra Bold, Semibold for a missing Bold) (Proposal: strict, as the customer asked for greyed).
- Buttons for Thin, Medium, Semibold, Black, and for widths (Condensed, Expanded); they stay reachable as face rows in the Fonts tab.
- Variable-font axes and named instances: the engine uses a variable font's default instance only (`0074` criterion 21), so a variable family counts as one face here.
- Per-character weight or italic (rich text), underline, strike-through, small caps.
- Weight or italic keys outside edit mode, and keys for Light, Extra Light, Extra Bold.
- More faces for Source Serif 4 or Great Vibes; downloading faces.
- A hover preview of a weight on the canvas before the press.
- Changing how the Fonts tab lists faces (order, grouping by weight).

## Open questions

None for the customer. Proposal items vetoable at the demo: greyed buttons keep their Tab stop (criterion 6), strict matching without nearest-weight fallback (Out of scope), Ctrl+B and Ctrl+I in edit mode only (criteria 19 and 20), the family pick and the preview tiles keeping the style (criteria 22 to 25), the extra Source Sans 3 faces (criterion 27).

## UX notes

In `ux-notes.md` (same folder). Its §0 clarifications 1, 3, 4 and 5 are taken into criteria 1, 6, 11, 12 and 21; clarification 2 (glyphs as SVG paths) into criterion 1.

## What the architect must decide (`adrs.md`)

Done 2026-10-23 in `adrs.md`; its notes for the product owner are taken into the Terms (tie rule) and criteria 8 and 27.

1. Where the weight class, width and italic state of a listed face live (Curvyo and Included listings from `curvyo-text-core`, the system listing from `fontdb` in `curvyo-storage-io`) and how they reach the front end (the face list the Fonts tab already gets).
2. Whether the target-face choice runs in `curvyo-text-core` or the editor glue, and whether "Change font" for several texts with different target faces needs a new editor command or is the existing pick per text inside one step.
3. The licence record and bundle size of the extra Source Sans 3 faces (criterion 27).

## Links

Requirements: R-FONT-010 (new, to be added: weight and italic buttons for a text), R-EDIT-048, R-FONT-005
Depends on: `specs/0074-text-objects/`, `specs/0075-font-library/`, `specs/0076-single-line-fonts/`, `specs/0077-font-preview/`
Changes accepted text of: `0074` criterion 18 (family pick keeps the style), `0077` criteria 14 and 19 (tiles in the subject's style); both Proposal, criteria 22 to 25
ADRs: 0020 (text engine and font sources)
PR: -

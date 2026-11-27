# QR codes, barcodes and Data Matrix: live code objects with styles, a logo clear area and colours, cut-ready for laser and plotter

Status: Slice 1 (QR) Done (PR #TBD, 2026-11-26); slices 2 and 3 Draft. Was: Ready for slice 1 (QR, revision 2, 2026-11-26; Frage 321 B; ADR 0091 Proposed, `adrs.md` and UX notes revision 1 exist; Fragen 319, 320, 321, 322, 323 and 406 stand at their defaults until the customer answers). Slices 2 (1D barcodes) and 3 (Data Matrix) stay Draft: their criteria are outlined, ADR 0091 §9 and the UX notes §11 are outlines only.
Priority: Should
Origin: Customer (2026-11-20, translated): generate QR codes as SVG paths, also "pretty" ones (a logo field in the centre, different styles: module shapes, rounded, dots, finder-pattern styles), plus 1D barcodes (Code 128, EAN/UPC, Code 39) and Data Matrix; also coloured (fill colours, gradients where sensible); everything somewhat adjustable (gap between modules, module size, rounding, error-correction level, quiet zone, logo size and clear area). Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As a maker I want to type a text, pick QR, Data Matrix or a barcode and get a code I can engrave, cut or plot at an exact size, restyle it (round modules, dots, a logo in the middle, colours) and change the text later without redrawing, so that serial plates, labels, name tags and product stickers come straight from the drawing and still scan.

**What other tools do.** Inkscape: Extensions > Render > Barcode > QR Code / Barcode / Datamatrix (one-shot dialog, square modules only, a plain group of rects afterwards, no logo, no styles). LightBurn: a Barcode tool for QR, Data Matrix, Code 128/39, EAN/UPC as a live object with text, size and error level, but square modules only and no logo, no styles, no colours. Online QR generators (QR Code Monkey and similar) offer styles and logos but are web services with downloads. We do better: styles, a safe logo clear area, colours and gradients, and cut-ready output (merged outlines or separate modules), all local and kept live (change the text, keep the style, Convert to paths when done).

## Terms

- **Code object:** one live object in the document: Symbology, Content and Style parameters; its geometry is computed from them (like the generator objects of `0141`). Convert to paths turns it into plain paths and drops the parameters.
- **Module:** one square cell of a 2D code or the narrowest bar of a 1D code. **Module size** is its edge (2D) or the narrow bar width (1D, the "X dimension"), in mm.
- **Quiet zone:** the empty margin around the code, in modules (the standard minimum is part of the default).
- **Function patterns (QR):** the three finder patterns, the alignment patterns, timing patterns and format/version areas. They are never knocked out and keep their shape within the limits of criterion 8.
- **Clear area:** the part of the QR centre left empty for a logo. The logo is not part of the code (Frage 320).
- **Merged / Separate (output):** Merged = adjacent dark modules are one outline, one compound path with holes (`0035`); Separate = one closed path per module.

One-line meaning (customer rule): **Code:** "A text turned into a scannable pattern of squares, dots or bars at a size you set."

## Relation to neighbouring specs

- **`0141` parametric generators, `0065` extensions SDK, `0099` sketch instances:** the same kind of thing: pure function (parameters) to paths, live object with typed parameters in the Properties panel (not a dialog), one undo step per change, Convert to paths, parameters saved in the file, scaling changes the size parameters, no hidden transform. Whether a Code object is the second or third generator of that one mechanism or its own object kind is **Frage 319**.
- **`0124` gradient fill:** the code's fill is an ordinary fill of the object, so it can be a Gradient; the gradient is laid over the code's box (all modules shade by position). No per-module colours.
- **`0035` combine:** a Merged code is one compound path with holes; no new path kind.
- **`0101` laser operations:** operations assign to a Code object like to any item: Fill/Image engraves the dark modules, Line cuts their outlines; the Inverted switch decides which modules are the dark ones.
- **`0111`/`0112` kerf and materials:** not used; a bar width adjustment (criterion 11) is the only burn-spread compensation here.
- **Fonts / text (`0074`, `0062`):** the human-readable text under a 1D code uses the font engine and a font from the maker's fonts.

## Acceptance criteria (as far as clear)

Pure function and data (tests-first, in the core crate, no I/O; in slice 1 criteria 3 and 4 cover QR only, the other symbologies come with slices 2 and 3)

1. Given `encode(content, symbology, options)` in a `*-core` crate (`curvyo-codes-core`, ADR 0091; CLAUDE.md §6: no filesystem, network, clock, threads), then it returns a module grid (2D) or a run list of bar and space widths in modules (1D), or a typed error (`thiserror`); the same input gives the identical output on every platform (tested native and `wasm32-unknown-unknown`). Given `style(grid, params)` then it returns the paths in mm; encode and style are separate public functions (the generator runs both on each edit, no cache, 7 ms at version 40; **revised 2026-11-26**, was "a style change never re-encodes").
2. Given the content "Hello" with QR error correction M, then the grid is 21 x 21 modules (version 1); with module size 1.0 mm and quiet zone 4 the **Width parameter** is 29.0 mm (within 0.001 mm); the object's box (selection box, W readout, Align, the gradient frame) is the symbol, 21.0 x 21.0 mm, without the quiet zone (**revised 2026-11-26**, ADR 0091 §4; a box with the quiet zone needs a document-model change and is not in slice 1). Given 2953 bytes at level L, then version 40 (177 x 177); given 2954 bytes, then a typed "too long" error with the limit.
3. Given each symbology, then known vectors encode to the same modules as the standard's examples: EAN-13 `4006381333931` is 95 modules wide plus the quiet zones (11 left, 7 right); a 12-digit input gets its check digit appended and shown; a 13-digit input with a wrong check digit is corrected to the right one with a chip "Check digit corrected to 1" (permit over refuse); Code 128 "Curvyo" (set B) is 101 modules (start 11, six symbols 66, check 11, stop 13); Code 39 "CURVYO" at wide:narrow 3:1 is 127 modules including start and stop `*`; Data Matrix ECC 200 "123456" is 10 x 10.
4. Given the output of any code at 10 pixels per module rendered as a bitmap, then an **independent decoder** (a test-only dev-dependency) reads the content back: QR versions 1 to 10 at L, M, Q and H, Data Matrix 10 x 10 to 32 x 32, EAN-13, UPC-A, Code 128, Code 39. This test is the definition of "scannable" for criteria 7 to 9.

The object and its parameters (slice 1 is QR, Frage 321 B)

5. Given the Generators entry of `0141` (toolbox card `More tools`, **Proposal**) offers a **Code** segment, then pressing it places a QR code at the pointer, centred, with the content "https://example.com" **(Proposal: a visible, harmless default)**, selected, its parameters in the Properties panel (groups Content, Size, Modules, Finder, Error correction, Logo, Colours, Advanced, UX notes §3), one undo step **"Add QR code"** (**revised**, was "Add code"; **Proposal**). Parameters in slice 1: Content (multi-line text field), Module size (mm, default 1.0, 0.1 to 20), **Width** (mm, derived: the outer width including the quiet zone; typing or dragging it writes Module size = Width / (modules + 2 x quiet zone), the 29 mm of criterion 2), Error correction (L, M, Q, H; default M), Quiet zone (modules, default 4, in the collapsed Advanced group; 0 to 10), Inverted (off). **There is no Symbology field in slice 1** (the kind is QR, ADR 0091 §2; **revised**); the Symbology choice arrives with slice 2, where Quiet zone defaults per symbology (1 for Data Matrix, 10 for 1D) as before. Turning the logo clear area on raises Error correction to H in the same step when it is lower (to the highest level the content fits); turning it off leaves the level (**revised**, UX notes §6, **Proposal**).
6. Given a QR code, then the **module style** parameters are: Module shape (Square, Rounded square, Dot, Diamond), Rounding (0 to 50 % of the module edge, Rounded square only; 50 % is a circle), Module fill (50 to 100 % of the cell: the gap between modules is the rest; 100 % = touching), and Finder style: Outer shape (Square or Rounded; **Circle is Later**, lead decision 2026-11-26 **(Proposal)**: the independent decoder cannot read a circular outer finder, so it returns after a phone check or a second decoder; a stored `circle` reads the default) and Inner shape (Square, Rounded or Circle), independent of the module shape. Each change is one undo step and regenerates within the budget of criterion 17. At Module fill 100 %, Square, the output is the same dark area as the straight grid within 0.001 mm. The finder patterns are drawn by the Finder style, not by the module style, with radii as fixed fractions of the module edge **(Proposal)**.
7. Given a Square or Rounded square code at Module fill 100 %, then the default output is **Merged**: touching dark modules form one outline, the finder rings and any enclosed light area become holes of one compound path; given Dot, Diamond or Module fill below 100 %, then the modules are **Separate**. The **Output** choice (Merged / Separate, Advanced group) is shown only where both make sense, Square or Rounded square at Module fill 100 %; elsewhere the row is hidden and the output is Separate (the stored choice is kept and comes back at 100 %; **revised**, UX notes §3 and §12.7, **Proposal**). At a corner where two dark modules touch only diagonally they stay two loops (no pinch vertex); Rounded square rounds convex corners only. Merged has no more nodes than 4 per module corner run (no boolean library: the outline is traced from the grid). Test: a QR version 1 at 100 % Square merged has the same area as the sum of its dark modules (within 0.001 mm^2) and no overlapping edges.
8. Given a QR code with a **logo clear area** (Shape: Square, Rounded, Circle, default Circle; Size: percent of the **symbol width without the quiet zone**, 5 to 35 %, default 20 %; Margin: 0 to 2 modules, default 1), then the modules whose centre lies inside the shape (grown by Margin, whole modules, centred, symmetric), dark or light, are cleared and no finder, separator, timing, format or version pattern is touched (the size is limited first); an alignment pattern inside the area, from version 7 the one in the centre, is kept and shows through or around the logo (**Proposal**, UX notes §12.5). The area is limited by the **error-correction budget** (ADR 0091 §5, **revised**, replaces the 4 / 9 / 15 / 20 % table, which failed its own decoder test at small versions): the number of cleared non-function modules is at most 0.4 x 8 x E, where E is the number of correctable codewords of the version and level. A larger request is limited to the budget (permit over refuse; the stored Size is the limited value) with the status text in the house clamp form, "Logo size set to 24 %, the most error correction M allows." (likewise "Logo margin set to 1 module, the most error correction M allows."; **revised** from the chip "Clear area limited by error correction M", UX notes §12.2); a higher level, or a lower one that shrinks the budget, is a knock-on with the same sentence; the maker may raise the level. Test: with the largest allowed clear area at each level, the decoder of criterion 4 reads the content back for versions 3, 5 and 10 and every level; the budget is not blindly trusted: the test also covers versions 1 to 10 at the default size.
9. Given a QR code in slice 1, then its colour is the object's ordinary **Fill** (Colour or Gradient, `0124`; the gradient is laid over the code's box, default black, no stroke); given the contrast between the darkest fill colour (for a gradient its lightest stop is the worst case against a light ground) and the document background is below 3 : 1 (WCAG), then a status line "Low contrast, may not scan" shows in the Colours group (a notice for something to act on; none for a light code on a dark document background, which most scanners read; none on a laser page, where colours are operations, `0101`; **Proposal**, UX notes §7 and §12.8). **A separate Finder colour and a Background rectangle are not in slice 1** (ADR 0091 §8, they need per-part paint and a `format_version` change): see Frage 406, default B; they become their own later slice when the customer asks for it.
10. Given an **Inverted** code, then the dark and light modules swap (Background shape and quiet zone stay); the output is then what is engraved or cut for a stencil. Cut-out stencils need bridges to keep islands (the finder rings' inner squares, the inside of an O) from falling out; bridges are out of scope (Frage 322); a Merged inverted code is one compound path with its islands as holes' contents, so Break apart and the maker's own bridges work. Inverted is one rectangle of symbol plus quiet zone with the ink reversed inside it, so the box of an inverted code includes the quiet zone (ADR 0091 §4).

1D codes and Data Matrix (the later slices, Frage 321)

11. Given a 1D code (Code 128, Code 39, EAN-13, EAN-8, UPC-A), then its parameters are Content, Module width (mm, default 0.33), Bar height (mm, default 15), Quiet zone (modules, default 10, 11 and 7 for EAN/UPC as the standard says), Bar width adjust (-30 to +30 % of the module, default 0, **Proposal**: a laser burns bars wider and a cut kerf narrows them, so the maker compensates), Human-readable text (on for EAN and UPC, off for Code 128 and 39), with the font picked from the maker's fonts. Bars are Merged by default (neighbouring bars of one group are separate rectangles anyway); Code 39 has Wide:narrow (2.0 to 3.0, default 3.0).
12. Given Data Matrix, then the parameters are Content, Module size, Quiet zone (default 1), Shape (square sizes 10 x 10 to 144 x 144 by the smallest that fits; rectangular sizes are Later) and the module styles of criterion 6 for the data modules; the **L-shaped finder and the alternating clock edge are always solid squares** (they are not styled). Decoder test as in criterion 4.

Everywhere

13. Given invalid or too long content, then the field shows the reason in place (red text, no dialog) and the object **keeps its last valid geometry** (it does not vanish); given Content empty, then the last valid geometry is kept the same way; a QR with Content that fits only in a higher version than 40 shows the limit in the message, in the unit the encoder used ("Too long for a QR code at error correction M: 2412 bytes, at most 2331."). A commit of such a text **is stored** with the last valid paths (**revised**, ADR 0091 §3, UX notes §5; so lowering the error correction can make the same text fit without retyping): the canvas draws the kept outline in the "No solution" look (screen only, never exported or cut) and the line stays while the object is selected, also after reopening; an error-correction level the content does not fit is lowered to the highest that fits. The content never appears in a log, an undo step name or a notice.
14. Given a Code object, then it moves, rotates, mirrors and aligns like any object; scaling by a corner handle changes Module size (so the numbers stay true, clamped to 0.1 to 20 mm) and a non-uniform scale is refused for 2D codes; a change that alters the code's size (content, Module size, Quiet zone) keeps its centre in place (**Proposal**) and changes bar height and module width for 1D codes; it takes strokes, fills and operations like a path.
15. Given Convert to paths, then the object becomes **one compound path**, for a Merged and for a Separate code alike (one part, ADR 0091 §4, **revised**: was one path per module for Separate); Break apart (`0035`) gives one path per module (for 1D codes the text as outlines, slice 2) in one undo step, with parameters gone; the SVG export of a Code object is those paths, so no Curvyo-specific markup is needed in the SVG.
16. Given save and reopen, then Symbology, Content and every parameter survive and the paths are recomputed; a file with a symbology this build does not know opens with its last stored paths as plain paths and a notice **(Proposal)**; **slice 1 changes no `format_version`** (a new generator kind is additive, ADR 0091 §12); a file with a symbology this build does not know is an unknown kind and opens with its stored outline and the notice of ADR 0080 §5.
17. Given the budgets, measured as thread CPU time in `curvyo-editor-wasm/tests/cpu_time/`: a QR up to version 10 (57 x 57) regenerates within 20 ms and a version 40 code within 100 ms for Square Merged and for Dot Separate, including the style step (**Proposal**, ADR 0091 §10). Measured 2026-11-26 (release, thread CPU time, one edit with commit and canvas view): version 10 3.8 ms Merged and 8.0 ms Dot, version 40 42 ms Merged and 94 ms Dot (the generator is 11 ms of it, the rest is the commit of 16,000 subpaths; saved file 2.2 MB at version 40 Dot, 167 KB at version 10 Dot); version 40 Dot has 6 % headroom.
18. Given the code is generated, then **no data leaves the machine**: the core crate and its decoder test dependency have no network or font-service dependency (`cargo tree` shows none), and the content is never logged.
19. Given golden-file tests (CLAUDE.md §5), then `curvyo-codes-core/tests/fixtures/` holds the SVG of at least, in slice 1: QR v1 Square merged, QR v3 Rounded with logo circle at level H, QR Dot separate, QR inverted merged; slices 2 and 3 add Data Matrix 10 x 10, EAN-13 and Code 128; each compares to 0.001 mm.

## Crate candidates (for the architect; none decided, all licences to be checked with `cargo deny`)

QR: `qrcode` (MIT or Apache-2.0, encoding only, plain module grid), `fast_qr` (MIT or Apache-2.0, fast, grid); the logo clear area and styles are ours either way. Several symbologies in one crate: `rxing` (Apache-2.0, a port of ZXing: QR, Data Matrix, Code 128/39, EAN/UPC encoders and decoders; large). 1D only: `barcoders` (MIT or Apache-2.0). Data Matrix: `datamatrix` (check licence). Decoder for the test only: `rqrr` (MIT, QR), or `rxing` as the all-in-one test decoder. One option is a small own encoder for 1D (they are simple, 150 lines each) and a crate for QR and Data Matrix; the architect decides, the PR justifies each dependency (CLAUDE.md §3).

## Out of scope

- Online services, short links, tracking redirects, dynamic QR codes, accounts (data stays on the machine, criterion 18).
- Content helpers (Wi-Fi, vCard, SEPA forms), batches and serial numbers from a table (Later; the Content field takes any text).
- Stencil bridges for cut-out codes, camera scan check inside the app, print-and-cut registration of codes (`0121`).
- Other symbologies: PDF417, Aztec, Code 93, ITF, GS1 DataBar, Micro QR, rectangular Data Matrix (Later, same contract).
- A logo stored inside the code object (Frage 320 B) and per-module colour, image-filled modules, gradient per module.
- Smoothing adjacent modules into liquid blobs (Later, a Module shape).

## Questions for the customer

Defaults apply if unanswered; nothing blocks. **Fragen 319 A, 320 A, 321 B, 322 A, 323 A are recorded as customer-pending defaults** (built as the default; a different answer changes only what its Frage names); Frage 406 is new.

**Frage 319 (CODE-HOME): Is a Code object the same kind of thing as the box and gear generators of `0141`?**
- A: Yes. One mechanism (extension contract of `0065`, live parametric instance shared with `0099`), the Code is its second or third generator.
- B: No. A Code is its own object kind in the document, outside the extension contract.
- C: Build the pure functions now (no object), and decide the object after `0141`'s ADR.
Recommendation: A, because a third generator is the real test of the contract and the Properties panel, the undo, Convert to paths and the file record come once. Default: A; criteria 1 to 4 and 19 (the core, invisible work) are built first and need nothing of `0141`, so the order of the two specs does not block.

**Frage 320 (CODE-LOGO): What is the logo?**
- A: Only a clear area; the maker places the logo (any drawing, a picture, text) over it and groups both.
- B: The Code holds the chosen artwork as a child (a button "Use selection as logo"), scaled into the clear area and moving with the code.
- C: A now, B later.
Recommendation: C. Default: A (B needs a child inside a generated object, a model question for `0141`).

**Frage 321 (CODE-SCOPE): How many symbologies in the first PR?**
- A: All (QR, Data Matrix, Code 128, Code 39, EAN/UPC) in one PR.
- B: Three slices, each its own demo: 1) QR with styles, logo, colours (criteria 1 to 10, 13 to 19); 2) 1D barcodes; 3) Data Matrix.
- C: QR only, the rest later.
Recommendation: B (thin slices, QR is what you described first and most). Default: B.

**Frage 322 (CODE-STENCIL): Bridges so an inverted, cut-out code holds together?**
- A: Not now; engrave inverted codes, cut Merged ones (the finder rings' islands are the maker's to bridge).
- B: A Bridges option (width in mm) in this spec.
Recommendation: A (a cut-out QR is rare; LightBurn has none). Default: A.

**Frage 323 (CODE-TEXT): Human-readable text under the 1D codes in the 1D slice?**
- A: Yes, with font choice and size, as outlines.
- B: Not in the first 1D slice; the maker adds text by hand.
Recommendation: A (EAN and UPC without digits look wrong, and the font engine exists). Default: A. (Slice 2; needs glyph outlines passed into the generator, ADR 0091 §9.)

**Frage 406 (CODE-PAINT): Own colour for the three corner squares, and a background colour?**
- A: In slice 1, as part of the first QR PR (a file-format change: every view and export learns colours per piece of one object).
- B: Slice 1 has one fill colour or gradient for the whole code (criterion 9); finder colour and background come later as their own slice.
- C: None; the maker recolours after Convert to paths and Break apart.
Recommendation: B (the format change is the riskiest part, and the code is pretty and scannable without it). Default: B.

## UX notes

See `specs/0144-qr-and-barcodes/ux-notes.md` (revision 1, 2026-11-26, Proposals; slice 1 in full, slices 2 and 3 outlined; read together with the decisions of revision 2 below): the Code segment in the Generators bar, the groups Content, Size, Modules, Finder, Error correction, Logo, Colours and Advanced, the multi-line Content field with its lasting error line and the No solution outline on kept geometry, Error correction raised to H by the clear area, clamp texts, the clear area outline, the contrast line, tracks; eleven points for the product owner and the architect in section 12.

**Decisions of revision 2 (2026-11-26, product owner, all Proposal, vetoable at the demo) on UX notes section 12:** 1 step name "Add QR code" (criterion 5); 2 the house clamp form (criterion 8); 3 invalid content stored with kept geometry (criterion 13, ADR 0091 §3); 4 option a, the switch raises to H, off leaves the level (criterion 5); 5 the centre alignment pattern is accepted, the area stays centred (criterion 8); 6 Size is a percent of the symbol width without the quiet zone (criterion 8); 7 Output hidden below 100 % fill (criterion 7); 8 contrast against the document background, worst stop, hidden on laser pages (criterion 9); 9 schema elements are the architect's (ADR 0091 §3); 10 box without the quiet zone in slice 1, Width carries it (criterion 2); 11 no check on export. Where UX notes disagree with ADR 0091 (§9 Separate converts to a group; §7 and §12.9 Finder colour and Background; §12.10), this spec follows the ADR: criteria 2, 9 and 15.

## Links

Requirements: R-EDIT-056 (new)
Depends on: `0141` (the generator mechanism, Frage 319 A, built), `0124` (the gradient fill, done); ADR 0091 (Proposed; slice 1 built, see its implementation notes)
Related: `0035`, `0065`, `0099`, `0101`, `0121`
PR: slice 1 #TBD

# QR codes, barcodes and Data Matrix: live code objects with styles, a logo clear area and colours, cut-ready for laser and plotter

Status: Draft (2026-11-20; Fragen 319 to 323 open, every one has a default; not Ready: `adrs.md` (crate, dependency choice, the instance mechanism of `0141`) and UX notes are missing)
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

Pure function and data (tests-first, in the core crate, no I/O)

1. Given `encode(content, symbology, options)` in a `*-core` crate (CLAUDE.md §6: no filesystem, network, clock, threads), then it returns a module grid (2D) or a run list of bar and space widths in modules (1D), or a typed error (`thiserror`); the same input gives the identical output on every platform (tested native and `wasm32-unknown-unknown`). Given `style(grid, params)` then it returns the paths in mm; the two steps are separate functions so a style change never re-encodes.
2. Given the content "Hello" with QR error correction M, then the grid is 21 x 21 modules (version 1); with module size 1.0 mm and quiet zone 4 the object is 29.0 x 29.0 mm (within 0.001 mm). Given 2953 bytes at level L, then version 40 (177 x 177); given 2954 bytes, then a typed "too long" error with the limit.
3. Given each symbology, then known vectors encode to the same modules as the standard's examples: EAN-13 `4006381333931` is 95 modules wide plus the quiet zones (11 left, 7 right); a 12-digit input gets its check digit appended and shown; a 13-digit input with a wrong check digit is corrected to the right one with a chip "Check digit corrected to 1" (permit over refuse); Code 128 "Curvyo" (set B) is 101 modules (start 11, six symbols 66, check 11, stop 13); Code 39 "CURVYO" at wide:narrow 3:1 is 127 modules including start and stop `*`; Data Matrix ECC 200 "123456" is 10 x 10.
4. Given the output of any code at 10 pixels per module rendered as a bitmap, then an **independent decoder** (a test-only dev-dependency) reads the content back: QR versions 1 to 10 at L, M, Q and H, Data Matrix 10 x 10 to 32 x 32, EAN-13, UPC-A, Code 128, Code 39. This test is the definition of "scannable" for criteria 7 to 9.

The object and its parameters (the first slice is QR, Frage 321)

5. Given the Generators entry of `0141` (toolbox card `More tools`, **Proposal**) offers Code, then pressing it places a QR code at the pointer with the content "https://example.com" **(Proposal: a visible, harmless default)**, selected, its parameters in the Properties panel, one undo step "Add code". Parameters: Symbology, Content (text field, multi-line for QR), Module size (mm, default 1.0, 0.1 to 20), or Width (mm; typing one changes the other), Quiet zone (modules, default 4 for QR, 1 for Data Matrix, 10 for 1D), Error correction (QR: L, M, Q, H; default M, H when a logo is on), Inverted (off).
6. Given a QR code, then the **module style** parameters are: Module shape (Square, Rounded square, Dot, Diamond), Rounding (0 to 50 % of the module edge, Rounded square only; 50 % is a circle), Module fill (50 to 100 % of the cell: the gap between modules is the rest; 100 % = touching), and Finder style: Outer shape and Inner shape each Square, Rounded or Circle, independent of the module shape. Each change is one undo step and regenerates within the budget of criterion 17. At Module fill 100 %, Square, the output is the same dark area as the straight grid within 0.001 mm.
7. Given a Square or Rounded square code at Module fill 100 %, then the default output is **Merged**: touching dark modules form one outline, the finder rings and any enclosed light area become holes of one compound path; given Dot, Diamond or Module fill below 100 %, then the modules are **Separate**. The maker may switch Merged/Separate where both make sense. Merged has no more nodes than 4 per module corner run (no boolean library: the outline is traced from the grid). Test: a QR version 1 at 100 % Square merged has the same area as the sum of its dark modules (within 0.001 mm^2) and no overlapping edges.
8. Given a QR code with a **logo clear area** (Shape: Square, Rounded, Circle; Size: percent of the code width, 5 to 35 %; Margin: 0 to 2 modules), then the dark modules inside the area (rounded outward to whole modules, kept centred, symmetric) are removed and no function pattern is touched; the area is limited by the **error-correction budget** (**Proposal**: the removed share of all modules at most 4 % at L, 9 % at M, 15 % at Q, 20 % at H, about 0.6 times the level's recovery capacity); a larger request is limited to the budget with the chip "Clear area limited by error correction M" (permit over refuse), the maker may raise the level. Test: with the largest allowed clear area at each level, the decoder of criterion 4 reads the content back for versions 3, 5 and 10; with the area one module row larger it is allowed to fail (the budget is not blindly trusted).
9. Given a QR code whose colours are set, then the object has Fill (Colour or Gradient, `0124`), optional own Finder colour (finder rings and centres) and an optional Background (the quiet zone and light modules as one rectangle behind, none by default); given the contrast between dark and light is below 3 : 1 (WCAG), then a chip "Low contrast, may not scan" shows (a notice for something to act on; none for a light code on a dark Background, which most scanners read). The Background is part of the object's geometry only when it is on.
10. Given an **Inverted** code, then the dark and light modules swap (Background shape and quiet zone stay); the output is then what is engraved or cut for a stencil. Cut-out stencils need bridges to keep islands (the finder rings' inner squares, the inside of an O) from falling out; bridges are out of scope (Frage 322); a Merged inverted code is one compound path with its islands as holes' contents, so Break apart and the maker's own bridges work.

1D codes and Data Matrix (the later slices, Frage 321)

11. Given a 1D code (Code 128, Code 39, EAN-13, EAN-8, UPC-A), then its parameters are Content, Module width (mm, default 0.33), Bar height (mm, default 15), Quiet zone (modules, default 10, 11 and 7 for EAN/UPC as the standard says), Bar width adjust (-30 to +30 % of the module, default 0, **Proposal**: a laser burns bars wider and a cut kerf narrows them, so the maker compensates), Human-readable text (on for EAN and UPC, off for Code 128 and 39), with the font picked from the maker's fonts. Bars are Merged by default (neighbouring bars of one group are separate rectangles anyway); Code 39 has Wide:narrow (2.0 to 3.0, default 3.0).
12. Given Data Matrix, then the parameters are Content, Module size, Quiet zone (default 1), Shape (square sizes 10 x 10 to 144 x 144 by the smallest that fits; rectangular sizes are Later) and the module styles of criterion 6 for the data modules; the **L-shaped finder and the alternating clock edge are always solid squares** (they are not styled). Decoder test as in criterion 4.

Everywhere

13. Given invalid or too long content, then the field shows the reason in place (red text, no dialog) and the object **keeps its last valid geometry** (it does not vanish); given Content empty, then the last valid geometry is kept the same way; a QR with Content that fits only in a higher version than 40 shows the limit in the message.
14. Given a Code object, then it moves, rotates, mirrors and aligns like any object; scaling by a handle changes Module size (so the numbers stay true) and a non-uniform scale is refused for 2D codes and changes bar height and module width for 1D codes; it takes strokes, fills and operations like a path.
15. Given Convert to paths, then the object becomes ordinary paths (one compound path for a Merged code, one path per module for Separate, the text as outlines) in one undo step, with parameters gone; the SVG export of a Code object is those paths, so no Curvyo-specific markup is needed in the SVG.
16. Given save and reopen, then Symbology, Content and every parameter survive and the paths are recomputed; a file with a symbology this build does not know opens with its last stored paths as plain paths and a notice **(Proposal)**; `format_version` change by the architect (a new object kind or the generator record of `0141`).
17. Given the budgets, measured as thread CPU time in `curvyo-editor-wasm/tests/cpu_time/`: a QR up to version 10 (57 x 57) regenerates within 20 ms and a version 40 code within 100 ms for Square Merged and for Dot Separate, including the style step (**Proposal**, to be corrected by the architect after the first measurement; the number must then be stated, not removed).
18. Given the code is generated, then **no data leaves the machine**: the core crate and its decoder test dependency have no network or font-service dependency (`cargo tree` shows none), and the content is never logged.
19. Given golden-file tests (CLAUDE.md §5), then `tests/fixtures/` holds the SVG of at least: QR v1 Square merged, QR v3 Rounded with logo circle at level H, QR Dot separate, Data Matrix 10 x 10, EAN-13, Code 128; each compares to 0.001 mm.

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

Defaults apply if unanswered; nothing blocks.

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
Recommendation: A (EAN and UPC without digits look wrong, and the font engine exists). Default: A.

## UX notes

(ux-engineer, before Ready: the Generators entry, the Code section of the Properties panel with Style, Logo, Colours and Output groups, the chips of criteria 3, 8, 9 and 13, the Logo clear area as a canvas outline while the section is open.)

## Links

Requirements: R-EDIT-056 (new)
Depends on: `0141` (the generator mechanism, Frage 319 A), `0124` (the gradient fill, done)
Related: `0035`, `0065`, `0099`, `0101`, `0121`
PR: none yet

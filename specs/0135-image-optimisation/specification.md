# Image optimisation: adjust a picture before it burns, by hand, with an Auto proposal and (by decision) local AI tools

Status: Draft (2026-11-19; waiting for Fragen 272 to 276; builds on `0123` raster-engraving, Done)
Priority: Should
Origin: Customer (review of PR #394, 2026-11-19: the engraved result of a test photo is hard to recognise; wants image optimisation "automatic, AI-assisted and manual"). Everything marked **(Proposal)** is the product owner's and vetoable at the demo. Platform, runtime, model and data-leaving decisions are the customer's (`CLAUDE.md` §3) and are asked in Fragen 273 to 275; until answered, slice 3 is not Ready.

## User value

As a maker engraving a photo on wood, slate or anodised metal I want to improve the picture for the laser (tones, edges, noise, background) and see the dots that will burn next to the original, and I want an Auto that proposes the improvements for my material and dot size, so that the first burn is recognisable and I do not need a photo editor and trial burns.

**What other tools do.** LightBurn has one "Adjust image" window (brightness, contrast, gamma, enhance edge, sharpen radius and amount, denoise, a few dither modes) that edits the image object destructively-in-place, previews only in a separate window and has no automatic mode; its background removal does not exist (people use remove.bg or Photoshop). xTool Creative Space and Glowforge have one-button "enhance" and a cloud or local background remover, but the result is a black box: no steps to edit, no reproducible settings, and the picture goes to a server. Inkscape has nothing for pixels. **We do better:** every improvement is an ordinary, editable, undoable step in a visible stack; Auto only fills that stack (never magic); the before/after shows the real dots at the operation's Interval; nothing leaves the machine by default; the job file stays reproducible (derived pixels are recomputed, the one stored exception is decided in Frage 275).

## Words (shown in the UI)

- **Adjustments:** the ordered stack of steps applied to a picture before the dots are made (criterion 1). Steps can be switched off, reordered, edited and removed.
- **Step:** one entry of the stack with its own fields. One-line meaning of each (customer rule: the meaning of a new tool before it is built):
  - **Crop:** "Keeps a rectangle of the picture and drops the rest."
  - **Resize:** "Sets how many pixels the picture has, with a clean resampling." (Reduces only; see criterion 14.)
  - **Channel mix (grey):** "Chooses how colour becomes grey: how much red, green and blue count, so a red lip does not turn the same grey as a green leaf."
  - **Levels:** "Sets which grey counts as black and which as white, so the picture uses the whole range."
  - **Curves:** "Bends the tones: lifts the shadows or darkens the highlights without touching the rest."
  - **Local contrast:** "Brings out detail inside small areas (faces, wood grain) without changing the picture as a whole."
  - **Sharpen:** "Makes edges crisper so they survive being turned into dots."
  - **Denoise:** "Smooths grain and speckle that would turn into stray dots."
  - **Remove background (mask):** "Hides everything except the subject, so only the subject burns."
- **Auto:** "Proposes a stack for this picture, this material and this dot size; you can change every step afterwards."
- **Before and after:** "Shows the original next to the dots that will burn, at the real dot size."
- **Working image, dot grid, dither mode, Interval:** as in `0058` and `0123`.
- **Adjusted image:** the working image after the stack; derived, never stored (criterion 24), except an AI mask (Frage 275).

## Acceptance criteria

All numbers are mm unless a unit is named. "Image operation", "raster item", "Image settings", "dot grid", "Interval" as in `0123`; "working image" as in `0058`; "test grid", "Image set" as in `0112`; "job", "Toolpath view", "checks" as in `0029`. Criteria marked (Frage n) depend on the answer; each states its default. Slices (each ends in something the maker can try): **1** the stack, the manual steps Crop, Channel mix, Levels, Curves, Sharpen, and Before and after (criteria 1 to 24, 33 to 40); **2** Auto, Local contrast, Denoise, Resize (25 to 28 and the steps of 5); **3** AI tools (29 to 32; not Ready before Fragen 273 to 275).

### The stack (manual)

1. Given a selected image, then the Properties panel offers an Adjustments group (a symbol-only add button with tooltip "Add adjustment" opening the step list; Proposal, as the tool-specific bar rule of the customer: UX notes decide the exact place) showing the picture's stack, one row per step: switch (on/off), name, its fields, a remove button. A new picture has an empty stack, and an empty stack changes nothing: the dots equal those of `0123` byte for byte (golden regression, criterion 36).
2. Given steps, then each can be switched off (the picture and dots are as if it were absent, the fields kept), removed, edited in place and moved up or down (drag handle; Proposal: keyboard Alt+Up and Alt+Down); every change is one undo step "Edit adjustments" (`0020`), and one undo of a whole Auto result (criterion 25).
3. Given the stack, then the steps run top to bottom on the working image (RGBA8, `0058`) in the order of the rows; Proposal: a new step is inserted at the sensible place of the fixed pipeline (Crop, Resize, Channel mix, Denoise, Levels, Curves, Local contrast, Sharpen, Mask), and the maker may reorder only within what keeps the result defined (the mask last; Crop and Resize first). Test: adding Sharpen then Levels in either order leaves them in the pipeline order.
4. Given where the stack lives (Frage 272; default C): **A** on the image object, **B** on the Image operation, **C** split: the picture steps (Crop, Resize, Channel mix, Denoise, Levels, Curves, Local contrast, Mask) on the image object, so every operation, the Select view and a clone see the same picture; the dot-scale steps (Sharpen, and the tone fields Brightness, Contrast, Gamma, Invert, Threshold of `0123` that stay as built) on the Image operation, because they depend on the Interval and the material. Criteria 5 to 24 hold for the steps wherever they live; the rows of the Adjustments group then show the object steps and, below a hairline, a read-only line "Sharpen and tone: set in <operation name>" that selects the operation.
5. Given each step, then its fields are plain number fields with units, never sliders (customer rule; the Levels and Curves graphs are readouts and direct-manipulation handles with the same values in fields), values rounded to the field's step, out-of-range refused with the range in the message (as `0123` criterion 12):
   - **Crop:** left, top, width, height in % of the picture (whole numbers, default the whole picture; width and height at least 1 %). The crop is also dragged on the canvas by a frame with handles. The crop makes the image object's frame the cropped rectangle at the same scale (the visible part keeps its size and place on the page; the object's width and height shrink to match; Proposal) and keeps the removed pixels in the asset, so un-cropping is lossless.
   - **Channel mix (grey):** Red, Green, Blue each -200 to 200 % (whole numbers, default 21.26, 71.52, 7.22 shown as 21, 72, 7; the default is the luminance of `0123` criterion 6); the sum may differ from 100 %; the result is clamped to 0 to 255. Presets as a strip (Luminance, Red filter, Green filter, Blue filter; text segments) set the three fields.
   - **Levels:** Black point 0 to 98 %, White point 2 to 100 % (black below white by at least 2), Gamma 0.10 to 4.00 (two decimals), all on grey; a histogram of the picture's greys (256 bins, drawn as hairline bars in `--panel-muted-fg`; no new colour) with the two points as handles; a button "Stretch" (symbol, tooltip "Stretch to the picture's range") sets Black and White to the 0.5 % and 99.5 % percentiles of the histogram.
   - **Curves:** at most 8 points, each Input % and Output % (0 to 100, whole numbers), the end points at 0 and 100 are fixed in input; the curve is monotone cubic through the points (no overshoot); handles on the graph; a reset button (symbol, tooltip "Straight curve").
   - **Local contrast:** Radius in mm of the page (0.2 to 20.0, one decimal, default 3.0) and Amount 0 to 200 % (default 50); an adaptive histogram method (CLAHE, clip limit from Amount) on tiles of the stated radius; the mm are converted to pixels with the image's pixel pitch on the page.
   - **Sharpen:** Radius in mm (0.05 to 5.00, two decimals, default the Interval) and Amount 0 to 500 % (default 100), threshold 0 to 50 % (default 0): an unsharp mask; with Frage 272 C it runs on the dot grid's grey values after the resample of `0123` criterion 6 (so the radius is in dots of the real engraving), with A or B on the working image at the pixel pitch.
   - **Denoise:** Strength 0 to 100 % (default 30) and Detail kept 0 to 100 % (default 60): an edge-preserving filter (Proposal: non-local-means-style bilateral, architect chooses); mm are not involved.
   - **Resize:** Width in pixels (the height follows; a lock keeps the ratio), 16 to the working image's width; only reduces (criterion 14).
6. Given a step that needs a colour image and the picture is already grey, or a step on a picture with alpha, then alpha is blended over white as in `0123` criterion 6 before any grey step, and Channel mix on a grey picture is accepted and changes nothing (no error).
7. Given the first step that produces grey, then later steps run on grey; before it a colour step (Denoise, Resize, Crop) runs per channel. Test: Denoise before and after Channel mix are both valid and both listed.
8. Given a stack of steps, then it is stored on the object (or operation, criterion 4) in a new register `adjustments` as an ordered list of `{kind, enabled, fields}`; absent or empty is no stack; unknown kinds from a later version are kept and not run, with a check warning (damage rule of ADR 0056). The format bump is the architect's (Proposal: one bump of `format_version`; the project file is unreleased, no migration; Curvyo is not deployed).
9. Given Edit steps in a clone (`0049`) or a placed page (`0106`), then they edit the original's stack; the clone shows the same result. Given a duplicate image object (Ctrl+D), then it has its own copy of the stack.
10. Given Copy and Paste of an image to another document or another page, then its stack travels with it. Given a "Copy adjustments" and "Paste adjustments" pair (two symbol buttons in the group header; Proposal), then the stack of one picture can be applied to another; pasting replaces the target's stack in one undo step.

### Before and after

11. Given a selected image that has an Image operation (or a test-grid cell), then a Before and after view can be opened with a symbol button (tooltip "Before and after") in the Adjustments group: a pane in the canvas area (the split of the font preview `0077`, switchable to full view) shows the original, and the engraved result as the dots, at the operation's Interval, Scan angle and dither, side by side, with a Split switch (a draggable divider instead of two panes; Proposal) and zoom and pan locked together.
12. Given the engraved result in that view, then it is the same dots as the Toolpath view and the file (`0123` criteria 14 and 20) at the zoom shown: at 100 % one dot is Interval mm wide on screen at the document's scale; below one device pixel per dot it is the shaded dot area of `0123` criterion 20. Test: the dot count of a 10 x 10 mm region read from the view equals that of the job.
13. Given an edit of a step's field, then the view updates in the same frame at a reduced level while the pointer or the key is held (the working image's mip level that fits the pane, the dither on the dot grid of the visible part only) and settles to the exact dots within 500 ms of release for the working set of criterion 37; the exact dots are the ones the job will burn.
14. Given Resize, then it can only reduce the working image (there is no enlargement; an enlargement is the AI Upscale of criterion 31 or nothing); Resize never goes above the working limit of `0058` criterion 18. A smaller picture than the dot grid needs is the "coarser than the Interval" warning of `0123` criterion 35, which stays.
15. Given a picture whose stack is non-empty, then the canvas shows the adjusted image (greyscale or as the colour stands after the steps, as the Select view shows an image) with a small symbol on the object when the stack is non-empty and some steps are off. Nothing else about the object's look changes. (Proposal; UX decides.)
16. Given the histogram of Levels, then it is of the picture after the steps above it, live, from the working image at the mip level of the pane.

### Determinism, derived data, caching

17. Given the same image bytes, stack and Interval, then the adjusted image and the dots are identical on every platform (native and wasm): every step except Remove background (criterion 29) uses integer or fixed-point arithmetic only, as `0123` criterion 8 (Proposal: the Gaussian and bilateral kernels are integer tables; the CLAHE uses integer histograms; the curve is a 256-entry integer table). Test: golden outputs byte-identical native and wasm.
18. Given the work, then the adjusted image is derived: computed from the asset bytes of `0058` and the stack, cached in memory by the object's revision and dropped when it changes, never written into the project (the project does not grow by adjusting). The one possible exception is an AI mask (Frage 275).
19. Given the job (`0029`), then the laser-core job builder takes the adjusted image as the "working image" of `0123` criterion 7; the stack steps live in a `*-core` crate (Proposal: the pixel steps in `curvyo-image-core`, the stack and dot-scale integration in `curvyo-laser-core`; the architect decides in `adrs.md`); the job itself stays pixel-free and format-neutral (`0029` criterion 39), the G-code is as built.
20. Given two exports of the same project, then they are byte-identical (`0029` criterion 31).
21. Given an image whose bytes cannot be read, then the stack is kept and the checks warn as `0123` criterion 36; the steps run again when the bytes return.
22. Given an unfolded Image operation row and an image with a stack, then the row is unchanged by this story apart from the line of criterion 4 (Frage 272 C) and the Image settings fields of `0123` as built. There are no machine settings in the Adjustments group.
23. Given the checks (`0029`), then a stack adds no block and at most one warn line: "1 step in the Adjustments of <image name> could not run." / "<n> steps ..." (unknown kind, damaged fields; a button selects the image).
24. Given Undo (`0020`) after any edit of the stack (including Auto, criterion 25, and a mask, criterion 30), then the stack is exactly as before, the Before and after view follows, and no pixel is stored in the history beyond the stack's fields (and, for an AI mask, the asset reference).

### Auto

25. Given a selected image with an Image operation, then an Auto button (symbol, tooltip "Auto adjust") in the Adjustments group replaces the stack with a proposal in one undo step; the result is an ordinary stack (criteria 1 to 5), every step switched on and editable; nothing is hidden and nothing recomputes later when the material changes (Auto is a one-time fill, not a live mode).
26. Given Auto, then the proposal is computed deterministically from the picture, the operation's material set (Image set of `0112`) and its Interval, by these rules (Proposal; numbers tuned on a test set of 20 pictures in the demo): (a) **Levels** from the 0.5 % and 99.5 % histogram percentiles, Gamma chosen so the median grey lands at 45 % for Floyd, Jarvis, Atkinson and Stucki, 50 % for Threshold and Ordered, and 50 % of the material's power range for Grey; (b) **Local contrast** Radius about 100 dots (Radius mm = 100 x Interval), Amount 40 % only when the global contrast (the 5 % to 95 % spread) is below 60 %; (c) **Sharpen** Radius = the Interval and Amount 80 % to 150 % by how much detail the source has finer than the dots (the ratio of the image's pixel pitch to the Interval); (d) **Denoise** only when the source's high-frequency noise estimate is above a stated level; (e) **Channel mix** from the picture's colour only when it has a strong cast (Proposal: a skin-tone default of Red 40 %, Green 50 %, Blue 10 % is offered as a preset, not applied by Auto). Test: three golden pictures give the three golden stacks within one field step.
27. Given Auto on a picture with a transparent or flat background, then it does not remove it (that is a manual or AI step, criterion 29); Auto never crops and never resizes.
28. Given Auto with no Image operation assigned to the picture, then it is disabled with the tooltip "Assign the picture to an Image operation to adjust it for your material." (the maker assigns first); Auto in a test-grid cell uses the grid's Interval.

### AI tools (slice 3; depends on Fragen 273, 274, 275; defaults below)

29. Given the default of Frage 273 (B: Remove background only) and Frage 274 (A: local, model file ships with the app), then a Remove background step (symbol, tooltip "Remove background") runs a small salient-object model on the device, nothing about the picture leaves the machine, no account, no telemetry (`0029` criterion 35); the result is a soft mask (8 bit) of the subject; masked-out pixels count as white (burn nothing). The step has Edge softness 0 to 20 % (whole numbers, default 5) and Threshold 0 to 100 % (default 50) so the maker can tighten or grow the subject; a brush to Keep or Erase parts by hand is an own follow-up (Could, slice 4, Frage 273).
30. Given the mask, then it is stored with the object as a derived asset in `assets/` (an 8-bit PNG, at most the working image's size, referenced by the step) together with the model name and version, so the job is reproducible without the model and a second device needs no AI to burn the same dots (Frage 275 A; default). With Frage 275 B the mask is recomputed from the model, and the step stores only the model version and a warning states that output may differ between runtimes. Test: a project with the mask burns byte-identical G-code on a machine where the model is not installed (A).
31. Given the candidates not in the default (Upscale, AI denoise, "make engravable" enhancement, depth or relief), then they are not built in this story; they are asked in Frage 273 and, if wanted, are separate stories that follow this stack (a step of the same kind of row, stored result as in criterion 30); each requires its own licence and size check (criterion 32).
32. Given a model, then it has a permissive licence compatible with the project's (Apache-2.0, MIT, BSD, CC0; not research-only or non-commercial weights; the architect lists name, licence and size in the ADR; `cargo deny` for the runtime crate), a size stated in the PR (target: under 50 MB for the first model, loaded on first use, not at start), and a CPU fallback with a stated time for a 1 Mpx picture (target: under 5 s on the build host; WebGPU only as an accelerator, never required). The runtime (ONNX Runtime Web, `tract` or `candle` as wasm, or a native crate) is a platform and dependency decision for an ADR marked `needs-customer` (`CLAUDE.md` §3); a `*-core` crate never contains the runtime (inference sits in a platform crate; the core only reads the stored mask).

### Test grids and materials

33. Given the test grid of `0112` (Frage 276; default A), then no new axis is added: the axes stay Power, Speed, Passes and Interval and the Image grid keeps the dither choices of `0123` criterion 44. **B** would add one adjustment axis (Gamma or Contrast of Levels, 5 values) to the Image grid; **C** a stack preset pick for the whole grid. The test picture of a grid is a built-in grey ramp that has no stack.
34. Given a material's Image set (`0123`, Frage 270 B), then it stores no adjustment and no stack (the picture belongs to the design, the set to the machine and material); Auto reads the set but never writes it.

### Cut-out and the contour (out of scope here)

35. Given an image with a mask, then the mask changes the dots only; Cut-out still cuts the frame rectangle (`0123` criterion 1); following the contour of the subject (or of a transparent background) is a later idea (Out of scope).

### Quality

36. Golden files (`CLAUDE.md` §5): `curvyo-image-core/tests/fixtures/adjust/` holds small inputs and the expected adjusted images as PNG per step (`levels_stretch`, `levels_gamma`, `curves_s`, `channel_mix_red_filter`, `sharpen_unsharp`, `local_contrast_clahe`, `denoise_nlm`, `crop_20pct`, `resize_half`), and `curvyo-laser-core/tests/fixtures/raster/` gains `ramp_adjusted_stack` (dot map); an empty stack equals the `0123` dot maps byte for byte; native and wasm agree.
37. Budgets (thread CPU time, `curvyo-editor-wasm/tests/cpu_time/`; the architect confirms; Proposal): on a 4 Mpx image (2,000 x 2,000) each of Channel mix, Levels, Curves, Crop, Resize takes at most 40 ms, Sharpen at most 120 ms, Local contrast at most 400 ms, Denoise at most 800 ms, the whole stack below 1.2 s; the reduced Before and after update during an edit within 100 ms CPU on a 2 Mpx pane mip; the exact dots of a 200 x 200 mm image at Interval 0.1 within the `0123` budget plus the stack; the model's first run is stated, not budgeted.
38. Given a 16 Mpx working image, then a stack runs row-band by row-band without a second full-size copy of all steps at once (memory at most 2.5 times the working image; Proposal, the architect confirms), and a step that cannot fit warns instead of aborting the tab.
39. Given the UI (`docs/design-system.md`), then: symbol-only buttons with tooltip and accessible name, the step list in the panel's fold rules, no invented colours (the histogram and curve use `--panel-muted-fg` and the accent for the handles), no sliders, units in the fields, the Adjustments group in the tool-specific bar rule (a selected image shows it; nothing is added to the global chrome); UX notes before Ready.
40. Given the docs, then `docs/requirements.md` gains the next free R-MFG-LASER requirement (Image preparation: manual, automatic and, where decided, AI-assisted), `docs/design-system.md` the Adjustments group, `docs/technical-debt.md` any deferred item, and an ADR set for the stack's place (0135 A/B/C), the pipeline order, the fixed-point kernels, the format bump and, when slice 3 starts, the runtime and model (needs-customer).

## Decided questions

None yet. Fragen 272 to 276 are open; each has a default.

## Open questions

**Frage 272 (IMGOPT-STACK): Where does the adjustment stack live?**
A: On the image object only; every operation and clone sees the same picture; the stack cannot know the Interval or the material (Auto then writes only picture-level steps).
B: On the Image operation only; the same picture can be adjusted differently per material and Interval; the Select view shows the unadjusted picture; two operations on the same image keep two stacks.
C: Split: picture steps on the image object, dot-scale steps (Sharpen and the existing tone fields) on the Image operation; every step lives where the data it depends on lives.
Recommendation: C. Default if unanswered: C.

**Frage 273 (IMGOPT-AI-SCOPE): Which AI-assisted tools do you want?**
A: None in this story; Auto and the manual stack only, AI as a later story.
B: Background removal (subject mask) only, run on the device; the first model in the ranking is small (under 50 MB) and permissively licensed; later tools follow if the result is good.
C: B plus Upscale (more pixels for a small picture) and AI denoise.
(Depth or relief and "make engravable" enhancement stay later ideas under every option; say if you want one first.)
Recommendation: B (the biggest visible gain for photos on wood and the smallest model). Default if unanswered: B, slice 3 is built last.

**Frage 274 (IMGOPT-AI-PLACE): Where do AI tools run?**
A: On the device only; the model file ships with the app as a lazily loaded static asset from the app's own host (loaded on the first use, not at start); no picture, no thumbnail and no usage data ever leaves the machine; works offline after the first load.
B: On the device, with the model downloaded once from a separate Curvyo model host on the first use (the file comes in; nothing goes out; a network request exists).
C: In a cloud service when the maker presses the button (needs an account or an API key, the picture leaves the machine; opt-in per press with a notice; Curvyo pays or the maker brings a key; the result is not reproducible).
Recommendation: A. Default if unanswered: A (no network use for the picture; C is not built).

**Frage 275 (IMGOPT-AI-STORE): How does an AI result stay reproducible in the project file?**
A: The mask is stored with the project as a derived 8-bit asset (about 100 KB to 2 MB, PNG) next to the picture, with the model name and version; the job is identical on every device even without the model.
B: Only the model version is stored and the mask is recomputed on open (small file; the dots can differ between runtimes and model versions, so a job could change after an update).
Recommendation: A. Default if unanswered: A.

**Frage 276 (IMGOPT-GRID): Should the Image test grid get an axis for an adjustment?**
A: No; the axes stay Power, Speed, Passes, Interval, and the dither choices as built.
B: Yes, one more axis option "Gamma" (and later "Contrast"), 5 values, so a maker can find the tone that suits a wood.
C: A stack preset for the whole grid (the cells use one stack of the picture; Proposal), no axis.
Recommendation: A for now (the two tests of `0112` and `0123` stay the quick way; revisit after the demo). Default if unanswered: A.

## Out of scope

- A Cut-out that follows the contour of a subject, mask or transparent background (later idea, from the same review).
- Colour management, colour editing, retouching, clone stamp, perspective or lens correction, rotate or skew (an image rotates and skews as an object).
- Upscaling, depth or relief maps, "make engravable" enhancement (unless Frage 273 C or a later story).
- Live Auto (recomputing when the material changes) and an AI that chooses the dither mode or power; the maker chooses those.
- Batch processing of many pictures, presets saved to a library (a later idea; Copy and Paste adjustments is the first step).
- A cloud service, accounts, paid content and telemetry (not built under any default; Frage 274 C only if the customer asks).
- Changes to `0123`'s dithers, dot grid, material sets, G-code and the checks, except where this story says so.
- Mask painting (Keep and Erase brush): slice 4, Could, after Frage 273.

## UX notes

(To be filled in by the ux-engineer before Ready.) Open for UX: the exact place and fold of the Adjustments group, the tool-specific bar for Crop (frame handles on the canvas), the split or two-pane Before and after view with locked zoom, the step glyphs (custom, not invented colours), the histogram and curve graph with fields, the first-use notice of the local model.

## Links

Requirements: R-MFG-LASER-001 (raster engrave fills; this story improves their input), a new R-MFG-LASER requirement (criterion 40). Builds on: `0123` raster-engraving (Image settings, dot grid, resample, Image set), `0058` bitmap-import (working image, assets), `0112` test grids, `0049` clones, `0077` font preview (split-pane precedent). Related: `0025`, `0088`, `0090` to `0094` (vector trace: `vectrace`, the external trace library, CLAUDE.md §6, takes the adjusted image as its input in a later story so trace and engrave share the same cleaned picture; not part of this story), `0124` gradient-fill (not changed). PR: not yet.

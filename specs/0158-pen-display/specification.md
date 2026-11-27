# Pen display: draw with a pen on a second window, with pressure, brushes and shape recognition

Status: In progress (epic, not Done). Slice 0 built (PR #522, merged 2026-11-27; the customer's measurement on the Kamvas is still open, ADR 0094 stays Proposed). Slice 1 Ready 2026-11-27, to be built "dry" (the customer cannot test on the tablet for now: "go on, rework stacks up later"). Slices 2 to 6 are drafts. Fragen 408 to 413 answered 2026-11-26; Frage 414 answered A, default A
Priority: Should
Origin: Customer request 2026-11-26 (translated): "I own a Huion Kamvas pen display. I want a separate window for the tablet that can optionally stay in sync with the drawing window on the big screen but has its own position and zoom, so I can draw relaxed on the tablet; maybe its own views there (fewer tools, a smaller property panel). The pen with pressure sensitivity. Brushes become relevant: calligraphy, variable stroke width, path optimisation on the go. A shape builder: draw a rectangle with the pen and it becomes a rectangle ('gestures'). If possible, write text by hand with the pen. Think about what else is useful, compare with competitors and make a plan." Everything marked **(Proposal)** is the product owner's and vetoable at the demo.
Supersedes: `0032-pen-tablet-input` (absorbed by slice 1; 0032 stays as a folder and history, see Links).
Depends on: `0110-pen-modes-variable-stroke` and `0115` (Freehand mode, width profiles, ADR 0053), `0085-stroke-to-path`, `0033-stroke-brushes` (stage 1, Ready, nothing built), `0020-undo-redo`, `0009`/`0031` (Loro sync, only if the architect picks it for the pen window).
Related: `0088-trace-recognition` and `0091-trace-text-recognition` (recognition from rasters only), `0074-text-objects`, `0101-laser-operations`.

## User value

As a maker who owns a pen display, I want to draw on it as I would on paper while the big screen keeps the overview, so that sketching, lettering and tracing by hand feed straight into the cut, plot, engrave or stitch job. As the one who ends at a machine, I want a pen stroke to stay a clean, editable centre line with a width profile, so the same stroke can be cut as an outline, plotted as a line or later stitched as a satin column.

## Field reference

Researched 2026-11-26 (sources in the lead's notes; "research 2026-11-26").

| Tool | Second view or window | Pressure and brush | Stroke clean-up and shapes | Pen UI |
|---|---|---|---|---|
| Illustrator | New Window on the same file | Blob Brush (merges into one fill), Width tool, pressure profiles | Shaper tool turns scribbles into shapes, scribble deletes | Touch workspace with few tools |
| Affinity Designer | Split or new window | Pressure profile editable after drawing, stabiliser (rope, window), redraw/continue stroke | Pencil smoothing | Pen-optimised UI on iPad |
| Inkscape | Second window of the file | Calligraphy tool writes fills, pressure feeds PowerStroke | Pencil smoothing, shape recognition mode only basic | Desktop UI unchanged |
| CorelDRAW | Second window | Pressure on Artistic Media | LiveSketch and Smart Drawing recognise shapes | Desktop UI unchanged |
| Krita, Clip Studio | New View with own zoom and rotation, canvas-only mode | Pressure curve, tablet tester, stabilisers | Vector eraser to the next intersection | Pop-up palette at the pen, express-key presets |
| Concepts | n/a | Pressure, tilt | Shape snapping | Tool wheel, two-finger undo |
| LightBurn, xTool Creative Space, Cricut Design Space, Silhouette Studio | none | next to no pressure support; Cricut draws an outline hollow | basic | desktop UI |

**What we do better (Proposal).** No maker tool offers a pen window at all. We keep a pen stroke as centre line plus width profile and derive the output per operation: an outline for cut and engrave, the centre line for plotter and score, later satin rails for embroidery. The stroke stays editable (Node tool, width diamonds of `0110`), which Cricut and Inkscape's fills do not.

## Facts needed from the customer (not Fragen)

1. Kamvas model: **Huion Kamvas 13** (customer, 2026-11-27; pen only, no touch as far as the lead knows, not confirmed by the customer).
2. Linux session: **X11** on the main machine; **Wayland must work too**, the customer has a second test device for it (2026-11-27). Desktop still open.
3. Driver: Huion driver or OpenTabletDriver.
4. Whether the pen display is the only second screen or a third one.

## Slices (Proposal; each ends in something the customer can try)

0. **Pen probe** (measures the native route chosen in Frage 408 A). **Built, PR #522 (2026-11-27).** The customer's measurement on the Kamvas (ADR 0094 decision 8) is still open.
1. **Pressure Pencil**.
2. **Pen window**.
3. **Stroke quality on the go**.
4. **Brushes**.
5. **Shape builder**.
6. **Handwriting to text** (experiment).

Order: 0, then 1 and 2 (either order after 0; 2 needs an ADR first, so 1 starts while the architect works), then 3, 4, 5, 6. The customer can stop after any slice.

### Slice 0: Pen probe

Status: Built (PR #522, merged 2026-11-27); criteria 1 to 5 are covered by tests and the gate. Open: the customer runs the probe on the Kamvas 13 (X11 and Wayland) and sends the "Copy report" numbers; until then ADR 0094 stays Proposed and Frage 408 stays at default A. Slice 1 is built without waiting for it.

A diagnostics page showing, live, pointer type, pressure, tilt, eraser end, barrel button and event rate, in the app and in a browser build, so the input route is chosen on the real Kamvas, not guessed. Why first: **blocker risk.** WebKitGTK (the Tauri webview on Linux, ADR 0001) does not deliver pen pressure (WebKit bug 268458, still NEW); Chromium and Firefox on Linux mostly do but vary by driver, compositor and model; tilt is unreliable (research 2026-11-26).

Draft criteria:
1. Given the app on the customer's machine, when the pen hovers and draws on the probe page, then pointer type "pen", pressure (0 to 1), tilt, eraser-end flag, barrel-button state and events per second are shown within one frame.
2. Given the same page opened in a Chromium-based browser build, then the same fields are shown, so the two routes can be compared side by side.
3. Given a mouse, then the page says "mouse" and shows no pressure, without an error.
4. Given the probe is closed, then nothing about the device is stored or sent anywhere (no telemetry).
5. Given the app on the customer's machine, when the pen draws on the probe page, then the native pen reading of `curvyo-app` and the webview's values are shown side by side with the delay between them in milliseconds, so Frage 408 can be confirmed on the real Kamvas **(Proposal, Frage 408 answer of 2026-11-26)**.

### Slice 1: Pressure Pencil

Status: Ready 2026-11-27 (built "dry": the customer cannot test on the tablet for now and said to go on, rework stacks up later; ADR 0094 stays Proposed, so the native route may change before the demo with a pen).

**What it is.** A new rail tool, the Pencil (key `P`, design system rows "Pencil tool" and "Pencil bar", written for `0032`, **(Proposal)** reused), that turns the pen's samples into one path: a fitted Bézier centre line plus a width profile on its normalised arc length (ADR 0053). That is the same object a Pen path with a width profile is today, so Node tool, width diamonds (`0110` criterion 4), width profile buttons (`0110` criterion 5), Stroke to path and machine output work on it unchanged (Frage 410 C replaces the 0032 decision to store a filled outline). Pressure and the coalesced samples between frames reach the session as batches of the `PenSample` record of slice 0 (native on Linux, ADR 0094 decisions 3 to 5; Pointer Events with coalesced events on Windows, macOS and the browser build, decision 6). A mouse or a pen without pressure draws a uniform stroke; the pressure is simulated from speed only when "Simulate pressure" is on.

**Reused from `0032` (it stays as history):** the has-pressure rule (criterion 3), the pressure smoothing and the three curves (criterion 9), the default widths, preview and Escape rules (criteria 11, 12), the touch rule (criterion 4), the glyph, key and cursor rows of the design system. **Not reused:** the closed filled outline (Frage 410 C), the pressure readout (slice 0 shows it), Pointer-Events-only input.

**Decided by the customer:** Frage 410 C (centre line plus editable width profile), curves Soft/Linear/Firm and Min/Max width, mouse or pen without pressure uniform unless "Simulate pressure", fitted curve with a Smoothing value, batches of `PenSample`, eraser end erases. **Everything else here is a Proposal**, vetoable at the demo: own rail tool, Freehand mode untouched (Frage 414), eraser deletes whole objects, the numbers.

Words: **sample** = one `PenSample`; **stroke** = the samples from the first with phase contact and end tip to the last contact; **has pressure** = the rule of `0032` criterion 3 applied to the `Pressure` of the stroke's samples (some sample not 0 and not 0.5); **centre line** = the fitted Bézier. The acceptance tests feed batches of `PenSample` to the session and need no tablet; the frontend tests feed Pointer Events to the route that builds the same records.

Acceptance criteria:
1. Given the Pencil and a stroke with pressure rising from 0.1 to 0.9 over 100 mm, when the last contact sample arrives, then exactly one new open path exists with a Bézier centre line and a width profile that is narrower at the start than at the end; it is selected, the tool stays the Pencil, and exactly one commit is made, one undo step named "Pencil stroke".
2. Given 2000 samples along a smooth S-curve 150 mm long, then the centre line lies within the Smoothing tolerance of every sample (0.4 mm at the default 40 %; architect, ADR 0095), has at most 1 node per 3 mm (at most 50) and no node per sample, and the release is processed in under 200 ms thread CPU time in a release build **(Proposal)**.
3. Given the pressure samples of a stroke, then the width at a sample is Min + (Max - Min) x curve(smoothed pressure) with the smoothing and curves of `0032` criterion 9 (mean of the sample and the two before it; Soft p^0.5, Linear p, Firm p squared); tests: pressures 0.2, 0.2, 1.0, 0.2, 0.2 smooth to 0.2, 0.2, 0.467, 0.467, 0.467 (+/-0.001); smoothed 0.25 gives 0.5 with Soft, 0.25 with Linear, 0.0625 with Firm.
4. Given the same stroke drawn with Soft, Linear and Firm, then the three width profiles differ (Soft widest, Firm narrowest at mid pressure), and Min and Max width set the narrowest and widest profile width in mm (defaults 0.2 and 2, ranges 0.05 to 20 and 0.1 to 50, Max never below Min, as `0032` criterion 14).
5. Given a finished stroke, then its profile has no more width points than needed to stay within 5 percent of Max - Min of the sampled widths **(Proposal)** and is stored on normalised arc length; after save and reopen the path, its centre line and its profile are identical (ADR 0053, no format change).
6. Given a finished stroke, when it is selected with the Node tool, then its nodes and width diamonds are the ones of `0110` (add, drag, Delete a diamond; one drag one undo step) and the Style panel's profile buttons (Uniform, Taper start, Taper end, Taper both) work on it as on a Pen path.
7. Given a stroke without pressure (a mouse, or a pen whose samples are all 0 or all 0.5), then the path has a centre line and no width profile and its width is the current stroke width; "Simulate pressure" is off by default and a mouse behaves as a plain freehand line.
8. Given "Simulate pressure" is on and a stroke without pressure, then the width follows the speed from the sample times: pressure = clamp(1 - v / 300 mm/s, 0, 1) **(Proposal)**, then the same curves and Min/Max; tests: constant speed gives a constant width, a stroke half as fast gives a wider width than the faster one, fast and slow parts of one stroke give a wider slow part. Given a stroke with pressure, then the option is ignored.
9. Given the Smoothing value (the scale and default of the Pen's Freehand slider, `0110` criterion 1), then the same samples with a higher value never give more nodes, with 0 the centre line stays within 0.1 mm of every sample, and the value applies from the next stroke (session setting, not saved in the document).
10. Given a stroke in progress, then the canvas shows the blue preview with the variable width (the hollow outline, design-system row "Live preview outline"; a 1.5 px line for a stroke without pressure and without simulation), updated at least once per animation frame for strokes up to 2000 samples, and on release the object lies within the Smoothing tolerance plus 5 % of Max − Min of the last preview (the preview shows the raw samples; the live fitted curve is slice 3; architect, ADR 0095).
11. Given a batch with hover samples (phase hover or out), then they move the pointer as a mouse move does and start nothing; given samples of one stroke spread over several batches, then the stroke equals the same samples in one batch; given a touch pointer, then the Pencil ignores it; given samples with barrel bits or tilt, then they are carried and change nothing.
12. Given the pen's eraser end (end eraser, contact), then objects whose line comes within the 8 px hit rule of `0014` of its path are deleted when it lifts (hidden and locked objects are not touched), in one undo step named "Erase", with the Pencil active; no stroke is created; the hovering eraser end highlights what it would delete as the hover box does **(Proposal)**. Partial erasing up to the next crossing is slice 3.
13. Given a new stroke, then it takes the current stroke style and the operation of the current layer or page as a new Pen path does, and its fill is None; the profile widths come from Min and Max, not from the style's stroke width.
14. Given a Pencil stroke in a job, then machine output follows the centre line and ignores the width (ADR 0053, `0110` criterion 6): the job geometry equals that of the same path without a profile; Stroke to path makes the closed outline; SVG export follows the visible outline as for any variable stroke.
15. Given Escape during a stroke, then it is cancelled and nothing is written; given fewer than 2 samples or a length below 0.1 mm, then nothing is created and no commit is made (`0032` criterion 12).
16. Given the Pen's Freehand mode, then it behaves exactly as before and all its tests pass unchanged (Frage 414 A).
17. Given any input, then no pressure samples, device names or tilt are stored in the document or sent anywhere; only the fitted centre line and profile are kept (R-SYS-007).

Edge cases: a stroke that never leaves 0 pressure with a pen (has no pressure, criterion 7); pressure that jumps from 0 to 1 between two samples (smoothed, criterion 3); a stroke of one batch of 256 samples with dropped ones (ADR 0094 decision 4: the fit uses what arrived, the stroke is still one path); a closed loop (stays an open path; closing is slice 3 or later); a stroke released after the pen left the window (the last contact sample ends it); two strokes within one frame (two paths, two undo steps); an eraser end that touches nothing (no commit); a pen on a hidden or locked layer (nothing is created, a notice as for the Pen).

Out of scope for slice 1: tilt effects, a stabiliser and live curve fit while drawing (slice 3), continuing or redrawing a stroke (slice 3), erasing part of a stroke (slice 3), shape recognition (slice 5), brushes and calligraphy outlines (slice 4), the second window and Pen layout (slice 2), pressure calibration, barrel and express-key bindings, touch drawing, removing the Pen's Freehand mode (Frage 414).

**UX and architect, to do before the build starts (not blockers for the criteria):** UX notes for the Pencil rail slot (the rail has changed since `0032`: Text and others took slots) and the Pencil bar (Smoothing, Simulate pressure, Min, Max, Curve; the `0032` rule "hidden until a pen was seen" does not hold because a mouse needs Smoothing and Simulate pressure) **(Proposal)**; architect to confirm the fit routine and profile reduction place (no new crate expected) and to put the session-side record in `adrs.md`.

### Slice 2: Pen window

A second window on the **same open document**, live both ways, with its own pan and zoom, remembered as "the pen window". A curated **Pen layout** (few tools, a slim property strip, large hit targets, a canvas-only toggle). Optional **Follow view** (the pen window mirrors the main window's view) and a frame in the main window showing what the pen window sees. Window placement on a chosen screen is unreliable (Wayland): the plan is "drag the window over, it remembers it is the pen window" **(Proposal)**. The mechanism (second webview plus a local CRDT bridge, a second Tauri window sharing the wasm document, or something else) is **needs-architect**: today there is one window `main`, one Viewport per session, `host.ts` has no window API, there is no compact or touch layout, and the only way two webviews could share a document is the Loro sync through the relay (ADRs 0009, 0031), with no local bridge. Nothing here is decided.

Draft criteria:
1. Given a document open in the main window, when "Open pen window" is used, then a second window shows the same document; a stroke drawn in either appears in the other within 200 ms **(Proposal)** and one Ctrl+Z in either window undoes the last step of that window's author.
2. Given the two windows, then each has its own pan and zoom; with Follow view on, the pen window takes the main window's view and a pan in one moves the other.
3. Given the pen window is closed and reopened, or the app is restarted, then it is offered again as the pen window with its last size, and a window dragged to the pen display keeps that place where the compositor allows it.
4. Given the pen window is active, then the fixed Pen layout (Frage 413 A, not configurable) shows Pencil, Select, Node, Eraser, Undo and Redo (curated list **(Proposal)**, the UX notes fix the final set), a slim strip for colour, width and Smoothing, and a toggle that hides everything except the canvas.
5. Given the main window shows the frame of the pen window's view, when the pen window pans or zooms, then the frame follows in the same frame.
6. Given an object selected in one window (Frage 409 A, selection shared), then it is selected in the other window too, in the same frame as far as the 200 ms of criterion 1 allow; given a different tool or view in each window, then neither changes the other's (tool and view are per window); given Delete in one window, then the other window's selection is empty and one Ctrl+Z restores both.

### Slice 3: Stroke quality on the go

Stabiliser (rope or pulled string), live curve fit with a Smoothing value that updates while drawing, redraw or continue an existing stroke, and a vector eraser that removes up to the next crossing (Krita, Affinity).

Draft criteria:
1. Given the stabiliser with length L mm, then a jittery hand path of 1 mm amplitude comes out smoothed to within L/4 of the intended line **(Proposal, measured on a recorded stroke)**.
2. Given a stroke while drawing, then the fitted curve is shown live and its node count follows the Smoothing value.
3. Given a finished stroke, when the pen starts on its end and draws on, then it continues that path (as `0034`), and a redraw over a stroke with Alt replaces its middle part.
4. Given the vector eraser dragged across a stroke, then it removes only the part between the two nearest crossings, in one undo step.

### Slice 4: Brushes

A calligraphy nib (fixed angle, width from the direction of travel; tilt optional), the built-in brushes of `0033` stage 1, a blob mode (overlapping strokes of one operation merge into one closed shape, Illustrator's Blob Brush), profile presets.

Draft criteria:
1. Given the calligraphy nib at 45 degrees, then a stroke's width is largest across the nib and smallest along it, and the result is a closed outline (Frage 410 C), exportable to a cut operation.
2. Given blob mode, when two overlapping strokes are drawn in one operation, then they become one closed shape with no inner line.
3. Given a profile preset (taper both ends, taper end, pressure-like), then it applies to the selected stroke in one undo step.
4. Given a plotter operation, then a calligraphy outline is drawn as its outline unless hatch fill is on (later list).

### Slice 5: Shape builder

A rough stroke becomes a real primitive: line, arc, rectangle, ellipse, triangle, polygon, star. Scribbling over an object deletes it. One undo step returns the raw stroke. `0088` part B recognises shapes only from traced rasters, so this is new input, not a reuse.

Draft criteria:
1. Given a roughly rectangular stroke that closes, when the pen is held still for 0.5 s at its end (Frage 411 C, the Pencil's trigger) **(Proposal)**, then it becomes a rectangle (a real primitive, parameters editable by `0009` handles), centred on the stroke's box within 2 mm for a 100 mm stroke.
2. Given a stroke the recogniser is not sure about, then it stays a path (never a wrong shape).
3. Given a scribble of at least 6 direction changes across a selected object, then it is deleted in one undo step.
4. Given the conversion, when Ctrl+Z is pressed once, then the raw stroke is back.
5. Given the separate shape mode (Frage 411 C) is on, when a closed rough stroke is released, then it converts at once without the hold, under the same rules as criteria 1 and 2; with the mode off, the same stroke stays a path **(Proposal)**.

### Slice 6: Handwriting to text (experiment)

Write in a box, get editable text in the Text tool (`0074`). Riskiest item and last on purpose: the Web Handwriting API exists effectively on ChromeOS only, no permissive offline stroke-based recogniser is ready for wasm, Linux has no OS handwriting panel (research 2026-11-26). `0091` OCR reads printed text from rasters and does not read handwriting. Frage 412 A: offline only; dropped if the quality is poor.

Draft criteria:
1. Given the handwriting box and the word "Curvyo" written once in block letters, then the Text tool receives that word as editable text, correct in at least 9 of 10 attempts for the customer's own hand **(Proposal, measured at the demo)**.
2. Given recognition runs, then it runs on the machine; no drawing data leaves it (Frage 412 B is excluded for good). The one download allowed is a local model fetched the same way as the models of image optimisation and image AI (customer clarification 2026-11-26); its licence must be permissive, checked by the architect **(Proposal)**.
3. Given a wrong result, then the maker can edit it as normal text and can keep the handwriting as a path instead.

## Later and optional (not sliced)

Pop-up radial palette at the pen tip; express-key shortcut preset; canvas rotate and mirror in the pen window only; touch gestures (two-finger undo, pinch) on touch-capable models; pressure calibration; a minimum-feature-width warning per machine while drawing; hatch fill so plotter pens draw calligraphy solid; satin rails for embroidery from a centre line and width profile.

## Maker angle (Proposal)

A stroke stays centre line plus width profile; the operation decides the output: outline for cut and engrave, centre line for plotter and score, later satin rails for embroidery. Machine output keeps the centre line (ADR 0053). Calligraphy and blob write outlines (Frage 410 C).

## Open questions

**Answered by the customer 2026-11-26 (Fragen 408 to 413).**

**Frage 408 (PEN-INPUT): A** (customer, 2026-11-26: the app reads the pen natively; slice 0 measures the delay). Options were:
- A: read the pen natively in `curvyo-app` and hand it to the webview (needs an ADR)
- B: the pen window runs in a Chromium browser
- C: change the shell's webview

Customer, 2026-11-26: B is ruled out in practice (a second window in a browser seems too hard to them). A stays the default and is confirmed after slice 0 has measured it. Slice 0 therefore also measures the native route (criterion 5).
B is ruled out in practice (a second window in a browser seems too hard to the customer). Slice 0 measures the native route (criterion 5).
**Frage 409 (PEN-SYNC): Answered A.** The same document live, selection shared, tool and view per window, optional Follow view.

**Frage 410 (PEN-STROKE): Answered C.** Per brush: calligraphy and blob write outlines, the Pencil writes a centre line with a width profile.

**Frage 411 (PEN-SHAPE): Answered C.** Both triggers: holding still at the stroke end in the Pencil (slice 5, criterion 1), and a separate always-converting mode (slice 5, criterion 5).

**Frage 412 (PEN-HAND): Answered A.** Offline only, an experiment as the last slice, dropped if the quality is poor. Option B is excluded for good ("auf keinen Fall"). Clarification (customer, 2026-11-26): a downloadable AI model that runs locally, fetched the same way as the models of image optimisation and image AI, is allowed; its licence must be permissive, checked by the architect (slice 6, criterion 2).

**Frage 413 (PEN-LAYOUT): Answered A, fixed.** The Pen layout is one fixed, curated layout. The customer does not want it configurable at all (slice 2, criterion 4).

**Frage 414 (PEN-FREEHAND): Answered A** (customer, 2026-11-27: both stay for now). Slice 1 adds the Pencil as its own rail tool. The Pen's Freehand mode (`0110`) draws a similar line without pressure. What happens to it is the customer's call (removing accepted behaviour).
- A (default): keep both for now; the Freehand mode is untouched and revisited after the demo of slice 1.
- B: remove the Freehand mode from the Pen bar once the Pencil ships (one tool for freehand).
- C: the Freehand button switches to the Pencil tool.
Recommendation: A, nothing is lost and the demo shows which one is used. Without an answer slice 1 is built as A.

## Out of scope

- Painting and raster brushes (no pixel layers); a general stylus API for plugins.
- Touch-only drawing without a pen; mobile and tablet apps (a pen display on a PC only).
- Sending drawing data to a service (Frage 412 B is excluded for good).
- Pressure-driven machine power or speed (an idea for the laser, not now).
- A universal "any pen display" support promise: the first target is the customer's Huion Kamvas.

## UX notes

(filled in by ux-engineer before Ready: Pen layout, the pen-window frame in the main window, the probe page, shape-builder feedback.)

## Links

Requirements: R-INP-001 (absorbed), R-INP-002 to R-INP-005, R-EDIT-050 (variable width, done by `0110`).
Specs: `specs/0032-pen-tablet-input/` (superseded by this epic; its code is on `origin/story/pen-tablet-input`: Schneider fit, variable outline, Pencil tool, pressure curves, reusable for slice 1), `specs/0110-pen-modes-variable-stroke/`, `specs/0033-stroke-brushes/`, `specs/0088-trace-recognition/`, `specs/0091-trace-text-recognition/`.
ADRs: 0001 (Tauri 2, WebKitGTK), 0009 and 0031 (sync), 0053 (centre line and width profile); new ADRs needed: input route (Frage 408) and the pen window mechanism (needs-architect).
PR: none yet.

**Frage 415 (PEN-WINDOW-MECH): Answered A** (customer, 2026-11-27) (ADR 0096, `needs-customer`; slice 2 only, slice 1 does not depend on it). How the pen window holds the document:
- A: a second window with its own copy of the document, kept equal locally all the time; undo per window, saving and recovery in the main window only, pictures twice in memory, no pen window in the browser build
- B: one document drawing into both windows; a trial first, then a large rebuild; one shared undo stack
- C: the document moves into the app process; a very large rebuild, the browser build goes away

Recommendation: A. Default: A.

# Pen display: draw with a pen on a second window, with pressure, brushes and shape recognition

Status: Draft (epic overview; Fragen 408 to 413 answered 2026-11-26; slice 0 measures the delay of the native route of Frage 408 A; model and session known, driver and desktop still open)
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

0. **Pen probe** (measures the native route chosen in Frage 408 A).
1. **Pressure Pencil**.
2. **Pen window**.
3. **Stroke quality on the go**.
4. **Brushes**.
5. **Shape builder**.
6. **Handwriting to text** (experiment).

Order: 0, then 1 and 2 (either order after 0; 2 needs an ADR first, so 1 starts while the architect works), then 3, 4, 5, 6. The customer can stop after any slice.

### Slice 0: Pen probe

A diagnostics page showing, live, pointer type, pressure, tilt, eraser end, barrel button and event rate, in the app and in a browser build, so the input route is chosen on the real Kamvas, not guessed. Why first: **blocker risk.** WebKitGTK (the Tauri webview on Linux, ADR 0001) does not deliver pen pressure (WebKit bug 268458, still NEW); Chromium and Firefox on Linux mostly do but vary by driver, compositor and model; tilt is unreliable (research 2026-11-26).

Draft criteria:
1. Given the app on the customer's machine, when the pen hovers and draws on the probe page, then pointer type "pen", pressure (0 to 1), tilt, eraser-end flag, barrel-button state and events per second are shown within one frame.
2. Given the same page opened in a Chromium-based browser build, then the same fields are shown, so the two routes can be compared side by side.
3. Given a mouse, then the page says "mouse" and shows no pressure, without an error.
4. Given the probe is closed, then nothing about the device is stored or sent anywhere (no telemetry).
5. Given the app on the customer's machine, when the pen draws on the probe page, then the native pen reading of `curvyo-app` and the webview's values are shown side by side with the delay between them in milliseconds, so Frage 408 can be confirmed on the real Kamvas **(Proposal, Frage 408 answer of 2026-11-26)**.

### Slice 1: Pressure Pencil

Pen pressure and the coalesced samples between frames go into the session (today the wasm pointer API has no pressure field and the frontend reads no pressure or coalesced events). A Pencil stroke becomes a fitted curve with a width profile derived from pressure (the ADR 0053 model, centre line plus profile points on normalised arc length). This replaces the 0032 decision to store a filled outline (Frage 410). The Pencil is the Freehand mode of `0110`, not a second tool **(Proposal)**.

Draft criteria:
1. Given a pen with pressure, when drawing a stroke with light then firm pressure, then the stroke is one path whose width grows along it, editable with the width diamonds of `0110`, and its centre line is a fitted Bézier (no per-sample nodes; node count at most 1 per 3 mm of a smooth stroke **(Proposal)**).
2. Given the setting Pressure curve (Soft, Linear, Firm) and Min and Max width in mm, then the same stroke gets a visibly different profile and the result is identical after save and reopen.
3. Given the pen's eraser end, when it touches the canvas, then it erases (default: like the Eraser tool; the erase-to-next-crossing behaviour is slice 3).
4. Given a mouse or a pen without pressure, then speed-simulated pressure (slow wide, fast thin) is available as an option, default off, and plain Freehand behaviour is unchanged otherwise.
5. Given an old build opens the file, then the width profile is ignored and the centre line shows (format impact decided by the architect, ADR 0053 already covers the profile).

### Slice 2: Pen window

A second window on the **same open document**, live both ways, with its own pan and zoom, remembered as "the pen window". A curated **Pen layout** (few tools, a slim property strip, large hit targets, a canvas-only toggle). Optional **Follow view** (the pen window mirrors the main window's view) and a frame in the main window showing what the pen window sees. Window placement on a chosen screen is unreliable (Wayland): the plan is "drag the window over, it remembers it is the pen window" **(Proposal)**. The mechanism (second webview plus a local CRDT bridge, a second Tauri window sharing the wasm document, or something else) is **needs-architect**: today there is one window `main`, one Viewport per session, `host.ts` has no window API, there is no compact or touch layout, and the only way two webviews could share a document is the Loro sync through the relay (ADRs 0009, 0031), with no local bridge. Nothing here is decided.

Draft criteria:
1. Given a document open in the main window, when "Open pen window" is used, then a second window shows the same document; a stroke drawn in either appears in the other within 200 ms **(Proposal)** and one Ctrl+Z in either window undoes the last step of that window's author.
2. Given the two windows, then each has its own pan and zoom; with Follow view on, the pen window takes the main window's view and a pan in one moves the other.
3. Given the pen window is closed and reopened, or the app is restarted, then it is offered again as the pen window with its last size, and a window dragged to the pen display keeps that place where the compositor allows it.
4. Given the pen window is active, then the fixed Pen layout (Frage 413 A, not configurable) shows Pencil, Select, Node, Eraser, Undo and Redo (curated list **(Proposal)**, the UX notes fix the final set), a slim strip for colour, width and Smoothing, and a toggle that hides everything except the canvas.
5. Given the main window shows the frame of the pen window's view, when the pen window pans or zooms, then the frame follows in the same frame.

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

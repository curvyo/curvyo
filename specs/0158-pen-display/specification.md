# Pen display: draw with a pen on a second window, with pressure, brushes and shape recognition

Status: In progress (epic, not Done). Slice 0 built (PR #522, merged 2026-11-27; the customer's measurement on the Kamvas is still open, ADR 0094 stays Proposed). Slice 1 built (PR #525, merged 2026-11-27), "dry" (the customer cannot test on the tablet for now: "go on, rework stacks up later"). Slice 2 built 2026-11-28, parts 2a and 2b in one PR (PR #528; customer 2026-11-28: both on one branch; ADR 0096 Accepted, Frage 415 A), "dry" like slice 1: not tried with two real windows under a window manager or with the tablet. Slices 3 to 6 are drafts. Fragen 408 to 413 answered 2026-11-26; Frage 414 answered A, default A
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
1. **Pressure Pencil**. **Built, PR #525 (2026-11-27).**
2. **Pen window**. Ready, in two PRs: 2a Pen window, 2b Pen layout and Follow view.
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

Status: Built (PR #525, merged 2026-11-27); criteria 1 to 17 are covered by tests and the gate. Open: the customer tries it with the pen on the Kamvas (rework stacks up). Was Ready 2026-11-27 (built "dry": the customer cannot test on the tablet for now and said to go on, rework stacks up later; ADR 0094 stays Proposed, so the native route may change before the demo with a pen).

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

Status: Built 2026-11-28 (PR #528), both parts in one PR at the customer's wish: **2a Pen window** and **2b Pen layout and Follow view**; the criteria are covered by session and frontend tests, the "by hand" parts (real windows, window manager, Wayland, tablet, the measurements of 2a criterion 20) are open. Mechanism decided: ADR 0096 (Accepted, Frage 415 A). Frage 409 A (same document live both ways, selection shared, tool and view per window, optional Follow view) and Frage 413 A (one fixed Pen layout, not configurable) are binding. Not tested on the tablet before the build (as slice 1); the testable core needs no tablet and no second real window.

**What it is.** A second Curvyo window for the pen display on the **same open project**, live both ways, with its own pan, zoom and tool. "The pen window" is the window labelled `pen`: the maker opens it from the main window, drags it to the pen display once, and the app remembers its size and (where the platform allows) position and screen. Placing a window on a chosen screen is unreliable on Wayland (research 2026-11-26), so the app never tries to force a screen; it restores what the platform honours and otherwise opens where the compositor puts it.

**How it holds the document (ADR 0096, in plain words).** The pen window is a second session with its own copy of the project; both copies are kept equal inside the computer, a frame after each finished step, nothing sent anywhere. The main window owns the file: path, saving, unsaved state and the recovery copy stay there. Undo is per window. No pen window in the browser build. The pen reader of ADR 0094 attaches to the pen window too (ADR 0096 decision 8).

**Why two parts (Proposal).** The bridge, the shared document, the lifecycle rules and the placement are the risk and the large part; the curated layout, the Eraser button, Follow view and the frame are UI on top. Part 2a gives the customer a working pen window with the Pencil and pressure (the thing they asked for), part 2b makes it comfortable. Cutting by "bridge first" and "everything else" would end 2a in nothing the customer can try, so 2a includes a minimal usable window. The customer can stop after 2a.

Words: **link** = the two sessions and the bridge between them (`window_link` and the hub of ADR 0096); **step** = one finished undo step; **a frame later** = in the next animation frame after the other side exported it (tests count frames, not milliseconds; the 200 ms of criterion 2 is measured on the customer's machine, ADR 0096 decision 11).

#### Part 2a: Pen window

What the maker sees: a button "Open pen window" in the main window's File tab (Pen display section, above Pen probe...; desktop only, the browser build does not show it). A second window opens on the same project. It shows **the canvas and a tool rail only (Proposal)**, no Properties panel, no menus, no import or export, no status bar: a rail card **Tools** (Pencil, Select, Node), a card **Edit** (Undo, Redo, Delete) and a card **View** (Zoom in, Zoom out, Fit page, a latching Pan toggle), and the active tool's own bar (the Pencil bar unchanged; the Node bar without Centre lines; none for Select). The pen draws with pressure there. Undo, Redo, Delete, zoom and pan exist as buttons because the pen has no keyboard, wheel or Space. The fixed Pen layout of Frage 413 A (Eraser, Layer, Max width, Smoothing strip, canvas-only, Follow view) comes in part 2b. Details: `ux-notes.md` section 9.

Acceptance criteria:
1. Given a project open in the main window of the desktop app, when "Open pen window" is used, then a second window labelled `pen` shows the same project within 3 s **(Proposal, ADR 0096 decision 11)**; the one button shows its state as its label: "Open pen window" (closed), "Opening pen window..." (from the press to the pen window's first frame, presses ignored), "Show pen window" (open; it raises, un-minimises and focuses the window and opens no third one); given no first frame within 10 s or a window that cannot be made, then the label returns to "Open pen window" and the notice "The pen window could not be opened." shows for 8 s; given the pen window is closed by the window manager or crashes, then the label returns to "Open pen window" without a notice; given the browser build, then the button is not offered and the pen window route answers "not available" without an error.
2. Given two linked sessions, when a step finishes in either, then the other session's document equals it (same canonical content: same nodes, geometry, styles, order) a frame later; the same holds for 50 steps alternating between both sides and for steps made while the other side is mid-drag; on the customer's machine a finished stroke is visible in the other window within 200 ms **(Proposal)**. Test: two `WasmSession`s and the bridge codec in one process, no window.
3. Given a picture imported in the main window, then it shows in the pen window and the asset travels once; given the same picture is already in both, then no asset message is sent; given an asset the other side lacks when an update arrives, then the update waits for the asset and the document stays consistent (no empty picture).
4. Given an object selected in one window (Frage 409 A), then the same selection is the other window's selection a frame later; given a selection of ids the other side no longer has, then they are dropped without an error; given Delete in one window, then the other window's selection is empty and one Ctrl+Z in the deleting window restores the object and the selection in both; given a value field in the receiving window is being typed in or dragged (or a chip is open), then the arriving selection waits until that field commits or is cancelled, so a typed value never lands on another object **(Proposal, UX 9.4)**; this is the one exception to "a frame later". The arrival itself shows no notice and no animation.
5. Given each window has its own tool, tool options, pan, zoom, pointer and panels (Frage 409 A), then changing any of them in one window changes nothing in the other (test: tool Pencil in the pen window, Select in the main window, different zoom, after 20 steps both are as set).
6. Given steps by both windows, when Ctrl+Z (or the Undo button) is used in the pen window, then it undoes the pen window's last step only, remapped over the main window's later steps (ADR 0009 §1), and the same for the main window; Redo likewise; the History tab names the pen window's steps "Pen window" **(Proposal, ADR 0096 decision 6)**. Test: A draws, B draws, A undoes: B's object stays in both.
7. Given the pen window with the Pencil and a native pen batch with pressure rising from 0.1 to 0.9 (ADR 0094, ADR 0095), then the stroke is created in the pen session exactly as in slice 1 criterion 1 (one path, one undo step "Pencil stroke", selected), is in the main session a frame later, and the main window's Pencil does not receive it; given pen samples arrive for the main webview, then they reach only the main page's channel and the pen window's samples only its own (per-label routing test of `pen_input` with fake channels); given the pen's eraser end in the pen window, then slice 1 criterion 12 holds there and the erase reaches the main window.
8. Given a stroke or drag in progress in one window, then the other window shows the result when the step finishes, not before (ADR 0096 decision 4); given Escape cancels it, then nothing reaches the other window.
9. Given a step made in the pen window, then the main window's project is marked unsaved as for a local step, the main window's recovery copy (ADR 0079) contains the step within the usual recovery interval, and the pen session has no path, writes no recovery copy and never writes the project file; given Ctrl+S in the pen window, then the main window saves (to its path, or its Save as dialog opens in the main window).
10. Given the pen window is closed by the window manager (its close button, Alt+F4, the compositor), then the close is never held and nothing is asked, the main window is unchanged and keeps working, the project is not marked unsaved by the close, and "Open pen window" opens it again showing the current project. A crash or kill of the pen webview is treated the same: the main window continues and the link is dropped (detected on Linux; Windows and macOS see the window's destruction only).
11. Given the pen window is open and the maker closes the main window or quits the app, then the **main window asks first** (the usual unsaved-changes check, R-SYS-010) and the pen window shows "Waiting for the main window" meanwhile; on Cancel both windows stay open; on Discard or a successful Save the pen window closes and then the app exits; a close of the main window that is not held closes the pen window and the app (the app never keeps running with the pen window alone) (ADR 0096 A2). Nothing is saved or asked for by the pen window.
12. Given the main window's New or Open (after its unsaved check), then the pen window shows the new project (a full project message), with its selection empty, its undo stack empty, its tool kept and its view fitted to the new project; steps of the old project never reach the new one; given Close of the project to the start state, then the pen window closes **(Proposal)**.
13. Given a dialog in the main window that decides the project's fate (the unsaved-changes dialog of New, Open, Close or Quit, or Save as), then the pen window shows a "Waiting for the main window" layer and ignores input until it is closed, so no step is made against a project about to be replaced; given any other main-window dialog (import, export, settings), then the pen window keeps working; given an error in the pen window, it is shown there as a notice, never as a dialog that blocks the main window **(Proposal)**.
14. Given both windows change the same object at once (two steps before either import), then after both imports the two documents are equal and no step is lost silently: different properties both apply, the same property ends with one value in both windows (last writer by Loro's rule), a move in one and a delete in the other ends deleted in both, and an undo of a step on an object the other side deleted does nothing and raises no error (ADR 0009). Test: two sessions, steps made, then updates exchanged both ways.
15. Given a text object being edited in one window (caret, selection, IME composition) while the other window draws elsewhere, then the edit continues without losing the caret or the composition and the new object appears; given the other window deletes the object under edit, then the editing window leaves edit mode with a notice "The text was deleted in the other window" and keeps its state valid; given the pen window selects the text object under edit, then nothing changes in the edit. The Text tool is not in the pen window in 2a.
16. Given the pen window's size, position and maximised state, then they are saved in the app's config folder as `pen_window.json` (internal record `format_version` 1: width, height, x, y, maximised **(Proposal)**; unknown fields refused; golden-file test, `CLAUDE.md` §5; 2b takes format 2 with `follow_view` and `canvas_only`) after a move or resize settles (500 ms **(Proposal)**) and on close, and are applied at the next open: size and maximised state always, position and screen where the platform honours them; given Wayland (position ignored by the compositor), then the window opens where the compositor puts it, no error, no message; given a saved position outside every connected screen, then it opens centred on the main window's screen; given a missing, damaged or newer file, then the defaults apply (1200 x 720 logical px **(Proposal)**, not maximised) and a read never rewrites it, and the file never holds project content. The pen window is not reopened automatically at app start; "offered again" means the button is there **(Proposal, ADR 0096 A3)**. No first-run hint about dragging the window to the tablet (customer rule of 2026-10-16: no operating tips).
17. Given a project with relay sync on, then only the main session talks to the relay; a step made in the pen window reaches a collaborator as the same step made in the main window would (through the main window's next sync export; in a shared project without its width profile, ADR 0053, see criteria 23 to 26), and the collaborator's History tab names it "<name> (pen window)" **(Proposal, ADR 0096 A7)**; a collaborator's edit arrives in the main window and reaches the pen window through the link; collaborators' pointers and selections are not shown in the pen window in 2a. The main window's History names a step of this device's pen window "Pen window". Test before the PR against the local relay: updates with the pen peer id inside an envelope of the main peer pass the receivers' import checks.
18. Given an update that fails the import checks of the receiving session, then it is refused as a relay import is, the receiving window shows "Out of sync, reloading..." and asks the main window for a full project message; the main window is the truth; afterwards both documents are equal, the pen window keeps tool and view, its selection and undo start empty with one notice "Reloaded from the main window.", and nothing is written to the file by the refusal.
19. Given no pen window is open, then the main window does no link work (no export per frame, no message) and all existing tests pass unchanged; given any use of the link, then no document content, pressure, device name or tilt is stored, logged or sent anywhere beyond the two windows (the hub keeps no copy; R-SYS-007), and the document `format_version` does not change.
20. Given a 10,000-object project and a project with 200 MB of pictures, then the PR description reports the second webview's memory, the time from the command to the pen window's first frame and the time from a finished stroke to its frame in the other window; targets: first frame at most 3 s, stroke to frame at most 200 ms **(Proposal)**; a miss is a finding for the architect (lazy asset fetch is the named fallback in ADR 0096), not a silent pass.
21. Given the pen page, then it calls only the commands of its allow-list (pen reader, bridge, read-only font readers; ADR 0096 A8) and every other host command (dialogs, export, import, save, open, connector, the main window's project state and recovery, quit) is refused with `not-available-in-this-window` before it acts, by the app and again by the page's host layer; test: `window_commands` allows each listed command for `pen`, refuses every other registered command for `pen`, allows all for `main`, and refuses everything for any other label; a test asserts `pen` has no capability file (it cannot listen to broadcast events); starting the pen page changes nothing of the main window's path, title or unsaved state.
22. Given the pen window opens, then it starts with the Pencil, its view fitted to the main window's active page (the open request carries that page id; the same fit as the Fit page button) with a 24 px margin, and the rail holds exactly Pencil, Select, Node, Undo, Redo, Delete, Zoom in, Zoom out, Fit page and Pan (each at least 48 x 48 CSS px **(Proposal)**, accessible name, tool keys P, S, N); the Pencil shows the Pencil bar unchanged, the Node tool the Node bar without Centre lines, Select no bar; Undo and Redo are dimmed when this window has nothing to undo or redo; Delete deletes the selection (or the selected nodes or width point in Node) as the key does, in one step "Delete"; Zoom in and out step by 1.414 about the centre of the free canvas within 2 % to 8000 %; Pan is a latching toggle that releases on a second tap and when a tool is chosen, and while on a tip drag pans in any tool; keys of tools not in the pen window do nothing; a press on a sketch page shows "Sketch pages are edited in the main window." and draws nothing **(Proposal, UX 9.2, 9.3)**.
23. Given a shared project (relay sync on, or any state in which ADR 0053 refuses width profiles) and a Pencil stroke with pressure, then the stroke is kept as a **uniform stroke at Max width**, selected like any Pencil stroke, in one undo step "Pencil stroke", and the notice "Pressure width is not available in a shared project yet." shows **once per session** for the first such stroke in either window; given the same stroke in an unshared project, then it keeps its width profile; given a stroke without pressure in a shared project, then nothing shows. Same in the main window and the pen window. This fixes a slice 1 defect: today the stroke silently becomes an unselected uniform stroke without a notice **(Proposal; the notice wording)**.
24. Given the link between two sessions in an unshared project, then a Pencil stroke with pressure drawn in one session arrives in the other with an identical centre line and an identical width profile, in both directions and after a reload from the main window's project message (ADR 0096 A12).
25. Given a shared project, then a pressure stroke drawn in the pen window ends as one drawn in the main window (criterion 23), writes no marker for editable curves or width profiles, and the main window keeps syncing with the relay afterwards (no "cannot be shared yet" state appears from the stroke); the main window tells the pen session whenever its blocked state changes.
26. Given a pen update that carries a width profile or editable-curve marker while the main window is shared (the pen session ignored the blocked state), then the main window refuses it as a relay import is, the main document stays unchanged and keeps syncing, and the pen window reloads (criterion 18).

Edge cases: the pen window opened while a stroke is in progress in the main window (the full project message is a committed state, the stroke follows when finished); two pen windows (not possible, criterion 1); pen window open on a project with unsaved changes (it gets them, they are the same project); the main window minimised or on another screen (the link keeps running); the pen display unplugged while the window is open (the window manager moves it, criterion 16 on the next open); the compositor ignores the saved position (criterion 16); Save in the pen window while the main window waits in a modal (criterion 13 blocks it); a very large paste in the main window (the update is one message, the pen window may lag a frame or two, never a partial state).

Out of scope for 2a: the Pen layout, Eraser button, Layer and strip controls, canvas-only, Follow view and the frame of the pen view (2b); collaborators' presence in the pen window; a pen window in the browser build (option A2 of ADR 0096 stays possible later); more than one pen window; a Text tool, Properties panel and export in the pen window; reopening the pen window at app start; an in-app Close button (the window manager's close); canvas rotation and mirroring in the pen window; touch gestures; barrel-button bindings; Windows and macOS placement tuning (the link and the rules are platform-independent, only the pen reader of ADR 0094 differs).

#### Part 2b: Pen layout and Follow view

Depends on 2a (which already holds the pen page's command allow-list, criterion 21). The pen window gets the **one fixed Pen layout** (Frage 413 A; the customer does not want it configurable at all, so there is no layout setting, no customise button and no file for it), the Eraser button, Follow view and the frame in the main window. Details: `ux-notes.md` section 10.

Acceptance criteria:
1. Given the pen window, then it always shows the Pen layout and nothing else: a left rail with the tools Pencil, Eraser, Select, Node, then Undo, Redo, Delete, then Zoom in, Zoom out, Fit page, Pan and Follow view; a slim top strip with Layer, Max width and Smoothing; and a canvas-only toggle in the top-right corner. There is no way to switch the window to the full layout, to reorder or hide its controls, and no layout setting is stored; the rail stays left for everyone, no mirror switch **(Proposal)**. Every control is at least 48 x 48 CSS px **(Proposal)** and works with the pen tip alone (no key, no double-click needed for any function of the layout); a touch press on a layout control does nothing **(Proposal)**.
2. Given the Eraser button on, when the pen tip touches down and moves, then the objects whose line comes within the 8 px hit rule of `0014` of its path are deleted when it lifts, exactly as the eraser end does in slice 1 criterion 12 (hidden and locked objects untouched, one undo step "Erase", nothing created when nothing was touched); given the button off (or a second tap on it), then the tip draws with the Pencil again; the button state is per window and not remembered (a new pen window starts with the Pencil); the real eraser end of a pen that has one keeps working in the Pencil and the Eraser. Reason: the Kamvas 13 pen probably has no eraser end (customer to confirm). Test: the same samples through the Pencil's eraser mode and through the eraser-end samples give the same result.
3. Given the strip, then **Layer** lists the active page's layers (front-most first, hidden and locked ones chooseable with the Pen's notice) and sets the pen window's active layer (session state, no document step) so the next strokes take that layer's machine operation (slice 1 criterion 13); a selection made in either window moves it; a page without layers shows "Page", dimmed. **Max width** and **Smoothing** set the Pencil's Max width and Smoothing with the ranges and defaults of slice 1 criteria 4 and 9. The values are per window and not saved in the document. The strip has no colour control: colour stays the current stroke style, changed in the main window's Style panel **(Proposal, UX 10.3; replaces the earlier Proposal that strip colour applies to the selection)**. With the Node tool the strip shows the Node bar's pen subset at 48 px (no Centre lines, no Close path); with Select nothing.
4. Given the canvas-only toggle, when it is switched on, then the rail and the strip are hidden, the canvas fills the window and only the toggle (at least 48 px, in the same place in both states) stays; the pen still draws; switching off brings the layout back; the state is remembered in `pen_window.json` (`format_version` 2 adds `canvas_only` and `follow_view`; format 1 files read as both false) **(Proposal, ADR 0096 A4)**. The banner of "Waiting for the main window" and "Out of sync, reloading..." stays visible in canvas-only.
5. Given Follow view off (the default **(Proposal)**), then the views stay independent as in 2a; given the Follow view toggle (the View card of the rail) is switched on, then the pen window takes the main window's view at once and afterwards every pan or zoom in either window moves the other to the same centre point and the same zoom (the visible areas differ with the windows' shapes) **(Proposal)**; given it is switched off, then both keep the views they have; the setting is remembered in `pen_window.json`. The main window has no Follow control. Test: two sessions, an alternating series of 20 pans and zooms ends with equal centre and zoom and does not loop (each change is applied once, no echo messages).
6. Given the pen window is open, then the main window draws a frame on its canvas around the area the pen window currently shows (editor overlay, never part of the document, never exported, not selectable, ignored by hit tests, snapping and machine output; the parts outside the main canvas are clipped), updated in the same frame as the pen window's view message arrives: after a pan, a zoom or a window resize the frame's rectangle equals the pen window's visible rectangle in document coordinates within 0.01 mm; "Show pen view" is a switch in the File tab's Pen display section, under the "Show pen window" button, present only while the pen window is open and on each time it opens **(Proposal; there is no View menu since `0051`)**; the frame is not drawn when the pen window is closed or the switch is off.
7. Given the Pen layout, then it calls no command outside the allow-list of 2a criterion 21 (no import, export, open, save dialog, connector; ADR 0096 decision 9); test: every host call the layout's controls make, recorded in a vitest run of the layout, is on the list.
8. Given a pen display of 1920 x 1080 px and a window of 1366 x 768 px, then no control of the layout is clipped or overlapped by another control, and below 640 px of window height the View card moves to a second column **(Proposal; UX notes)**.
9. Given the Pen layout, then every control has an accessible name, a visible focus state and the design-system tokens; no new colours or sizes outside `docs/design-system.md` rows ("Pen rail", "Pen strip", "Pen layer pick", "Pen window states", "Pen window frame", written by UX).

Edge cases: Follow view on while the main window is much larger (the pen window shows a small part, the frame is about the same centre); both windows zoomed with the pen at the same moment (last message wins per window, no loop); Eraser button left on when the pen window is closed (not remembered, the next window starts with the Pencil); canvas-only on and Undo needed (the toggle comes back with one tap, Ctrl+Z still works on a keyboard); the main window shows the frame while the pen window is minimised (frame stays at the last view).

Out of scope for 2b: configuring the Pen layout in any way (Frage 413 A); a colour control in the strip; a radial palette at the pen tip, express-key or barrel-button bindings and canvas rotation (later list); touch gestures; the frame being draggable to move the pen window's view (a later idea); pressure calibration.

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

(`ux-notes.md`: slice 1 (sections 1 to 8), slice 2 (sections 9 to 11: pen window, Pen layout, Follow view, frame). Slices 3 to 6 need their own notes.)

## Links

Requirements: R-INP-001 (absorbed), R-INP-002 to R-INP-005, R-EDIT-050 (variable width, done by `0110`).
Specs: `specs/0032-pen-tablet-input/` (superseded by this epic; its code is on `origin/story/pen-tablet-input`: Schneider fit, variable outline, Pencil tool, pressure curves, reusable for slice 1), `specs/0110-pen-modes-variable-stroke/`, `specs/0033-stroke-brushes/`, `specs/0088-trace-recognition/`, `specs/0091-trace-text-recognition/`.
ADRs: 0001 (Tauri 2, WebKitGTK), 0009 and 0031 (sync), 0053 (centre line and width profile), 0094 (input route, Proposed), 0095 (Pencil pipeline), 0096 (pen window, Accepted).
PR: #522 (slice 0), #525 (slice 1); slice 2a and 2b not yet. Slice 1 defect found 2026-11-27 (a pressure stroke in a shared project silently loses its width without a notice) is fixed by 2a criterion 23.

**Frage 415 (PEN-WINDOW-MECH): Answered A** (customer, 2026-11-27) (ADR 0096, `needs-customer`; slice 2 only, slice 1 does not depend on it). How the pen window holds the document:
- A: a second window with its own copy of the document, kept equal locally all the time; undo per window, saving and recovery in the main window only, pictures twice in memory, no pen window in the browser build
- B: one document drawing into both windows; a trial first, then a large rebuild; one shared undo stack
- C: the document moves into the app process; a very large rebuild, the browser build goes away

Recommendation: A. Default: A.

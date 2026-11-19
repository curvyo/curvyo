# ADRs for raster engraving

Status: Proposed (architect, 2026-11-18). Builds after `0124` (ADR 0066, 0067). Two new ADRs: [ADR 0068](../../docs/adr/0068-raster-dot-values-and-dithering.md) (dot values and dithering, architect) and [ADR 0069](../../docs/adr/0069-raster-rows-output-and-settings.md) (rows, output, preview, checks, stored keys; `needs-customer`: document model, file format, persistence; accepted with the story PR). Decisions marked **(Proposal)** are the architect's and vetoable at the demo. One customer question: Frage 237 (closed 2026-11-19: built A). Amended 2026-11-19 after the customer's feedback on PR #394 (the Image kind, ADR 0069 §13): Fragen 270 and 271, answered the same day 270 B (a fourth material set Image, ADR 0069 §14) and 271 A with a clarification (an image is its frame rectangle in every other kind).

- [ADR 0068](../../docs/adr/0068-raster-dot-values-and-dithering.md): all pixel work in `curvyo-laser-core` (`job_raster_*`), new edges `laser-core → image-core` (for `Arc<WorkingImage>`, no copy) and `libm` (gamma table only), both already in the lock file; dot grid on the hatch lines with the hatch's inside test; grey pyramid per asset with power-of-two box then bilinear; gradient grey from ADR 0066; one tone table; the six dithers and Grey in integers; 50 million dots per item confirmed; budgets 37 a, d, g confirmed. The spec's "ADR 0066" for this call is ADR 0068.
- [ADR 0069](../../docs/adr/0069-raster-rows-output-and-settings.md): `MoveKind::Sweep` into a sweep table (one bit per dot, one byte in Grey), split at gaps over 5 mm; one `runs()` and `run_power` for GRBL, timeline and preview; M4 `G1 … S` per run, M3 `M3`/`M5` per run; coverage pyramid for the zoomed-out Toolpath view, a `k·I` preview grid while editing; new gate causes; seven `op_options` keys; recipe `fill` and `dither`, material test record format 3; project format next free at merge after `0124`'s; budgets 37 b, c, e, f confirmed.
- [ADR 0064](../../docs/adr/0064-laser-job-builder-and-output-writers.md) §2, §4, §6 to §10, §12: units, the hatch frame, the move list, gate, cache and preview are extended, not replaced; "images in Engrave are dropped" (§4) ends here.
- [ADR 0066](../../docs/adr/0066-gradient-fill-in-the-document-model.md) §4: `colour_at` and `over_white_grey`, the one rule for what a gradient engraves.
- [ADR 0056](../../docs/adr/0056-laser-operation-tools-in-the-project-file.md) §5, §6: Interval always present; `op_options` damage rules apply to the new keys.
- [ADR 0057](../../docs/adr/0057-test-grid-object-with-derived-operations.md) §1, §2 and [ADR 0058](../../docs/adr/0058-material-records-shared-across-lasers.md) §2: the recipe gains kind `image` and `dither` (`fill` dropped again, ADR 0069 §13).
- [ADR 0058](../../docs/adr/0058-material-records-shared-across-lasers.md) §2, §6, §7 and [ADR 0055](../../docs/adr/0055-material-records.md) §9: a laser's `sets` gain `image` (interval required) in record format 3 in place; judging an Image test fills it; "no Image set for this laser" as any kind; the undo journal is unchanged (ADR 0069 §14).
- [ADR 0054](../../docs/adr/0054-laser-settings-and-test-grids-in-the-project-file.md) §2, §3: Image operations resolve, link and refresh (material edit, save, export) against the Image set through the one rule; no fallback to Engrave.
- [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md) §4: the working image and the `mip_level` box rule.

## Feature-local decisions

- 2026-11-18: **No "has no interval" check** (ADR 0069 §6). An Engrave always has an Interval (ADR 0056 §5: absent reads 0.10, damage is Damaged settings), so criterion 32's first sentence cannot happen. **For the PO:** drop it or keep only its "No settings" sentence.
- 2026-11-18: **Key names with units** (ADR 0069 §7, Proposal): `threshold_pct`, `brightness_pct`, `contrast_pct`, `lowest_power_pct`, as `overscan_pct` and `scan_angle_deg`; `dither`, `gamma`, `invert` as written. **For the PO:** criterion 13.
- 2026-11-18: **Area average** (ADR 0068 §3, Proposal): the box is the power-of-two level nearest below the dot, then bilinear; criterion 6's "average over the dot's square" is met within a factor of two of the footprint at a fixed cost per dot. **For the PO:** criterion 6.
- 2026-11-18: **Order in Engrave + Outline** (ADR 0069 §2): vector hatch, then raster rows, then all outlines with the Score set. Criterion 14's "rows after the operation's vector paths" is read as "after the vector hatch, before the outlines", which is criterion 2's "dots first". **For the PO:** criterion 14.
- 2026-11-18: **Row step** (ADR 0069 §2): the travel goes from the end of one sweep to the start of the next; it is criterion 15's one-Interval step when both rows have the same extent. Criterion 21's test (a solid square) is that case. **For the PO:** criterion 15.
- 2026-11-18: **Lowest power above the set's power** is capped at the set's power (Proposal), so the ramp never inverts.
- 2026-11-18: **Grey levels** are 1 to 255; a level 0 dot never burns, also in Grey with Lowest power above 0 (white burns nothing, criterion 10).
- 2026-11-18: **Dot-map golden format**: one text line per grid row, `.` off and `#` on; Grey two hex digits per dot; rows outside the frame as spaces.
- 2026-11-18: **Format**: project `format_version` next free at merge after `0124`'s (50 if only `0124` merges first); material record format 3 (it holds the Image set; Frage 237 built A); no migration; the job has no format.

- 2026-11-19: **Image operation kind** (customer feedback on PR #394; ADR 0069 §13, ADR 0068 amendment). A sixth kind `image` engraves images (own, linked clones, members of cloned groups and of placed pages) and gradient-filled shapes as raster; Engrave and Engrave + Outline hatch only again. Image reads the material's Engrave set (Frage 270 default A; superseded the same day by 270 B, see below); the seven image keys are valid on `image` only, so on an Engrave they read as Damaged settings (no migration: unreleased). A plain vector shape in an Image operation runs nothing, silently (Proposal; the PO's default in the spec wins). Cut, Score, Cut-out and Engrave + Outline keep their accepted image rules (Frage 271 default A; sharpened the same day, see below). Test grid: a fourth kind Image (cells always the gradient rectangle, `dither`; `fill` leaves the recipe, Proposal); Engrave grids lose Fill and Dither. Still format 50 and material record format 3; goldens per ADR 0069 §10's note.
- 2026-11-19: **Linked clone of an image is not engraved: root cause** (bug, not an ADR change). `build_leaves` (`curvyo-editor-wasm/src/session/operations/index.rs`) expands only clones whose origin is a group; a clone of an image (or a text) falls to `own_leaf`, whose `shape_of` gives `Outlines { closed: 1, open: 0 }` instead of `Image`, and `geometry_of` calls `leaf_geometry` on the clone snapshot, whose `drawn_path()` is `None` for every picture clone (`primitive_model.rs`), so its geometry is `None`. `operation_block` (`output/input.rs`) then skips the leaf (`geometry[at]` is `None`) and `is_raster` is false: the clone is in no block, not in the job, the Operations view or the Image settings condition. A clone of a text is dropped from every kind by the same path, and on a placed page `placed_leaves` reads every member through `leaf_geometry`, so any picture clone there (group, text or image) is dropped too. Fix: plan task R1.
- 2026-11-19: **Image set** (Frage 270 B; ADR 0069 §14). `SetKind::Image` (text `image`, label "Image", after Engrave); a laser's `sets` gain `image`, shaped as `engrave` with `interval_mm` required, in material record format 3 in place (unreleased; a format-2 record holding it is damaged, like a format-2 Gradient recipe). `SetKind::has_interval()` (Engrave, Image) replaces the Engrave-only interval tests. `main_set(Image) = Image`; **no fallback to the Engrave set** (Proposal): a material without an Image set is "Material has no Image set for this laser" as any kind (Use material skips, a link runs its stored numbers). Refresh on material edit, save and export is kind-generic, nothing new. The test grid's kind is the set kind (no separate `TestKind`); judging an Image test writes the Image set and a `set` history entry of kind `image`. Materials tab: four set rows, Image last. Fixture jobs offer no Image kind.
- 2026-11-19: **An image in a vector kind is its frame rectangle** (Frage 271 A with the customer's clarification; ADR 0069 §13). In Cut, Score, Cut-out, Engrave and Engrave + Outline, `kind_run(kind, Image)` equals `kind_run(kind, one closed outline)` and the session hands the frame on as one closed ring, so the existing code runs it: Score now scores the frame, Engrave hatches it solid, Engrave + Outline hatches and traces it; Cut and Cut-out unchanged. `Lines::Frame` goes. Clones and placed-page members of images the same (R1). Supersedes "Engrave: an image runs nothing" above.
- 2026-11-19: **Future, not built**: Cut-out (and Cut) following the visible contour of an image with a transparent background (ADR 0069, Consequences). **For the PO:** a line in `0133` image-optimisation's backlog row or a story of its own (this agent may not edit `specs/README.md` or `docs/technical-debt.md`).
- 2026-11-19: **Dither picker** becomes an always-visible segmented control (frontend and UX notes only; no ADR, no stored change).

Reconciled 2026-11-18 (product owner): every "For the PO" below is applied in the specification (criteria 6, 11, 13, 14, 15, 32, 42), Lowest power above Power warns (`LowestPowerAbovePower`, ADR 0069 §6).

## Frage 237 (RASTER-RECIPE): Where the Dither of the Image test grid is kept

A test grid's settings (its "recipe") are kept on the grid and also in the material's test record, so "Place on page" from Materials rebuilds the same grid. (Asked 2026-11-18 for Fill and Dither of the Engrave grid; since ADR 0069 §13 the Image grid holds Dither and there is no Fill.)
- **A** The kind Image and Dither go into the recipe like Crosshatch: an Image test placed again from Materials comes back as an Image test with its dither.
- **B** They stay on the grid in the project only: an Image test placed again from Materials does not come back.

Recommendation: **A** (the test is reproducible, which is what the record is for). Default if unanswered: **A**.

**Closed 2026-11-19: built A** (architect review A6). The question's premise, a material format change for the recipe alone, is gone: record format 3 is needed for the Image set anyway (Frage 270 B, ADR 0069 §14).

## Frage 270 (IMAGE-SET): Which material numbers an Image operation uses

A material holds one set of numbers per kind: Cut, Score and Engrave (Speed, Power, Passes, Interval).
- **A** Image uses the material's Engrave numbers. Nothing in the material file or the Materials tab changes; an Image test grid's "Apply" writes the Engrave set, so pictures and vector fills share one Speed and Power per material.
- **B** Materials get a fourth set, Image. Pictures and fills can have different numbers (pictures are often run faster or weaker); the material file format, the Materials tab and its commands change, and a material without an Image set shows "No settings" for Image operations.

Recommendation: **A** now (smaller; B can be added later without breaking files, Image falling back to Engrave when a material has no Image set). Default if unanswered: **A**.

**Answered 2026-11-19: B.** A fourth set Image per laser, added to record format 3 in place (ADR 0069 §14), no fallback to Engrave.

## Frage 271 (IMAGE-OTHER-KINDS): What the other kinds do with a picture

Today Cut cuts a picture's frame, Score skips it, Cut-out cuts around it, Engrave + Outline traces its frame; only the dots move to Image.
- **A** Keep these frame rules; Engrave alone skips pictures (silently).
- **B** Every kind except Image skips pictures entirely (no frame cut, Cut-out ignores them).

Recommendation: **A** (accepted behaviour stays; a frame cut is how a picture tile is cut out). Default if unanswered: **A**.

**Answered 2026-11-19: A, sharpened.** In Cut, Score, Cut-out, Engrave and Engrave + Outline an image is strictly a rectangle, its frame, run by the existing line, hatch and outline code (so Score scores it and Engrave hatches it; ADR 0069 §13). Later idea: Cut-out along the visible contour of an image with a transparent background.

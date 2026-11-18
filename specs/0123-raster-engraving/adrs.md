# ADRs for raster engraving

Status: Proposed (architect, 2026-11-18). Builds after `0124` (ADR 0066, 0067). Two new ADRs: [ADR 0068](../../docs/adr/0068-raster-dot-values-and-dithering.md) (dot values and dithering, architect) and [ADR 0069](../../docs/adr/0069-raster-rows-output-and-settings.md) (rows, output, preview, checks, stored keys; `needs-customer`: document model, file format, persistence; accepted with the story PR). Decisions marked **(Proposal)** are the architect's and vetoable at the demo. One customer question: Frage 237.

- [ADR 0068](../../docs/adr/0068-raster-dot-values-and-dithering.md): all pixel work in `curvyo-laser-core` (`job_raster_*`), new edges `laser-core → image-core` (for `Arc<WorkingImage>`, no copy) and `libm` (gamma table only), both already in the lock file; dot grid on the hatch lines with the hatch's inside test; grey pyramid per asset with power-of-two box then bilinear; gradient grey from ADR 0066; one tone table; the six dithers and Grey in integers; 50 million dots per item confirmed; budgets 37 a, d, g confirmed. The spec's "ADR 0066" for this call is ADR 0068.
- [ADR 0069](../../docs/adr/0069-raster-rows-output-and-settings.md): `MoveKind::Sweep` into a sweep table (one bit per dot, one byte in Grey), split at gaps over 5 mm; one `runs()` and `run_power` for GRBL, timeline and preview; M4 `G1 … S` per run, M3 `M3`/`M5` per run; coverage pyramid for the zoomed-out Toolpath view, a `k·I` preview grid while editing; new gate causes; seven `op_options` keys; recipe `fill` and `dither`, material test record format 3; project format next free at merge after `0124`'s; budgets 37 b, c, e, f confirmed.
- [ADR 0064](../../docs/adr/0064-laser-job-builder-and-output-writers.md) §2, §4, §6 to §10, §12: units, the hatch frame, the move list, gate, cache and preview are extended, not replaced; "images in Engrave are dropped" (§4) ends here.
- [ADR 0066](../../docs/adr/0066-gradient-fill-in-the-document-model.md) §4: `colour_at` and `over_white_grey`, the one rule for what a gradient engraves.
- [ADR 0056](../../docs/adr/0056-laser-operation-tools-in-the-project-file.md) §5, §6: Interval always present; `op_options` damage rules apply to the new keys.
- [ADR 0057](../../docs/adr/0057-test-grid-object-with-derived-operations.md) §1, §2 and [ADR 0058](../../docs/adr/0058-material-records-shared-across-lasers.md) §2: the recipe gains `fill` and `dither`.
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
- 2026-11-18: **Format**: project `format_version` next free at merge after `0124`'s (50 if only `0124` merges first); material record format 3 if Frage 237 A; no migration; the job has no format.

Reconciled 2026-11-18 (product owner): every "For the PO" below is applied in the specification (criteria 6, 11, 13, 14, 15, 32, 42), Lowest power above Power warns (`LowestPowerAbovePower`, ADR 0069 §6).

## Frage 237 (RASTER-RECIPE): Where Fill and Dither of the Engrave test grid are kept

A test grid's settings (its "recipe") are kept on the grid and also in the material's test record, so "Place on page" from Materials rebuilds the same grid.
- **A** Fill and Dither go into the recipe like Crosshatch: the material file format changes once (older Curvyo builds skip such records), and a gradient test placed again from Materials comes back as a gradient test with its dither.
- **B** Fill and Dither stay on the grid in the project only: no material format change, but a gradient test placed again from Materials comes back as a plain Solid test.

Recommendation: **A** (the test is reproducible, which is what the record is for). Default if unanswered: **A**.

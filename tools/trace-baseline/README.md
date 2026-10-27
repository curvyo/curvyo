# Trace baselines (offline)

The quality harness of `curvyo-vectorize-core` (`specs/0025-raster-trace` criteria 27 to 29,
ADR 0023 §7) compares our trace with Inkscape's Trace Bitmap. Inkscape (GPL) never runs in CI and
none of its output is committed: it runs here, by hand, and only its numbers go into
`curvyo-vectorize-core/tests/fixtures/trace/baseline.json`.

1. Write the synthetic black and white set (PGM, one per image of `research.md` section 5):

   ```text
   cargo test --release -p curvyo-vectorize-core --test quality write_quality_set -- --ignored --nocapture
   ```

   The pictures land in `target/trace-quality/` with their resolution printed beside them.

2. For each picture, in Inkscape 1.x: import it (embed, its resolution as printed), Path, Trace
   Bitmap, Single scan, Brightness cutoff at the automatic Threshold the harness uses (it prints
   it; 0.5 is close for these pictures), everything else at its default. Delete the bitmap, set the
   page to the content's size of the picture, and save as Plain SVG to
   `target/trace-baseline/<image>.inkscape-default.svg`. Repeat with the best manual settings
   (Speckles, Smooth corners, Optimize) as `<image>.inkscape-best.svg`.

3. Measure and store the numbers (the harness reads the SVGs with `curvyo-svg-core`, splits them
   into outlines and measures them like ours):

   ```text
   TRACE_BASELINE_DIR=target/trace-baseline cargo test --release -p curvyo-vectorize-core \
     --test quality quality_table -- --ignored --nocapture
   ```

   This rewrites `baseline.json` with `inkscape_default`, `inkscape_best` and our `candidate`.
   Without `TRACE_BASELINE_DIR` the run compares with the committed numbers and reports every
   miss of the criterion 28 targets; the relative targets (node density at most 60 %, wobble at
   most 50 % of Inkscape's) need the Inkscape numbers.

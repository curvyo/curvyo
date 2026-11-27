# Notice: curvyo-inkstitch-core

This crate is a Rust port of parts of **Ink/Stitch**, an embroidery extension for Inkscape.

- Upstream: https://github.com/inkstitch/inkstitch
- Upstream licence: GNU General Public License version 3.0 or later ("Licensed under the GNU GPL
  version 3.0 or later", file headers, checked in `lib/stitches/fill.py` on 2026-11-27).
- Upstream copyright: "Copyright (c) 2010 Authors"; authors: see the upstream git history.
- Licence of this crate: GPL-3.0-or-later (the full text is in `LICENSE`). The Curvyo application
  that links it is conveyed under AGPL-3.0-or-later (GPLv3 §13, AGPLv3 §13; ADR 0097 decision 7).
- Pinned upstream commit: `42e771e9d4338ce32a80f4d8c0df186a8c8210e7` (2026-11-03), read on
  2026-11-27. Every file below was ported from that commit.

## Ported files (GPLv3 §5a)

Each entry names the upstream path, the date of the port and the nature of the changes.

| Upstream path | Rust module | Date | Changes |
|---|---|---|---|
| `lib/stitches/running_stitch.py` (`AngleInterval`, `cut_segment_with_angle`, `cut_segment_with_circle`, `take_stitch`, `stitch_curve_evenly`, `path_to_curves`, `even_running_stitch`) and `lib/elements/stroke.py` (`Stroke.running_stitch`, the repeats) | `running_stitch` | 2026-11-27 | Rust; points are `kurbo::Point` in mm instead of px; one stitch length instead of a length list; no random stitch length, bean stitch or zigzag; the cuts that upstream assumes exist are clamped onto their segment, and an empty sleeve falls back to the last point inside it. |
| `lib/stitches/fill.py` (`legacy_fill`, `east`, `north`, `row_num`, `adjust_stagger`, `stitch_row`, `intersect_region_with_grating`, `section_to_stitches`, `is_same_run`, `pull_runs`) | `tatami_fill` | 2026-11-27 | Rust; mm instead of px; the shapely line-polygon intersection is replaced by an own even-odd scanline clip; `is_same_run`'s 90 % quadrilateral area test is replaced by an overlap of the runs along the rows and a midpoint-inside test; staggers fixed at 4; no end row spacing, flip, reverse, skip-last or random stitch length; a row start is pushed once, not twice. The travel graph of `lib/stitches/tatami_fill.py` is not ported: sections are joined by jumps. |
| `lib/elements/satin_column/stitches.py` (`_do_satin`) and `lib/elements/satin_column/rails.py` (`plot_points_on_rails`) | `satin_zigzag` | 2026-11-27 | Rust; one line of constant width instead of two rails: both rail points of a station are the line point plus and minus half the width along its normal; stations evenly spaced at most the zigzag spacing apart; no underlay, pull compensation, short stitches, split stitches or randomness. |
| `lib/stitch_plan/lock_stitch.py` (`RelativeLock`, `half_stitch`) | `ties` | 2026-11-27 | Rust; mm instead of px; only the default half stitch, unscaled. |
| (none, own code) | `units` | 2026-11-27 | The `Mm` and `Angle` newtypes of the port. |

# Real Inkscape files of the slice 3 tester

`inkscape_shapes.svg` and `inkscape_shapes.paths.svg` were written by **Inkscape 1.4.4
(dcaf3e7d9e, 2026-05-05)** on 2026-10-27 by the tester of `specs/0024-svg-import-export` slice 3
(criteria 111 to 127), project licence. They replace the hand-written `inkscape_shapes.svg` of the
implementer. Used by `tests/acceptance_0024_slice3_tester.rs`, `tests/svg_import_inkscape_shapes.rs`
(the kinds of criterion 125) and the golden `inkscape_shapes.document.json`.

## How they were made

Inkscape's shape tools are interactive, so the seed
`inkscape_shapes_src/seed_inkscape_shapes.svg.txt` holds only each object's tool numbers
(`sodipodi:type` arc, star, spiral, `inkscape:box3d` with six `box3dside` children, a `rect`)
and **no `d` at all**; every `d` in the files is Inkscape's own. Run
`TMPDIR=<scratch dir> inkscape_shapes_src/make_inkscape_shapes.sh`:

1. Pass 1 moves every object by 0,0 (`transform-translate:0,0`), which makes Inkscape rebuild
   and write it, and saves (`export-plain-svg:false`).
2. Pass 2 opens that file, moves every object by 0,0 again, rotates `a_rot30` by 30°, `p6_rot`
   by 15° and `s_rot` by 20° (`transform-rotate`), flips `a_mirror` and `s_flip`
   (`object-flip-horizontal`) and saves `inkscape_shapes.svg`.
3. `object-to-path` of every object of that file gives `inkscape_shapes.paths.svg`, the geometric
   reference (Object to Path).

The seed's `inkscape:version` attribute is copied through: command-line export does not write it.
The `sodipodi:docname` is set to the file name afterwards (`sed`).

## What Inkscape 1.4.4 does with these objects (observed)

- **`sodipodi:r2` is always written.** A flat-sided polygon without `r2`/`arg2` in the seed
  (`p3`, `p6`) is saved with `sodipodi:r2="0"` and `sodipodi:arg2="0"`. No star or polygon
  saved by Inkscape 1.4.4 lacks `r2`.
- An arc-tool ellipse **without angles** is saved as an `<ellipse>` element, not as a
  `sodipodi:type="arc"` path (`a_whole`); a sweep of 2π − 0.0001 rad stays an arc path with an
  open `d` (`a_full`).
- Arcs in `d` are `A` commands split at most every 90°. A chord and an arc are written with
  `sodipodi:open="true"` next to `arc-type`. The first write of a chord made from bare numbers has
  no closing `Z`; any later write has it (hence pass 2). A negative start angle is stored plus 2π
  (`a_neg`: −1 as 5.2831853).
- A rotated or flipped arc keeps `cx`/`cy` moved and a `transform` (`rotate(30)`,
  `scale(-1,1)`); a rotated polygon or star keeps its numbers and gets `rotate(a,cx,cy)`; a
  flipped star gets a `matrix(-1,0,0,1,…)`.
- The faces of a 3D box are reordered by depth when written (f3 f1 f4 f2 f5 f6), and Object to
  Path of the box renames them `path1` to `path6`.
- Object to Path of an `rx ≠ ry` rectangle writes the corners with two decimals (handles 5.54,
  2.77), 0.0066 mm from the true elliptical corner.

## SHA-256

```text
4650b2c78717d1f788a88c31078c4ea707d03e6b7d4215ce1149b711a5a5c7ef  inkscape_shapes.svg
0d34d59f9ef485c3883d3d0054c2d87eef5d881dc7301e6df8eea4422470e893  inkscape_shapes.paths.svg
d47604dbe31400d7d374b0a65c14a6b04bbce417de3ee3920ebb2067f6bc4823  inkscape_shapes_src/seed_inkscape_shapes.svg.txt
6c67a869c273824a71f594251d1b0cf23e5ac015172037d61d7543ea0accffe7  inkscape_shapes_src/make_inkscape_shapes.sh
```

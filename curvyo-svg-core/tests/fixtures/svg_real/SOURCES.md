# Real SVG files for the import tests

Tester fixtures of `specs/0024-svg-import-export` (criteria 15 to 51, 68 to 70), used by
`curvyo-svg-core/tests/acceptance_0024_tester.rs` and `curvyo-editor-wasm/tests/acceptance_0024_tester.rs`.
Unlike `curvyo-ui-core/tests/fixtures/svg/` (hand-written in the layout of the programs), every
file here was written by the program named, or downloaded unchanged from its original source on
2026-10-26. Downloaded files are parser input only; nothing in them is run.

## The Inkscape oracle

Each `name.inkscape-bbox.csv` holds Inkscape 1.4.4's bounds of every element of `name.svg` that is
drawn as one leaf (a shape, or a `use` of one; not inside `defs`, `symbol`, `clipPath`, `mask`,
`pattern`, `marker`, `switch` or `text`; no `clip-path`, `mask` or `filter` on it or an ancestor),
as `id,x,y,width,height` in Inkscape px (96 per inch). They were measured with
`inkscape --query-all` on a scratch copy in which `select-by-selector:*;object-set-property:stroke,none`
removed every stroke, so the bounds are geometric. Script: `inkscape_oracle.py`
(`TMPOUT=<scratch dir> python3 -I inkscape_oracle.py name.svg name.inkscape-bbox.csv`).
Inkscape reads px as 1/96 inch also in Illustrator files; the test converts with 1/72 there
(criterion 19 (a)).

## inkscape/ (made here with Inkscape 1.4.4 (dcaf3e7d9e, 2026-05-05), project licence)

| File | How it was made |
|---|---|
| `inkscape_mm_a4.svg` | `make_inkscape_mm_a4.sh` on `seed_mm_a4.svg.txt` (hand-written seed: A4 mm page, namedview with `inkscape:pageopacity="0.0"`, layers Cut, Engrave (locked, 50 %), Reference (hidden), Empty). Inkscape actions: stars rebuilt, Alt+D clones of `p1` moved, rotated 30°, mirrored, scaled 2 (`clone`, `transform-*`, `object-flip-horizontal`), a clone of a rectangle, `r4` rotated 30°, two circles combined twice with `path-combine` and `fill-rule:evenodd` (donut, overlapping squares), the text "Boho" converted with `object-to-path`, group `g1` rotated −20° and a clone of its path; saved as Inkscape SVG. |
| `inkscape_mm_a4_plain.svg` | `inkscape inkscape_mm_a4.svg --actions="export-plain-svg:true;export-filename:inkscape_mm_a4_plain.svg;export-type:svg;export-do"` |
| `inkscape_in_css.svg` | `seed_in_css.svg.txt` (8 × 4 in, user unit px, `<style>` sheet with element, class, id, combinator and pseudo-class rules, nested groups with translate, rotate(30), matrix, skewX and a non-uniform scale, even-odd triple ring and pentagram, page colour #336699 at 0.5, document units in) saved by Inkscape (`export-plain-svg:false`). |
| `inkscape_in_css_plain.svg` | the same seed exported as plain SVG. |

## w3c/ (W3C SVG 1.1 Second Edition test suite)

Source: `https://www.w3.org/Graphics/SVG/Test/20110816/svg/<name>.svg`, downloaded 2026-10-26.
Licence: W3C Test Suite Licence / 3-clause BSD Licence (dual licensed, "Copyright 2009 World Wide
Web Consortium"; https://www.w3.org/copyright/test-suites-licence/).
Files: `coords-trans-01-b`, `coords-trans-07-t`, `coords-trans-09-t`, `coords-units-01-b`,
`coords-viewattr-01-b` (refused: its DOCTYPE entities hold markup, criterion 68),
`painting-fill-01-t`, `painting-fill-02-t`, `painting-stroke-01-t`, `paths-data-01-t`,
`paths-data-02-t`, `paths-data-03-f`, `paths-data-10-t`, `shapes-circle-01-t`,
`shapes-ellipse-01-t`, `shapes-line-01-t`, `shapes-polygon-01-t`, `shapes-polyline-01-t`,
`shapes-rect-01-t`, `shapes-rect-02-t`, `struct-defs-01-t`, `struct-group-01-t`,
`struct-use-01-t`, `struct-use-03-t`, `styling-css-01-b`, `styling-css-02-b`.

## wikimedia/ (Wikimedia Commons, public domain)

Downloaded 2026-10-26 from `https://upload.wikimedia.org/wikipedia/commons/<path>`; licence read
from the Commons API (`extmetadata.LicenseShortName` = "Public domain") the same day.

| File | Path | Notes |
|---|---|---|
| `Biohazard_symbol.svg` | `c/c0/Biohazard_symbol.svg` | clip path and mask (ignored, criterion 48) |
| `Flag_of_Japan.svg` | `9/9e/Flag_of_Japan.svg` | px size, no viewBox |
| `Flag_of_Switzerland.svg` | `f/f3/Flag_of_Switzerland.svg` | 512 px onto a 32 viewBox |
| `Peace_symbol.svg` | `4/4f/Peace_symbol.svg` | strokes, `rotate(a cx cy)` |
| `Radiation_warning_symbol.svg` | `0/0b/Radiation_warning_symbol.svg` | `use` with `scale(-1,1)` and `rotate(120)`, mask |
| `Recycling_symbol.svg` | `7/7b/Recycling_symbol.svg` | saved by Inkscape 1.0, mm page |
| `Star_of_David.svg` | `4/49/Star_of_David.svg` | compound path, wide stroke |
| `Yin_yang.svg` | `1/17/Yin_yang.svg` | viewBox only, negative origin, arcs |

## illustrator/ (Adobe Illustrator exports from open-source test suites)

| File | Source | Licence | Notes |
|---|---|---|---|
| `topcoat_home.svg`, `topcoat_cart.svg`, `topcoat_brush.svg` | `https://raw.githubusercontent.com/topcoat/icons/master/svg/{home,cart,brush}.svg` (commit 1f96d6286932ffcea8ab1182bbd91e21f2f943db) | Apache-2.0 | Illustrator 16.0.4, DOCTYPE with the `ns_*` entities |
| `svgo_svg2js_test.svg` | `https://raw.githubusercontent.com/svg/svgo/main/test/svg2js/test.svg` (commit 96e756b1c5aedeb3d6b8532033060bcd17bbb76e) | MIT | Illustrator 15.0.0, `<style>` in CDATA, lengths with `px` |

## SHA-256

```text
fee195596e4546b9c4199a713d8888d983666af4482cb2032cf09896ade895ca  illustrator/svgo_svg2js_test.svg
0395dcdad0ccf56e86f656681356321a8e047dc86fc7f28308bea442660073bb  illustrator/topcoat_brush.svg
e0df4a56ad5b055e3e1a779efcbb48d2331cfd0416930a8d749fa01f42113248  illustrator/topcoat_cart.svg
a3cff21ae22442702795e78cfe199dbdfa21d5c37a220fb50c8f3e28e7ceb7c1  illustrator/topcoat_home.svg
6deaae2310dd12dae26269460b196f092d1e4c5206a80b04ca8c218b7abfc158  w3c/coords-trans-01-b.svg
6d31db6833cdb6fd554d3dd4f3ff67ee706d3a43840f6a4d522823565511281f  w3c/coords-trans-07-t.svg
5e953c9706562057d2d72cac4160a13e10d2b2611094df3fca9af9ac9d7dfe35  w3c/coords-trans-09-t.svg
b35fe337db73d3ab8f4d8c815fd36742936e8b7813c369f3f134e9cc23a7ebe5  w3c/coords-units-01-b.svg
ff014a8f87ccfe05b59dd9ce54adea93a8b1cad8a4c0ce84b075413150005754  w3c/coords-viewattr-01-b.svg
ea79ff2e62a7e52e6a7211860c5f162e8f6f5d20cfe659aa30baf9a6a667b125  w3c/painting-fill-01-t.svg
42c44aaad7e361f57f2f4d9aa3684fb9fdce86fbbce348bcaac3a24b1582be38  w3c/painting-fill-02-t.svg
db7134500348c8936421970b4546606177ef468ab269608ce4d631d49c1c8529  w3c/painting-stroke-01-t.svg
f33ffa2e22037943add890d492f66d92b1ff6ab9d42125255d5efe58165fa13c  w3c/paths-data-01-t.svg
12c20b87175edc14344e0468644cef93fa5dba4a4181e0dd765aca9295adc0ca  w3c/paths-data-02-t.svg
481329ef9e971c56adc879c5d9a0e7f170ed46243f275f1d34f0c635ac3aaf1a  w3c/paths-data-03-f.svg
4f54b92e6416e6c9f2ef40efa9c04c9c5d27e9b26a7f95d6e2648d71f2af5026  w3c/paths-data-10-t.svg
ac94b9981a472c9f59c3c2133d9d98de497d433b5b103804f312e52eaa7bdf3a  w3c/shapes-circle-01-t.svg
c56e32b48ecac021f2b7971a6f943feac5a70efab60335062dbfe18853f2b228  w3c/shapes-ellipse-01-t.svg
3c04eb555f4232e5ef213e75b5d789bc7aa14edd9ab40cb08a757aea78c34084  w3c/shapes-line-01-t.svg
a9458883495032b6f28b8f936b01047a7c57d2919525aff9f78e829f8be591a9  w3c/shapes-polygon-01-t.svg
df185fdd31c2df3fdb796105dd8a06bdf9a512863d3529338ff88d7d04d196eb  w3c/shapes-polyline-01-t.svg
e0d5298a759a920abb0803a507eaf80584a9586a12e0cf54c7efe5d50d14fbba  w3c/shapes-rect-01-t.svg
f10cc2dc53ad3b46efdeb995d1b6f82a07757b1bca496f462d257311c073a703  w3c/shapes-rect-02-t.svg
962069282e41e91a6fc64e23fbd5590f3a457cf57d94560bd3eaae00664c0138  w3c/struct-defs-01-t.svg
0a2a946764866d7ba39a161cd129ecf13d400a5dc5420ec821b5d729fff07f2a  w3c/struct-group-01-t.svg
e03e24264422e25e234ea9041985da39598db4b22ff4b8469950c3a08cff64bf  w3c/struct-use-01-t.svg
a1abb39cfff1e032d66a3616b3d2c1c1e34e6dcd5fbe1995883c2e2be50f0c8f  w3c/struct-use-03-t.svg
be2801e5807d744a51d7be961669ca3487fedee17e8ebed4909d08fc5e2f6880  w3c/styling-css-01-b.svg
3948d71f6b98c1c08493688b3b4776a070d6f21e05ca57fd8160cb0b76df3c2c  w3c/styling-css-02-b.svg
ae18e5cc74e5d150adef97a08fb8d094f027e5ea788ceab7a95e5db54ebb9830  wikimedia/Biohazard_symbol.svg
35e785339e19d1ec1987cb9c3d8e66fc97f29a287db0b0b590b8dfdd96d4766b  wikimedia/Flag_of_Japan.svg
900a0befb655721c1b52b5ec2754f2f553f52b230443aaf016cc224e6a0707ce  wikimedia/Flag_of_Switzerland.svg
5e99f888049030aad3f380852e4de81900a5ea0a9693fcc33ca469d5a719ca6f  wikimedia/Peace_symbol.svg
8ed8917051b3389f6dd05b1325a5cd9d8a835ec220d76c0963130c2b3ba967d0  wikimedia/Radiation_warning_symbol.svg
79227f964c8ded56d19a0dad4b4b26741e809bb9813fc53c7e6c632e043493b1  wikimedia/Recycling_symbol.svg
ff14f6628c6a3de12a6ee6ccbe2c26e04b3eeeba76eb4ffeb28d85e0c555c9b8  wikimedia/Star_of_David.svg
e6e74a305acde52cdcb070d8fdf7a46847bea34060c484ea58481cc8b98fdb98  wikimedia/Yin_yang.svg
```

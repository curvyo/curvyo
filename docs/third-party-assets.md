# Third-party assets

Artwork and other non-code assets taken from other projects, with source and licence. Code
dependencies are covered by `Cargo.lock`, `frontend/package-lock.json` and `deny.toml`.

## FreeCAD Sketcher constraint icons

- **Used in:** `frontend/src/components/sketch/icons/constraintIcons.tsx` (Sketch constraint rail
  card and canvas constraint symbols, `specs/0066-constraint-sketcher` AC24, AC32).
- **Source:** FreeCAD, <https://github.com/FreeCAD/FreeCAD>, commit
  `faf7e671f47eb04f8d6a5a52a721ac42a342f276`, directory
  `src/Mod/Sketcher/Gui/Resources/icons/constraints/`.
- **Files:** `Constraint_PointOnPoint.svg` (Coincident), `Constraint_Horizontal.svg`,
  `Constraint_Vertical.svg`, `Constraint_Parallel.svg`, `Constraint_Perpendicular.svg`,
  `Constraint_Tangent.svg`, `Constraint_EqualLength.svg` (Equal), `Constraint_Lock.svg` (Fixed),
  `Constraint_Symmetric.svg`, `Constraint_PointOnMidPoint.svg` (Midpoint),
  `Constraint_PointOnObject.svg` (On axis), `Constraint_Length.svg` (Dimension).
- **Authors:** as recorded in each file's metadata: [wmayer] (creator), [agryson] Alexander
  Gryson (contributor); `Constraint_PointOnPoint.svg`: [bitacovir] (creator), [agryson] Alexander
  Gryson and [Abdullah Tahiri] (contributors).
- **Licence:** each file states "FreeCAD LGPL2+" with the licence
  <https://www.gnu.org/copyleft/lesser.html>; FreeCAD's repository licence is the GNU Lesser
  General Public License 2.1. LGPL-2.1-or-later is compatible with Curvyo's AGPL-3.0-or-later.
- **Changes:** the colour 64px originals are not included. Curvyo uses single-colour line
  redrawings of their shapes in a 16px grid (`currentColor`, 1.5px stroke), which as derived works
  remain under LGPL-2.1-or-later.
- **Checked:** 2026-10-31, by downloading the files at the commit above and reading their
  metadata.

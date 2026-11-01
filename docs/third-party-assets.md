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
  `Constraint_PointOnObject.svg` (On axis), `Constraint_Length.svg` (Dimension, aligned),
  `Constraint_HorizontalDistance.svg` (Horizontal distance), `Constraint_VerticalDistance.svg`
  (Vertical distance; both added 2026-10-31, customer revision 4, AC44).
- **Authors:** as recorded in each file's metadata: [wmayer] (creator), [agryson] Alexander
  Gryson (contributor); `Constraint_PointOnPoint.svg`: [bitacovir] (creator), [agryson] Alexander
  Gryson and [Abdullah Tahiri] (contributors).
- **Licence:** each file states "FreeCAD LGPL2+" with the licence
  <https://www.gnu.org/copyleft/lesser.html>; FreeCAD's repository licence is the GNU Lesser
  General Public License 2.1. The licence text ships with Curvyo as
  [`docs/licenses/LGPL-2.1.txt`](licenses/LGPL-2.1.txt) (verbatim from
  <https://www.gnu.org/licenses/old-licenses/lgpl-2.1.txt>, fetched 2026-10-31).
  `constraintIcons.tsx` as a whole is under LGPL-2.1-or-later, not under Curvyo's
  AGPL-3.0-or-later; the two licences are compatible, so the file can be combined with the rest
  of Curvyo.
- **Changes:** the colour 64px originals are not included. Curvyo uses single-colour line
  redrawings of their shapes in a 16px grid (`currentColor`, 1.5px stroke), which as derived works
  remain under LGPL-2.1-or-later.
- **Checked:** 2026-10-31, by downloading the files at the commit above and reading their
  metadata (the two distance files: creator [wmayer], contributor [agryson] Alexander Gryson,
  "FreeCAD LGPL2+").

## PaddleOCR Latin text recognition weights

- **Used in:** `frontend/public/models/paddleocr-latin/` (`det.onnx`, `rec.onnx`, `dict.txt`), read
  by `curvyo-ocr-core` in the trace worker (`specs/0091-trace-text-recognition`, ADR 0027).
- **Source:** the ONNX conversion <https://huggingface.co/monkt/paddleocr-onnx> (`detection/v3`,
  `languages/latin`), of PaddleOCR's `PP-OCRv3_mobile_det` and `latin_PP-OCRv5_mobile_rec`,
  <https://github.com/PaddlePaddle/PaddleOCR>, fetched 2026-10-31, unmodified.
- **Licence:** Apache-2.0; the text ships next to the files as `LICENSE`. The record with the
  SHA-256 of each file is `model.toml`, checked by `curvyo-ocr-core/tests/model_record.rs`.
- **Training data:** not itemised upstream; accepted by the customer (Frage 112 A, 2026-10-31).

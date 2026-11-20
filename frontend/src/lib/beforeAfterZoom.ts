/**
 * The zoom and pan of the Before and after pane (`specs/0135-image-optimisation` `ux-notes.md` §7).
 * 100 % is the document's 100 % (`spec` criterion 12): one dot is Interval mm wide on screen at 96 CSS
 * px per inch; the pane opens fitted; the zoom is held to 1 % to 3,200 %.
 */

/** CSS pixels per millimetre at 100 % (`curvyo-ui-core`'s `PX_PER_MM_AT_100`). */
export const PX_PER_MM_AT_100 = 96 / 25.4;

/** The zoom factor of one wheel notch. */
export const NOTCH_FACTOR = 1.25;

/** The smallest and largest zoom, as a factor (1 is 100 %). */
export const ZOOM_MIN = 0.01;
export const ZOOM_MAX = 32;

/** The share of the viewport a fitted picture fills, so its edge is visible. */
const FIT_SHARE = 0.96;

/** `zoom` held to the range. */
export function clampZoom(zoom: number): number {
  if (!Number.isFinite(zoom)) return 1;
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, zoom));
}

/** The zoom at which `image` (in dots) just fits `viewport` (in CSS px); 1 for an empty viewport. */
export function fittedZoom(viewport: { width: number; height: number }, image: { width: number; height: number }): number {
  if (viewport.width <= 0 || viewport.height <= 0 || image.width <= 0 || image.height <= 0) return 1;
  return clampZoom(Math.min(viewport.width / image.width, viewport.height / image.height) * FIT_SHARE);
}

/** The pan after a zoom from `from` to `to` about the point `at` (CSS px from the viewport's
 * centre), so the dot under the pointer stays under it. */
export function zoomedAbout(
  pan: { x: number; y: number },
  at: { x: number; y: number },
  from: number,
  to: number,
): { x: number; y: number } {
  const ratio = to / from;
  return { x: at.x - (at.x - pan.x) * ratio, y: at.y - (at.y - pan.y) * ratio };
}

/** The size in page mm of a picture placed by the unit-square map `a b c d e f`. */
export function placedSizeMm(placement: readonly number[]): { width: number; height: number } {
  const [a = 0, b = 0, c = 0, d = 0] = placement;
  return { width: Math.hypot(a, b), height: Math.hypot(c, d) };
}

/** The CSS px size at 100 % of a picture `size` mm. */
export function sizeAt100(size: { width: number; height: number }): { width: number; height: number } {
  return { width: size.width * PX_PER_MM_AT_100, height: size.height * PX_PER_MM_AT_100 };
}

/** The zoom after a wheel turn of `deltaY` (`deltaMode` 1 is lines, else pixels): one notch
 * (100 px or 3 lines) is a factor of 1.25, a trackpad's small steps a part of it. */
export function wheelZoom(zoom: number, deltaY: number, deltaMode: number): number {
  const notches = deltaMode === 1 ? deltaY / 3 : deltaY / 100;
  return clampZoom(zoom * NOTCH_FACTOR ** -notches);
}

/** Whether `pixels` wide shown at `shownPx` CSS px (device ratio `dpr`) need smooth, area-averaged
 * scaling: fewer device pixels than picture pixels. */
export function needsSmoothing(pixels: number, shownPx: number, dpr: number): boolean {
  return shownPx * dpr < pixels;
}

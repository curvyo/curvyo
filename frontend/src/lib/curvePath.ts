/**
 * The drawn path of a Curves row's graph (`specs/0135-image-optimisation` criterion 5): the
 * monotone cubic of the session's arithmetic, for the drawing only.
 */

/** The graph's side in px (`ux-notes.md` §3.8: 120 x 120). */
export const CURVE_SIDE = 120;

/** The monotone cubic through `points` (Fritsch-Carlson, the session's arithmetic) as an SVG
 * path of `CURVE_SIDE` px, for the drawing only; the dots are the session's. */
export function curvePath(points: [number, number][]): string {
  const n = points.length;
  if (n < 2) return "";
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const d = xs.slice(1).map((x, k) => (ys[k + 1] - ys[k]) / (x - xs[k]));
  const m = xs.map((_, k) => (k === 0 ? d[0] : k === n - 1 ? d[n - 2] : d[k - 1] * d[k] <= 0 ? 0 : (d[k - 1] + d[k]) / 2));
  for (let k = 0; k < n - 1; k += 1) {
    if (d[k] === 0) {
      m[k] = 0;
      m[k + 1] = 0;
      continue;
    }
    const a = m[k] / d[k];
    const b = m[k + 1] / d[k];
    const s = a * a + b * b;
    if (s > 9) {
      const t = 3 / Math.sqrt(s);
      m[k] = t * a * d[k];
      m[k + 1] = t * b * d[k];
    }
  }
  const px = (x: number, y: number) => `${((x / 100) * CURVE_SIDE).toFixed(1)} ${(CURVE_SIDE - (y / 100) * CURVE_SIDE).toFixed(1)}`;
  let path = `M${px(xs[0], ys[0])}`;
  for (let k = 0; k < n - 1; k += 1) {
    const h = xs[k + 1] - xs[k];
    path += ` C${px(xs[k] + h / 3, ys[k] + (m[k] * h) / 3)} ${px(xs[k + 1] - h / 3, ys[k + 1] - (m[k + 1] * h) / 3)} ${px(xs[k + 1], ys[k + 1])}`;
  }
  return path;
}


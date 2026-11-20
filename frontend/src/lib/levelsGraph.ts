/** The geometry of the Levels histogram (`specs/0135` criterion 16, `ux-notes.md` §3.8). */

export const LEVELS_WIDTH = 228;
export const LEVELS_BARS_HEIGHT = 64;
export const LEVELS_MARKS_HEIGHT = 12;

/** The fraction along the grey axis (0 black to 1 white) of each of the three Levels marks: the
 * Black point, the grey that Gamma turns into mid grey, the White point (`0123` convention:
 * `out = in^(1/γ)` between the points). */
export function levelsMarks(black: number, white: number, gamma: number): [number, number, number] {
  const b = black / 100;
  const w = white / 100;
  const mid = b + (w - b) * Math.pow(0.5, Math.max(gamma, 0.01));
  return [b, mid, w];
}

/** The path of 256 hairline bars, each 0 to 255 of the graph's height. */
export function barsPath(bars: readonly number[]): string {
  const step = LEVELS_WIDTH / bars.length;
  let path = "";
  bars.forEach((bar, i) => {
    if (bar <= 0) return;
    const x = (i + 0.5) * step;
    const h = (bar / 255) * (LEVELS_BARS_HEIGHT - 2);
    path += `M${x.toFixed(2)} ${LEVELS_BARS_HEIGHT - 1}V${(LEVELS_BARS_HEIGHT - 1 - h).toFixed(2)}`;
  });
  return path;
}


/** The three handles of the Levels graph, in order along the axis. */
export const LEVELS_HANDLES = [
  { key: "black_pct", name: "Black" },
  { key: "gamma_centi", name: "Gamma" },
  { key: "white_pct", name: "White" },
] as const;

/** The field value handle `which` (0 Black, 1 Gamma, 2 White) takes at fraction `at` of the axis,
 * held to the field's range as the other two points stand (Black at most White − 2, White at
 * least Black + 2, whole percent; Gamma 0.10 to 4.00, two decimals). */
export function levelsValueAt(which: 0 | 1 | 2, at: number, black: number, white: number): number {
  const pct = Math.round(Math.min(1, Math.max(0, at)) * 100);
  if (which === 0) return Math.min(pct, white - 2, 98);
  if (which === 2) return Math.max(pct, black + 2, 2);
  const span = (white - black) / 100;
  const t = span > 0 ? (at - black / 100) / span : 0.5;
  const clamped = Math.min(0.999, Math.max(0.001, t));
  const gamma = Math.log(clamped) / Math.log(0.5);
  return Math.round(Math.min(4, Math.max(0.1, gamma)) * 100) / 100;
}

/** The value after an arrow on handle `which`: its mark moved by `stepPct` % of the axis. */
export function levelsValueStepped(which: 0 | 1 | 2, stepPct: number, black: number, white: number, gamma: number): number {
  const marks = levelsMarks(black, white, gamma);
  return levelsValueAt(which, marks[which] + stepPct / 100, black, white);
}

/** "125 × 75 mm": a page size for a pick row. */
export function sizeText(widthMm: number, heightMm: number): string {
  const n = (v: number) => (Number.isInteger(v) ? String(v) : v.toFixed(1));
  return `${n(widthMm)} × ${n(heightMm)} mm`;
}

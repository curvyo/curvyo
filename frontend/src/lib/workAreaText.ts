import type { UnitSymbol } from "@/hooks/useDocumentPanel";

/** Millimetres per display unit. */
const MM_PER: Record<UnitSymbol, number> = { mm: 1, cm: 10, in: 25.4 };
/** Decimals a length shows in each unit, as the Width and Height fields (`0059`). */
const DECIMALS: Record<UnitSymbol, number> = { mm: 3, cm: 4, in: 4 };

function length(mm: number, unit: UnitSymbol): string {
  const scale = 10 ** DECIMALS[unit];
  return String(Math.round((mm / MM_PER[unit]) * scale) / scale);
}

/**
 * The laser page's read-only work area line in the document's display unit, without trailing
 * zeros, for example "400 × 300 mm" or "40 × 30 cm" (`specs/0098-page-kinds` UX notes §9).
 */
export function formatWorkArea(widthMm: number, heightMm: number, unit: UnitSymbol): string {
  return `${length(widthMm, unit)} × ${length(heightMm, unit)} ${unit}`;
}

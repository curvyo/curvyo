// Display helpers of the Gradient block (`specs/0124-gradient-fill`): a stop's colour from its
// `#RRGGBBAA` text and the step text of the Angle, Radius and Position keys. The rules (what a
// value means, its range, the wrap of an Angle) are Rust's; this only formats and steps text.

/** The packed `0xRRGGBB` and the opacity percent of `#RRGGBBAA`, `null` for anything else. */
export function stopColour(hex: string): { rgb: number; opacity: number } | null {
  const match = /^#([0-9a-fA-F]{6})([0-9a-fA-F]{2})$/.exec(hex);
  if (!match) {
    return null;
  }
  return { rgb: Number.parseInt(match[1], 16), opacity: (Number.parseInt(match[2], 16) / 255) * 100 };
}

/** The text of `value` with at most `decimals` decimals, trailing zeros trimmed. */
export function trimmedNumber(value: number, decimals: number): string {
  const text = value.toFixed(decimals);
  return text.includes(".") ? text.replace(/0+$/, "").replace(/\.$/, "") : text;
}

/** `stepText` of the Angle field: Up and Down 1, with Shift 15; one decimal kept ("31.0"). */
export function stepAngle(text: string, key: string, shift: boolean): string | null {
  return stepBy(text, key, shift ? 15 : 1, (n) => n.toFixed(1));
}

/** `stepText` of the Radius field: 0.1 mm, with Shift 1 mm, trailing zeros trimmed. */
export function stepRadius(text: string, key: string, shift: boolean): string | null {
  return stepBy(text, key, shift ? 1 : 0.1, (n) => trimmedNumber(n, 3));
}

function stepBy(text: string, key: string, by: number, show: (n: number) => string): string | null {
  if (key !== "ArrowUp" && key !== "ArrowDown") {
    return null;
  }
  const value = Number.parseFloat(text);
  if (!Number.isFinite(value)) {
    return null;
  }
  return show(value + (key === "ArrowUp" ? by : -by));
}

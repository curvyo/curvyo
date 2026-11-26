// The spoken name of a slot row of the Into slot list (`specs/0154-library-into-slots` UX notes
// 8.2): the shape and the size in words, the outcome as "scaled to 50 percent", never the signs
// "×", "⌀" or "%" a screen reader reads letter by letter or not at all.

/** "Slot 2, rectangle, 30 by 30 millimetres, scaled to 50 percent" from the row's shown words. */
export function slotOptionLabel(name: string, shape: "rect" | "circle", size: string, outcome: string | null, filled: boolean): string {
  const sizeWords = size
    .replace(/^⌀\s*/u, "")
    .replace(/\s*×\s*/gu, " by ")
    .replace(/\s*mm$/u, " millimetres");
  const parts = [name, shape === "circle" ? "circle" : "rectangle", sizeWords];
  if (filled) {
    parts.push("filled");
  } else if (outcome) {
    parts.push(outcomeWords(outcome));
  }
  return parts.join(", ");
}

/** "50 %" → "scaled to 50 percent"; "67 %, turned" → "scaled to 67 percent, turned"; other words stay. */
function outcomeWords(outcome: string): string {
  const scaled = outcome.match(/^([\d.]+)\s*%(.*)$/u);
  return scaled ? `scaled to ${scaled[1]} percent${scaled[2] ?? ""}` : outcome;
}

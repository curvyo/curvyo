// The keys and texts of the Machine block (`specs/0026-machine-profile` UX notes §3, §6): the
// profile list's row keys, the origin corner grid's arrows, and the typed-number reading. Pure, so
// the tests load it without a bundler.

/** A corner of the output page, as stored. */
export type Corner = "top_left" | "top_right" | "bottom_left" | "bottom_right";

/** The corners in grid order: top row, then bottom row. */
export const CORNERS: Corner[] = ["top_left", "top_right", "bottom_left", "bottom_right"];

/** The name and tooltip of each corner radio. */
export const CORNER_NAMES: Record<Corner, string> = {
  top_left: "Top left",
  top_right: "Top right",
  bottom_left: "Bottom left",
  bottom_right: "Bottom right",
};

/** The corner an arrow, Home or End moves to from `corner` in the 2 x 2 grid; `null` for any
 * other key. Arrows stop at the grid's edge. */
export function cornerKey(key: string, corner: Corner): Corner | null {
  const index = CORNERS.indexOf(corner);
  const row = Math.floor(index / 2);
  const col = index % 2;
  const at = (r: number, c: number) => CORNERS[r * 2 + c];
  switch (key) {
    case "ArrowLeft":
      return at(row, 0);
    case "ArrowRight":
      return at(row, 1);
    case "ArrowUp":
      return at(0, col);
    case "ArrowDown":
      return at(1, col);
    case "Home":
      return "top_left";
    case "End":
      return "bottom_right";
    default:
      return null;
  }
}

/** What a key does on a focused profile row. */
export type ProfileRowAction =
  | { kind: "focus"; index: number }
  | { kind: "pick" }
  | { kind: "rename" }
  | { kind: "duplicate" }
  | { kind: "delete" }
  | { kind: "close" };

/** The action of `key` on the row `index` of `count` rows (UX notes §3): Up/Down move, Home/End
 * jump, no wrap; Enter/Space pick; F2 rename; Ctrl+D duplicate; Delete arms or removes; Escape
 * closes the list. `null` for any other key. */
export function profileRowKey(
  key: string,
  ctrl: boolean,
  index: number,
  count: number,
): ProfileRowAction | null {
  if (ctrl) {
    return key.toLowerCase() === "d" ? { kind: "duplicate" } : null;
  }
  switch (key) {
    case "ArrowUp":
      return { kind: "focus", index: Math.max(0, index - 1) };
    case "ArrowDown":
      return { kind: "focus", index: Math.min(count - 1, index + 1) };
    case "Home":
      return { kind: "focus", index: 0 };
    case "End":
      return { kind: "focus", index: count - 1 };
    case "Enter":
    case " ":
      return { kind: "pick" };
    case "F2":
      return { kind: "rename" };
    case "Delete":
      return { kind: "delete" };
    case "Escape":
      return { kind: "close" };
    default:
      return null;
  }
}

/** A typed number: a comma reads as the decimal point; empty or not a number is `null`, which
 * reverts the field silently. */
export function typedNumber(text: string): number | null {
  const trimmed = text.trim();
  if (trimmed === "") {
    return null;
  }
  const value = Number(trimmed.replace(",", "."));
  return Number.isFinite(value) ? value : null;
}

/** "400 × 300 mm": a work area with up to three decimals, trailing zeros trimmed. */
export function areaText(width: number, height: number): string {
  const mm = (value: number) => String(Number(value.toFixed(3)));
  return `${mm(width)} × ${mm(height)} mm`;
}

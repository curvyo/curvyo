// The words and keys of the Style group (`specs/0078-font-style-buttons` criteria 1, 6, 11, 12,
// 19 and 21; UX notes §4 to §7): button names, tooltip lines, the notices after a press, roving
// focus over the weight buttons and the Ctrl+B target. Pure functions, so the tests read them
// without a session.

/** The weight buttons' codes, in their order. */
export const WEIGHT_CODES = ["extra-light", "light", "regular", "bold", "extra-bold"] as const;
export type WeightCode = (typeof WEIGHT_CODES)[number];

/** The weight buttons' names: accessible name and tooltip line 1. */
export const WEIGHT_NAMES: Record<WeightCode, string> = {
  "extra-light": "Extra Light",
  light: "Light",
  regular: "Regular",
  bold: "Bold",
  "extra-bold": "Extra Bold",
};

/** Why a button is greyed, as the session states it. */
export interface StyleGreyed {
  kind: "not_in" | "not_in_as_italic" | "not_in_as_upright" | "not_in_any" | "font_missing";
  name: string;
  count: number;
}

/** One button of the Style group. */
export interface StyleButtonState {
  pressed: "off" | "on" | "mixed";
  greyed: StyleGreyed | null;
}

/** The Style group: five weight buttons and Italic. */
export interface StyleState {
  weights: StyleButtonState[];
  italic: StyleButtonState;
}

const IDLE: StyleButtonState = { pressed: "off", greyed: null };

export const EMPTY_STYLE: StyleState = {
  weights: WEIGHT_CODES.map(() => IDLE),
  italic: IDLE,
};

/** The session's `style_json`; the idle group when it does not read. */
export function parseStyle(json: string): StyleState {
  try {
    const value = JSON.parse(json) as StyleState;
    if (Array.isArray(value.weights) && value.weights.length === WEIGHT_CODES.length && value.italic) {
      return value;
    }
  } catch {
    // The idle group below.
  }
  return EMPTY_STYLE;
}

/** The tooltip's second line for a greyed button (criterion 6). */
export function greyedLine(greyed: StyleGreyed): string {
  switch (greyed.kind) {
    case "not_in":
      return `Not in ${greyed.name}`;
    case "not_in_as_italic":
      return `Not in ${greyed.name} as italic`;
    case "not_in_as_upright":
      return `Not in ${greyed.name} as upright`;
    case "not_in_any":
      return `Not in any of the ${greyed.count} fonts`;
    case "font_missing":
      return "Font missing";
  }
}

/** A button's tooltip: the name, with its key in edit mode for Bold and Italic (criterion 1),
 * and a second line for a greyed button or a mixed Italic (UX notes §4). */
export function styleTooltip(
  button: WeightCode | "italic",
  state: StyleButtonState,
  editing: boolean,
  mac: boolean,
): { name: string; line: string | null } {
  const plain = button === "italic" ? "Italic" : WEIGHT_NAMES[button];
  const key = mac ? "Cmd" : "Ctrl";
  let name = plain;
  if (editing && button === "bold") {
    name = `Bold (${key}+B)`;
  } else if (editing && button === "italic") {
    name = `Italic (${key}+I)`;
  }
  if (state.greyed) {
    return { name, line: greyedLine(state.greyed) };
  }
  return { name, line: button === "italic" && state.pressed === "mixed" ? "Mixed" : null };
}

/** What a press did, as the session returns it. */
export interface StyleOutcome {
  set: number;
  left: number;
  upright: boolean;
}

/** The notice after a press that left texts as they were (criteria 11, 12; UX notes §7), or
 * `null` when every text got the style. `step` is the button's name, or "italic". */
export function styleNotice(step: WeightCode | "italic", outcome: StyleOutcome): string | null {
  if (outcome.left === 0 || outcome.set === 0) {
    return null;
  }
  const total = outcome.set + outcome.left;
  let name: string;
  let lacks: string;
  if (step === "italic") {
    name = outcome.upright ? "Upright" : "Italic";
    lacks = outcome.upright ? "upright face" : "italic";
  } else {
    name = WEIGHT_NAMES[step];
    lacks = name;
  }
  const others = outcome.left === 1 ? `1 has no ${lacks}` : `the others have no ${lacks}`;
  return `${name} set on ${outcome.set} of ${total} texts; ${others}.`;
}

/** The weight button an arrow, Home or End moves focus to from `index`, without wrapping
 * (criterion 21); `null` for any other key. */
export function weightFocusMove(key: string, index: number): number | null {
  const last = WEIGHT_CODES.length - 1;
  switch (key) {
    case "ArrowLeft":
    case "ArrowUp":
      return Math.max(0, index - 1);
    case "ArrowRight":
    case "ArrowDown":
      return Math.min(last, index + 1);
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return null;
  }
}

/** The weight button that takes the Tab stop: the pressed one, else Regular (UX notes §6). */
export function weightTabStop(style: StyleState): number {
  const pressed = style.weights.findIndex((button) => button.pressed === "on");
  return pressed >= 0 ? pressed : WEIGHT_CODES.indexOf("regular");
}

/** What Ctrl+B presses in edit mode (criterion 19): Bold, or Regular when the text is Bold;
 * `null` when that button is greyed. */
export function boldKeyTarget(style: StyleState): WeightCode | null {
  const bold = WEIGHT_CODES.indexOf("bold");
  const target: WeightCode = style.weights[bold]?.pressed === "on" ? "regular" : "bold";
  return style.weights[WEIGHT_CODES.indexOf(target)]?.greyed ? null : target;
}

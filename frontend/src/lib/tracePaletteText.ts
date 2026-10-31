/**
 * The words and plain rules of a multicolour trace's palette (`specs/0089` slice M, UX notes §7,
 * §10, §12): the strip's order, the swatch names for makers who do not see the colour, the five
 * actions and their notices. No React and no wasm, so node tests can check them.
 */

/** The palette's name, its actions and their words (UX notes §7, §10, §12). */
export const PALETTE = "Palette";
export const ISOLATE = "Isolate";
export const RECOLOUR = "Recolour";
export const RESET = "Reset";
export const MERGE_INTO = "Merge into";
export const EXCLUDE = "Exclude";
export const REMOVE = "Remove";
export const EXCLUDED = "Excluded";

export function isolatingNote(hex: string): string {
  return `Isolating ${hex}`;
}

export function mergingNote(hex: string): string {
  return `Merging ${hex} into...`;
}

export function mergedIntoText(hex: string): string {
  return `Merged into ${hex}`;
}

export function sameAsNote(hex: string): string {
  return `Same as ${hex}`;
}

export function excludedNote(count: number): string {
  return `${count} ${count === 1 ? "colour" : "colours"} excluded`;
}

/** The detail row's text of a recoloured swatch: "#FFE000 (was #FFF2A8)". */
export function recolouredText(fill: string, was: string): string {
  return `${fill} (was ${was})`;
}

/** The message of a refused Recolour hex. */
export const RECOLOUR_HEX_MESSAGE = "Enter 6 hex digits";

/** The Recolour hex field's text as `#RRGGBB` (six digits, `#` optional), or `null`. */
export function recolourHex(text: string): string | null {
  const digits = text.trim().replace(/^#/, "");
  return /^[0-9A-Fa-f]{6}$/.test(digits) ? `#${digits.toUpperCase()}` : null;
}

/**
 * A swatch's name before its state words: the hex and name of the colour it is drawn in,
 * "#FFE000 (was #FFF2A8) · Yellow" once recoloured (UX notes §7: the detail row's text).
 */
export function swatchName(hex: string, fill: string): string {
  const shown = fill === hex ? hex : recolouredText(fill, hex);
  return `${shown} · ${colourName(fill)}`;
}

/** The pipette of the palette (Brightness + Multicolour) adds a fixed colour (UX notes §7). */
export const PICK_PALETTE_COLOUR = "Pick palette colour from the image";

/** `#RRGGBB` as its channels. */
export function hexChannels(hex: string): [number, number, number] {
  const n = Number.parseInt(hex.slice(1, 7), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** The palette merge distance in bytes (0.03 of the sRGB unit cube, the crate's). */
export const MERGE_DISTANCE_BYTES = 0.03 * 255;

/** The palette colour of `palette` within the merge distance of `hex`, if any. */
export function sameColour(palette: readonly string[], hex: string): string | null {
  const [r, g, b] = hexChannels(hex);
  for (const other of palette) {
    const [r2, g2, b2] = hexChannels(other);
    if (Math.hypot(r - r2, g - g2, b - b2) < MERGE_DISTANCE_BYTES) {
      return other;
    }
  }
  return null;
}

/** A colour's name from a fixed hue and lightness table (UX notes §7: the swatch's name for
 * makers who do not see the colour): greys by lightness, else the hue word, "Pale" when light,
 * "Dark" when dark, dark orange is "Brown". */
export function colourName(hex: string): string {
  const [r, g, b] = hexChannels(hex).map((c) => c / 255);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const light = (max + min) / 2;
  const chroma = max - min;
  const saturation = chroma === 0 ? 0 : chroma / (1 - Math.abs(2 * light - 1));
  if (saturation < 0.12 || chroma < 0.04) {
    return light < 0.12 ? "Black" : light < 0.35 ? "Dark grey" : light < 0.65 ? "Grey" : light < 0.92 ? "Light grey" : "White";
  }
  let hue = 0;
  if (max === r) {
    hue = ((g - b) / chroma) % 6;
  } else if (max === g) {
    hue = (b - r) / chroma + 2;
  } else {
    hue = (r - g) / chroma + 4;
  }
  hue = (hue * 60 + 360) % 360;
  const words: [number, string][] = [
    [15, "red"],
    [40, "orange"],
    [65, "yellow"],
    [160, "green"],
    [200, "teal"],
    [255, "blue"],
    [290, "purple"],
    [335, "pink"],
    [360, "red"],
  ];
  const word = words.find(([upTo]) => hue < upTo)?.[1] ?? "red";
  if (light < 0.3) {
    return word === "orange" ? "Brown" : `Dark ${word}`;
  }
  if (light > 0.78) {
    return `Pale ${word}`;
  }
  return word[0].toUpperCase() + word.slice(1);
}

/** One colour of the strip. */
export interface PaletteSwatch {
  hex: string;
  fixed: boolean;
  pixels: number;
}

/** The palette of the shown preview as the session reads it. */
export interface PaletteInfo {
  colours: PaletteSwatch[];
  /** The colours that became layers. */
  layers: string[];
  joined: number;
}

/** The strip's order (UX notes §7): picked colours first, then by area, most first (ties by
 * value). */
export function stripOrder(colours: readonly PaletteSwatch[]): PaletteSwatch[] {
  return [...colours].sort((a, b) => Number(b.fixed) - Number(a.fixed) || b.pixels - a.pixels || a.hex.localeCompare(b.hex));
}

/** The palette JSON of `trace_palette()`, or `null` for none. */
export function readPalette(json: string): PaletteInfo | null {
  if (!json) {
    return null;
  }
  try {
    const raw = JSON.parse(json) as PaletteInfo;
    return Array.isArray(raw.colours) ? raw : null;
  } catch {
    return null;
  }
}

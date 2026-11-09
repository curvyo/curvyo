/**
 * The words and plain rules of a multicolour trace's palette (`specs/0089` slice M, UX notes §7,
 * §26): the swatch row's order, the swatch names for makers who do not see the colour, the keys
 * of a swatch, the palette after a delete and the announcements. No React and no wasm, so node
 * tests can check them.
 */

/** The name of the swatch row (UX notes §26). */
export const PALETTE = "Palette";

/** The tooltip's second line, the swatch's ways of use (customer 2026-11-09: "ganz kurz"). */
export const PALETTE_HINT = "Click: exclude · Shift: isolate / delete";

export function isolatingNote(hex: string): string {
  return `Isolating ${hex}`;
}

export function sameAsNote(hex: string): string {
  return `Same as ${hex}`;
}

export function excludedNote(count: number): string {
  return `${count} ${count === 1 ? "colour" : "colours"} excluded`;
}

/** A swatch's tooltip line: "#4B392F · Brown". */
export function paletteTooltip(hex: string): string {
  return `${hex} · ${colourName(hex)}`;
}

/** A swatch's accessible name: its colour code and name, then "excluded" when it is. */
export function paletteSwatchLabel(hex: string, excluded: boolean): string {
  return excluded ? `${paletteTooltip(hex)}, excluded` : paletteTooltip(hex);
}

/** What a key on swatch `index` of `count` does (UX notes §26): move the roving stop, toggle
 * Exclude (Enter, Space), delete (Shift+Enter, Delete, Backspace), or nothing. */
export function paletteKey(
  key: string,
  shift: boolean,
  index: number,
  count: number,
): { move: number } | { toggle: number } | { remove: number } | null {
  switch (key) {
    case "ArrowRight":
      return { move: Math.min(count - 1, index + 1) };
    case "ArrowLeft":
      return { move: Math.max(0, index - 1) };
    case "Home":
      return { move: 0 };
    case "End":
      return { move: count - 1 };
    case "Enter":
      return shift ? { remove: index } : { toggle: index };
    case " ":
      return { toggle: index };
    case "Delete":
    case "Backspace":
      return { remove: index };
    default:
      return null;
  }
}

/** The fewest colours a palette keeps after a delete (the Colours field's lowest count). */
export const MIN_PALETTE = 2;

/** The palette edits after deleting `hex` from the shown `palette` (UX notes §26, Proposal): the
 * palette becomes exactly the other colours, matched again as if `hex` had never been in it,
 * and `hex` is no longer excluded; `null` when `hex` is not shown or only `MIN_PALETTE` colours
 * are left. */
export function deleteFromPalette(
  palette: readonly string[],
  excluded: readonly string[],
  hex: string,
): { keptPalette: string[]; excluded: string[] } | null {
  if (!palette.includes(hex) || palette.length <= MIN_PALETTE) {
    return null;
  }
  return { keptPalette: palette.filter((c) => c !== hex), excluded: excluded.filter((c) => c !== hex) };
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

/** A colour's name from a fixed hue and lightness table (UX notes §7, §26: the swatch's name for
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

/** One colour of the swatch row. */
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

/** The swatch row's order (UX notes §7): picked colours first, then by area, most first (ties by
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

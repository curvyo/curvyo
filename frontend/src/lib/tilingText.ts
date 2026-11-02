// Tiling in the frontend (`specs/0069-tiling`, UX notes §1 to §7): the bar's strings, the shape of
// the recipe and status JSON the session hands out, the field rules and the count status text.
// Every decision about the layout is Rust's; this holds texts and parses.

export const TILE = "Tile";
export const EDIT_TILING = "Edit tiling";
export const RELEASE_TILING = "Release tiling";
export const TILE_RULE = "Repeat the selection in a grid or a circle.";
export const TILE_EMPTY_RULE = "Select an object to repeat.";
export const TILE_WITH_OTHERS_RULE = "A tiling cannot be tiled. Release it first.";
export const EDIT_TILING_RULE = "Change the grid or circle, the variance and the seed.";
export const RELEASE_TILING_RULE = "Makes a plain group of the instances. The tiling can no longer be changed.";
export const KEEP_SOURCE = "Keep source in place";
export const KEEP_SOURCE_RULE = "On: the source becomes the first cell, inside the group. Off: the source stays untouched and the group sits above it.";
export const NEW_SEED = "New random seed";
export const WARN_TEXT = "Over 10,000 instances. The preview is simplified.";
export const REFUSED_TEXT = "Too many instances. 100,000 at most.";
export const IMAGES_TEXT = "Images become copies.";

export const ENTER_NUMBER = "Enter a number";
export const ENTER_WHOLE = "Enter a whole number";
export const AT_LEAST_ZERO = "Must be 0 or more.";
export const AT_LEAST_ONE = "Must be 1 or more.";
export const MORE_THAN_ZERO = "Must be more than 0.";
export const PERCENT_RANGE = "Must be between 0 and 100.";
export const SWEEP_RANGE = "Must be between -360 and 360.";

/** The most instances a field may name (UX notes §2.2). */
export const MAX_COUNT = 100_000;
/** The largest seed (the generator's u32). */
export const MAX_SEED = 4_294_967_295;

/** One variance channel: random ±, per row, per column (circle: per step). */
export type Triple = [number, number, number];

/** The recipe as `Session::tiling_recipe_json` gives it, in the units the fields show. */
export interface TilingRecipe {
  layout: "grid" | "circle";
  columns: number;
  rows: number;
  spacing_x: number;
  spacing_y: number;
  pitch: boolean;
  mirror_rows: boolean;
  mirror_columns: boolean;
  offset: "none" | "rows" | "columns";
  shift: number;
  count: number;
  radius: number;
  start: number;
  sweep: number;
  centre_x: number;
  centre_y: number;
  face_centre: boolean;
  rotation: Triple;
  scale: Triple;
  offset_x: Triple;
  offset_y: Triple;
  opacity: Triple;
  hue: Triple;
  seed: number;
  copies: boolean;
  keep_source: boolean;
}

/** The status as `Session::tiling_status_json` gives it. */
export interface TilingStatus {
  count: number;
  level: "fine" | "simplified" | "refused";
  images_copies: boolean;
  refusal: string;
  reopened: boolean;
}

export function parseRecipe(json: string): TilingRecipe | null {
  if (json === "") {
    return null;
  }
  try {
    return JSON.parse(json) as TilingRecipe;
  } catch {
    return null;
  }
}

export function parseStatus(json: string): TilingStatus | null {
  if (json === "") {
    return null;
  }
  try {
    return JSON.parse(json) as TilingStatus;
  } catch {
    return null;
  }
}

/** What a field's text gives: a value, or the message of its chip. */
export type FieldResult = { value: number } | { error: string };

/** The rule of one field. */
export type FieldRule = "count" | "length" | "positive" | "angle" | "sweep" | "shift" | "range" | "step" | "seed";

const NUMBER = /^-?(\d+\.?\d*|\.\d+)$/;
const WHOLE = /^\d+$/;

/** Reads `text` by `rule` (UX notes §2.2 to §2.4, §7). A minus is the real one or a hyphen. */
export function readField(text: string, rule: FieldRule): FieldResult {
  const plain = text.trim().replace("−", "-").replace(",", ".");
  if (rule === "count" || rule === "seed") {
    if (!WHOLE.test(plain)) {
      return { error: ENTER_WHOLE };
    }
    const value = Number(plain);
    if (rule === "count") {
      // Above 100,000 the value is taken: the bar shows the refusal (UX notes §4).
      return value < 1 ? { error: AT_LEAST_ONE } : value > MAX_SEED ? { error: ENTER_WHOLE } : { value };
    }
    return value > MAX_SEED ? { error: ENTER_WHOLE } : { value };
  }
  if (!NUMBER.test(plain)) {
    return { error: ENTER_NUMBER };
  }
  const value = Number(plain);
  switch (rule) {
    case "positive":
      return value > 0 ? { value } : { error: MORE_THAN_ZERO };
    case "sweep":
      return value >= -360 && value <= 360 ? { value } : { error: SWEEP_RANGE };
    case "shift":
      return value >= 0 && value <= 100 ? { value } : { error: PERCENT_RANGE };
    case "range":
      return value >= 0 ? { value } : { error: AT_LEAST_ZERO };
    default:
      return { value };
  }
}

/** The count status, "48 instances" or "1 instance". */
export function countText(count: number): string {
  return count === 1 ? "1 instance" : `${count.toLocaleString("en-US")} instances`;
}

/** The channels of the Variance block, in order, with their row labels. */
export const VARIANCE_ROWS: { key: "rotation" | "scale" | "offset_x" | "offset_y" | "opacity" | "hue"; label: string; name: string; unit: string }[] = [
  { key: "rotation", label: "Rotation °", name: "rotation", unit: "degrees" },
  { key: "scale", label: "Scale %", name: "scale", unit: "percent" },
  { key: "offset_x", label: "Offset X mm", name: "offset X", unit: "millimetres" },
  { key: "offset_y", label: "Offset Y mm", name: "offset Y", unit: "millimetres" },
  { key: "opacity", label: "Opacity %", name: "opacity", unit: "percent" },
  { key: "hue", label: "Colour °", name: "colour", unit: "degrees" },
];

/** The tooltip of the Variance button: "Variance: none" or "Variance: rotation, scale". */
export function varianceSummary(recipe: TilingRecipe): string {
  const on = VARIANCE_ROWS.filter((row) => recipe[row.key].some((v) => v !== 0)).map((row) => row.name);
  return on.length === 0 ? "Variance: none" : `Variance: ${on.join(", ")}`;
}

/** A fresh seed from the platform's generator, never 0 by surprise (criterion 19). */
export function freshSeed(random: (into: Uint32Array<ArrayBuffer>) => void): number {
  const into = new Uint32Array(1);
  random(into);
  const draw = into[0] ?? 1;
  return draw === 0 ? 1 : draw;
}

/** The Tile slot's state from `WasmSession.tile_button_state`. */
export type TileSlot = "nothing" | "with-others" | "tile" | "edit";

export function tileSlot(code: number): TileSlot {
  return code === 3 ? "edit" : code === 2 ? "tile" : code === 1 ? "with-others" : "nothing";
}

/** The rule line of the Tile button's tooltip. */
export function tileRule(slot: TileSlot): string {
  switch (slot) {
    case "nothing":
      return TILE_EMPTY_RULE;
    case "with-others":
      return TILE_WITH_OTHERS_RULE;
    case "edit":
      return EDIT_TILING_RULE;
    default:
      return TILE_RULE;
  }
}

/** Typed values reach the preview this long after the last keystroke (UX notes §2.1). */
export const FIELD_DEBOUNCE_MS = 120;
/** The live region speaks a new count this long after the last change (UX notes §6). */
export const ANNOUNCE_DELAY_MS = 500;

/** What the polite live region says on entering and after a count change (UX notes §6). */
export function tilingAnnouncement(count: number): string {
  return `Tiling, ${countText(count)}`;
}

/** The timer functions a {@link ChangeQueue} uses (the platform's, or a test's). */
export interface Timers {
  set: (run: () => void, ms: number) => number;
  clear: (id: number) => void;
}

/** Field changes typed within {@link FIELD_DEBOUNCE_MS} of each other, sent as one (UX notes
 * §2.1); a refused send tells every field that took part. */
export class ChangeQueue<T extends object> {
  #change: Partial<T> = {};
  #refused: (() => void)[] = [];
  #timer: number | null = null;
  readonly #send: (change: Partial<T>) => boolean;
  readonly #timers: Timers;

  constructor(send: (change: Partial<T>) => boolean, timers: Timers) {
    this.#send = send;
    this.#timers = timers;
  }

  /** Adds `change`; the send runs once the fields are quiet. */
  queue(change: Partial<T>, onRefused?: () => void): void {
    this.#change = { ...this.#change, ...change };
    if (onRefused) {
      this.#refused.push(onRefused);
    }
    if (this.#timer !== null) {
      this.#timers.clear(this.#timer);
    }
    this.#timer = this.#timers.set(() => this.flush(), FIELD_DEBOUNCE_MS);
  }

  /** Sends what is waiting now (before Apply); true when nothing waited or it was taken. */
  flush(): boolean {
    if (this.#timer === null) {
      return true;
    }
    this.#timers.clear(this.#timer);
    this.#timer = null;
    const change = this.#change;
    const refused = this.#refused;
    this.#change = {};
    this.#refused = [];
    const taken = this.#send(change);
    if (!taken) {
      for (const tell of refused) {
        tell();
      }
    }
    return taken;
  }

  /** Forgets what is waiting (Cancel). */
  drop(): void {
    if (this.#timer !== null) {
      this.#timers.clear(this.#timer);
    }
    this.#timer = null;
    this.#change = {};
    this.#refused = [];
  }
}

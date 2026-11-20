/**
 * The one decoder of the Before and after pane's JSON (`specs/0135-image-optimisation` criteria
 * 11 to 13; `ux-notes.md` §7): `curvyo-editor-wasm/src/session/image_adjust/before_after.rs`
 * builds it, and every field is checked here at runtime. No cast: a value of the wrong shape
 * decodes to `null` and the pane shows nothing rather than a guess. The Rust contract test reads
 * the same fixtures (`curvyo-editor-wasm/tests/fixtures/before_after_view/`).
 */
import { asId, type OperationId } from "./operationsView.ts";
import { isObject } from "./operationSettingsView.ts";

/** One segment of the operation pick (two or more Image operations). */
export interface PaneOperation {
  id: OperationId;
  name: string;
  pressed: boolean;
}

/** One half of the pane: the pixels `before_after_pixels` gives, placed in page mm. */
export interface PaneImage {
  width: number;
  height: number;
  /** The unit square to page mm, `a b c d e f`, so both halves line up. */
  placement: [number, number, number, number, number, number];
}

/** The After half: the job's dots, or while they are worked a reduced live picture. */
export interface PaneDots {
  image: PaneImage;
  /** The reduced live picture of the current stack (criterion 13); the exact dots follow. */
  reduced: boolean;
}

/** The Before and after pane. */
export interface BeforeAfterView {
  open: boolean;
  view: "split" | "side_by_side";
  /** Empty with fewer than two Image operations. */
  operations: PaneOperation[];
  /** "photo.jpg · Image 1 · Jarvis · 0.10 mm"; empty without a picture. */
  subject: string;
  /** "No picture selected." when there is none. */
  empty: string | null;
  before: PaneImage | null;
  /** `null` while the picture is in no Image operation. */
  after: PaneDots | null;
  /** The Interval of the shown operation (one dot at 100 %). */
  intervalMm: number;
}

function finite(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function size(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) > 0;
}

function asOperation(value: unknown): PaneOperation | null {
  if (!isObject(value) || typeof value.name !== "string" || typeof value.pressed !== "boolean") return null;
  const id = asId(value.id);
  return id === null ? null : { id, name: value.name, pressed: value.pressed };
}

function asImage(value: unknown): PaneImage | null {
  if (
    !isObject(value) ||
    !size(value.width) ||
    !size(value.height) ||
    !Array.isArray(value.placement) ||
    value.placement.length !== 6 ||
    !value.placement.every(finite)
  ) {
    return null;
  }
  const [a, b, c, d, e, f] = value.placement as number[];
  return { width: value.width, height: value.height, placement: [a, b, c, d, e, f] };
}

function asDots(value: unknown): PaneDots | null {
  if (!isObject(value) || typeof value.reduced !== "boolean") return null;
  const image = asImage(value.image);
  return image === null ? null : { image, reduced: value.reduced };
}

/** The pane from its parsed JSON; `null` when of the wrong shape. */
export function decodeBeforeAfterView(value: unknown): BeforeAfterView | null {
  if (!isObject(value) || !Array.isArray(value.operations)) return null;
  const operations = value.operations.map(asOperation);
  const before = value.before === null ? null : asImage(value.before);
  const after = value.after === null ? null : asDots(value.after);
  const view = value.view;
  if (
    typeof value.open !== "boolean" ||
    !(view === "split" || view === "side_by_side") ||
    operations.some((op) => op === null) ||
    typeof value.subject !== "string" ||
    !(value.empty === null || typeof value.empty === "string") ||
    (value.before !== null && before === null) ||
    (value.after !== null && after === null) ||
    !finite(value.interval_mm)
  ) {
    return null;
  }
  return {
    open: value.open,
    view,
    operations: operations as PaneOperation[],
    subject: value.subject,
    empty: value.empty,
    before,
    after,
    intervalMm: value.interval_mm,
  };
}

/** The pane from `before_after_view_json`; `null` when of the wrong shape. */
export function decodeBeforeAfterText(text: string): BeforeAfterView | null {
  try {
    return decodeBeforeAfterView(JSON.parse(text));
  } catch {
    return null;
  }
}

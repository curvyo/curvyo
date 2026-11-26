// A library item put into a fixture slot (`specs/0154-library-into-slots`): the unit's size, the
// put, the replace and the fill through the wasm, the notices they answer, and the filled-slot
// refusal whose armed Replace the canvas notice slot shows (UX notes section 7).
import { useSyncExternalStore } from "react";

import type { ImportFrom } from "@/lib/libraryImport";
import type { WasmLibrarySource, WasmSession } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** One source for a slot: a page (whole) or picks of a read-only reader. */
export interface SlotUnitSource {
  reader: WasmLibrarySource;
  from: ImportFrom;
  whole: boolean;
  roots: string[];
}

/** What the page needs of the editor. */
export interface SlotContext {
  getSession: () => WasmSession | null;
  sync: () => void;
  showNotice: (text: string, ms?: number, source?: "canvas" | "panel", kind?: "status" | "alert") => void;
}

/** The answer of a put or a fill. */
interface SlotAnswer {
  refused?: string | null;
  notice?: string | null;
  error?: string;
}

/** One slot row of the Into slot list. */
export interface SlotTargetRow {
  id: string;
  name: string;
  shape: "rect" | "circle";
  size: string;
  filled: boolean;
  outcome: string | null;
}

/** One holder of the Into slot list. */
export interface HolderTargets {
  id: string;
  name: string;
  hidden: boolean;
  filled: number;
  of: number;
  slots: SlotTargetRow[];
}

/** The Into slot list of the active laser page. */
export interface SlotTargets {
  holders: HolderTargets[];
  selected: string | null;
}

/** The cue of a library drag over a slot. */
export interface SlotCue {
  kind: "slot" | "fill" | "filled" | "refused" | "none";
  slot: string | null;
  holder: string | null;
  badge: string | null;
  /** The polite status words. */
  status: string | null;
  /** The laser page a release activates before the put, when the slot lies on another one. */
  page: string | null;
}

function recordOf(from: ImportFrom): string {
  return JSON.stringify({
    document: from.documentName,
    page: from.pageName,
    page_id: from.pageId,
    at: Date.now(),
    online: from.online,
    without_fonts: from.withoutFonts ?? false,
  });
}

/** The unit's size in mm, or null when it cannot be read. */
export function unitSize(source: SlotUnitSource): [number, number] | null {
  try {
    return JSON.parse(
      source.reader.unit_size_json(`[${source.roots.join(",")}]`, source.whole ? source.from.pageId : undefined),
    ) as [number, number] | null;
  } catch {
    return null;
  }
}

/** The Into slot list for a unit of `size` (null: no outcome words). */
export function slotTargets(session: WasmSession, size: [number, number] | null, realSize: boolean): SlotTargets {
  try {
    return JSON.parse(
      session.slot_targets_json(size?.[0] ?? 0, size?.[1] ?? 0, size !== null, realSize),
    ) as SlotTargets;
  } catch {
    return { holders: [], selected: null };
  }
}

/** The height of the list for the active page's targets, px (the row it opens under grows by it,
 * UX notes V20): the caption, a heading per holder, its Fill row and its slots, at most 8 rows. */
export function intoSlotListHeight(ctx: SlotContext): number {
  const session = ctx.getSession();
  const targets = session ? slotTargets(session, null, false) : { holders: [], selected: null };
  const rows = targets.holders.reduce((n, h) => n + h.slots.length + (h.filled < h.of ? 1 : 0), 0);
  return 28 + 8 + targets.holders.length * 24 + Math.min(rows, 8) * 28;
}

/** Whether the active page has a target slot (the Into slot… button is hidden otherwise). */
export function hasSlotTargets(session: WasmSession | null): boolean {
  return session !== null && slotTargets(session, null, false).holders.length > 0;
}

function parse(text: string): SlotAnswer {
  try {
    return JSON.parse(text) as SlotAnswer;
  } catch {
    return { error: "the put did not answer" };
  }
}

/** Shows what an answer says; true when something was written. A refusal with `retry` shows the
 * armed Replace, which holds `release` (the source's reader) until it is pressed or goes. */
function answered(ctx: SlotContext, answer: SlotAnswer, retry: (() => void) | null, release: () => void): boolean {
  ctx.sync();
  if (answer.refused && retry) {
    const entry: SlotReplace = {
      text: answer.refused,
      replace: () => {
        // The replace takes the source over: clearing the notice must not free it.
        entry.release = undefined;
        retry();
      },
      release,
    };
    setSlotReplace(entry);
    return false;
  }
  release();
  if (answer.error) {
    ctx.showNotice(`Nothing was changed: ${answer.error}.`, 8000, "canvas", "alert");
    return false;
  }
  if (answer.refused) {
    ctx.showNotice(answer.refused, 8000, "canvas", "alert");
    return false;
  }
  if (answer.notice) {
    ctx.showNotice(answer.notice, 3000, "canvas", "status");
  }
  return true;
}

/** Puts `source` into the slot `slot` (one step); a filled slot offers the armed Replace.
 * `release` frees the source's reader once nothing needs it any more: after the put, or, for a
 * filled slot, when the Replace is pressed and done or its notice goes. */
export function putIntoSlot(
  ctx: SlotContext,
  source: SlotUnitSource,
  slot: string,
  realSize: boolean,
  replace = false,
  release: () => void = () => {},
): boolean {
  const session = ctx.getSession();
  if (!session) {
    release();
    return false;
  }
  setSlotReplace(null);
  const answer = parse(
    session.put_into_slot(
      source.reader,
      `[${source.roots.join(",")}]`,
      source.whole ? source.from.pageId : undefined,
      recordOf(source.from),
      slot,
      realSize,
      replace,
    ),
  );
  const retry = replace ? null : () => putIntoSlot(ctx, source, slot, realSize, true, release);
  return answered(ctx, answer, retry, release);
}

/** Puts one copy into every empty slot of `holder` (one step "Fill slots"). */
export function fillSlots(ctx: SlotContext, source: SlotUnitSource, holder: string, realSize: boolean): boolean {
  const session = ctx.getSession();
  if (!session) {
    return false;
  }
  setSlotReplace(null);
  const answer = parse(
    session.fill_empty_slots(
      source.reader,
      `[${source.roots.join(",")}]`,
      source.whole ? source.from.pageId : undefined,
      recordOf(source.from),
      holder,
      realSize,
    ),
  );
  return answered(ctx, answer, null, () => {});
}

/** The refusal of a filled slot with its Replace, until it is pressed or goes stale. */
export interface SlotReplace {
  text: string;
  replace: () => void;
  /** Frees the source the Replace would put; called when the notice goes unpressed. */
  release?: () => void;
}

let current: SlotReplace | null = null;
const listeners = new Set<() => void>();

/** Shows (or with null clears) the filled-slot refusal. */
export function setSlotReplace(next: SlotReplace | null): void {
  if (current === next) {
    return;
  }
  current?.release?.();
  current = next;
  for (const listener of listeners) {
    listener();
  }
}

/** The filled-slot refusal the canvas notice slot shows. */
export function useSlotReplace(): SlotReplace | null {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => current,
  );
}

/** The filled-slot refusal shown now, outside React (tests, the drag). */
export function currentSlotReplace(): SlotReplace | null {
  return current;
}

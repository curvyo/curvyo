// Tiling in the frontend (`specs/0069-tiling` criteria 1, 2, 14, 19, 26, 32 and 35; UX notes §1 to
// §6 and §12): whether the tiling bar replaces the Select bar, the recipe, status and open handle
// entry box read from the session, the edits sent back (typed values debounced), the fresh seed
// from the platform, and Enter and Escape. Every decision is the session's.
import { type RefObject, useCallback, useEffect, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  ChangeQueue,
  type TileSlot,
  type TilingEntry,
  type TilingRecipe,
  type TilingStatus,
  freshSeed,
  parseEntry,
  parseRecipe,
  parseStatus,
  tileSlot,
} from "@/lib/tilingText";

export interface TilingApi {
  /** The tiling bar replaces the Select bar. */
  open: boolean;
  /** The Tile slot of the Select bar. */
  slot: TileSlot;
  /** The recipe the bar shows while it is open. */
  recipe: TilingRecipe | null;
  status: TilingStatus | null;
  /** The open entry box of a circle handle. */
  entry: TilingEntry | null;
  /** Opens the bar for the selection; from a key the focus goes to the first field. */
  enter: (byKeyboard: boolean) => void;
  /** Sends changed fields; false when the session refused them (the last valid preview stays). */
  update: (change: Partial<TilingRecipe>) => boolean;
  /** A typed field value: sent 120 ms after the last keystroke; `onRefused` marks the field. */
  type: (change: Partial<TilingRecipe>, onRefused: () => void) => void;
  /** Switches Gap and Pitch, converting the numbers. */
  setPitch: (pitch: boolean) => void;
  /** Closes the entry box: `keep` (Enter) sends what waits and keeps the typed values, false when
   * the session refused them (the box stays open); otherwise (Escape, a press elsewhere, a blur)
   * the values it opened with come back. The focus returns to the canvas. */
  closeEntry: (keep: boolean) => boolean;
  apply: () => void;
  cancel: () => void;
  release: () => void;
  /** The first field, for the focus on entering. */
  firstFieldRef: RefObject<HTMLInputElement | null>;
}

const platformRandom = (into: Uint32Array<ArrayBuffer>) => {
  crypto.getRandomValues(into);
};

export function useTiling(
  getSession: () => WasmSession | null,
  onChange: () => void,
  options: { syncRevision: number; containerRef: RefObject<HTMLElement | null>; firstFieldRef: RefObject<HTMLInputElement | null> },
): TilingApi {
  const { syncRevision, containerRef, firstFieldRef } = options;
  const [, setTick] = useState(0);
  const focusField = useRef(false);
  const session = getSession();
  // Read on every sync: the session holds the mode, a tool switch or a handle drag changes it.
  void syncRevision;
  const open = session?.tiling_open() ?? false;
  const slot = tileSlot(session?.tile_button_state() ?? 0);
  const recipe = open && session ? parseRecipe(session.tiling_recipe_json()) : null;
  const status = open && session ? parseStatus(session.tiling_status_json()) : null;
  const entry = open && session ? parseEntry(session.tiling_entry_json()) : null;

  const refresh = useCallback(() => {
    setTick((t) => t + 1);
    onChange();
  }, [onChange]);

  const enter = useCallback(
    (byKeyboard: boolean) => {
      const current = getSession();
      if (current?.tiling_enter(freshSeed(platformRandom))) {
        focusField.current = true;
        void byKeyboard;
        refresh();
      }
    },
    [getSession, refresh],
  );

  // Entering puts the focus on the first field with its text selected (UX notes §6).
  useEffect(() => {
    if (open && focusField.current) {
      focusField.current = false;
      firstFieldRef.current?.focus();
      firstFieldRef.current?.select();
    }
  }, [open, firstFieldRef]);

  const update = useCallback(
    (change: Partial<TilingRecipe>) => {
      const current = getSession();
      const before = current ? parseRecipe(current.tiling_recipe_json()) : null;
      if (!current || !before) {
        return false;
      }
      const taken = current.tiling_set_recipe_json(JSON.stringify({ ...before, ...change }));
      refresh();
      return taken;
    },
    [getSession, refresh],
  );

  // One queue for the hook's life, made on the first typed value; it always sends through the
  // latest `update`.
  const latestUpdate = useRef(update);
  useEffect(() => {
    latestUpdate.current = update;
  }, [update]);
  const queueRef = useRef<ChangeQueue<TilingRecipe> | null>(null);
  useEffect(() => {
    const queue = queueRef;
    return () => queue.current?.drop();
  }, []);

  const type = useCallback((change: Partial<TilingRecipe>, onRefused: () => void) => {
    queueRef.current ??= new ChangeQueue<TilingRecipe>((typed) => latestUpdate.current(typed), {
      set: (run, ms) => window.setTimeout(run, ms),
      clear: (id) => window.clearTimeout(id),
    });
    queueRef.current.queue(change, onRefused);
  }, []);

  const setPitch = useCallback(
    (pitch: boolean) => {
      getSession()?.tiling_set_pitch(pitch);
      refresh();
    },
    [getSession, refresh],
  );

  const focusCanvas = useCallback(() => containerRef.current?.focus({ preventScroll: true }), [containerRef]);

  const apply = useCallback(() => {
    // The typed text is in the preview before Apply (UX notes §3); a refused one stays marked.
    if (queueRef.current && !queueRef.current.flush()) {
      return;
    }
    const current = getSession();
    if (current?.tiling_apply()) {
      focusCanvas();
    }
    refresh();
  }, [getSession, refresh, focusCanvas]);

  const cancel = useCallback(() => {
    queueRef.current?.drop();
    getSession()?.tiling_cancel();
    focusCanvas();
    refresh();
  }, [getSession, refresh, focusCanvas]);

  const closeEntry = useCallback(
    (keep: boolean) => {
      const current = getSession();
      if (!current?.tiling_entry_json()) {
        return true;
      }
      if (keep) {
        if (queueRef.current && !queueRef.current.flush()) {
          return false;
        }
      } else {
        queueRef.current?.drop();
      }
      current.tiling_close_entry(!keep);
      focusCanvas();
      refresh();
      return true;
    },
    [getSession, refresh, focusCanvas],
  );

  const release = useCallback(() => {
    getSession()?.tiling_release();
    refresh();
  }, [getSession, refresh]);

  const entryOpen = entry !== null;
  // Enter applies and Escape cancels anywhere, also inside a field (UX notes §3); with a handle
  // entry box open both belong to the box (UX notes §12.2). Enter on the canvas with one tiling
  // selected reopens it (UX notes §1).
  useEffect(() => {
    const keyDown = (event: KeyboardEvent) => {
      if (event.isComposing) {
        return;
      }
      const target = event.target as HTMLElement | null;
      if (!open) {
        if (event.key === "Enter" && slot === "edit" && target === containerRef.current) {
          event.preventDefault();
          event.stopPropagation();
          enter(true);
        }
        return;
      }
      if (entryOpen && (event.key === "Escape" || event.key === "Enter")) {
        // The box's own keys; outside it (it never keeps the focus away) Escape closes it.
        if (!(target instanceof Element && target.closest("[data-tiling-entry]"))) {
          event.preventDefault();
          event.stopPropagation();
          closeEntry(event.key === "Enter");
        }
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        cancel();
      } else if (event.key === "Enter") {
        // An invalid field keeps its chip and writes nothing.
        if (target?.getAttribute("aria-invalid") === "true" || document.querySelector('[data-context-bar] [aria-invalid="true"]')) {
          return;
        }
        if (target instanceof HTMLButtonElement && target !== containerRef.current) {
          return;
        }
        event.preventDefault();
        event.stopPropagation();
        apply();
      }
    };
    window.addEventListener("keydown", keyDown, true);
    return () => window.removeEventListener("keydown", keyDown, true);
  }, [open, slot, enter, cancel, apply, containerRef, entryOpen, closeEntry]);

  return { open, slot, recipe, status, entry, enter, update, type, setPitch, closeEntry, apply, cancel, release, firstFieldRef };
}

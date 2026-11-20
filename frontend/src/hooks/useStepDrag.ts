import { type PointerEvent, type RefObject, useCallback, useEffect, useRef, useState } from "react";

import { dropSlots, landingIndex } from "@/lib/adjustText";
import type { StepRow } from "@/lib/adjustmentsView";

/** A drag starts after the pointer moved this far (`specs/0101` 4.6). */
const DRAG_SLOP_PX = 4;

/** A reorder drag in progress: the row, the slot it would land in (`null` outside its band) and
 * where the ghost is. */
export interface StepDrag {
  from: number;
  slot: number | null;
  x: number;
  y: number;
  name: string;
}

/**
 * The reorder drag of the step list (`specs/0135-image-optimisation` `ux-notes.md` §3.7): it starts
 * after 4 px on a row's glyph or name, may land only inside the source row's band, and a release
 * outside the band writes nothing. Escape drops it. `onMove(from, to)` is called once with the
 * new index.
 */
export function useStepDrag(
  rows: readonly StepRow[],
  list: RefObject<HTMLElement | null>,
  onMove: (from: number, to: number) => void,
): { drag: StepDrag | null; start: (index: number, event: PointerEvent<HTMLElement>) => void } {
  const [drag, setDrag] = useState<StepDrag | null>(null);
  const press = useRef<{ index: number; x: number; y: number; active: boolean } | null>(null);
  const latest = useRef({ rows, onMove });
  useEffect(() => {
    latest.current = { rows, onMove };
  });

  const slotAt = useCallback(
    (from: number, y: number): number | null => {
      const bounds = dropSlots(latest.current.rows, from);
      const items = list.current?.querySelectorAll<HTMLElement>("[data-adjust-row]");
      if (!bounds || !items) return null;
      let slot = items.length;
      for (let i = 0; i < items.length; i += 1) {
        const box = items[i].getBoundingClientRect();
        if (y < box.top + box.height / 2) {
          slot = i;
          break;
        }
      }
      return slot >= bounds.first && slot <= bounds.last ? slot : null;
    },
    [list],
  );

  const start = useCallback(
    (index: number, event: PointerEvent<HTMLElement>) => {
      if (event.button !== 0) return;
      press.current = { index, x: event.clientX, y: event.clientY, active: false };
      const name = latest.current.rows[index]?.name ?? "";
      const stop = () => {
        window.removeEventListener("pointermove", move, true);
        window.removeEventListener("pointerup", up, true);
        window.removeEventListener("keydown", key, true);
        window.removeEventListener("pointercancel", cancel, true);
        press.current = null;
        setDrag(null);
      };
      const move = (e: globalThis.PointerEvent) => {
        const p = press.current;
        if (!p) return;
        if (!p.active) {
          if (Math.hypot(e.clientX - p.x, e.clientY - p.y) < DRAG_SLOP_PX) return;
          p.active = true;
        }
        setDrag({ from: p.index, slot: slotAt(p.index, e.clientY), x: e.clientX, y: e.clientY, name });
      };
      const up = (e: globalThis.PointerEvent) => {
        const p = press.current;
        const slot = p?.active ? slotAt(p.index, e.clientY) : null;
        stop();
        if (p && slot !== null) {
          const to = landingIndex(p.index, slot);
          if (to !== p.index) latest.current.onMove(p.index, to);
        }
      };
      const key = (e: KeyboardEvent) => {
        if (e.key === "Escape") {
          e.preventDefault();
          e.stopPropagation();
          stop();
        }
      };
      const cancel = () => stop();
      window.addEventListener("pointermove", move, true);
      window.addEventListener("pointerup", up, true);
      window.addEventListener("keydown", key, true);
      window.addEventListener("pointercancel", cancel, true);
    },
    [slotAt],
  );
  return { drag, start };
}

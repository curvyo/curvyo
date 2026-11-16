import { type PointerEvent as ReactPointerEvent, type RefObject, useCallback, useState } from "react";

import { dragStarts, setLanding } from "@/lib/fixtureSets";

/** A set row being dragged: where it started, the gap its insertion line sits in and the pointer. */
export interface SetDrag {
  source: number;
  /** The gap of the 2px insertion line; `null` where the drop would not move the set. */
  gap: number | null;
  /** The pointer's y and the list's left edge, for the ghost that follows the pointer. */
  y: number;
  left: number;
}

/**
 * The drag of a machine set row (`specs/0113-fixture-sets-and-pins` `ux-notes.md` 4.1): it starts
 * after 4px of travel from a press that is not on a control, shows the insertion line, drops with
 * one `onMove(from, to)` and is cancelled by Escape. The click that ends a drag is swallowed.
 */
export function useSetDrag(list: RefObject<HTMLElement | null>, onMove: (from: number, to: number) => void) {
  const [drag, setDrag] = useState<SetDrag | null>(null);
  const press = useCallback(
    (event: ReactPointerEvent<HTMLElement>, index: number) => {
      const target = event.target as HTMLElement;
      // Never from the check box, the trash or the arrows (nor a rename field).
      if (event.button !== 0 || target.closest("[data-set-body]") || target.closest("li") !== event.currentTarget || target.closest("button[role=checkbox], button[data-arm-trash], button[data-order], input")) return;
      const startY = event.clientY;
      let started = false;
      const middles = () =>
        Array.from(list.current?.querySelectorAll<HTMLElement>(":scope > li") ?? []).map((row) => {
          const box = row.getBoundingClientRect();
          return box.top + box.height / 2;
        });
      const land = (y: number) => setLanding(index, middles(), y);
      const stop = () => {
        window.removeEventListener("pointermove", onPointerMove);
        window.removeEventListener("pointerup", onPointerUp);
        window.removeEventListener("keydown", onKey, true);
        setDrag(null);
      };
      const onPointerMove = (move: PointerEvent) => {
        if (!started) {
          if (!dragStarts(startY, move.clientY)) return;
          started = true;
        }
        const landing = land(move.clientY);
        setDrag({ source: index, gap: landing.to === null ? null : landing.gap, y: move.clientY, left: list.current?.getBoundingClientRect().left ?? 0 });
      };
      const onPointerUp = (up: PointerEvent) => {
        const landing = started ? land(up.clientY) : null;
        stop();
        if (!started) return;
        // The click that follows a drag must not fold the row it ends on.
        const swallow = (click: Event) => click.stopPropagation();
        window.addEventListener("click", swallow, { capture: true, once: true });
        window.setTimeout(() => window.removeEventListener("click", swallow, true), 0);
        if (landing?.to != null) onMove(index, landing.to);
      };
      const onKey = (key: KeyboardEvent) => {
        if (key.key === "Escape") {
          key.preventDefault();
          key.stopPropagation();
          started = false;
          stop();
        }
      };
      window.addEventListener("pointermove", onPointerMove);
      window.addEventListener("pointerup", onPointerUp);
      window.addEventListener("keydown", onKey, true);
    },
    [list, onMove],
  );
  return { drag, press };
}

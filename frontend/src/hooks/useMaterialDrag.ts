import { useCallback, useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type RefObject } from "react";

import { ghostTop } from "@/lib/operationRows";

/** The ghost of a drag in flight (`0101` ux-notes 4.6): the pointer's Y and the list's box. */
export interface DragGhost {
  y: number;
  offset: number;
  left: number;
  top: number;
  bottom: number;
  width: number;
  label: string;
  /** Its top edge in the viewport. */
  at: number;
}

export interface MaterialDrag {
  /** The row index being dragged once the drag is past 4 px, else `null`. */
  from: number | null;
  /** The index the row would land on. */
  dropAt: number | null;
  ghost: DragGhost | null;
  /** A pointer press on a row's name starts a possible drag. */
  start: (index: number, label: string, event: ReactPointerEvent<HTMLElement>) => void;
}

/**
 * The reorder drag of the Materials list (`specs/0028` `ux-notes.md` 10.2, as `0101` 4.6):
 * starts after 4 px, lands on the row under the pointer, is one move on release, and Escape
 * cancels it. `onMove(from, to)` performs the move. The rows are `li[data-material-index]`.
 */
export function useMaterialDrag(
  listRef: RefObject<HTMLElement | null>,
  onMove: (from: number, to: number) => void,
): MaterialDrag {
  const [from, setFrom] = useState<number | null>(null);
  const [dropAt, setDropAt] = useState<number | null>(null);
  const [ghost, setGhost] = useState<DragGhost | null>(null);
  const press = useRef<{ index: number; label: string; startY: number; offset: number; active: boolean } | null>(null);
  const target = useRef<number | null>(null);
  const move = useRef(onMove);
  useEffect(() => {
    move.current = onMove;
  }, [onMove]);

  const landingAt = useCallback(
    (clientY: number, source: number) => {
      const rows = Array.from(listRef.current?.querySelectorAll<HTMLElement>("li[data-material-index]") ?? []);
      let at = source;
      for (const element of rows) {
        const index = Number(element.dataset.materialIndex);
        const box = element.getBoundingClientRect();
        const middle = box.top + 20;
        if (index > source && clientY >= middle) at = index;
        if (index < source && clientY < middle) at = Math.min(at, index);
      }
      return at;
    },
    [listRef],
  );

  useEffect(() => {
    const finish = (apply: boolean) => {
      const current = press.current;
      press.current = null;
      const to = target.current;
      target.current = null;
      setFrom(null);
      setDropAt(null);
      setGhost(null);
      if (current?.active && apply && to !== null && to !== current.index) move.current(current.index, to);
    };
    const onPointerMove = (event: PointerEvent) => {
      const current = press.current;
      if (!current) return;
      if (!current.active) {
        if (Math.abs(event.clientY - current.startY) < 4) return;
        current.active = true;
        setFrom(current.index);
      }
      const box = listRef.current?.getBoundingClientRect();
      setGhost({
        y: event.clientY,
        offset: current.offset,
        left: box?.left ?? 0,
        top: box?.top ?? 0,
        bottom: box?.bottom ?? 0,
        width: box?.width ?? 0,
        label: current.label,
        at: ghostTop(event.clientY, current.offset, box?.top ?? 0, box?.bottom ?? 0, 40),
      });
      const to = landingAt(event.clientY, current.index);
      target.current = to;
      setDropAt(to);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && press.current?.active) {
        event.preventDefault();
        event.stopPropagation();
        finish(false);
      }
    };
    const onUp = () => finish(true);
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("keydown", onKey, true);
    };
  }, [landingAt, listRef]);

  const start = useCallback((index: number, label: string, event: ReactPointerEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    const top = event.currentTarget.closest("li")?.getBoundingClientRect().top ?? event.clientY;
    press.current = { index, label, startY: event.clientY, offset: event.clientY - top, active: false };
  }, []);

  return { from, dropAt, ghost, start };
}

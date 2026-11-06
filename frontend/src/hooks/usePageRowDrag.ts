import { useCallback, useEffect, useRef, useState } from "react";
import type { WasmSession } from "@/lib/editorSession";
import { type DropBadgeKind, badgeWord } from "@/lib/sketchPages";

/** The drop badge beside the pointer (UX notes §2 "Drop badge"). */
export interface DropBadge {
  word: string;
  refused: boolean;
  x: number;
  y: number;
}

/** The answer of one `row_drag_move`. */
interface Answer {
  badge: DropBadgeKind;
  target: string | null;
  faint: boolean;
}

/** About the badge's size, for the mirror at a viewport edge. */
const BADGE_W = 200;
const BADGE_H = 20;
const OFFSET = 16;

function badgePlace(x: number, y: number): { x: number; y: number } {
  const right = x + OFFSET + BADGE_W > window.innerWidth;
  const below = y + OFFSET + BADGE_H > window.innerHeight;
  return {
    x: right ? x - OFFSET - BADGE_W : x + OFFSET,
    y: below ? y - OFFSET - BADGE_H : y + OFFSET,
  };
}

/**
 * The drag of a page row out of the page list onto the canvas (`specs/0099-sketch-pages`
 * criteria 8 to 10, `adrs.md` note 4): forwards pointer positions in canvas coordinates to the
 * session and draws the badge from its answer. The ghost and the frame cue are drawn from the
 * session's state; nothing is written before the drop.
 */
export function usePageRowDrag(
  getSession: () => WasmSession | null,
  canvas: () => HTMLElement | null,
  onPlaced: () => void,
) {
  const [badge, setBadge] = useState<DropBadge | null>(null);
  const dragging = useRef<{ name: string } | null>(null);
  const setCursor = (refused: boolean) => {
    document.body.style.cursor = refused ? "not-allowed" : "";
  };
  const cancel = useCallback(() => {
    if (!dragging.current) return;
    dragging.current = null;
    getSession()?.row_drag_cancel();
    setBadge(null);
    setCursor(false);
  }, [getSession]);
  // Escape cancels in the same frame (UX notes §2).
  useEffect(() => {
    if (!badge) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        cancel();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [badge, cancel]);
  /** Begins a drag of row `id`; `false` for a row that places nothing (no place mode). */
  const begin = (id: string, name: string) => {
    const session = getSession();
    if (!session?.row_drag_begin(id)) return false;
    dragging.current = { name };
    return true;
  };
  /** One pointer move; `inList` while the pointer is over the page list (reorder mode). */
  const move = (clientX: number, clientY: number, inList: boolean) => {
    const session = getSession();
    const drag = dragging.current;
    if (!session || !drag) return;
    const area = canvas();
    const under = document.elementFromPoint(clientX, clientY);
    const onCanvas = !inList && !!area && !!under && area.contains(under);
    const rect = area?.getBoundingClientRect();
    const answer = JSON.parse(
      session.row_drag_move(
        clientX - (rect?.left ?? 0),
        clientY - (rect?.top ?? 0),
        onCanvas,
      ),
    ) as Answer;
    const word = badgeWord(answer.badge, drag.name);
    const refused = answer.badge === "not_on_sketch_page" || answer.badge === "no_sketch";
    setCursor(refused);
    setBadge(word ? { word, refused, ...badgePlace(clientX, clientY) } : null);
  };
  /** The release: places the clone where the session allows it. Whether it did. */
  const drop = () => {
    if (!dragging.current) return false;
    dragging.current = null;
    const placed = getSession()?.row_drag_drop() ?? false;
    setBadge(null);
    setCursor(false);
    if (placed) onPlaced();
    return placed;
  };
  return { badge, begin, move, drop, cancel, active: () => dragging.current !== null };
}
export type PageRowDragApi = ReturnType<typeof usePageRowDrag>;

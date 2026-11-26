import { useCallback, useEffect, useRef, useState } from "react";
import type { WasmSession } from "@/lib/editorSession";
import type { DropBadge } from "@/components/DropBadgePill";
import { badgeWord, decodeDragAnswer, isRefusal } from "@/lib/dragAnswer";

/** About the badge's size, for the mirror at a viewport edge. */
const BADGE_W = 200;
const BADGE_H = 20;
const OFFSET = 16;

/** What the Ctrl release over a slot answers. */
interface SlotAnswer {
  refused?: string | null;
  notice?: string | null;
  error?: string;
}

export function badgePlace(x: number, y: number): { x: number; y: number } {
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
  onNotice?: (text: string, kind: "status" | "alert") => void,
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
    const answer = decodeDragAnswer(
      session.row_drag_move(clientX - (rect?.left ?? 0), clientY - (rect?.top ?? 0), onCanvas),
    );
    if (!answer) {
      setCursor(false);
      setBadge(null);
      return;
    }
    const word = badgeWord(answer.badge, drag.name);
    const refused = isRefusal(answer.badge);
    setCursor(refused);
    setBadge(word ? { kind: answer.badge, word, refused, ...badgePlace(clientX, clientY) } : null);
  };
  /** The release: places the clone where the session allows it. Whether it did. */
  const drop = (ctrl = false, alt = false) => {
    if (!dragging.current) return false;
    dragging.current = null;
    const session = getSession();
    // Ctrl over a slot places the page live, fitted into the slot (`specs/0154`, slice 2).
    const into = ctrl && session ? (JSON.parse(session.row_drag_drop_into_slot(alt)) as SlotAnswer | null) : null;
    if (into) {
      setBadge(null);
      setCursor(false);
      const text = into.error ? `Nothing was changed: ${into.error}.` : (into.refused ?? into.notice);
      if (text) onNotice?.(text, into.error || into.refused ? "alert" : "status");
      const written = !into.error && !into.refused;
      if (written) onPlaced();
      return written;
    }
    const placed = session?.row_drag_drop() ?? false;
    setBadge(null);
    setCursor(false);
    if (placed) onPlaced();
    return placed;
  };
  return { badge, begin, move, drop, cancel, active: () => dragging.current !== null };
}
export type PageRowDragApi = ReturnType<typeof usePageRowDrag>;

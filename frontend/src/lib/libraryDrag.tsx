// The drag of a library page row or of picked elements onto the drawing (`specs/0137` UX notes
// sections 6.5 and 7.1): past 4 px a ghost follows the pointer, over the canvas with its Drop badge
// (`Import` "New page", `Copy` "Copy", `Ban` "Not on a sketch page"); a release over the canvas
// hands the canvas point (CSS px) to `onDrop`, anywhere else or Escape does nothing.
import { createRoot, type Root } from "react-dom/client";

import { LibraryDragGhost, type LibraryDropKind } from "@/components/LibraryDragGhost";

const THRESHOLD_PX = 4;

/** Watches the press `down` for a drag onto `canvas`. `badge()` names the badge over the canvas,
 * `null` to refuse there (a sketch page). `picture`: the page thumbnail that follows the pointer. */
export function watchCanvasDrag(
  down: { clientX: number; clientY: number; pointerId: number },
  canvas: () => HTMLCanvasElement | null,
  label: string,
  badge: () => LibraryDropKind,
  onDrop: (at: { x: number; y: number }) => void,
  picture: string | null = null,
): void {
  let ghost: HTMLDivElement | null = null;
  let root: Root | null = null;
  const inside = (x: number, y: number) => {
    const rect = canvas()?.getBoundingClientRect();
    return rect && x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom ? rect : null;
  };
  const end = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    window.removeEventListener("keydown", key, true);
    delete document.documentElement.dataset.libraryDrag;
    root?.unmount();
    root = null;
    ghost?.remove();
    ghost = null;
  };
  const move = (event: PointerEvent) => {
    if (event.pointerId !== down.pointerId) {
      return;
    }
    if (!ghost && Math.hypot(event.clientX - down.clientX, event.clientY - down.clientY) <= THRESHOLD_PX) {
      return;
    }
    if (!ghost) {
      ghost = document.createElement("div");
      ghost.className = "library-drag-ghost";
      ghost.setAttribute("aria-hidden", "true");
      document.body.appendChild(ghost);
      root = createRoot(ghost);
    }
    const over = inside(event.clientX, event.clientY) !== null;
    const kind = over ? badge() : null;
    // The drag's cursor over everything (the canvas sets its own otherwise).
    document.documentElement.dataset.libraryDrag = over && kind === null ? "refused" : "grabbing";
    ghost.style.left = `${event.clientX}px`;
    ghost.style.top = `${event.clientY}px`;
    root?.render(<LibraryDragGhost label={label} picture={picture} over={over} kind={kind} />);
  };
  const up = (event: PointerEvent) => {
    if (event.pointerId !== down.pointerId) {
      return;
    }
    const dragged = ghost !== null;
    end();
    const rect = dragged ? inside(event.clientX, event.clientY) : null;
    if (rect && badge() !== null) {
      onDrop({ x: event.clientX - rect.left, y: event.clientY - rect.top });
    }
  };
  const key = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      end();
    }
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
  window.addEventListener("keydown", key, true);
}

// The drag of a library page row or of picked elements onto the drawing (`specs/0137` UX notes
// sections 6.5 and 7.1): past 4 px a ghost follows the pointer, over the canvas with its Drop badge
// (`Import` "New page", `Copy` "Copy", `Ban` "Not on a sketch page"); a release over the canvas
// hands the canvas point (CSS px) to `onDrop`, anywhere else or Escape does nothing.
import { createRoot, type Root } from "react-dom/client";

import { LibraryDragGhost, type LibraryDropKind } from "@/components/LibraryDragGhost";
import type { SlotCue, SlotUnitSource } from "@/lib/intoSlot";
import type { WasmSession } from "@/wasm-bindings/curvyo_editor_wasm.js";

const THRESHOLD_PX = 4;

/** The slot route of a drag (`specs/0154` criteria 1, 6 d, 8): the cue over a slot of the active
 * laser page and the release into it. `size` is the unit's size in mm when known; `multiPage`
 * refuses a source of several pages over a slot. */
export interface SlotDrag {
  getSession: () => WasmSession | null;
  redraw: () => void;
  /** The source once it is read (its size and outlines go to the cue), else null. */
  source: () => SlotUnitSource | null;
  multiPage: () => boolean;
  onSlot: (cue: SlotCue, alt: boolean) => void;
  /** Called when the drag ends, after `onSlot` when there was a slot release. */
  end?: () => void;
}

function cueAt(slot: SlotDrag, at: { x: number; y: number }, alt: boolean, shift: boolean): SlotCue | null {
  const session = slot.getSession();
  if (!session) {
    return null;
  }
  try {
    const cue = JSON.parse(session.slot_drop_cue_json(at.x, at.y, alt, shift)) as SlotCue;
    return cue.kind === "none" ? null : cue;
  } catch {
    return null;
  }
}

/** Watches the press `down` for a drag onto `canvas`. `badge()` names the badge over the canvas,
 * `null` to refuse there (a sketch page). `picture`: the page thumbnail that follows the pointer. */
export function watchCanvasDrag(
  down: { clientX: number; clientY: number; pointerId: number },
  canvas: () => HTMLCanvasElement | null,
  label: string,
  badge: () => LibraryDropKind,
  onDrop: (at: { x: number; y: number }) => void,
  picture: string | null = null,
  slot: SlotDrag | null = null,
): void {
  let ghost: HTMLDivElement | null = null;
  let root: Root | null = null;
  let started = false;
  let given = false;
  let cue: SlotCue | null = null;
  // The polite status of the drag over a slot (UX notes 3.1), once per change.
  let status: HTMLDivElement | null = null;
  const startSlot = () => {
    const session = slot?.getSession();
    if (!slot || !session) {
      return;
    }
    if (!started) {
      started = true;
      session.begin_slot_drop(0, 0, false);
      status = document.createElement("div");
      status.className = "sr-only";
      status.setAttribute("aria-live", "polite");
      document.body.appendChild(status);
    }
    const source = given ? null : slot.source();
    if (source) {
      given = true;
      session.set_slot_drop_source(source.reader, `[${source.roots.join(",")}]`, source.whole ? source.from.pageId : undefined);
    }
  };
  const endSlot = () => {
    if (slot && started) {
      slot.getSession()?.end_slot_drop();
      slot.redraw();
    }
    status?.remove();
    status = null;
    slot?.end?.();
  };
  const inside = (x: number, y: number) => {
    const rect = canvas()?.getBoundingClientRect();
    return rect && x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom ? rect : null;
  };
  const end = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    window.removeEventListener("keydown", key, true);
    window.removeEventListener("keyup", key, true);
    endSlot();
    delete document.documentElement.dataset.libraryDrag;
    root?.unmount();
    root = null;
    ghost?.remove();
    ghost = null;
  };
  /** The pointer's last place, so Alt and Shift switch the cue without a move. */
  let last: { clientX: number; clientY: number } | null = null;
  const move = (event: Pick<PointerEvent, "pointerId" | "clientX" | "clientY" | "altKey" | "shiftKey">) => {
    if (event.pointerId !== down.pointerId) {
      return;
    }
    last = { clientX: event.clientX, clientY: event.clientY };
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
    const rect = inside(event.clientX, event.clientY);
    const over = rect !== null;
    const kind = over ? badge() : null;
    cue = null;
    if (slot && rect) {
      startSlot();
      cue = cueAt(slot, { x: event.clientX - rect.left, y: event.clientY - rect.top }, event.altKey, event.shiftKey);
      if (cue && slot.multiPage()) {
        cue = { ...cue, kind: "refused", badge: "Only one page goes into a slot.", status: "Only one page goes into a slot." };
      }
      const said = cue?.status ?? "";
      if (status && status.textContent !== said) {
        status.textContent = said;
      }
      slot.redraw();
    }
    // The drag's cursor over everything (the canvas sets its own otherwise).
    document.documentElement.dataset.libraryDrag = over && kind === null && !cue ? "refused" : "grabbing";
    ghost.style.left = `${event.clientX}px`;
    ghost.style.top = `${event.clientY}px`;
    root?.render(
      <LibraryDragGhost
        label={label}
        picture={cue && cue.kind !== "refused" ? null : picture}
        over={over}
        kind={kind}
        slot={cue ? { kind: cue.kind, word: cue.badge ?? "" } : null}
      />,
    );
  };
  const up = (event: PointerEvent) => {
    if (event.pointerId !== down.pointerId) {
      return;
    }
    const dragged = ghost !== null;
    const rect = dragged ? inside(event.clientX, event.clientY) : null;
    // The release follows the modifiers of the pointer-up (the `0100` rule).
    const last = slot && rect ? cueAt(slot, { x: event.clientX - rect.left, y: event.clientY - rect.top }, event.altKey, event.shiftKey) : null;
    if (slot && last && !slot.multiPage()) {
      if (last.kind !== "refused") {
        // A slot of another laser page: the release activates that page and puts into the slot
        // (criterion 9, Proposal 2026-11-26); the activation is no step, the put is one.
        if (last.page) {
          slot.getSession()?.activate_page(last.page);
        }
        slot.onSlot(last, event.altKey);
      }
      end();
      return;
    }
    end();
    if (rect && badge() !== null) {
      onDrop({ x: event.clientX - rect.left, y: event.clientY - rect.top });
    }
  };
  const key = (event: KeyboardEvent) => {
    // Alt keeps the real size over a slot; the browser's menu bar must not take it.
    if (event.key === "Alt" && ghost) {
      event.preventDefault();
    }
    // Alt and Shift switch the ghost and the badge live, also while the pointer rests (UX 3.1).
    if ((event.key === "Alt" || event.key === "Shift") && ghost && last && slot) {
      move({ pointerId: down.pointerId, ...last, altKey: event.altKey, shiftKey: event.shiftKey });
    }
    if (event.key === "Escape") {
      event.stopPropagation();
      end();
    }
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
  window.addEventListener("keydown", key, true);
  window.addEventListener("keyup", key, true);
}

// The box the context bars cover over the canvas, measured live, so canvas labels and tags keep
// clear of a bar of any height (`specs/0164-ui-consistency` F42, F50): the bar area publishes it,
// the clone tags and the page labels read it in their own coordinates.
import { useState, useSyncExternalStore } from "react";

/** A box in client (viewport) pixels. */
interface ClientBox {
  left: number;
  right: number;
  bottom: number;
}

/** The bars' box relative to a reader's own element, px. */
export interface BarBox {
  left: number;
  right: number;
  bottom: number;
}

let published: ClientBox | null = null;
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function publish(next: ClientBox | null) {
  const same =
    next !== null &&
    published !== null &&
    next.left === published.left &&
    next.right === published.right &&
    next.bottom === published.bottom;
  if (same || (next === null && published === null)) return;
  published = next;
  for (const listener of listeners) listener();
}

/** The union of the visible bars inside `area` (its children with a size), or null for none. */
function measure(area: HTMLElement): ClientBox | null {
  let box: ClientBox | null = null;
  for (const child of Array.from(area.children)) {
    const rect = child.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) continue;
    box = box
      ? { left: Math.min(box.left, rect.left), right: Math.max(box.right, rect.right), bottom: Math.max(box.bottom, rect.bottom) }
      : { left: rect.left, right: rect.right, bottom: rect.bottom };
  }
  return box && { left: Math.round(box.left), right: Math.round(box.right), bottom: Math.round(box.bottom) };
}

/** The ref callback of the bar area (`data-bar-area`): while mounted it publishes its bars' box
 * after every resize and change of its bars. */
export function publishBarBox(element: HTMLElement | null): (() => void) | undefined {
  if (!element) return undefined;
  const update = () => publish(measure(element));
  const resize = new ResizeObserver(update);
  const watch = () => {
    resize.observe(element);
    for (const child of Array.from(element.children)) resize.observe(child);
  };
  watch();
  update();
  const mutation = new MutationObserver(() => {
    watch();
    update();
  });
  mutation.observe(element, { childList: true });
  window.addEventListener("resize", update);
  return () => {
    resize.disconnect();
    mutation.disconnect();
    window.removeEventListener("resize", update);
    publish(null);
  };
}

/** The bars' box in the coordinates of the reader's element (its top-left corner is 0, 0), or
 * null while no bar shows; the first value is the ref callback for that element. */
export function useBarBox(): [(element: HTMLElement | null) => void, BarBox | null] {
  const client = useSyncExternalStore(subscribe, () => published);
  const [root, setRoot] = useState<HTMLElement | null>(null);
  if (!client || !root) return [setRoot, null];
  const origin = root.getBoundingClientRect();
  return [setRoot, { left: client.left - origin.left, right: client.right - origin.left, bottom: client.bottom - origin.top }];
}

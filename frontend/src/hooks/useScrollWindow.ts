import { useEffect, useState, type RefObject } from "react";

/** The viewport in the list's own coordinates (px): its top and height. */
export interface ScrollWindow {
  top: number;
  height: number;
}

/** The nearest ancestor that scrolls, or `null`: the panel's column (the tab scrolls as one). */
function scrollParent(element: HTMLElement | null): HTMLElement | null {
  for (let at = element?.parentElement ?? null; at; at = at.parentElement) {
    const overflow = getComputedStyle(at).overflowY;
    if (overflow === "auto" || overflow === "scroll") return at;
  }
  return null;
}

/**
 * Where the panel's scroll shows the list at `ref` (`specs/0028` `ux-notes.md` 15: a long
 * Materials or pick list is windowed against the tab's scroll). `null` while `enabled` is false or
 * nothing scrolls; `watch` makes it measure again when the list changes size.
 */
export function useScrollWindow(
  ref: RefObject<HTMLElement | null>,
  enabled: boolean,
  watch: unknown,
): ScrollWindow | null {
  const [window_, setWindow] = useState<ScrollWindow | null>(null);
  useEffect(() => {
    const list = ref.current;
    const container = scrollParent(list);
    if (!enabled || !list || !container) {
      setWindow(null);
      return;
    }
    const measure = () => {
      const offset = list.getBoundingClientRect().top - container.getBoundingClientRect().top;
      setWindow((previous) => {
        const next = { top: Math.max(0, -offset), height: container.clientHeight };
        return previous && previous.top === next.top && previous.height === next.height ? previous : next;
      });
    };
    measure();
    container.addEventListener("scroll", measure, { passive: true });
    window.addEventListener("resize", measure);
    return () => {
      container.removeEventListener("scroll", measure);
      window.removeEventListener("resize", measure);
    };
  }, [ref, enabled, watch]);
  return window_;
}

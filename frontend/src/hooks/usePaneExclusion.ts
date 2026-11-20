import { useEffect, useRef } from "react";

/**
 * One side pane at a time (`specs/0135-image-optimisation` `ux-notes.md` §7, P19): the one that
 * opened last stays, the other closes. `closeFont` and `closeAdjust` must be callable at any time.
 */
export function usePaneExclusion(
  fontOpen: boolean,
  adjustOpen: boolean,
  closeFont: () => void,
  closeAdjust: () => void,
): void {
  const was = useRef({ font: fontOpen, adjust: adjustOpen });
  useEffect(() => {
    const before = was.current;
    was.current = { font: fontOpen, adjust: adjustOpen };
    if (!fontOpen || !adjustOpen) return;
    if (fontOpen && !before.font) closeAdjust();
    else if (adjustOpen && !before.adjust) closeFont();
  }, [fontOpen, adjustOpen, closeFont, closeAdjust]);
}

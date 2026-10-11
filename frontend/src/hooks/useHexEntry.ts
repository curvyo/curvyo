import { type RefObject, useCallback, useRef } from "react";

/**
 * The hex field that a colour slider's value entry opens (`specs/0047-transform-polish/`
 * criteria 23 and 25): a double-click focuses it and Escape or Enter returns the keyboard to
 * the canvas, while Enter or F2 on a slider focuses it and the same keys return the keyboard to
 * that slider, as the Ratio sliders do.
 */
export function useHexEntry(
  hex: RefObject<HTMLInputElement | null>,
  onReturnFocus: () => void,
): {
  /** Focuses the hex field; `origin` is the slider whose key opened it, `null` for a pointer. */
  open: (origin: HTMLElement | null) => void;
  /** For the hex field's `onReturnFocus`. */
  returnFocus: () => void;
} {
  const origin = useRef<HTMLElement | null>(null);

  const open = useCallback(
    (from: HTMLElement | null) => {
      origin.current = from;
      const field = hex.current;
      if (!field) {
        return;
      }
      // A field that loses focus any other way (a click elsewhere) forgets the slider, but only
      // after the return of an Escape or Enter, which runs right after the blur.
      field.addEventListener(
        "blur",
        () => {
          window.setTimeout(() => {
            origin.current = null;
          }, 0);
        },
        { once: true },
      );
      field.focus();
    },
    [hex],
  );

  const returnFocus = useCallback(() => {
    const slider = origin.current;
    origin.current = null;
    if (slider?.isConnected) {
      slider.focus({ preventScroll: true });
    } else {
      onReturnFocus();
    }
  }, [onReturnFocus]);

  return { open, returnFocus };
}

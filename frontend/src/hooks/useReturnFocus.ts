// Focus return after an inline confirm closes (`specs/0096-own-devices-sync` ux-notes 10).

import { useEffect, useRef } from "react";

/** Puts the focus on `target` once after `when` turned true (a closed confirm returns focus to
 * the control it replaced). */
export function useReturnFocus(when: boolean, target: () => HTMLElement | null | undefined) {
  const pending = useRef(false);
  useEffect(() => {
    if (when && pending.current) {
      pending.current = false;
      target()?.focus();
    }
  });
  return () => {
    pending.current = true;
  };
}


// The one armed trash of the Fonts tab (`specs/0081-inline-delete-confirm` criteria 3 to 6, 11,
// 13 and 16; UX notes §4 to §6): it disarms after 4 s, on a pointer press anywhere but its
// trash, on Escape and when the focus moves out of its row; the polite status text follows it.
import { useCallback, useEffect, useRef, useState } from "react";

import { ARM_TIMEOUT_MS, type ArmState, IDLE, focusKeepsArm, pressTrash } from "@/lib/deleteArm";

/** The attribute naming an element's arm target: on a trash and on the row (or cell) it arms. */
export const ARM_TRASH = "data-arm-trash";
export const ARM_ZONE = "data-arm-zone";

export interface DeleteArm {
  /** The armed target id, or `null`. */
  armed: string | null;
  /** The text of the shared polite status region. */
  status: string;
  /** A press on the trash of `target`; `label` is its armed text; `remove` runs on the second
   * press after the guard and returns what the status region says then. */
  press: (target: string, label: string, remove: () => string) => void;
  disarm: () => void;
}

export function useDeleteArm(): DeleteArm {
  const [state, setState] = useState<ArmState>(IDLE);
  const [status, setStatus] = useState("");
  const stateRef = useRef(state);

  const disarm = useCallback(() => {
    if (stateRef.current.armed !== null) {
      stateRef.current = IDLE;
      setState(IDLE);
      setStatus("");
    }
  }, []);

  const press = useCallback((target: string, label: string, remove: () => string) => {
    const out = pressTrash(stateRef.current, target, performance.now());
    if (out.state === stateRef.current) {
      return;
    }
    stateRef.current = out.state;
    setState(out.state);
    setStatus(out.remove ? remove() : label);
  }, []);

  const armed = state.armed;
  useEffect(() => {
    if (armed === null) {
      return;
    }
    const timer = window.setTimeout(disarm, ARM_TIMEOUT_MS - (performance.now() - state.at));
    const onPointer = (event: PointerEvent) => {
      const trash = (event.target as Element | null)?.closest?.(`[${ARM_TRASH}]`);
      if (trash?.getAttribute(ARM_TRASH) !== armed) {
        disarm();
      }
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        // Consumed: the first Escape only disarms (UX notes §5).
        event.preventDefault();
        event.stopPropagation();
        disarm();
      }
    };
    const zoneOf = (target: EventTarget | null) =>
      (target as Element | null)?.closest?.(`[${ARM_ZONE}]`)?.getAttribute(ARM_ZONE);
    // Focus arriving outside the zone disarms; so does focus leaving it to nothing (Tab to the
    // body, `blur()`), where no `focusin` follows.
    const onFocusIn = (event: FocusEvent) => {
      if (!focusKeepsArm(armed, zoneOf(event.target))) {
        disarm();
      }
    };
    const onFocusOut = (event: FocusEvent) => {
      if (zoneOf(event.target) === armed && !focusKeepsArm(armed, zoneOf(event.relatedTarget))) {
        disarm();
      }
    };
    window.addEventListener("pointerdown", onPointer, true);
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("focusin", onFocusIn, true);
    window.addEventListener("focusout", onFocusOut, true);
    return () => {
      window.clearTimeout(timer);
      window.removeEventListener("pointerdown", onPointer, true);
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("focusin", onFocusIn, true);
      window.removeEventListener("focusout", onFocusOut, true);
    };
  }, [armed, state.at, disarm]);

  return { armed, status, press, disarm };
}

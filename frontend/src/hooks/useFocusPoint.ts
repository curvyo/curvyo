import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from "react";

import { type FramePage } from "@/hooks/useFramePage";
import { type MachineControlsModel } from "@/hooks/useMachineControls";
import { focusCause, ownFocus, pageCause, pressTakes, ringTarget } from "@/lib/focusPoint";

/**
 * The Focus point mode of this app (`specs/0118` slice 2d, UX notes W54, W56, W57): on or off,
 * shared by the jog pad, which owns it, and the canvas, which takes its presses. Session state,
 * off at start and never restored.
 */
const listeners = new Set<() => void>();
let on = false;
let press: ((x: number, y: number) => void) | null = null;

function set(next: boolean) {
  if (on === next) return;
  on = next;
  for (const listener of listeners) listener();
}

export const focusMode = {
  get: () => on,
  subscribe(listener: () => void) {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  },
  /** A press on the canvas at the document point `(x, y)` mm. */
  press(x: number, y: number) {
    press?.(x, y);
  },
  /** Ends the mode: the toggle, Escape, Stop, a tool change; it stops nothing. */
  end() {
    set(false);
  },
};

/** Whether the mode is on, for the canvas's cursor and its press. */
export function useFocusMode(): boolean {
  return useSyncExternalStore(focusMode.subscribe, focusMode.get);
}

export interface FocusPointToggle {
  on: boolean;
  cause: string | null;
  toggle: () => void;
}

/**
 * The jog pad's side of the mode: the toggle's cause (W55), the end on a new cause (W56), the
 * canvas press as a go to point (W57, W58) and the target ring at the connector's clamped target
 * (W59). `ownJog` is a jog of this pad, which keeps the mode on.
 */
export function useFocusPoint(model: MachineControlsModel, frame: FramePage, ownJog: boolean): FocusPointToggle {
  const active = useFocusMode();
  // This app sent the go to point the connector runs or waits with.
  const [pressed, setPressed] = useState(false);
  const status = model.entry.status;
  const own = ownFocus(status, pressed);
  const cause = focusCause(model.entry, pageCause(frame.target(0, 0)), { jog: ownJog, focus: own });
  const latest = useRef({ status, pressed, model, frame });
  useEffect(() => {
    latest.current = { status, pressed, model, frame };
  });
  // The own go to point is over once the connector leaves its states after it.
  const [wasOwn, setWasOwn] = useState(own);
  if (wasOwn !== own) {
    setWasOwn(own);
    if (!own) setPressed(false);
  }
  // Any new cause ends the mode, the toggle shows it (W56).
  useEffect(() => {
    if (active && cause !== null) focusMode.end();
  }, [active, cause]);
  const ring = ringTarget(status, pressed);
  // The page API is a new object after every sync, its functions are not: depending on the
  // object would place the ring, sync and place it again without end.
  const { ring: showRing, clearRing } = frame;
  useEffect(() => {
    if (active && ring !== null) showRing(ring[0], ring[1]);
  }, [active, ring?.[0], ring?.[1], showRing]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (!active) clearRing();
  }, [active, clearRing]);
  useEffect(() => {
    press = (x, y) => {
      const now = latest.current;
      if (!pressTakes(now.status, now.pressed)) return;
      const target = now.frame.target(x, y);
      if (target === null || "cause" in target) return;
      now.frame.clearRing();
      setPressed(true);
      void now.model.goToPoint(target.x, target.y, now.frame.pulse()).then((took) => {
        if (!took) setPressed(false);
      });
    };
    return () => {
      press = null;
    };
  }, []);
  // The pad closing, the tab or the page changing ends the mode.
  const pageId = frame.pageId;
  useEffect(() => () => focusMode.end(), [pageId]);
  const toggle = useCallback(() => {
    if (focusMode.get()) focusMode.end();
    else if (cause === null) set(true);
  }, [cause]);
  return { on: active, cause, toggle };
}

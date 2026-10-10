import { useEffect, useRef } from "react";
import type { PointerEvent } from "react";

import { type Point, SliderGesture, type Thumb } from "@/lib/sliderGesture";

/**
 * What a slider does with its pointer (`specs/0047-transform-polish/` Part B): the one
 * behaviour of all four sliders. The pure rules are `SliderGesture`'s; this reads the
 * pointer events, turns them into positions in the slider's own pixels and calls what
 * the slider gave it. Nothing changes at the press: a click acts at the release, a drag
 * after the slop, a double-click opens the entry.
 */
export interface SliderPointerOptions {
  /** The thumb now, in the slider's pixels, or `null` when it shows none (mixed). */
  thumb: () => Thumb | null;
  /** A click on the track (not on the thumb, not the second click of a double-click):
   * set the value at `at` and commit it. */
  onClick: (at: Point) => void;
  /** A drag step: the value goes to `at`, which already includes the offset of a grab. */
  onDrag: (at: Point) => void;
  /** The release of a drag. */
  onDragEnd: () => void;
  /** The system took the pointer away in the middle of a press. */
  onPointerCancel?: () => void;
  /** A double-click, at the release of its second click. */
  onDoubleClick: () => void;
}

/** The handlers to spread on the slider element. */
export interface SliderPointerHandlers {
  onPointerDown: (event: PointerEvent<HTMLElement>) => void;
  onPointerMove: (event: PointerEvent<HTMLElement>) => void;
  onPointerUp: (event: PointerEvent<HTMLElement>) => void;
  onPointerCancel: (event: PointerEvent<HTMLElement>) => void;
}

const local = (event: PointerEvent<HTMLElement>): Point => {
  const box = event.currentTarget.getBoundingClientRect();
  return { x: event.clientX - box.left, y: event.clientY - box.top };
};

export function useSliderPointer(
  options: SliderPointerOptions,
): SliderPointerHandlers {
  const gesture = useRef(new SliderGesture());
  const grab = useRef<Point | null>(null);
  const latest = useRef(options);
  // After every render, so an event handler sees the latest values.
  useEffect(() => {
    latest.current = options;
  });

  const follow = (at: Point) => {
    const offset = grab.current;
    latest.current.onDrag(
      offset ? { x: at.x + offset.x, y: at.y + offset.y } : at,
    );
  };

  return {
    onPointerDown: (event) => {
      if (event.button !== 0) {
        return;
      }
      try {
        event.currentTarget.setPointerCapture(event.pointerId);
      } catch {
        // Not capturable (a synthetic event): the gesture works while the pointer stays over it.
      }
      // The slider takes the focus at the press, which also closes an entry beside it.
      event.currentTarget.focus({ preventScroll: true });
      grab.current = null;
      gesture.current.down(
        local(event),
        event.pointerType !== "mouse",
        latest.current.thumb(),
      );
    },
    onPointerMove: (event) => {
      if (event.buttons === 0) {
        return;
      }
      const at = local(event);
      const result = gesture.current.move(at);
      if (result.kind === "drag-start") {
        grab.current = result.grab;
        follow(at);
      } else if (result.kind === "drag") {
        follow(at);
      }
    },
    onPointerUp: (event) => {
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
      const at = local(event);
      const result = gesture.current.up(at, event.timeStamp);
      if (result.kind === "click" && !result.onThumb) {
        latest.current.onClick(at);
      } else if (result.kind === "double-click") {
        latest.current.onDoubleClick();
      } else if (result.kind === "drag-end") {
        latest.current.onDragEnd();
      }
    },
    onPointerCancel: () => {
      gesture.current.cancel();
      latest.current.onPointerCancel?.();
    },
  };
}

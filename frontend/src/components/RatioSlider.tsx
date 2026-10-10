import { useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { useSliderPointer } from "@/hooks/useSliderPointer";
import {
  RATIO_MESSAGE,
  RATIO_RANGE,
  parseRatioEntry,
  positionOfValue,
  snapToStep,
  valueAtPosition,
} from "@/lib/sliderValue";

/** The slider's width in px, and the inset that keeps the thumb inside it. */
const WIDTH = 96;
const THUMB = 14;
const INSET = THUMB / 2 + 1;
/** The pointer target is 24 px high around a 4 px track (WCAG 2.5.8). */
const TARGET_HEIGHT = 24;

const STEP_KEYS = new Set([
  "ArrowLeft",
  "ArrowRight",
  "ArrowUp",
  "ArrowDown",
  "PageUp",
  "PageDown",
  "Home",
  "End",
]);

export interface RatioSliderProps {
  /** The accessible name of the slider and of its entry: "Inner ratio". */
  label: string;
  /** The tooltip. Both gestures are in it, since nothing else shows them. */
  title: string;
  value: number;
  /** The edited objects differ: no thumb, "Mixed". */
  mixed?: boolean;
  /** Every tick of a drag and every step of a key: shown, not written. */
  onPreview: (value: number) => void;
  /** The release of a drag or a key: the one commit of what was previewed. */
  onRelease: () => void;
  /** A click or a typed value: set and commit in one go. */
  onSet: (value: number) => void;
  /** Escape during a drag: drop what was previewed. `start` is the value at the press. */
  onCancel: (start: number) => void;
  /** After an entry opened with the pointer closes: the keyboard goes to the canvas. */
  onReturnFocus: () => void;
}

/**
 * The slider of the inner ratio of a star (`specs/0047-transform-polish/` Part B, S1 in the
 * Select bar and S2 in the Polygon/Star bar): a click on the track sets the value at the
 * pointer, a drag moves the thumb as before, a double-click (or Enter or F2) turns the
 * readout beside it into a text field. The behaviour is `useSliderPointer`'s, the same as
 * the colour sliders'. The range, the rounding and the rule of the typed value are
 * `lib/sliderValue.ts`.
 */
export function RatioSlider({
  label,
  title,
  value,
  mixed = false,
  onPreview,
  onRelease,
  onSet,
  onCancel,
  onReturnFocus,
}: RatioSliderProps) {
  const track = useRef<HTMLDivElement>(null);
  const [entry, setEntry] = useState<"pointer" | "keyboard" | null>(null);
  const start = useRef(value);
  const cancelled = useRef(false);
  /** The primary button is down on the slider. */
  const pressed = useRef(false);
  const keyRun = useRef(false);
  const width = WIDTH - 2 * INSET;
  const shown = mixed ? 0.5 : value;

  const pointer = useSliderPointer({
    thumb: () =>
      mixed
        ? null
        : {
            at: {
              x: INSET + positionOfValue(value, width, RATIO_RANGE),
              y: TARGET_HEIGHT / 2,
            },
            radius: THUMB / 2,
          },
    onClick: (at) => onSet(valueAtPosition(at.x - INSET, width, RATIO_RANGE)),
    onDrag: (at) => {
      if (!cancelled.current) {
        onPreview(valueAtPosition(at.x - INSET, width, RATIO_RANGE));
      }
    },
    onDragEnd: () => {
      const wasCancelled = cancelled.current;
      cancelled.current = false;
      if (!wasCancelled) {
        onRelease();
      }
    },
    onPointerCancel: () => {
      pressed.current = false;
      cancelled.current = false;
      onCancel(start.current);
    },
    onDoubleClick: () => setEntry("pointer"),
  });

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Enter" || event.key === "F2") {
      event.preventDefault();
      setEntry("keyboard");
      return;
    }
    if (event.key === "Escape" && pressed.current && !cancelled.current) {
      // Escape in the middle of a drag drops the preview; the release writes nothing.
      event.preventDefault();
      cancelled.current = true;
      onCancel(start.current);
      return;
    }
    const { min, max, step } = RATIO_RANGE;
    const by: Record<string, number> = {
      ArrowLeft: shown - step,
      ArrowDown: shown - step,
      ArrowRight: shown + step,
      ArrowUp: shown + step,
      PageDown: shown - 10 * step,
      PageUp: shown + 10 * step,
      Home: min,
      End: max,
    };
    const next = by[event.key];
    if (next !== undefined) {
      event.preventDefault();
      keyRun.current = true;
      onPreview(snapToStep(next, RATIO_RANGE));
    }
  };

  const endKeyRun = () => {
    if (keyRun.current) {
      keyRun.current = false;
      onRelease();
    }
  };

  const submit = (text: string): string => {
    const parsed = parseRatioEntry(text);
    if (!parsed.ok) {
      return "invalid:range";
    }
    if (!mixed && parsed.value === value) {
      return "unchanged";
    }
    onSet(parsed.value);
    return "committed";
  };

  const thumbLeft = INSET + positionOfValue(value, width, RATIO_RANGE);
  const trackColour =
    "color-mix(in srgb, var(--toolbar-icon) 35%, transparent)";

  return (
    <>
      <div
        ref={track}
        role="slider"
        tabIndex={0}
        aria-label={label}
        aria-valuemin={RATIO_RANGE.min}
        aria-valuemax={RATIO_RANGE.max}
        aria-valuenow={mixed ? undefined : value}
        aria-valuetext={mixed ? "Mixed" : value.toFixed(2)}
        aria-keyshortcuts="Enter F2"
        title={title}
        className="relative shrink-0 cursor-pointer rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        style={{
          width: WIDTH,
          height: TARGET_HEIGHT,
          touchAction: "manipulation",
        }}
        onPointerDown={(event) => {
          start.current = shown;
          cancelled.current = false;
          pressed.current = event.button === 0;
          pointer.onPointerDown(event);
        }}
        onPointerMove={pointer.onPointerMove}
        onPointerUp={(event) => {
          pressed.current = false;
          pointer.onPointerUp(event);
        }}
        onPointerCancel={pointer.onPointerCancel}
        onKeyDown={onKeyDown}
        onKeyUp={(event) => {
          if (STEP_KEYS.has(event.key)) {
            endKeyRun();
          }
        }}
        onBlur={endKeyRun}
      >
        <span
          aria-hidden
          className="pointer-events-none absolute top-1/2 h-1 -translate-y-1/2 rounded-full"
          style={{ left: INSET, right: INSET, background: trackColour }}
        />
        {!mixed && (
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 h-1 -translate-y-1/2 rounded-full bg-[var(--editor-accent)]"
            style={{ left: INSET, width: thumbLeft - INSET }}
          />
        )}
        {!mixed && (
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 rounded-full border-2 border-[var(--editor-accent)] bg-white"
            style={{
              width: THUMB,
              height: THUMB,
              left: thumbLeft - THUMB / 2,
              marginTop: -THUMB / 2,
            }}
          />
        )}
      </div>
      {entry ? (
        <EntryField
          label={label}
          shown={value.toFixed(2)}
          mixed={mixed}
          width={44}
          autoFocus
          inputMode="decimal"
          messages={{ range: RATIO_MESSAGE }}
          onSubmit={submit}
          onClose={() => setEntry(null)}
          onReturnFocus={() =>
            entry === "keyboard" ? track.current?.focus() : onReturnFocus()
          }
        />
      ) : (
        <span className="w-11 text-right tabular-nums">
          {mixed ? "Mixed" : value.toFixed(2)}
        </span>
      )}
    </>
  );
}

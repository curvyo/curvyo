// The font preview's Size slider (`specs/0077-font-preview` criterion 12, UX notes §2): the Ratio
// slider's look and pointer behaviour (`useSliderPointer`: click sets, drag moves, double-click
// types), 16 to 160 px of cap height; the tooltip is the name and value only.
import { useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { Tooltip } from "@/components/ui/tooltip";
import { useSliderPointer } from "@/hooks/useSliderPointer";
import { DEFAULT_SIZE, SIZE_RANGE, clampSize } from "@/lib/fontPreviewState";
import { sizeTip } from "@/lib/fontPreviewText";
import { positionOfValue, valueAtPosition } from "@/lib/sliderValue";

const THUMB = 14;
const INSET = THUMB / 2 + 1;
const TARGET_HEIGHT = 24;
/** A slider's range, default and arrow steps; the font preview's unless given. */
export interface SizeScale {
  min: number;
  max: number;
  fallback: number;
  step: number;
  bigStep: number;
}

const FONT_SCALE: SizeScale = { min: SIZE_RANGE.min, max: SIZE_RANGE.max, fallback: DEFAULT_SIZE, step: 4, bigStep: 16 };

interface SizeSliderProps {
  /** The range, default and steps (the library browser's 48 to 256, `specs/0152` criterion 4). */
  scale?: SizeScale;
  size: number;
  /** The slider's width in px (120 in one header row, flexible in two). */
  width: number;
  onSize: (size: number) => void;
  onReturnFocus: () => void;
}

export function SizeSlider({ scale = FONT_SCALE, size, width, onSize, onReturnFocus }: SizeSliderProps) {
  const range = { min: scale.min, max: scale.max, step: 1 };
  const clamp = (value: number) =>
    scale === FONT_SCALE
      ? clampSize(value)
      : Number.isFinite(value)
        ? Math.min(scale.max, Math.max(scale.min, Math.round(value)))
        : scale.fallback;
  const message = `Enter a size from ${scale.min} to ${scale.max} px.`;
  const track = useRef<HTMLDivElement>(null);
  const [entry, setEntry] = useState<"pointer" | "keyboard" | null>(null);
  const start = useRef(size);
  const inner = Math.max(1, width - 2 * INSET);
  const thumbLeft = INSET + positionOfValue(size, inner, range);

  const pointer = useSliderPointer({
    thumb: () => ({ at: { x: thumbLeft, y: TARGET_HEIGHT / 2 }, radius: THUMB / 2 }),
    onClick: (at) => onSize(clamp(valueAtPosition(at.x - INSET, inner, range))),
    onDrag: (at) => onSize(clamp(valueAtPosition(at.x - INSET, inner, range))),
    onDragEnd: () => {},
    onPointerCancel: () => onSize(start.current),
    onEscape: () => onSize(start.current),
    onDoubleClick: () => setEntry("pointer"),
  });

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Enter" || event.key === "F2") {
      event.preventDefault();
      setEntry("keyboard");
      return;
    }
    if (event.key === "Backspace" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      onSize(scale.fallback);
      return;
    }
    const step = event.shiftKey ? scale.bigStep : scale.step;
    const by: Record<string, number> = {
      ArrowLeft: size - step,
      ArrowDown: size - step,
      ArrowRight: size + step,
      ArrowUp: size + step,
      PageDown: size - scale.bigStep,
      PageUp: size + scale.bigStep,
      Home: scale.min,
      End: scale.max,
    };
    const next = by[event.key];
    if (next !== undefined) {
      event.preventDefault();
      onSize(clamp(next));
    }
  };

  const submit = (text: string): string => {
    const typed = Number(text.replace(/px$/i, "").trim().replace(",", "."));
    if (!Number.isFinite(typed) || typed < scale.min || typed > scale.max) {
      return "invalid:range";
    }
    if (Math.round(typed) === size) {
      return "unchanged";
    }
    onSize(clamp(typed));
    return "committed";
  };

  return (
    <>
      <Tooltip side="bottom" content={sizeTip(size)}>
        <div
          ref={track}
          role="slider"
          tabIndex={0}
          aria-label="Size"
          aria-valuemin={scale.min}
          aria-valuemax={scale.max}
          aria-valuenow={size}
          aria-valuetext={`${size} pixels`}
          aria-keyshortcuts="Enter F2"
          className="relative shrink-0 cursor-pointer rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          style={{ width, height: TARGET_HEIGHT, touchAction: "manipulation" }}
          onPointerDown={(event) => {
            start.current = size;
            pointer.onPointerDown(event);
          }}
          onPointerMove={pointer.onPointerMove}
          onPointerUp={pointer.onPointerUp}
          onPointerCancel={pointer.onPointerCancel}
          onKeyDown={onKeyDown}
        >
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 h-1 -translate-y-1/2 rounded-full"
            style={{ left: INSET, right: INSET, background: "color-mix(in srgb, var(--toolbar-icon) 35%, transparent)" }}
          />
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 h-1 -translate-y-1/2 rounded-full bg-[var(--editor-accent)]"
            style={{ left: INSET, width: thumbLeft - INSET }}
          />
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 rounded-full border-2 border-[var(--editor-accent)] bg-white"
            style={{ width: THUMB, height: THUMB, left: thumbLeft - THUMB / 2, marginTop: -THUMB / 2 }}
          />
        </div>
      </Tooltip>
      {entry ? (
        <EntryField
          label="Size"
          shown={String(size)}
          mixed={false}
          width={44}
          autoFocus
          inputMode="decimal"
          messages={{ range: message }}
          onSubmit={submit}
          onClose={() => setEntry(null)}
          onReturnFocus={() => (entry === "keyboard" ? track.current?.focus() : onReturnFocus())}
        />
      ) : (
        <span className="w-11 shrink-0 text-right text-xs tabular-nums text-[var(--toolbar-icon)]">{size} px</span>
      )}
    </>
  );
}

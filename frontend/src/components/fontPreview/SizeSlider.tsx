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
const SIZE_MESSAGE = "Enter a size from 16 to 160 px.";

interface SizeSliderProps {
  size: number;
  /** The slider's width in px (120 in one header row, flexible in two). */
  width: number;
  onSize: (size: number) => void;
  onReturnFocus: () => void;
}

export function SizeSlider({ size, width, onSize, onReturnFocus }: SizeSliderProps) {
  const track = useRef<HTMLDivElement>(null);
  const [entry, setEntry] = useState<"pointer" | "keyboard" | null>(null);
  const start = useRef(size);
  const inner = Math.max(1, width - 2 * INSET);
  const thumbLeft = INSET + positionOfValue(size, inner, SIZE_RANGE);

  const pointer = useSliderPointer({
    thumb: () => ({ at: { x: thumbLeft, y: TARGET_HEIGHT / 2 }, radius: THUMB / 2 }),
    onClick: (at) => onSize(clampSize(valueAtPosition(at.x - INSET, inner, SIZE_RANGE))),
    onDrag: (at) => onSize(clampSize(valueAtPosition(at.x - INSET, inner, SIZE_RANGE))),
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
      onSize(DEFAULT_SIZE);
      return;
    }
    const step = event.shiftKey ? 16 : 4;
    const by: Record<string, number> = {
      ArrowLeft: size - step,
      ArrowDown: size - step,
      ArrowRight: size + step,
      ArrowUp: size + step,
      PageDown: size - 16,
      PageUp: size + 16,
      Home: SIZE_RANGE.min,
      End: SIZE_RANGE.max,
    };
    const next = by[event.key];
    if (next !== undefined) {
      event.preventDefault();
      onSize(clampSize(next));
    }
  };

  const submit = (text: string): string => {
    const typed = Number(text.replace(/px$/i, "").trim().replace(",", "."));
    if (!Number.isFinite(typed) || typed < SIZE_RANGE.min || typed > SIZE_RANGE.max) {
      return "invalid:range";
    }
    if (Math.round(typed) === size) {
      return "unchanged";
    }
    onSize(clampSize(typed));
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
          aria-valuemin={SIZE_RANGE.min}
          aria-valuemax={SIZE_RANGE.max}
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
          messages={{ range: SIZE_MESSAGE }}
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

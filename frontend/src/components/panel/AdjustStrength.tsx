import { useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { useSliderPointer } from "@/hooks/useSliderPointer";
import {
  STRENGTH_MESSAGE,
  parseStrength,
  strengthAfterKey,
  strengthAtPosition,
  strengthPosition,
} from "@/lib/adjustText";
import { SLIDER_INSET_PX, SLIDER_THUMB_PX, SliderMarks } from "@/components/ui/slider";

/** The slider's width in px (`ux-notes.md` §3.3: with the 8 px gap and the 112 px field it fills
 * the 256 px column) and the inset that keeps the thumb inside it. */
const WIDTH = 136;
const THUMB = SLIDER_THUMB_PX;
const INSET = SLIDER_INSET_PX;
/** The pointer target is 24 px high around a 4 px track. */
const TARGET_HEIGHT = 24;
const STEP_KEYS = new Set(["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End"]);

interface StrengthProps {
  value: number;
  onHold: (value: number) => void;
  onRelease: () => void;
  onCancel: () => void;
  /** A click or a typed value: the refusal text, empty when taken. */
  onSet: (value: number) => string;
  onReturnFocus: () => void;
}

/**
 * The Strength row of the Adjustments group, the panel's one slider (`ux-notes.md` §3.3; design
 * system "Image adjustments"): a 136 px slider with its percent field, "Gentle" and "Strong"
 * under its ends. A drag or a key run is held (shown, not written) and released as one step;
 * Escape or a cancelled pointer drops it. No tooltip: the field shows the value.
 */
export function AdjustStrength({ value, onHold, onRelease, onCancel, onSet, onReturnFocus }: StrengthProps) {
  const track = useRef<HTMLDivElement>(null);
  const keyRun = useRef(false);
  const [held, setHeld] = useState<number | null>(null);
  const shown = held ?? value;
  const width = WIDTH - 2 * INSET;

  const hold = (next: number) => {
    if (next === shown && held !== null) return;
    setHeld(next);
    onHold(next);
  };
  const release = () => {
    setHeld(null);
    onRelease();
  };
  const cancel = () => {
    keyRun.current = false;
    setHeld(null);
    onCancel();
  };

  const pointer = useSliderPointer({
    thumb: () => ({ at: { x: INSET + strengthPosition(shown, width), y: TARGET_HEIGHT / 2 }, radius: THUMB / 2 }),
    onClick: (at) => onSet(strengthAtPosition(at.x - INSET, width)),
    onDrag: (at) => hold(strengthAtPosition(at.x - INSET, width)),
    onDragEnd: release,
    onPointerCancel: cancel,
    onEscape: cancel,
    onDoubleClick: () => undefined,
  });

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape" && keyRun.current) {
      event.preventDefault();
      event.stopPropagation();
      cancel();
      return;
    }
    const next = strengthAfterKey(shown, event.key);
    if (next === null) return;
    event.preventDefault();
    keyRun.current = true;
    hold(next);
  };
  const endKeyRun = () => {
    if (keyRun.current) {
      keyRun.current = false;
      release();
    }
  };

  const submit = (text: string): string => {
    const parsed = parseStrength(text);
    if (parsed === null) return "invalid:range";
    if (parsed === value) return "unchanged";
    return onSet(parsed) === "" ? "committed" : "invalid:range";
  };

  const thumbLeft = INSET + strengthPosition(shown, width);
  return (
    <div data-adjust-strength="" className="flex flex-col">
      <div className="flex h-7 items-center gap-2">
        <div
          ref={track}
          role="slider"
          tabIndex={0}
          aria-label="Strength"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={shown}
          aria-valuetext={`${shown} %`}
          className="group relative shrink-0 cursor-pointer rounded-[5px] outline-none"
          style={{ width: WIDTH, height: TARGET_HEIGHT, touchAction: "manipulation" }}
          onPointerDown={pointer.onPointerDown}
          onPointerMove={pointer.onPointerMove}
          onPointerUp={pointer.onPointerUp}
          onPointerCancel={pointer.onPointerCancel}
          onKeyDown={onKeyDown}
          onKeyUp={(event) => {
            if (STEP_KEYS.has(event.key)) endKeyRun();
          }}
          onBlur={endKeyRun}
        >
          <SliderMarks thumbLeft={thumbLeft} />
        </div>
        <div className="shrink-0">
          <EntryField
            label="Strength, percent"
            prefix="Strength"
            shown={String(shown)}
            mixed={false}
            suffix="%"
            width={112}
            commitOnBlur
            inputMode="decimal"
            messages={{ range: STRENGTH_MESSAGE }}
            onSubmit={submit}
            stepText={(text, key, shift) => {
              if (key !== "ArrowUp" && key !== "ArrowDown") return null;
              const typed = Number(text.trim());
              if (text.trim() === "" || !Number.isFinite(typed)) return null;
              return String(Math.min(100, Math.max(0, typed + (key === "ArrowUp" ? 1 : -1) * (shift ? 10 : 1))));
            }}
            onReturnFocus={onReturnFocus}
          />
        </div>
      </div>
      <div aria-hidden className="relative h-4 text-xs text-[var(--panel-muted-fg)]" style={{ width: WIDTH }}>
        <span className="absolute left-0">Gentle</span>
        <span className="absolute right-0">Strong</span>
      </div>
    </div>
  );
}

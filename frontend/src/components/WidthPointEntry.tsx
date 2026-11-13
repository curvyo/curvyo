import { type RefObject, useEffect, useLayoutEffect, useRef, useState } from "react";

import { type Placement, placeWidthChip } from "@/lib/readoutPlacement";

type ContainerRef = RefObject<HTMLDivElement | null>;

type EntryProps = {
  onCommit: (mm: number) => boolean;
  onClose: () => void;
  containerRef: ContainerRef;
};

/** A chip beside a width point's diamond: the typed width, or the wheel readout. */
export interface WidthChipAt {
  /** Canvas-relative CSS pixels of the diamond. */
  x: number;
  y: number;
  /** The width point's full width in millimetres. */
  mm: number;
  /** Changes on every opening. */
  id: number;
}

export type WidthEntry = WidthChipAt;

const CHIP_STYLE = {
  background: "var(--toolbar-bg)",
  boxShadow: "var(--panel-elevation-shadow)",
  color: "var(--toolbar-icon)",
} as const;

/** Where the chip in `chipRef` goes beside the diamond at `at`, measured again whenever the
 * diamond moves or the chip's `content` changes its size. */
function useWidthChipPlacement(
  chipRef: RefObject<HTMLDivElement | null>,
  containerRef: ContainerRef,
  x: number,
  y: number,
  side: "below" | "above",
  content: unknown,
): Placement {
  const [placement, setPlacement] = useState<Placement>({ left: x, top: y });
  useLayoutEffect(() => {
    const chip = chipRef.current;
    const container = containerRef.current;
    if (!chip || !container) return;
    const next = placeWidthChip(
      { x, y },
      { width: chip.offsetWidth, height: chip.offsetHeight },
      { width: container.clientWidth, height: container.clientHeight },
      side,
    );
    setPlacement((previous) =>
      previous.left === next.left && previous.top === next.top ? previous : next,
    );
  }, [chipRef, containerRef, x, y, side, content]);
  return placement;
}

/** A width as the field shows it: up to three decimals, no trailing zeros. */
function formatMm(mm: number): string {
  return Number.isFinite(mm) ? String(Number(mm.toFixed(3))) : "";
}

/**
 * The typed width of a width point (`specs/0115` criterion 6): a small field below the
 * double-clicked diamond (above it without room, UX review 2026-11-12), in the entry chip look,
 * with the width in mm selected. Enter sets it (one undo step), Escape or leaving the field
 * cancels; a value that is not a width of 0 mm or more stays open and says so under the field.
 */
export function WidthPointEntry({ entry, ...props }: EntryProps & { entry: WidthEntry | null }) {
  // A new opening is a new field: its text starts from the point's width.
  return entry ? <EntryField key={entry.id} entry={entry} {...props} /> : null;
}

function EntryField({ entry, onCommit, onClose, containerRef }: EntryProps & { entry: WidthEntry }) {
  const inputRef = useRef<HTMLInputElement>(null);
  const chipRef = useRef<HTMLDivElement>(null);
  const [text, setText] = useState(() => formatMm(entry.mm));
  const [invalid, setInvalid] = useState(false);

  useEffect(() => {
    const frame = requestAnimationFrame(() => inputRef.current?.select());
    return () => cancelAnimationFrame(frame);
  }, []);
  const placement = useWidthChipPlacement(chipRef, containerRef, entry.x, entry.y, "below", invalid);

  return (
    <div
      ref={chipRef}
      data-width-entry
      className="pointer-events-auto absolute z-20 flex flex-col gap-0.5 rounded-md px-2 py-1 text-xs whitespace-nowrap"
      style={{ ...placement, ...CHIP_STYLE }}
      onPointerDown={(event) => event.stopPropagation()}
    >
      <label className="flex items-center gap-1">
        Width
        <input
          ref={inputRef}
          aria-label="Width at this point, millimetres"
          aria-invalid={invalid}
          inputMode="decimal"
          className="h-6 w-16 rounded border bg-white px-1 text-right"
          style={{ borderColor: invalid ? "var(--destructive, #B3261E)" : "var(--panel-border, #B4B4B8)" }}
          value={text}
          onChange={(event) => {
            setText(event.target.value);
            setInvalid(false);
          }}
          onKeyDown={(event) => {
            event.stopPropagation();
            if (event.key === "Enter") {
              const mm = Number(text.trim().replace(",", "."));
              if (text.trim() === "" || !onCommit(mm)) setInvalid(true);
            } else if (event.key === "Escape") {
              onClose();
            }
          }}
          onBlur={onClose}
        />
        mm
      </label>
      {invalid ? (
        <span role="alert" style={{ color: "var(--destructive, #B3261E)" }}>
          Must be 0 or more.
        </span>
      ) : null}
    </div>
  );
}

/**
 * The live width while the wheel turns over a width point (`specs/0115` criterion 7, UX review
 * 2026-11-12): "Width 2.5 mm" above its diamond in the entry chip look; the host drops it shortly
 * after the last notch.
 */
export function WidthWheelReadout({
  readout,
  containerRef,
}: {
  readout: WidthChipAt | null;
  containerRef: ContainerRef;
}) {
  const chipRef = useRef<HTMLDivElement>(null);
  const at = readout ?? { x: 0, y: 0 };
  const placement = useWidthChipPlacement(chipRef, containerRef, at.x, at.y, "above", readout?.mm);
  if (!readout) return null;
  return (
    <div
      ref={chipRef}
      data-width-wheel-readout
      role="status"
      aria-live="polite"
      className="pointer-events-none absolute z-20 rounded-md px-2 py-1 text-xs whitespace-nowrap tabular-nums"
      style={{ ...placement, ...CHIP_STYLE }}
    >
      Width {formatMm(readout.mm)} mm
    </div>
  );
}

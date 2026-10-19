import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";

import { EntryField } from "@/components/EntryField";
import type { PenEntryApi, PenEntryChipState } from "@/hooks/usePenEntry";
import { placePenEntryChip } from "@/lib/readoutPlacement";

/** Field widths and label paddings (`docs/design-system.md`, "Pen entry chip"): each field keeps
 * 72px for the text, room for 9 digits at 14px (criterion 33). */
const FIELD_PX = [154, 150];
const LABEL_PX = [48, 44];
/** The tool rail's clearance from the canvas's left edge, as the other entry chips keep it. */
const TOOL_RAIL_CLEAR_PX = 128;
/** The modifier keys reach the window listener, so Ctrl stays tracked while a field has focus. */
const MODIFIER_KEYS = new Set(["Shift", "Control", "Meta", "Alt"]);

interface PenEntryChipProps {
  penEntry: PenEntryApi;
  chip: PenEntryChipState;
  containerRef: React.RefObject<HTMLDivElement | null>;
}

/**
 * The Pen entry chip (`specs/0072-pen-angle-snap-and-typed-values/` criteria 12 to 23): the
 * length and the absolute angle of the next segment. Rust holds the values and the rules; this
 * component holds the text focus and forwards each edit, Enter and Escape. The placement chip's
 * surface and fields; rendered inside the canvas container so modifier tracking keeps working.
 */
export function PenEntryChip({ penEntry, chip, containerRef }: PenEntryChipProps) {
  const chipRef = useRef<HTMLDivElement>(null);
  const inputs = useRef<(HTMLInputElement | null)[]>([]);
  const messageId = useId();
  const [sizes, setSizes] = useState({
    chip: { width: 0, height: 0 },
    canvas: { width: Number.POSITIVE_INFINITY, height: Number.POSITIVE_INFINITY },
  });

  useLayoutEffect(() => {
    const element = chipRef.current;
    const container = containerRef.current;
    if (!element || !container) {
      return;
    }
    const next = {
      chip: { width: element.offsetWidth, height: element.offsetHeight },
      canvas: { width: container.clientWidth, height: container.clientHeight },
    };
    setSizes((previous) => (JSON.stringify(previous) === JSON.stringify(next) ? previous : next));
  }, [chip, containerRef]);

  // Length has the focus and its text selected in the frame the chip opens (criterion 12), and
  // again after a refused Enter (criterion 26).
  const { refused } = penEntry;
  useEffect(() => {
    inputs.current[0]?.focus();
    inputs.current[0]?.select();
  }, [refused]);

  // Until the first edit each pointer move rewrites the focused field's live value; it stays
  // selected so the first key replaces it.
  const liveTexts = chip.frozen ? "" : chip.fields.map((field) => field.text).join("|");
  useLayoutEffect(() => {
    if (!liveTexts) {
      return;
    }
    const focused = document.activeElement;
    if (focused instanceof HTMLInputElement && inputs.current.includes(focused)) {
      focused.select();
    }
  }, [liveTexts]);

  const count = chip.fields.length;
  const onKeyDown = (index: number) => (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (MODIFIER_KEYS.has(event.key)) {
      return;
    }
    if (event.key === "Tab") {
      // Tab and Shift+Tab cycle inside the chip and do not freeze it (criterion 18).
      event.preventDefault();
      const next = (index + (event.shiftKey ? count - 1 : 1)) % count;
      inputs.current[next]?.focus();
      inputs.current[next]?.select();
    } else if (event.key === "Enter") {
      event.preventDefault();
      penEntry.place();
    } else if (event.key === "Escape") {
      event.preventDefault();
      penEntry.cancel();
    }
    // Every other key stays in the field (criterion 23).
    event.stopPropagation();
  };

  // Frozen, the chip also keeps off the segment to the typed point (UX review 0072).
  const at = placePenEntryChip(
    chip.pointer,
    chip.last,
    chip.frozen ? chip.end : null,
    sizes.chip,
    sizes.canvas,
    TOOL_RAIL_CLEAR_PX,
  );

  return (
    <div
      ref={chipRef}
      role="group"
      aria-label={chip.groupName}
      data-pen-entry-chip
      className="absolute z-30 rounded-[8px] p-1.5"
      style={{
        left: at.left,
        top: at.top,
        background: "var(--toolbar-bg)",
        boxShadow: "var(--panel-elevation-shadow)",
        color: "var(--toolbar-icon)",
        // Before the first edit the chip moves with the pointer: a press under it is a canvas
        // press.
        pointerEvents: chip.frozen ? undefined : "none",
      }}
    >
      <div className="flex items-center gap-1">
        {chip.fields.map((field, index) => (
          <EntryField
            key={field.label}
            label={field.label}
            width={FIELD_PX[index]}
            labelWidth={LABEL_PX[index]}
            suffix={field.suffix}
            ariaLabel={field.name}
            invalid={false}
            messageId={messageId}
            value={field.text}
            inputRef={(node) => {
              inputs.current[index] = node;
            }}
            onChange={(text) => penEntry.edit(index, text)}
            onKeyDown={onKeyDown(index)}
            onFocus={(event) => event.target.select()}
          />
        ))}
      </div>
    </div>
  );
}

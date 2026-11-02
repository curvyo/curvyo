import { CircleAlert } from "lucide-react";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";

import { EntryField } from "@/components/EntryField";
import type { TilingApi } from "@/hooks/useTiling";
import { placeEntryChip, placeMoveEntryChip } from "@/lib/readoutPlacement";
import { ENTER_NUMBER, ENTRY_FIELDS, type TilingEntry, type TilingRecipe, entryText, readField } from "@/lib/tilingText";

/** The tool rail's clearance from the canvas's left edge, px (`--rail-right` + 12). */
const TOOL_RAIL_CLEAR_PX = 128;
/** Half the circle handles' glyph (the 12 px centre ring), px. */
const HANDLE_GLYPH_RADIUS_PX = 6;

interface TilingEntryChipProps {
  entry: TilingEntry;
  recipe: TilingRecipe;
  tiling: TilingApi;
  /** The canvas container: the chip's coordinate space. */
  containerRef: React.RefObject<HTMLDivElement | null>;
}

/**
 * The entry box of a circle handle of the tiling (`specs/0069-tiling` criterion 14, UX notes
 * §12.4; the "Transform entry chip" surface with `EntryField` and the message card of the move
 * chip): Centre X and Y, Radius, Start or Sweep, prefilled with the value it opens with, the
 * first field focused with its text selected. Every valid keystroke reaches the preview
 * (debounced). Enter keeps the values and closes it, Escape, a press elsewhere or a blur restore
 * the values it opened with; neither applies nor cancels the tiling. A refused value keeps it open
 * with the field marked and the message card.
 */
export function TilingEntryChip({ entry, recipe, tiling, containerRef }: TilingEntryChipProps) {
  const fields = ENTRY_FIELDS[entry.handle];
  const chipRef = useRef<HTMLDivElement>(null);
  const inputRefs = useRef<Array<HTMLInputElement | null>>([]);
  const messageId = useId();
  const [texts, setTexts] = useState(() => fields.map((field) => entryText(recipe[field.key])));
  const [invalid, setInvalid] = useState<{ index: number; message: string } | null>(null);
  const [sizes, setSizes] = useState({
    chip: { width: 0, height: 0 },
    canvas: { width: Number.POSITIVE_INFINITY, height: Number.POSITIVE_INFINITY },
  });

  useEffect(() => {
    inputRefs.current[0]?.focus();
    inputRefs.current[0]?.select();
  }, []);

  useLayoutEffect(() => {
    const chip = chipRef.current;
    const container = containerRef.current;
    if (!chip || !container) {
      return;
    }
    const next = {
      chip: { width: chip.offsetWidth, height: chip.offsetHeight },
      canvas: { width: container.clientWidth, height: container.clientHeight },
    };
    setSizes((previous) =>
      previous.chip.width === next.chip.width &&
      previous.chip.height === next.chip.height &&
      previous.canvas.width === next.canvas.width &&
      previous.canvas.height === next.canvas.height
        ? previous
        : next,
    );
  }, [entry, invalid, containerRef]);

  const handle = { x: entry.x, y: entry.y };
  const placed =
    entry.handle === "centre"
      ? placeMoveEntryChip({ center: handle, boxBottomY: entry.y }, sizes.chip, sizes.canvas, sizes.chip.height)
      : placeEntryChip(handle, { x: entry.cx, y: entry.cy }, sizes.chip, sizes.canvas, HANDLE_GLYPH_RADIUS_PX);
  const left = Math.max(placed.left, TOOL_RAIL_CLEAR_PX);
  const roomBelow = sizes.canvas.height - (placed.top + sizes.chip.height) >= 28;

  const mark = (index: number, message: string) => {
    setInvalid({ index, message });
    inputRefs.current[index]?.focus();
    inputRefs.current[index]?.select();
  };

  const onChange = (index: number, text: string) => {
    setTexts((previous) => previous.map((old, i) => (i === index ? text : old)));
    setInvalid(null);
    const field = fields[index];
    const result = readField(text, field.rule);
    if ("value" in result) {
      tiling.type({ [field.key]: result.value }, () => mark(index, ENTER_NUMBER));
    }
  };

  const submit = () => {
    for (const [index, field] of fields.entries()) {
      const result = readField(texts[index] ?? "", field.rule);
      if ("error" in result) {
        mark(index, result.error);
        return;
      }
    }
    if (!tiling.closeEntry(true)) {
      mark(0, ENTER_NUMBER);
    }
  };

  const onKeyDown = (index: number, event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      submit();
    } else if (event.key === "Escape") {
      event.preventDefault();
      tiling.closeEntry(false);
    } else if (event.key === "Tab") {
      // The box is a closed loop: Tab never leaves it (and closes it by a blur).
      event.preventDefault();
      const step = event.shiftKey ? fields.length - 1 : 1;
      const target = inputRefs.current[(index + step) % fields.length];
      target?.focus();
      target?.select();
    }
  };

  const onBlur = (event: React.FocusEvent<HTMLElement>) => {
    const next = event.relatedTarget;
    if (next instanceof Node && chipRef.current?.contains(next)) {
      return;
    }
    tiling.closeEntry(false);
  };

  return (
    <div
      ref={chipRef}
      role="group"
      aria-label={fields.length > 1 ? "Centre" : fields[0]?.name}
      data-tiling-entry=""
      className="absolute z-30 rounded-[8px] p-1.5 outline-none"
      style={{
        left,
        top: placed.top,
        background: "var(--toolbar-bg)",
        boxShadow: "var(--panel-elevation-shadow)",
        color: "var(--toolbar-icon)",
      }}
    >
      <div className="flex items-center gap-1">
        {fields.map((field, index) => (
          <EntryField
            key={field.key}
            label={field.label}
            width={field.width}
            labelWidth={field.labelWidth}
            suffix={field.suffix}
            ariaLabel={field.name}
            invalid={invalid?.index === index}
            messageId={messageId}
            value={texts[index] ?? ""}
            inputRef={(node) => {
              inputRefs.current[index] = node;
            }}
            onChange={(text) => onChange(index, text)}
            onKeyDown={(event) => onKeyDown(index, event)}
            onBlur={onBlur}
          />
        ))}
      </div>
      <div
        id={messageId}
        role="status"
        aria-live="polite"
        className={
          invalid !== null
            ? `absolute left-0 flex w-max min-w-full items-center gap-1 whitespace-nowrap rounded-[8px] px-1.5 py-1 text-xs ${
                roomBelow ? "top-full mt-1" : "bottom-full mb-1"
              }`
            : "absolute h-0 overflow-hidden text-xs"
        }
        style={invalid !== null ? { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" } : undefined}
      >
        {invalid !== null && (
          <>
            <CircleAlert aria-hidden className="size-3 shrink-0" />
            <span>{invalid.message}</span>
          </>
        )}
      </div>
    </div>
  );
}

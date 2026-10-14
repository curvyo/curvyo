import { CircleAlert, Link2 } from "lucide-react";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";

import { EntryField } from "@/components/EntryField";
import type { PlacementApi, PlacementChipState } from "@/hooks/usePlacement";
import { type Rect, placePlacementChip } from "@/lib/readoutPlacement";

/** Field widths and label paddings (`docs/design-system.md`, "Placement chip"). */
const LENGTH_FIELD_PX = 132;
const ANGLE_FIELD_PX = 132;
const ANGLE_LABEL_PX = 44;
const SIZE_LABEL_PX = 28;
/** The tool rail's clearance from the canvas's left edge, as the other entry chips keep it. */
const TOOL_RAIL_CLEAR_PX = 128;
/** The slot between W and H, reserved so H never moves when the link glyph shows. */
const LINK_SLOT_PX = 16;

/** The active tool's bar (`data-placement-keeps`) in the canvas's own coordinates, or `null`. */
function measureToolBar(container: HTMLElement): Rect | null {
  const bar = document.querySelector("[data-placement-keeps]");
  if (!bar) {
    return null;
  }
  const outer = container.getBoundingClientRect();
  const box = bar.getBoundingClientRect();
  return {
    left: box.left - outer.left,
    top: box.top - outer.top,
    right: box.right - outer.left,
    bottom: box.bottom - outer.top,
  };
}

interface PlacementChipProps {
  placement: PlacementApi;
  chip: PlacementChipState;
  containerRef: React.RefObject<HTMLDivElement | null>;
}

/**
 * The click-to-create placement chip (`specs/0052-click-to-create-shapes/` criteria 10 to 30):
 * the size and the angle typed while the shape follows the pointer. Rust holds the values and
 * the rules; this component holds the text focus and forwards each edit, Space, Enter and
 * Escape. It is rendered inside the canvas container so modifier tracking keeps working.
 */
export function PlacementChip({ placement, chip, containerRef }: PlacementChipProps) {
  const chipRef = useRef<HTMLDivElement>(null);
  const inputs = useRef<(HTMLInputElement | null)[]>([]);
  const messageId = useId();
  const [sizes, setSizes] = useState({
    chip: { width: 0, height: 0 },
    canvas: { width: Number.POSITIVE_INFINITY, height: Number.POSITIVE_INFINITY },
    bar: null as Rect | null,
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
      bar: measureToolBar(container),
    };
    setSizes((previous) => (JSON.stringify(previous) === JSON.stringify(next) ? previous : next));
  }, [chip, containerRef]);

  // The first field has the focus and its text selected in the frame the placement starts
  // (criterion 11); on touch only when a field is tapped (criterion 31).
  const { touchStarted } = placement;
  useEffect(() => {
    if (!touchStarted) {
      inputs.current[0]?.focus();
      inputs.current[0]?.select();
    }
  }, [touchStarted]);

  // Until the first edit each pointer move rewrites the focused field's live value; it stays
  // selected so the first key replaces it (UX review must-fix 1).
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

  // A refusal puts the focus on the first invalid field with its text selected (criterion 23).
  const { errorField } = chip;
  useEffect(() => {
    if (errorField >= 0) {
      inputs.current[errorField]?.focus();
      inputs.current[errorField]?.select();
    }
  }, [errorField]);

  const count = chip.fields.length;
  const sizeField = chip.fields.some((field) => field.size);
  const onKeyDown = (index: number) => (event: React.KeyboardEvent<HTMLInputElement>) => {
    // This handler stops the event, so the window never sees it: a Shift or Ctrl tap, and the
    // chord check of the key after it, are reported here (criterion 32).
    placement.keyDown(event);
    if (event.key === "Tab") {
      // Tab and Shift+Tab cycle inside the chip (criterion 13).
      event.preventDefault();
      const next = (index + (event.shiftKey ? count - 1 : 1)) % count;
      inputs.current[next]?.focus();
      inputs.current[next]?.select();
    } else if (event.key === " ") {
      // Space switches r and ⌀; it types nothing and does not pan (criterion 20).
      event.preventDefault();
      if (sizeField) {
        placement.toggleDiameter();
      }
    } else if (event.key === "Enter") {
      event.preventDefault();
      placement.place();
    } else if (event.key === "Escape") {
      event.preventDefault();
      placement.cancel();
    }
    // Every other key stays in the field: the canvas ignores keys from a form control.
    event.stopPropagation();
  };

  const at = placePlacementChip(
    chip.pointer,
    chip.a,
    sizes.chip,
    sizes.canvas,
    sizes.bar ?? undefined,
    TOOL_RAIL_CLEAR_PX,
  );

  return (
    <div
      ref={chipRef}
      role="group"
      aria-label={chip.groupName}
      data-placement-chip
      className="absolute z-30 rounded-[8px] p-1.5"
      style={{
        left: at.left,
        top: at.top,
        background: "var(--toolbar-bg)",
        boxShadow: "var(--panel-elevation-shadow)",
        color: "var(--toolbar-icon)",
        // Before the first edit the pointer cannot reach the chip: a press under it is a canvas
        // press, and the shape keeps following (UX notes 2, review must-fix 2).
        pointerEvents: chip.frozen ? undefined : "none",
      }}
    >
      <div className="flex items-center gap-1">
        {chip.fields.map((field, index) => {
          const angle = field.label === "Angle";
          const label = field.size ? (
            <button
              type="button"
              tabIndex={-1}
              aria-label="Diameter"
              aria-pressed={field.label !== "r"}
              title="Radius or diameter (Space)"
              // Keeps the focus in the field it came from.
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => placement.toggleDiameter()}
              className="absolute top-0 left-0 z-10 flex h-7 w-6 cursor-pointer items-center justify-center text-xs"
              // Reachable before the first edit too, so the tooltip names Space.
              style={{ pointerEvents: "auto" }}
            >
              {field.label}
            </button>
          ) : (
            field.label
          );
          return (
            <div key={index} className="flex items-center gap-1">
              {index === 1 && field.label === "H" && (
                <span aria-hidden className="flex items-center justify-center" style={{ width: LINK_SLOT_PX }}>
                  {chip.linked && <Link2 className="size-3" />}
                </span>
              )}
              <EntryField
                label={label}
                width={angle ? ANGLE_FIELD_PX : LENGTH_FIELD_PX}
                labelWidth={angle ? ANGLE_LABEL_PX : field.size ? SIZE_LABEL_PX : undefined}
                suffix={field.suffix}
                ariaLabel={field.name}
                invalid={field.invalid || chip.errorField === index}
                messageId={messageId}
                value={field.text}
                placeholder={field.placeholder || undefined}
                inputRef={(node) => {
                  inputs.current[index] = node;
                }}
                onChange={(text) => placement.edit(index, text)}
                onKeyDown={onKeyDown(index)}
                onFocus={(event) => event.target.select()}
              />
            </div>
          );
        })}
      </div>
      {chip.checks.length > 0 && (
        // The checks take the pointer although the chip does not before the first edit; a press on
        // one keeps the focus in the field.
        <div className="mt-1 flex items-center gap-3" style={{ pointerEvents: "auto" }}>
          {chip.checks.map((check) => (
            <label key={check.label} className="flex h-6 cursor-pointer items-center gap-1.5 px-1 text-xs">
              <input
                type="checkbox"
                tabIndex={-1}
                checked={check.on}
                aria-label={check.label}
                onMouseDown={(event) => event.preventDefault()}
                onChange={() => placement.toggleModifier(check.shift)}
                className="size-3.5 cursor-pointer accent-[var(--editor-accent)]"
              />
              <span aria-hidden>{check.label}</span>
            </label>
          ))}
        </div>
      )}
      <div
        id={messageId}
        role="status"
        aria-live="polite"
        className={
          chip.errorField >= 0
            ? "mt-1 flex items-center gap-1 whitespace-nowrap text-xs"
            : "h-0 overflow-hidden text-xs"
        }
        style={chip.errorField >= 0 ? { color: "var(--field-invalid)" } : undefined}
      >
        {chip.errorField >= 0 && (
          <>
            <CircleAlert aria-hidden className="size-3 shrink-0" />
            <span>{chip.errorMessage}</span>
          </>
        )}
      </div>
    </div>
  );
}

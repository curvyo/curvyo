import { Plus, Trash2 } from "lucide-react";
import { type KeyboardEvent, useEffect, useRef } from "react";

import { ICON_BUTTON_CLASS } from "@/components/TextBar";
import { Tooltip } from "@/components/ui/tooltip";
import { ARM_TRASH, ARM_ZONE, useDeleteArm } from "@/hooks/useDeleteArm";
import {
  ADD_PART,
  KEEP_ONE_PART,
  PARTS,
  PARTS_AT_MOST,
  PARTS_MAX,
  type TracePartInfo,
  partName,
  partTooltip,
  removePartText,
} from "@/lib/traceParts";

const ZONE = "trace-parts";
/** The armed trash and its colour change, as the Fonts tab's (`0081`, `FontTreeRows`), which this
 * row does not import: that module loads the font files. */
const ARMED_TRASH = "bg-[var(--destructive)] text-white";
const ARM_TRANSITION = "transition-colors duration-[120ms] ease-out motion-reduce:transition-none";

interface TracePartsProps {
  parts: readonly TracePartInfo[];
  onSelect: (index: number) => void;
  onAdd: () => void;
  onRemove: (index: number) => void;
  onReturnFocus: () => void;
}

/**
 * The Parts group, first in the last row of the Advanced block (`specs/0094` slice 2, criteria 18,
 * 19, 28, 32; UX notes §3 to §5; `specs/0089` UX notes §24.3): the muted label "Parts", one radio
 * group of numbered chips (roving focus, the arrows, Home and End make a part active), Add part,
 * and the trash of the active part with the inline confirm of `0081` (Delete on a chip arms it
 * too). Left to right is creation and stacking order. It wraps as one whole item; past the
 * block's width Add and Remove go to a second line together.
 */
export function TraceParts({ parts, onSelect, onAdd, onRemove, onReturnFocus }: TracePartsProps) {
  const arm = useDeleteArm();
  const chips = useRef<(HTMLButtonElement | null)[]>([]);
  const active = Math.max(0, parts.findIndex((p) => p.active));
  const count = parts.length;
  // A chip picked or a part added disarms the trash (UX notes §5).
  const shape = parts.map((p) => `${p.id}${p.active ? "*" : ""}`).join();
  const { disarm } = arm;
  useEffect(() => {
    disarm();
  }, [shape, disarm]);
  const armed = arm.armed === ZONE;
  const canRemove = count > 1;
  const pressTrash = () => {
    if (!canRemove) {
      return;
    }
    arm.press(ZONE, removePartText(active, true), () => {
      onRemove(active);
      return "";
    });
  };
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Delete") {
      event.preventDefault();
      pressTrash();
      return;
    }
    if (event.key === "Enter" && armed) {
      event.preventDefault();
      pressTrash();
      return;
    }
    if (event.key === "Escape" && !armed) {
      event.preventDefault();
      event.stopPropagation();
      onReturnFocus();
      return;
    }
    const next =
      event.key === "ArrowRight" || event.key === "ArrowDown"
        ? Math.min(count - 1, active + 1)
        : event.key === "ArrowLeft" || event.key === "ArrowUp"
          ? Math.max(0, active - 1)
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? count - 1
              : -1;
    if (next >= 0) {
      event.preventDefault();
      if (next !== active) {
        onSelect(next);
      }
      chips.current[next]?.focus();
    }
  };
  const full = count >= PARTS_MAX;
  const plain = "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]";
  return (
    <div
      data-trace-parts=""
      {...{ [ARM_ZONE]: ZONE }}
      className={`flex min-h-9 max-w-full min-w-0 flex-wrap items-center gap-1 rounded-md ${ARM_TRANSITION} ${armed ? "bg-[var(--destructive-soft)]" : ""}`}
    >
      <span aria-hidden className="flex h-9 w-8 shrink-0 items-center text-xs text-[var(--toolbar-icon)] opacity-70">
        {PARTS}
      </span>
      <div role="radiogroup" aria-label={PARTS} className="flex flex-wrap items-center gap-1" onKeyDown={keyDown}>
        {parts.map((part, index) => {
          const on = index === active;
          const look = on
            ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
            : `${plain} ${part.muted ? "border border-dashed border-[var(--toolbar-icon)]/60" : ""}`;
          return (
            <Tooltip key={part.id} side="bottom" content={partTooltip(index, count, part)}>
              <button
                ref={(element) => {
                  chips.current[index] = element;
                }}
                type="button"
                role="radio"
                aria-checked={on}
                aria-label={partName(index, count, part)}
                data-trace-part={index + 1}
                tabIndex={on ? 0 : -1}
                onClick={(event) => {
                  if (!on) {
                    onSelect(index);
                  }
                  if (event.detail > 0) {
                    onReturnFocus();
                  }
                }}
                className={`${ICON_BUTTON_CLASS} relative text-sm tabular-nums ${look}`}
              >
                <span className={part.muted ? "opacity-50" : ""}>{index + 1}</span>
                {part.whole || part.muted ? null : (
                  <span
                    aria-hidden
                    className={`absolute right-0.5 bottom-0.5 size-1.5 ${part.outside ? "border border-current" : "bg-current"}`}
                  />
                )}
              </button>
            </Tooltip>
          );
        })}
      </div>
      <div className="flex h-9 items-center gap-1">
        <Tooltip side="bottom" content={full ? PARTS_AT_MOST : ADD_PART}>
          <button
            type="button"
            aria-label={ADD_PART}
            aria-disabled={full || undefined}
            data-trace-add-part=""
            onClick={() => {
              if (!full) {
                onAdd();
              }
            }}
            className={`${ICON_BUTTON_CLASS} ${full ? "cursor-default opacity-50 text-[var(--toolbar-icon)]" : plain}`}
          >
            <Plus size={16} strokeWidth={1.5} aria-hidden />
          </button>
        </Tooltip>
        <Tooltip side="bottom" content={canRemove ? removePartText(active, armed) : KEEP_ONE_PART}>
          <button
            type="button"
            tabIndex={-1}
            aria-label={canRemove ? removePartText(active, armed) : KEEP_ONE_PART}
            aria-disabled={!canRemove || undefined}
            data-trace-remove-part=""
            {...{ [ARM_TRASH]: ZONE }}
            onClick={pressTrash}
            className={`${ICON_BUTTON_CLASS} ${ARM_TRANSITION} ${
              !canRemove ? "cursor-default opacity-50 text-[var(--toolbar-icon)]" : armed ? ARMED_TRASH : plain
            }`}
          >
            <Trash2 size={16} strokeWidth={1.5} aria-hidden />
          </button>
        </Tooltip>
      </div>
      <span role="status" aria-live="polite" className="sr-only">
        {arm.status}
      </span>
    </div>
  );
}

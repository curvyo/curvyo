import { Check, ChevronDown, ChevronUp, type LucideIcon } from "lucide-react";
import { GLYPH } from "@/components/ui/looks";
import { useEffect, useId, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type RefObject } from "react";

import { Tooltip } from "@/components/ui/tooltip";

/** One option of the pick: a muted one stays listed with its note and cannot be picked. */
export interface PickOption {
  id: string;
  name: string;
  pickable: boolean;
  note: string | null;
}

/**
 * The pick of the Operations tab's output parts (`specs/0029-laser-job-preview-and-output` UX
 * notes 6.2 and 6.3; the look of the `0028` Material pick, inline list, no popup): the Fixture jobs
 * material and Start. A full-width 28 px button, the label inside at the left, the value at the
 * right; open, an inline radiogroup with the current option checked on the selected ground. Keys:
 * one Tab stop on the picked row, arrows move, Enter or Space picks, Escape closes to the button,
 * and focus leaving the pick closes it. `openRequest` opens it from outside (a check's button).
 */
export function OutputPick({
  label,
  Icon,
  value,
  valueText,
  options,
  buttonRef,
  openRequest,
  onPick,
  onReturnFocus,
  dataName,
  onPreview,
  pointerKeepsFocus = false,
}: {
  /** The accessible name and the label inside: "Start", "Fixture jobs run with". */
  label: string;
  /** The symbol at the left; the Dither pick has none (`specs/0123` P21). */
  Icon?: LucideIcon;
  /** The id of the picked option. */
  value: string;
  /** The words at the button's right. */
  valueText: string;
  options: readonly PickOption[];
  buttonRef: RefObject<HTMLButtonElement | null>;
  /** A counter: each increase opens the list and focuses the picked row. */
  openRequest: number;
  onPick: (id: string) => void;
  onReturnFocus: () => void;
  dataName: string;
  /**
   * The row the keyboard focuses in the open list, previewed without a step; `null` when the
   * list closes (`specs/0123` UX P23). Absent: no preview.
   */
  onPreview?: (id: string | null) => void;
  /** A pointer pick keeps the focus on the button instead of handing it to the canvas. */
  pointerKeepsFocus?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [stop, setStop] = useState(0);
  const [seen, setSeen] = useState(openRequest);
  const listId = useId();
  const listRef = useRef<HTMLDivElement>(null);
  const focusRow = useRef<number | null>(null);
  if (openRequest !== seen) {
    setSeen(openRequest);
    setOpen(true);
  }

  // Opening puts the focus on the picked row.
  useEffect(() => {
    if (!open) return;
    const at = Math.max(0, options.findIndex((option) => option.id === value));
    setStop(at);
    focusRow.current = at;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);
  useEffect(() => {
    const at = focusRow.current;
    if (at === null || !open) return;
    focusRow.current = null;
    // A list low in the panel scrolls fully into view (UX review U8).
    listRef.current?.scrollIntoView?.({ block: "nearest" });
    listRef.current?.querySelector<HTMLElement>(`[data-choice="${at}"]`)?.focus({ preventScroll: true });
  });

  const close = (refocus: boolean) => {
    setOpen(false);
    onPreview?.(null);
    if (refocus) buttonRef.current?.focus();
  };
  const choose = (option: PickOption, pointer: boolean) => {
    if (!option.pickable) return;
    onPick(option.id);
    // A press hands the focus back to the canvas, as the other panel controls do; a key keeps it
    // on the button.
    const handBack = pointer && !pointerKeepsFocus;
    close(!handBack);
    if (handBack) onReturnFocus();
  };
  const onListKey = (event: ReactKeyboardEvent<HTMLElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close(true);
      return;
    }
    const step = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    const edge = event.key === "Home" ? 0 : event.key === "End" ? options.length - 1 : null;
    if (step === 0 && edge === null) return;
    event.preventDefault();
    // Inside an operation row the list's own arrow keys would move to the next row.
    event.stopPropagation();
    const next = edge ?? Math.min(options.length - 1, Math.max(0, stop + step));
    setStop(next);
    const focused = options[next];
    if (focused?.pickable) onPreview?.(focused.id);
    listRef.current?.querySelector<HTMLElement>(`[data-choice="${next}"]`)?.focus();
  };
  const Chevron = open ? ChevronUp : ChevronDown;
  return (
    <div
      className="flex flex-col"
      onBlur={(event) => {
        // Focus leaving the button and the list closes it, with no focus move.
        if (open && !event.currentTarget.contains(event.relatedTarget as Node | null)) close(false);
      }}
    >
      <Tooltip side="left" content={label}>
        <button
          ref={buttonRef}
          type="button"
          aria-expanded={open}
          aria-controls={open ? listId : undefined}
          aria-label={`${label}, ${valueText}`}
          {...{ [dataName]: "" }}
          onClick={() => setOpen((was) => !was)}
          onKeyDown={(event) => {
            if (event.key === "ArrowDown" && !open) {
              event.preventDefault();
              event.stopPropagation();
              setOpen(true);
            }
          }}
          className="flex h-7 w-full items-center gap-1.5 rounded-[5px] border border-[var(--field-border)] bg-white px-1.5 text-sm text-[var(--toolbar-icon)] outline-none hover:border-[var(--editor-accent)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          {Icon ? <Icon size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="shrink-0" /> : null}
          <span className="shrink-0 text-xs text-[var(--panel-muted-fg)]">{label}</span>
          <span className="min-w-0 flex-1 truncate text-right">{valueText}</span>
          <Chevron size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="shrink-0" />
        </button>
      </Tooltip>
      {open ? (
        <div
          id={listId}
          ref={listRef}
          role="radiogroup"
          aria-label={label}
          className="-mt-px flex max-h-56 flex-col overflow-y-auto rounded-b-[5px] border border-t-0 border-[var(--field-border)] bg-white py-1"
          onKeyDown={onListKey}
        >
          {options.map((option, at) => {
            const checked = value === option.id;
            return (
              <button
                key={option.id}
                type="button"
                role="radio"
                data-choice={at}
                aria-checked={checked}
                aria-disabled={option.pickable ? undefined : true}
                tabIndex={at === stop ? 0 : -1}
                onFocus={() => setStop(at)}
                onClick={(event) => choose(option, event.detail > 0)}
                className={`relative flex h-7 shrink-0 items-center justify-between gap-2 rounded-[5px] pl-2 pr-1 text-left text-sm outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
                  checked ? "bg-[var(--value-fill)]" : ""
                } ${option.pickable ? "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]" : "text-[var(--panel-muted-fg)]"}`}
              >
                {checked ? (
                  <span aria-hidden className="absolute left-0 top-0 h-full w-[3px] rounded-full bg-[var(--editor-accent)]" />
                ) : null}
                <span className="truncate">{option.name}</span>
                {checked ? (
                  <Check {...GLYPH} aria-hidden className="shrink-0" />
                ) : option.note ? (
                  <span className="shrink-0 text-[11px]">{option.note}</span>
                ) : null}
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}

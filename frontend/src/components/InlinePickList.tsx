import { ChevronDown, ChevronUp, type LucideIcon } from "lucide-react";
import { SMALL_GLYPH } from "@/components/ui/looks";
import { useId, useRef, useState } from "react";
import { Tooltip } from "@/components/ui/tooltip";

/** One row of an inline pick list: a name and a muted detail at the right. */
export interface PickRow {
  id: string;
  name: string;
  detail: string;
}


/**
 * A full-width button that opens an inline list (no popup) under it (`specs/0100` criteria 15
 * and 25, the "Place clone on" pattern of `0099`): Enter or Space opens with focus on the first
 * row; Up, Down, Home and End move; Enter, Space or a press chooses; Escape closes, changes
 * nothing and returns focus to the button; Tab leaves and closes.
 */
export function InlinePickList({
  label,
  rows,
  onPick,
  className,
  icon: Icon,
}: {
  /** The symbol left of the label (`docs/ui-elements.md` §0 Icon rule). */
  icon?: LucideIcon;
  label: string;
  rows: PickRow[];
  onPick: (id: string) => void;
  className: string;
}) {
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const listId = useId();
  const options = () =>
    Array.from(listRef.current?.querySelectorAll<HTMLElement>("[role=option]") ?? []);
  const pick = (id: string) => {
    setOpen(false);
    onPick(id);
  };
  return (
    <>
      <button
        ref={button}
        type="button"
        className={className}
        aria-expanded={open}
        aria-controls={listId}
        onClick={() => {
          setOpen(!open);
          if (!open) requestAnimationFrame(() => options()[0]?.focus());
        }}
      >
        {Icon ? <Icon size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden /> : null}
        {label}
        {open ? (
          <ChevronUp {...SMALL_GLYPH} aria-hidden />
        ) : (
          <ChevronDown {...SMALL_GLYPH} aria-hidden />
        )}
      </button>
      {open && (
        <div
          id={listId}
          ref={listRef}
          role="listbox"
          aria-label={label}
          className="max-h-[216px] overflow-y-auto rounded-[5px] p-1"
          style={{ background: "var(--toolbar-bg)" }}
          onKeyDown={(e) => {
            const all = options();
            const at = all.indexOf(document.activeElement as HTMLElement);
            if (e.key === "ArrowDown" || e.key === "ArrowUp") {
              e.preventDefault();
              all[Math.max(0, Math.min(all.length - 1, at + (e.key === "ArrowDown" ? 1 : -1)))]?.focus();
            } else if (e.key === "Home" || e.key === "End") {
              e.preventDefault();
              all[e.key === "Home" ? 0 : all.length - 1]?.focus();
            } else if (e.key === "Escape") {
              e.preventDefault();
              e.stopPropagation();
              setOpen(false);
              button.current?.focus();
            } else if (e.key === "Tab") {
              setOpen(false);
            }
          }}
        >
          {rows.map((row) => (
            <Tooltip key={row.id} side="bottom" content={row.name}>
              <div
                role="option"
                aria-selected={false}
                tabIndex={-1}
                className="flex min-h-9 cursor-default items-center gap-2 rounded-[5px] px-2 text-xs hover:bg-[var(--editor-accent-hover)] focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)] pointer-coarse:min-h-11"
                onClick={() => pick(row.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    pick(row.id);
                  }
                }}
              >
                <span className="min-w-0 flex-1 truncate">{row.name}</span>
                <span className="tabular-nums text-[var(--panel-muted-fg)]">{row.detail}</span>
              </div>
            </Tooltip>
          ))}
        </div>
      )}
    </>
  );
}

import { useRef } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { CORNER_NAMES, CORNERS, type Corner, cornerKey } from "@/lib/machineBlockKeys";

const CORNER_BUTTON =
  "inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]";

/**
 * The 2 x 2 origin corner radiogroup of the laser Machine block and the Grid & Rulers pane
 * (`specs/0026-machine-profile` UX notes §6, `specs/0045-document-formats-library` criterion 41):
 * one Tab stop on the chosen corner, arrows move within a row or column and choose without
 * wrapping, Home and End jump. `onReturnFocus` runs after a pointer press (the Grid pane gives the
 * keyboard back to the canvas; the form-like Machine block keeps it).
 */
export function CornerControl({
  label,
  corner,
  onChange,
  onReturnFocus,
}: {
  /** The row's visible word: "Origin corner" or "Origin". */
  label: string;
  corner: Corner;
  onChange: (corner: Corner) => void;
  onReturnFocus?: () => void;
}) {
  const group = useRef<HTMLDivElement>(null);
  const select = (next: Corner) => {
    onChange(next);
    group.current?.querySelector<HTMLElement>(`[data-corner="${next}"]`)?.focus();
  };
  return (
    <div className="flex h-[60px] items-center justify-between">
      <span aria-hidden className="text-sm text-[var(--toolbar-icon)]">
        {label}
      </span>
      <div ref={group} role="radiogroup" aria-label="Origin corner" className="grid grid-cols-2 gap-1">
        {CORNERS.map((name) => (
          <Tooltip key={name} side="left" content={CORNER_NAMES[name]}>
            <button
              type="button"
              role="radio"
              aria-checked={name === corner}
              aria-label={CORNER_NAMES[name]}
              data-corner={name}
              tabIndex={name === corner ? 0 : -1}
              onClick={(event) => {
                onChange(name);
                if (event.detail > 0) {
                  onReturnFocus?.();
                }
              }}
              onKeyDown={(event) => {
                const next = cornerKey(event.key, corner);
                if (next !== null) {
                  event.preventDefault();
                  select(next);
                }
              }}
              className={`${CORNER_BUTTON} ${name === corner ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] hover:bg-[var(--toolbar-icon-active-bg)]" : ""}`}
            >
              <CornerGlyph corner={name} />
            </button>
          </Tooltip>
        ))}
      </div>
    </div>
  );
}

/** A 12px square outline with a 5px filled square in the named corner. */
function CornerGlyph({ corner }: { corner: Corner }) {
  const x = corner.endsWith("right") ? 9.25 : 2.75;
  const y = corner.startsWith("bottom") ? 9.25 : 2.75;
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" aria-hidden>
      <rect x={2} y={2} width={12} height={12} fill="none" stroke="currentColor" strokeWidth={1.5} />
      <rect x={x} y={y} width={4} height={4} fill="currentColor" />
    </svg>
  );
}

import { useRef } from "react";

import { EntryField } from "@/components/panel/EntryField";
import type { PencilBarState, PencilCurve } from "@/hooks/usePencil";

export interface PencilToolbarProps {
  bar: PencilBarState;
  onSetMinimum: (text: string) => string;
  onSetMaximum: (text: string) => string;
  onSetCurve: (curve: PencilCurve) => void;
  /** Returns the keyboard focus to the canvas (after Enter or Escape in a field). */
  onReturnFocus: () => void;
}

const CURVES: readonly { curve: PencilCurve; label: string }[] = [
  { curve: "soft", label: "Soft" },
  { curve: "linear", label: "Linear" },
  { curve: "firm", label: "Firm" },
];

const WIDTH_MESSAGES = { number: "Type a width in millimetres." } as const;

/** Shows a width with at most two decimals, without trailing zeros. */
function widthText(mm: number): string {
  return String(Math.round(mm * 100) / 100);
}

/** The Curve choice: one Tab stop, arrows move and pick (`role="radiogroup"`). */
function CurveGroup({
  curve,
  onSetCurve,
}: {
  curve: PencilCurve;
  onSetCurve: (curve: PencilCurve) => void;
}) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const pick = (index: number) => {
    const next = CURVES[(index + CURVES.length) % CURVES.length];
    onSetCurve(next.curve);
    buttons.current[CURVES.indexOf(next)]?.focus();
  };
  return (
    <div
      role="radiogroup"
      aria-label="Curve"
      className="flex overflow-hidden rounded-md ring-1 ring-border"
    >
      {CURVES.map((entry, index) => {
        const checked = entry.curve === curve;
        return (
          <button
            key={entry.curve}
            ref={(element) => {
              buttons.current[index] = element;
            }}
            type="button"
            role="radio"
            aria-checked={checked}
            tabIndex={checked ? 0 : -1}
            onClick={() => onSetCurve(entry.curve)}
            onKeyDown={(event) => {
              if (event.key === "ArrowRight" || event.key === "ArrowDown") {
                event.preventDefault();
                pick(index + 1);
              } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
                event.preventDefault();
                pick(index - 1);
              }
            }}
            className="h-7 px-2 outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
            style={{
              background: checked ? "var(--toolbar-icon-active-bg)" : "transparent",
              color: checked ? "var(--toolbar-icon-active-fg)" : "var(--toolbar-icon)",
            }}
          >
            {entry.label}
          </button>
        );
      })}
    </div>
  );
}

/**
 * The Pencil bar (`specs/0032-pen-tablet-input` criteria 14 and 15, `docs/design-system.md` row
 * "Pencil bar"): hidden while only a mouse has been used. Once a pen is seen it shows the muted
 * readout; after the first stroke with pressure it adds Min, Max and Curve. The settings are the
 * session's and apply from the next stroke; nothing here is saved.
 */
export function PencilToolbar({
  bar,
  onSetMinimum,
  onSetMaximum,
  onSetCurve,
  onReturnFocus,
}: PencilToolbarProps) {
  if (!bar.penSeen) {
    return null;
  }
  return (
    <div
      className="pointer-events-auto flex min-w-0 flex-wrap items-center gap-3 rounded-lg px-2 py-1 text-sm"
      style={{
        background: "var(--toolbar-bg)",
        boxShadow: "var(--panel-elevation-shadow)",
      }}
    >
      {bar.pressureSeen && (
        <>
          <div className="flex items-center gap-1 text-[var(--toolbar-icon)]">
            <span>Min</span>
            <EntryField
              label="Minimum width"
              shown={widthText(bar.minimumMm)}
              mixed={false}
              suffix="mm"
              width={88}
              inputMode="decimal"
              onSubmit={onSetMinimum}
              messages={WIDTH_MESSAGES}
              onReturnFocus={onReturnFocus}
            />
          </div>
          <div className="flex items-center gap-1 text-[var(--toolbar-icon)]">
            <span>Max</span>
            <EntryField
              label="Maximum width"
              shown={widthText(bar.maximumMm)}
              mixed={false}
              suffix="mm"
              width={88}
              inputMode="decimal"
              onSubmit={onSetMaximum}
              messages={WIDTH_MESSAGES}
              onReturnFocus={onReturnFocus}
            />
          </div>
          <div className="flex items-center gap-1 text-[var(--toolbar-icon)]">
            <span>Curve</span>
            <CurveGroup curve={bar.curve} onSetCurve={onSetCurve} />
          </div>
        </>
      )}
      <span
        className="w-[232px] truncate tabular-nums"
        style={{ color: "var(--panel-muted-fg)" }}
      >
        {bar.readout ?? " "}
      </span>
    </div>
  );
}

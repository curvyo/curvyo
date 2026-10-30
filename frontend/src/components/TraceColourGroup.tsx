import { Pipette, X } from "lucide-react";
import { useState } from "react";
import type { KeyboardEvent } from "react";

import { Swatch } from "@/components/panel/Swatch";
import { ICON_BUTTON_CLASS } from "@/components/TextBar";
import { Tooltip as AppTooltip } from "@/components/ui/tooltip";
import { AUTO, NO_BACKGROUND, PICK_BACKGROUND, PICK_COLOUR, PICK_TOOLTIP, removeSwatchLabel } from "@/lib/traceText";

const TEXT_BUTTON_CLASS =
  "my-1 h-7 whitespace-nowrap rounded-md px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** `#RRGGBB` as the packed colour the swatch takes. */
function packed(hex: string): number {
  return Number.parseInt(hex.slice(1, 7), 16);
}

function pressedClass(on: boolean): string {
  return on ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]" : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]";
}

/** The pipette button (UX notes §6, the Style eyedropper's look): pressed while armed. A mouse
 * press hands the keyboard back to the canvas. */
function PipetteButton({ label, armed, onToggle, onReturnFocus }: { label: string; armed: boolean; onToggle: () => void; onReturnFocus: () => void }) {
  return (
    <AppTooltip side="bottom" content={PICK_TOOLTIP}>
      <button
        type="button"
        aria-label={label}
        aria-pressed={armed}
        data-trace-pipette=""
        onClick={(event) => {
          onToggle();
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
        className={`${ICON_BUTTON_CLASS} ${pressedClass(armed)}`}
      >
        <Pipette size={16} strokeWidth={1.5} aria-hidden />
      </button>
    </AppTooltip>
  );
}

interface RemoveBackgroundProps {
  /** `#RRGGBB`, empty for a transparent background, `null` before Auto has read the image. */
  background: string | null;
  auto: boolean;
  armed: boolean;
  onAuto: () => void;
  onPipette: () => void;
  onReturnFocus: () => void;
}

/** Remove background's colour group (UX notes §5): the colour chip, its hex (or "none"), the Auto
 * toggle and the pipette. */
export function RemoveBackgroundGroup({ background, auto, armed, onAuto, onPipette, onReturnFocus }: RemoveBackgroundProps) {
  const hex = background === null ? "" : background === "" ? NO_BACKGROUND : background;
  return (
    <div data-trace-colours="background" className="flex h-9 items-center gap-1">
      <span className="flex size-7 items-center justify-center">
        {background ? <Swatch rgb={packed(background)} opacity={100} mixed={false} size={20} /> : <Swatch rgb={0xffffff} opacity={0} mixed={false} size={20} />}
      </span>
      <span data-trace-background="" className="w-14 text-xs tabular-nums text-[var(--toolbar-icon)]">
        {hex}
      </span>
      <button
        type="button"
        aria-pressed={auto}
        onClick={(event) => {
          onAuto();
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
        className={`${TEXT_BUTTON_CLASS} w-11 ${pressedClass(auto)}`}
      >
        {AUTO}
      </button>
      <PipetteButton label={PICK_BACKGROUND} armed={armed} onToggle={onPipette} onReturnFocus={onReturnFocus} />
    </div>
  );
}

interface PickColoursProps {
  swatches: readonly string[];
  armed: boolean;
  onPipette: () => void;
  onRemove: (hex: string) => void;
  onReturnFocus: () => void;
}

/** Pick colours' colour group (UX notes §5): the picked colours as swatches (one Tab stop, the
 * arrows move, Delete removes the focused one; a click selects one and shows its Remove button)
 * and the pipette. */
export function PickColoursGroup({ swatches, armed, onPipette, onRemove, onReturnFocus }: PickColoursProps) {
  const [selected, setSelected] = useState<string | null>(null);
  const [focus, setFocus] = useState(0);
  const at = Math.min(focus, Math.max(0, swatches.length - 1));
  const shown = selected !== null && swatches.includes(selected) ? selected : null;
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (step !== 0 && swatches.length > 0) {
      event.preventDefault();
      const next = (at + step + swatches.length) % swatches.length;
      setFocus(next);
      (event.currentTarget.querySelectorAll("[data-trace-swatch]")[next] as HTMLElement | undefined)?.focus();
    } else if ((event.key === "Delete" || event.key === "Backspace") && swatches[at]) {
      event.preventDefault();
      onRemove(swatches[at]);
    }
  };
  return (
    <div data-trace-colours="picked" className="flex h-9 items-center gap-1">
      {swatches.length > 0 ? (
        <div role="listbox" aria-label="Picked colours" aria-orientation="horizontal" className="flex items-center gap-1" onKeyDown={keyDown}>
          {swatches.map((hex, index) => (
            <button
              key={hex}
              type="button"
              role="option"
              aria-selected={shown === hex}
              aria-label={hex}
              data-trace-swatch={hex}
              tabIndex={index === at ? 0 : -1}
              onClick={() => {
                setFocus(index);
                setSelected(shown === hex ? null : hex);
              }}
              className={`flex size-7 items-center justify-center rounded-md outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
                shown === hex ? "ring-2 ring-[var(--editor-accent)]" : ""
              }`}
            >
              <Swatch rgb={packed(hex)} opacity={100} mixed={false} size={20} />
            </button>
          ))}
        </div>
      ) : null}
      {shown ? (
        <AppTooltip side="bottom" content={removeSwatchLabel(shown)}>
          <button
            type="button"
            aria-label={removeSwatchLabel(shown)}
            onClick={(event) => {
              onRemove(shown);
              setSelected(null);
              if (event.detail > 0) {
                onReturnFocus();
              }
            }}
            className={`${ICON_BUTTON_CLASS} text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]`}
          >
            <X size={16} strokeWidth={1.5} aria-hidden />
          </button>
        </AppTooltip>
      ) : null}
      <PipetteButton label={PICK_COLOUR} armed={armed} onToggle={onPipette} onReturnFocus={onReturnFocus} />
    </div>
  );
}

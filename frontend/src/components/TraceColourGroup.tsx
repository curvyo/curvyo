import { Pipette, X } from "lucide-react";
import { useRef, useState, type KeyboardEvent } from "react";

import { Swatch } from "@/components/panel/Swatch";
import { ICON_BUTTON_CLASS } from "@/components/TextBar";
import { Tooltip as AppTooltip } from "@/components/ui/tooltip";
import { focusAfterRemoval, pickedKey } from "@/lib/tracePickedColours";
import { AUTO, NO_BACKGROUND, PICK_BACKGROUND, PICK_COLOUR, PICK_TOOLTIP, PICKED_COLOURS, removePickedLabel } from "@/lib/traceText";

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
  /** The picked colours, `#RRGGBB`, in the order they were picked. */
  swatches: readonly string[];
  armed: boolean;
  onPipette: () => void;
  onRemove: (hex: string) => void;
  onReturnFocus: () => void;
}

/** One picked colour (`ux-notes.md` §25.4): a 24 x 28 button around the 20 px swatch; hover and
 * focus fade it to 40 % and show an x on a casing; the tooltip is the colour code, the name says
 * what a press does. */
function PickedSwatch({ hex, stop, onPress, onKey }: { hex: string; stop: boolean; onPress: (byKey: boolean) => void; onKey: (event: KeyboardEvent<HTMLButtonElement>) => void }) {
  return (
    <AppTooltip side="bottom" content={hex}>
      <button
        type="button"
        aria-label={removePickedLabel(hex)}
        data-trace-picked-swatch={hex}
        tabIndex={stop ? 0 : -1}
        onClick={(event) => onPress(event.detail === 0)}
        onKeyDown={onKey}
        className="group relative flex h-7 w-6 shrink-0 cursor-default items-center justify-center outline-none"
      >
        <span
          aria-hidden
          className="block size-5 rounded-[4px] border border-[var(--swatch-border)] group-hover:opacity-40 group-focus-visible:opacity-40"
          style={{ background: hex }}
        />
        <span aria-hidden className="pointer-events-none absolute size-5 rounded-[4px] group-focus-visible:ring-2 group-focus-visible:ring-[var(--editor-accent)]" />
        <span aria-hidden className="pointer-events-none absolute hidden group-hover:block group-focus-visible:block">
          <X size={12} absoluteStrokeWidth strokeWidth={7.5} className="absolute text-[var(--toolbar-bg)]" />
          <X size={12} absoluteStrokeWidth strokeWidth={1.5} className="relative text-[var(--toolbar-icon)]" />
        </span>
      </button>
    </AppTooltip>
  );
}

/** Pick colours' group in the bar (`ux-notes.md` §25.4): the pipette and the picked colours beside
 * it, one Tab stop with roving arrows; a press on a swatch removes its colour. The group keeps the
 * width of eight colours, so the bar does not move as colours come and go. */
export function PickColoursGroup({ swatches, armed, onPipette, onRemove, onReturnFocus }: PickColoursProps) {
  const [focus, setFocus] = useState(0);
  const row = useRef<HTMLDivElement>(null);
  const at = Math.min(focus, Math.max(0, swatches.length - 1));
  const focusSwatch = (index: number | null) => {
    const root = row.current?.parentElement;
    requestAnimationFrame(() => {
      const target = index === null ? root?.querySelector<HTMLElement>("[data-trace-pipette]") : row.current?.querySelectorAll<HTMLElement>("[data-trace-picked-swatch]")[index];
      target?.focus();
    });
  };
  const remove = (index: number, byKey: boolean) => {
    const hex = swatches[index];
    if (hex === undefined) {
      return;
    }
    const next = focusAfterRemoval(index, swatches.length);
    setFocus(next ?? 0);
    onRemove(hex);
    if (byKey) {
      focusSwatch(next);
    } else {
      onReturnFocus();
    }
  };
  const keyDown = (index: number) => (event: KeyboardEvent<HTMLButtonElement>) => {
    const action = pickedKey(event.key, index, swatches.length);
    if (action === null) {
      return;
    }
    event.preventDefault();
    // Delete and Backspace belong to the swatch, not to the canvas's delete.
    event.stopPropagation();
    if ("move" in action) {
      setFocus(action.move);
      row.current?.querySelectorAll<HTMLElement>("[data-trace-picked-swatch]")[action.move]?.focus();
    } else {
      remove(action.remove, true);
    }
  };
  return (
    <div data-trace-colours="picked" role="group" aria-label={PICKED_COLOURS} className="flex min-h-9 min-w-[220px] items-start">
      <span className="flex h-9 items-center">
        <PipetteButton label={PICK_COLOUR} armed={armed} onToggle={onPipette} onReturnFocus={onReturnFocus} />
      </span>
      {/* 24 px cells touch, so the 20 px swatches are 4 px apart; the row's margins put the first
          4 px after the pipette and keep eight colours at 220 px. */}
      <div ref={row} className="my-1 mr-[-2px] ml-[2px] flex flex-wrap">
        {swatches.map((hex, index) => (
          <PickedSwatch key={hex} hex={hex} stop={index === at} onPress={(byKey) => remove(index, byKey)} onKey={keyDown(index)} />
        ))}
      </div>
    </div>
  );
}

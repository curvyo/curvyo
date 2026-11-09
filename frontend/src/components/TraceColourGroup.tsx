import { Ban, Pipette, X, type LucideIcon } from "lucide-react";
import { useRef, useState, type KeyboardEvent, type MouseEvent, type PointerEvent, type ReactNode } from "react";

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

/** A glyph centred over a swatch on a 3 px `--toolbar-bg` casing, so it reads on any colour. */
function CasedGlyph({ icon: Icon, className }: { icon: LucideIcon; className: string }) {
  return (
    <span aria-hidden className={`pointer-events-none absolute ${className}`}>
      <Icon size={12} absoluteStrokeWidth strokeWidth={7.5} className="absolute text-[var(--toolbar-bg)]" />
      <Icon size={12} absoluteStrokeWidth strokeWidth={1.5} className="relative text-[var(--toolbar-icon)]" />
    </span>
  );
}

interface SwatchCellProps {
  hex: string;
  /** Which row it belongs to: its data attribute (`data-trace-picked-swatch`, `data-trace-palette-swatch`). */
  kind: "picked" | "palette";
  /** The row's one Tab stop. */
  stop: boolean;
  label: string;
  tooltip: ReactNode;
  /** A picked colour: hover and focus fade it and show the x. */
  removable?: boolean;
  /** An excluded palette colour: at 40 % with the `Ban` glyph. */
  excluded?: boolean;
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
  onKeyDown: (event: KeyboardEvent<HTMLButtonElement>) => void;
  /** Enter and every move: the palette reads Shift from them (Shift+hover isolates). */
  onPointerMove?: (event: PointerEvent<HTMLButtonElement>) => void;
  onPointerLeave?: () => void;
}

/** One colour of a swatch row (`ux-notes.md` §25.4, §26): a 24 x 28 button around the 20 px
 * swatch; the cells touch, so the swatches are 4 px apart. */
export function SwatchCell({ hex, kind, stop, label, tooltip, removable = false, excluded = false, onClick, onKeyDown, onPointerMove, onPointerLeave }: SwatchCellProps) {
  const fade = removable ? "group-hover:opacity-40 group-focus-visible:opacity-40" : excluded ? "opacity-40" : "";
  return (
    <AppTooltip side="bottom" content={tooltip}>
      <button
        type="button"
        aria-label={label}
        data-trace-picked-swatch={kind === "picked" ? hex : undefined}
        data-trace-palette-swatch={kind === "palette" ? hex : undefined}
        tabIndex={stop ? 0 : -1}
        onClick={onClick}
        onKeyDown={onKeyDown}
        onPointerEnter={onPointerMove}
        onPointerMove={onPointerMove}
        onPointerLeave={onPointerLeave}
        className="group relative flex h-7 w-6 shrink-0 cursor-default items-center justify-center outline-none"
      >
        <span aria-hidden className={`block size-5 rounded-[4px] border border-[var(--swatch-border)] ${fade}`} style={{ background: hex }} />
        <span aria-hidden className="pointer-events-none absolute size-5 rounded-[4px] group-focus-visible:ring-2 group-focus-visible:ring-[var(--editor-accent)]" />
        {removable ? <CasedGlyph icon={X} className="hidden group-hover:block group-focus-visible:block" /> : null}
        {excluded ? <CasedGlyph icon={Ban} className="block" /> : null}
      </button>
    </AppTooltip>
  );
}

/** Pick colours' group in the bar (`ux-notes.md` §25.4): the pipette and the picked colours beside
 * it, one Tab stop with roving arrows; a press on a swatch removes its colour. The group is as
 * wide as its content (customer 2026-11-09). */
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
    <div data-trace-colours="picked" role="group" aria-label={PICKED_COLOURS} className="flex min-h-9 items-start">
      <span className="flex h-9 items-center">
        <PipetteButton label={PICK_COLOUR} armed={armed} onToggle={onPipette} onReturnFocus={onReturnFocus} />
      </span>
      {/* 24 px cells touch, so the 20 px swatches are 4 px apart; the row's margins put the first
          4 px after the pipette; eight colours make 220 px. */}
      <div ref={row} className="my-1 mr-[-2px] ml-[2px] flex flex-wrap">
        {swatches.map((hex, index) => (
          <SwatchCell
            key={hex}
            hex={hex}
            kind="picked"
            stop={index === at}
            label={removePickedLabel(hex)}
            tooltip={hex}
            removable
            onClick={(event) => remove(index, event.detail === 0)}
            onKeyDown={keyDown(index)}
          />
        ))}
      </div>
    </div>
  );
}

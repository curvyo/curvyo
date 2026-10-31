import { Pipette } from "lucide-react";

import { Swatch } from "@/components/panel/Swatch";
import { ICON_BUTTON_CLASS } from "@/components/TextBar";
import { Tooltip as AppTooltip } from "@/components/ui/tooltip";
import { AUTO, NO_BACKGROUND, PICK_BACKGROUND, PICK_COLOUR, PICK_TOOLTIP, pickedCountText } from "@/lib/traceText";

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
  /** How many colours are picked (shown in Advanced, `ux-notes.md` §22.3). */
  count: number;
  armed: boolean;
  onPipette: () => void;
  onReturnFocus: () => void;
}

/** Pick colours' group in the bar (`ux-notes.md` §22.1): the pipette and the muted count of what
 * it picked; the swatches are shown and edited in the Advanced block. The label is a state of the
 * pipette, not a Tab stop. */
export function PickColoursGroup({ count, armed, onPipette, onReturnFocus }: PickColoursProps) {
  return (
    <div data-trace-colours="picked" className="flex h-9 items-center gap-1">
      <PipetteButton label={PICK_COLOUR} armed={armed} onToggle={onPipette} onReturnFocus={onReturnFocus} />
      <span data-trace-picked-count="" className="w-16 whitespace-nowrap text-xs tabular-nums text-[var(--toolbar-icon)] opacity-70">
        {pickedCountText(count)}
      </span>
    </div>
  );
}

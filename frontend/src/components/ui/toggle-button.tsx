import type { LucideIcon } from "lucide-react";
import { Toggle } from "radix-ui";
import type { MouseEvent } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { useControlTooltipSide } from "@/components/ui/tooltip-side";
import { type ToggleLook, toggleClass } from "@/components/ui/looks";

interface ToggleButtonProps {
  /** The name of the setting: the visible label (not with `look="icon"`) and the accessible
   * name. It does not change with the state. */
  label: string;
  /** The accessible name when it says more than the label ("Show group Slides"). */
  name?: string;
  /** A shorter visible label for a narrow bar; the accessible name keeps `label`. */
  shortLabel?: string;
  icon: LucideIcon;
  pressed: boolean;
  onPressedChange: (pressed: boolean) => void;
  /** Full-width panel row (Frage 416 A), labelled bar button, or icon-only. */
  look?: ToggleLook;
  /** The tooltip; it opens left of a toggle in the panel, below one elsewhere. */
  tooltip?: string;
  /** After a mouse press the keyboard goes back to the canvas. */
  onReturnFocus?: () => void;
  /** The roving-focus key of a list that owns this control. */
  control?: string;
  /** Armed to turn off (the armed pattern): the ground turns `--destructive`. */
  armed?: boolean;
  /** Extra attributes (the armed pattern's control id, `aria-disabled` with its reason). */
  extra?: Record<string, string>;
  className?: string;
}

/**
 * An on/off setting (`docs/ui-elements.md` §6, replaces the sliding switch): a button that stays
 * pressed, blue with a white symbol and label while on. Radix `Toggle` (`aria-pressed`): Enter
 * and Space both toggle. With `aria-disabled` in `extra` a press does nothing; the button stays a
 * Tab stop.
 */
export function ToggleButton({
  label,
  name,
  shortLabel,
  icon: Icon,
  pressed,
  onPressedChange,
  look = "panel",
  tooltip,
  onReturnFocus,
  control,
  armed = false,
  extra,
  className = "",
}: ToggleButtonProps) {
  const dimmed = extra?.["aria-disabled"] === "true";
  const tooltipSide = useControlTooltipSide();
  const visible = look === "icon" ? "" : (shortLabel ?? label);
  const button = (
    <Toggle.Root
      {...extra}
      pressed={pressed}
      aria-label={name ?? (visible === label ? undefined : label)}
      data-control={control}
      data-armed={armed ? "" : undefined}
      onPressedChange={(next) => {
        if (!dimmed) {
          onPressedChange(next);
        }
      }}
      onClick={(event: MouseEvent<HTMLButtonElement>) => {
        // `detail` is 0 for keyboard activation, 1 or more for a click.
        if (event.detail > 0) {
          onReturnFocus?.();
        }
      }}
      className={toggleClass(look, armed, className)}
    >
      <Icon size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
      {visible === "" ? null : <span className="truncate">{visible}</span>}
    </Toggle.Root>
  );
  return tooltip ? (
    <Tooltip side={look === "panel" ? "left" : tooltipSide} content={tooltip}>
      {button}
    </Tooltip>
  ) : (
    button
  );
}

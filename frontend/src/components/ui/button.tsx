import type { LucideIcon } from "lucide-react";
import { LoaderCircle } from "lucide-react";
import type { ComponentProps, MouseEvent, ReactNode } from "react";

import { type ButtonSize, type ButtonVariant, buttonClass, GLYPH } from "@/components/ui/looks";
import { Tooltip } from "@/components/ui/tooltip";
import { useControlTooltipSide } from "@/components/ui/tooltip-side";

type ButtonProps = Omit<ComponentProps<"button">, "children"> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** The symbol left of the label (`docs/ui-elements.md` §0 "Icon rule", Frage 417 A): every
   * labelled button has one. */
  icon: LucideIcon;
  /** The label: the act in one or two words. */
  children: ReactNode;
  /** The act is running: the symbol spins, label and width stay. */
  busy?: boolean;
  /** A compact bar (`docs/ui-elements.md` §0 Icon rule, 0159 S4): the symbol alone in a 28 px
   * square; the label stays the accessible name (a string label) and the tooltip's first line. */
  iconOnly?: boolean;
  /** The tooltip (below the button, left of it in the panel): the name, and for a compact button
   * what it does. */
  tooltip?: ReactNode;
};

/**
 * The one labelled button (`docs/ui-elements.md` §1 Primary, §2 Secondary, §3 Destructive):
 * symbol + label, 28 px, 5 px radius. `aria-disabled` dims it and makes a press do nothing while
 * it stays a Tab stop with its tooltip reason (§0 "States", Disabled).
 */
export function Button({
  variant = "secondary",
  size = "panel",
  icon: Icon,
  children,
  busy = false,
  iconOnly = false,
  tooltip,
  className = "",
  type = "button",
  onClick,
  ...props
}: ButtonProps) {
  const dimmed = props["aria-disabled"] === true || props["aria-disabled"] === "true";
  const tooltipSide = useControlTooltipSide();
  const glyph = busy ? (
    <LoaderCircle {...GLYPH} aria-hidden className="motion-safe:animate-spin" />
  ) : (
    <Icon size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
  );
  const classes = buttonClass(variant, size, iconOnly ? `size-7 px-0 pointer-coarse:size-9 ${className}` : className);
  const name = iconOnly && typeof children === "string" ? children : undefined;
  const button = (
    <button
      aria-label={name}
      type={type}
      data-variant={variant}
      aria-busy={busy || undefined}
      className={classes}
      onClick={(event: MouseEvent<HTMLButtonElement>) => {
        if (dimmed) {
          event.preventDefault();
          return;
        }
        onClick?.(event);
      }}
      {...props}
    >
      {glyph}
      {iconOnly ? null : children}
    </button>
  );
  return tooltip ? (
    <Tooltip side={tooltipSide} content={tooltip}>
      {button}
    </Tooltip>
  ) : (
    button
  );
}

import type { LucideIcon } from "lucide-react";
import type { ComponentProps, MouseEvent, ReactNode } from "react";

import { type IconButtonLook, type IconButtonSize, iconButtonClass } from "@/components/ui/looks";

type IconButtonProps = Omit<ComponentProps<"button">, "children" | "aria-label"> & {
  /** The name, read by assistive technology and shown as the tooltip by the caller. */
  "aria-label": string;
  icon: LucideIcon;
  look?: IconButtonLook;
  size?: IconButtonSize;
  /** A glyph that is not a Lucide icon (an own glyph set); replaces `icon`'s drawing. */
  glyph?: ReactNode;
};

/**
 * One glyph, no label (`docs/ui-elements.md` §5): 16 px Lucide (20 px in the rail), 5 px radius.
 * `aria-disabled` dims it and makes a press do nothing; it stays a Tab stop.
 */
export function IconButton({
  icon: Icon,
  look = "plain",
  size = 28,
  glyph,
  className = "",
  type = "button",
  onClick,
  ...props
}: IconButtonProps) {
  const dimmed = props["aria-disabled"] === true || props["aria-disabled"] === "true";
  return (
    <button
      type={type}
      className={iconButtonClass(look, size, className)}
      onClick={(event: MouseEvent<HTMLButtonElement>) => {
        if (dimmed) {
          event.preventDefault();
          return;
        }
        onClick?.(event);
      }}
      {...props}
    >
      {glyph ?? <Icon size={size === 40 ? 20 : 16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />}
    </button>
  );
}

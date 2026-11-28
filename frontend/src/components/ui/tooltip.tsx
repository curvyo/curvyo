import { Tooltip as TooltipPrimitive } from "radix-ui";
import type { ReactElement, ReactNode } from "react";

import { InPanelTooltips } from "@/components/ui/tooltip-side";

/** The one tooltip look (`docs/ui-elements.md` §15, 0164 S7): the canvas chip surface, 12 px,
 * 8 px radius, at most 260 px wide; also for the bars' own Radix tooltips. */
export const TOOLTIP_CLASS =
  "pointer-events-none z-[60] max-w-[260px] whitespace-pre-line rounded-[8px] bg-[var(--toolbar-bg)] px-2 py-1 text-xs text-[var(--toolbar-icon)] shadow-[var(--panel-elevation-shadow)]";

/** One provider for a group of tooltips (`docs/design-system.md`, "Frontend
 * components to add"). */
export const TooltipProvider = TooltipPrimitive.Provider;

/** Marks its subtree as the panel, so `Button` and `ToggleButton` tooltips open left. */
export function PanelTooltipScope({ children }: { children: ReactNode }) {
  return <InPanelTooltips.Provider value>{children}</InPanelTooltips.Provider>;
}

interface TooltipProps {
  /** The text after a 400 ms rest; empty for no tooltip. */
  content: ReactNode;
  /** Which side opens. No default: the panel's tooltips open left, over the
   * canvas, and the tool rail's open right. */
  side: "top" | "right" | "bottom" | "left";
  /** Distance from the trigger, px (default 8). */
  offset?: number;
  /** The trigger: one element that takes a ref. */
  children: ReactElement;
}

/**
 * A tooltip in the house style (`docs/ui-elements.md` §15): 400 ms, the canvas chip surface,
 * rendered in a portal so
 * it may cover the canvas but never the panel row it belongs to.
 */
export function Tooltip({ content, side, offset = 8, children }: TooltipProps) {
  // No content, no tooltip: the trigger alone (a row that needs none).
  if (content === undefined || content === null || content === "") {
    return children;
  }
  // Uncontrolled on purpose: every close goes through Radix, so the provider restarts its
  // 300 ms skip window and the next tooltip waits its 400 ms again. Not hoverable: it closes
  // when the pointer leaves the trigger, on a press and on blur (Radix's own handlers).
  return (
    <TooltipPrimitive.Root delayDuration={400} disableHoverableContent>
      <TooltipPrimitive.Trigger
        asChild
        onFocus={(event) => {
          // A focus that is not from the keyboard (a script moving it after a click) opens no
          // tooltip: a prevented event skips Radix's own open handler.
          if (!event.currentTarget.matches(":focus-visible")) {
            event.preventDefault();
          }
        }}
      >
        {children}
      </TooltipPrimitive.Trigger>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content
          side={side}
          sideOffset={offset}
          className={TOOLTIP_CLASS}
        >
          {content}
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  );
}

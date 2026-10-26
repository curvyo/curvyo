import { Import } from "lucide-react";
import { Tooltip } from "radix-ui";

import { COLUMN_A_TOOLTIP_OFFSET_PX, RAIL_CARD_CLASS, RAIL_CARD_STYLE } from "@/components/railCard";

interface ImportCardProps {
  /** The primary modifier's name: "Cmd" on macOS, else "Ctrl". */
  primary: string;
  onImport: () => void;
}

/**
 * The Import card of the tool rail (`specs/0058-bitmap-import` criterion 1, `docs/design-system.md`
 * row "Tool rail architecture"): column A, 8px under the Path tools card, one button. An action,
 * not a tool: a press opens the file picker at once, it never becomes the active tool and has no
 * pressed state; never dimmed. Focus: the import moves it to the canvas, a cancelled picker
 * returns it here.
 */
export function ImportCard({ primary, onImport }: ImportCardProps) {
  const name = `Import (${primary}+I)`;
  return (
    <div
      role="toolbar"
      aria-label="Import"
      aria-orientation="vertical"
      className={RAIL_CARD_CLASS}
      style={RAIL_CARD_STYLE}
    >
      <Tooltip.Root delayDuration={400}>
        <Tooltip.Trigger asChild>
          <button
            type="button"
            aria-label="Import"
            data-import-button
            onClick={onImport}
            className="flex size-10 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] active:bg-[var(--editor-accent-hover)]"
          >
            <Import size={20} />
          </button>
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content
            side="right"
            sideOffset={COLUMN_A_TOOLTIP_OFFSET_PX}
            className="z-50 rounded-md bg-popover px-2 py-1 text-xs text-popover-foreground ring-1 ring-foreground/10"
          >
            {name}
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </div>
  );
}

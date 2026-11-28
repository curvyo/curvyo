import { Toggle } from "radix-ui";
import { useId } from "react";

import { OperationGlyph, TabsGlyph } from "@/components/panel/OperationGlyphs";
import { Tooltip } from "@/components/ui/tooltip";
import { kindButtonName, leavesEnteredGroup } from "@/lib/kindButtons";
import type { KindButton } from "@/lib/operationsView";

interface OperationKindGroupProps {
  /** The five buttons in bar order, as the session lists them. */
  kinds: readonly KindButton[];
  /** A press; `shift` creates a new operation instead of filling the last one. */
  onPress: (kind: string, shift: boolean) => void;
  /** Escape on a button inside an entered group: leave it. Absent outside a group. */
  onLeaveGroup?: () => void;
  /** The Tabs toggle after the kinds (`specs/0139` `ux-notes.md` 4.2); absent while hidden. */
  tabs?: { on: boolean; onToggle: (on: boolean) => void };
}

/**
 * The Operation group of the Select bar (`specs/0111-laser-operation-tools` `ux-notes.md` 3):
 * five 28 px glyph buttons, name and tooltip the kind word and nothing else (no tip text), the
 * description "Assigns to Engrave 1" or "Creates Cut 1" for assistive technology. Not toggles; a
 * Shift press (Shift+Enter, Shift+Space too) creates a new operation. The focus stays on the
 * pressed button, so Escape there leaves an entered group through `onLeaveGroup`.
 */
export function OperationKindGroup({ kinds, onPress, onLeaveGroup, tabs }: OperationKindGroupProps) {
  const id = useId();
  return (
    <div
      role="group"
      aria-label="Operation"
      className="flex items-center gap-1 py-1"
      onKeyDown={(event) => {
        if (onLeaveGroup && leavesEnteredGroup(event.key, true)) {
          event.preventDefault();
          event.stopPropagation();
          onLeaveGroup();
        }
      }}
    >
      {kinds.map((button) => {
        const name = kindButtonName(button.kind);
        return (
          <Tooltip key={button.kind} side="bottom" offset={6} content={name}>
            <button
              type="button"
              aria-label={name}
              aria-describedby={`${id}-${button.kind}`}
              onClick={(event) => onPress(button.kind, event.shiftKey)}
              className="flex size-7 shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]"
            >
              <OperationGlyph kind={button.kind} />
              <span id={`${id}-${button.kind}`} className="sr-only">
                {button.description}
              </span>
            </button>
          </Tooltip>
        );
      })}
      {tabs ? (
        <Tooltip side="bottom" offset={6} content="Tabs">
          <Toggle.Root
            pressed={tabs.on}
            onPressedChange={tabs.onToggle}
            aria-label="Tabs"
            className="ml-1 flex size-7 shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] aria-pressed:hover:bg-[var(--toolbar-icon-active-bg)]"
          >
            <TabsGlyph pressed={tabs.on} />
          </Toggle.Root>
        </Tooltip>
      ) : null}
    </div>
  );
}

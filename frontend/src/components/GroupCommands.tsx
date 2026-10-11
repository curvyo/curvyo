import { useRef, useState } from "react";

import { CommandButton } from "@/components/CommandButton";
import { GroupGlyph } from "@/components/GroupGlyphs";
import { COLUMN_A_TOOLTIP_OFFSET_PX, RAIL_CARD_CLASS, RAIL_CARD_STYLE } from "@/components/railCard";
import type { GroupingCommands as GroupingCommandsState } from "@/hooks/useGrouping";
import {
  GROUP_COMMANDS,
  groupCommandDimmed,
  groupCommandName,
  groupCommandNote,
  groupCommandRule,
} from "@/lib/groupText";

interface GroupCommandsProps {
  commands: GroupingCommandsState;
  /** Whether the Select tool is active: with another tool the buttons are dimmed and the
   * tooltip says which tool to take. */
  selectTool: boolean;
  /** Whether this is macOS (the tooltips write Cmd). */
  mac: boolean;
  /** Called after a mouse press, so the tool letters keep working. */
  onReturnFocus: () => void;
}

/** The index a navigation key moves the focus to (no wrap), or `null` for any other key. */
function targetIndex(key: string, index: number): number | null {
  const last = GROUP_COMMANDS.length - 1;
  switch (key) {
    case "ArrowDown":
      return Math.min(index + 1, last);
    case "ArrowUp":
      return Math.max(index - 1, 0);
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return null;
  }
}

/**
 * The Grouping card of the tool rail (`docs/design-system.md`, row "Group toolbox"): the second
 * card of column A, Group above Ungroup, commands and not tools. One Tab stop with roving focus.
 * It shows what the session says is available and decides nothing; its notice is the canvas
 * notice slot's.
 */
export function GroupCommands({ commands, selectTool, mac, onReturnFocus }: GroupCommandsProps) {
  const { availability, apply } = commands;
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const [stop, setStop] = useState(0);

  return (
    <div
      role="toolbar"
      aria-label="Grouping"
      aria-orientation="vertical"
      className={RAIL_CARD_CLASS}
      style={RAIL_CARD_STYLE}
    >
      {GROUP_COMMANDS.map((command, index) => (
        <CommandButton
          key={command}
          ref={(element) => {
            buttons.current[index] = element;
          }}
          name={groupCommandName(command, mac)}
          rule={groupCommandRule(command)}
          note={groupCommandNote(command, availability, selectTool)}
          glyph={<GroupGlyph command={command} />}
          dimmed={groupCommandDimmed(command, availability)}
          tooltipOffset={COLUMN_A_TOOLTIP_OFFSET_PX}
          tabIndex={index === stop ? 0 : -1}
          pressed={false}
          onFocus={() => setStop(index)}
          onKeyDown={(event) => {
            const target = targetIndex(event.key, index);
            if (target !== null) {
              event.preventDefault();
              buttons.current[target]?.focus();
            }
          }}
          onActivate={(byMouse) => {
            apply(command);
            if (byMouse) {
              onReturnFocus();
            }
          }}
        />
      ))}
    </div>
  );
}

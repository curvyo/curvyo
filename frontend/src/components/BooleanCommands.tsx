import { useRef, useState } from "react";

import { BooleanGlyph } from "@/components/BooleanGlyphs";
import { CommandButton } from "@/components/CommandButton";
import {
  COLUMN_B_TOOLTIP_OFFSET_PX,
  RAIL_CARD_CLASS,
  RAIL_CARD_STYLE,
} from "@/components/railCard";
import type { BooleanCommands as BooleanCommandsState } from "@/hooks/useRailCommands";
import { operationName, operationRule, tooltipNote } from "@/lib/booleanText";
import { BRIDGE_NAME, BRIDGE_RULE, bridgeTooltipNote } from "@/lib/bridgeText";
import {
  type BooleanCardCommand,
  booleanCardCommands,
  isStackCommand,
  stackName,
  stackRule,
} from "@/lib/stackText";
import { WELD_NAME, WELD_RULE, weldTooltipNote } from "@/lib/weldText";

/** A button's name line. */
function buttonName(op: BooleanCardCommand): string {
  if (op === "bridge") return BRIDGE_NAME;
  if (op === "weld") return WELD_NAME;
  return isStackCommand(op) ? stackName(op) : operationName(op);
}

/** A button's rule line. */
function buttonRule(op: BooleanCardCommand): string {
  if (op === "bridge") return BRIDGE_RULE;
  if (op === "weld") return WELD_RULE;
  return isStackCommand(op) ? stackRule(op) : operationRule(op);
}

interface BooleanCommandsProps {
  commands: BooleanCommandsState;
  /** Whether the Select tool is active: with another tool the buttons are dimmed and the
   * tooltip says which tool to take. */
  selectTool: boolean;
  /** Called after a mouse press, so the tool letters keep working; a key press leaves the
   * focus on the button (`0016` criterion 22a). */
  onReturnFocus: () => void;
}

/** The index a navigation key moves the focus to (no wrap) among `count` buttons, or `null` for
 * any other key. */
function targetIndex(key: string, index: number, count: number): number | null {
  const last = count - 1;
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
 * The Boolean toolbox of the tool rail (`docs/design-system.md`, rows "Boolean toolbox" and
 * "Toolbox card"): a card of its own below the Path card in column B, eight commands, not tools,
 * one column: the five operations, then Fracture and Flatten (`specs/0037-fracture-and-flatten/`),
 * then Weld (`specs/0068-laserable-lettering/`), which has its own availability rule, and on a
 * laser page Bridge (`specs/0156-fused-lettering/`), absent elsewhere.
 * One Tab stop with roving focus. It shows what the session says is available and
 * decides nothing. Its notice is drawn by the rail (`BooleanNotice`).
 */
export function BooleanCommands({ commands, selectTool, onReturnFocus }: BooleanCommandsProps) {
  const { availability, weldAvailability, bridgeAvailability, busy, apply } = commands;
  const list = booleanCardCommands(bridgeAvailability !== "absent");
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const [stop, setStop] = useState(0);
  const [pressed, setPressed] = useState<BooleanCardCommand | null>(null);

  return (
    <div
      role="toolbar"
      aria-label="Boolean operations"
      aria-orientation="vertical"
      aria-busy={busy}
      className={RAIL_CARD_CLASS}
      style={RAIL_CARD_STYLE}
    >
      {list.map((op, index) => (
        <CommandButton
          key={op}
          ref={(element) => {
            buttons.current[index] = element;
          }}
          name={buttonName(op)}
          rule={buttonRule(op)}
          note={
            op === "bridge"
              ? bridgeTooltipNote(bridgeAvailability, selectTool)
              : op === "weld"
                ? weldTooltipNote(weldAvailability, selectTool)
                : tooltipNote(availability, selectTool)
          }
          glyph={<BooleanGlyph op={op} />}
          dimmed={
            op === "bridge"
              ? bridgeAvailability !== "ready"
              : op === "weld"
                ? weldAvailability.needsOne
                : availability.needsTwo
          }
          tooltipOffset={COLUMN_B_TOOLTIP_OFFSET_PX}
          tabIndex={index === stop ? 0 : -1}
          pressed={busy && pressed === op}
          onFocus={() => setStop(index)}
          onKeyDown={(event) => {
            const target = targetIndex(event.key, index, list.length);
            if (target !== null) {
              event.preventDefault();
              buttons.current[target]?.focus();
            }
          }}
          onActivate={(byMouse) => {
            if (busy) {
              return;
            }
            setPressed(op);
            apply(op);
            if (byMouse) {
              onReturnFocus();
            }
          }}
        />
      ))}
    </div>
  );
}

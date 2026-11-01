import {
  Circle as CircleIcon,
  MousePointer2,
  MousePointer as SelectIcon,
  PenTool as PenToolIcon,
  RulerDimensionLine,
  Square,
  Star,
  Type,
} from "lucide-react";
import { Tooltip } from "radix-ui";

import { BooleanCommands } from "@/components/BooleanCommands";
import { BooleanNotice } from "@/components/BooleanNotice";
import { GroupCommands } from "@/components/GroupCommands";
import { ImportCard } from "@/components/ImportCard";
import { PathCommands } from "@/components/PathCommands";
import {
  COLUMN_A_TOOLTIP_OFFSET_PX,
  RAIL_CARD_CLASS,
  RAIL_CARD_STYLE,
} from "@/components/railCard";
import type { GroupingCommands } from "@/hooks/useGrouping";
import { MAC } from "@/hooks/useHistory";
import type { RailCommands } from "@/hooks/useRailCommands";
import type { Tool } from "@/hooks/useEditorSession";
import { SELECT_KEYS_TOOLTIP } from "@/lib/nudgeText";

interface ToolRailProps {
  tool: Tool;
  /** How many objects are selected; with the Select tool active and one or
   * more selected the plain letters R and S act on the selection, so the
   * Rectangle tool is reached by "Esc, R" (`specs/0010-edit-interaction-polish/`
   * criterion 62). */
  selectionCount: number;
  onSelect: (tool: Tool) => void;
  /** Called after a mouse click on a button, so the tool letters keep working
   * (keyboard activation keeps focus on the rail for Tab navigation). */
  onReturnFocus: () => void;
  /** The Path card (`specs/0035-combine-and-break-apart/`) and the Boolean card below it
   * (`specs/0016-boolean-operations/`) in column B, and the notice both share. */
  commands: RailCommands;
  /** The Grouping card below the tools card in column A (`specs/0023-groups/`). */
  grouping: GroupingCommands;
  /** A document preview pauses editing: the rail is dimmed and inert (`0042` criterion 19). */
  paused?: boolean;
  /** The Import button below the Path tools card (`specs/0058-bitmap-import` criterion 1). */
  onImport: () => void;
}

/** The muted last tooltip line of the creation tools (`specs/0052-click-to-create-shapes/`
 * criterion 29). */

interface ToolButtonProps {
  tool: Tool;
  active: boolean;
  label: string;
  shortcut: string;
  /** An optional second tooltip line (the modifiers of the tool). */
  hint?: string;
  /** An optional muted last line (the keys that act on the tool's selection). */
  keys?: string;
  icon: React.ReactNode;
  onSelect: (tool: Tool) => void;
  onReturnFocus: () => void;
  /** The accessible name, if not "label (shortcut)". */
  name?: string;
}

/**
 * One tool rail button: a real focusable control, tab-reachable, with
 * an `aria-label` and a hover tooltip naming the shortcut
 * (`specification.md`'s UX notes, "Tool rail and tool switching").
 */
function ToolButton({
  tool,
  active,
  label,
  shortcut,
  hint,
  keys,
  icon,
  onSelect,
  onReturnFocus,
  name,
}: ToolButtonProps) {
  return (
    <Tooltip.Root delayDuration={400}>
      <Tooltip.Trigger asChild>
        <button
          type="button"
          aria-label={name ?? `${label} (${shortcut})`}
          aria-pressed={active}
          onClick={(event) => {
            onSelect(tool);
            // `detail` is 0 for keyboard activation, 1 or more for a mouse click.
            if (event.detail > 0) {
              onReturnFocus();
            }
          }}
          className="flex size-10 items-center justify-center rounded-md outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]"
          style={{
            background: active ? "var(--toolbar-icon-active-bg)" : "transparent",
            color: active ? "var(--toolbar-icon-active-fg)" : "var(--toolbar-icon)",
          }}
        >
          {icon}
        </button>
      </Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content
          side="right"
          sideOffset={COLUMN_A_TOOLTIP_OFFSET_PX}
          className="z-50 rounded-md bg-popover px-2 py-1 text-xs text-popover-foreground ring-1 ring-foreground/10"
        >
          <div>
            {label} ({shortcut})
          </div>
          {hint && <div>{hint}</div>}
          {keys && <div className="text-muted-foreground">{keys}</div>}
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}

/** The Offset tool's glyph: a solid rounded square inside a dashed concentric outline (the result
 * around the source). */
function OffsetGlyph() {
  return (
    <svg width={20} height={20} viewBox="0 0 20 20" fill="none" aria-hidden>
      <rect x={6} y={6} width={8} height={8} rx={1.5} fill="currentColor" />
      <rect
        x={2}
        y={2}
        width={16}
        height={16}
        rx={4}
        stroke="currentColor"
        strokeWidth={1.5}
        strokeDasharray="3 2"
      />
    </svg>
  );
}

/**
 * The tool rail (`docs/design-system.md`, rows "Tool rail architecture", "Toolbox card",
 * "Group toolbox", "Boolean toolbox" and "Path toolbox"): two columns of cards that float over
 * the canvas, inset 12px from the left and top edges (`--rail-inset`), each 48px wide and 8px apart
 * (`--rail-right` is the right edge of the second): column A holds the tools card and, 8px below
 * it, the Grouping card; column B holds the Path card and, 8px below it, the Boolean card. Select
 * is first, a deliberate, one-time exception to "new tools append in ship order" (acceptance
 * criterion 12), then Pen, Node, Rectangle, Ellipse, Polygon-star. The canvas is never resized or
 * repositioned to make room for it. Column A ends with the Path tools card (the Offset tool) and
 * the Import card (`specs/0058-bitmap-import`).
 *
 * The root keeps 4px clear under the columns; it passes no pointer event through to itself, so
 * the canvas around the cards stays usable. Column B, the taller one, is 500px high with the
 * seven-button Boolean card, which fits the 546px viewport of the smallest window.
 */
export function ToolRail({
  tool,
  selectionCount,
  onSelect,
  onReturnFocus,
  commands,
  grouping,
  paused = false,
  onImport,
}: ToolRailProps) {
  // The one state that changes a letter: the Select tool with a selection.
  const letterActsOnSelection = tool === "select" && selectionCount > 0;
  return (
    <Tooltip.Provider>
      <div
        inert={paused}
        className={`pointer-events-none absolute top-3 right-3 bottom-1 left-3 z-30 flex gap-2 ${paused ? "opacity-40" : ""}`}
      >
        <div className="flex min-h-0 w-12 flex-col gap-2">
          <div className={RAIL_CARD_CLASS} style={RAIL_CARD_STYLE}>
            <ToolButton
              tool="select"
              active={tool === "select"}
              label="Select tool"
              shortcut="S or Esc"
              hint="Drag a box. Shift: add. Ctrl: remove. Alt: invert, lasso, cycle"
              keys={SELECT_KEYS_TOOLTIP}
              icon={<SelectIcon size={20} />}
              onSelect={onSelect}
              onReturnFocus={onReturnFocus}
            />
            <ToolButton
              tool="pen"
              active={tool === "pen"}
              label="Pen tool"
              shortcut="B"
              icon={<PenToolIcon size={20} />}
              onSelect={onSelect}
              onReturnFocus={onReturnFocus}
            />
            <ToolButton
              tool="node"
              active={tool === "node"}
              label="Node tool"
              shortcut="N"
              hint="Drag a segment to bend it. Shift: one axis"
              icon={<MousePointer2 size={20} />}
              onSelect={onSelect}
              onReturnFocus={onReturnFocus}
            />
            <ToolButton
              tool="rectangle"
              active={tool === "rectangle"}
              label="Rectangle tool"
              shortcut={letterActsOnSelection ? "Esc, R" : "R"}
              hint="Shift: from center. Ctrl: square or circle"
              icon={<Square size={20} />}
              onSelect={onSelect}
              onReturnFocus={onReturnFocus}
            />
            <ToolButton
              tool="ellipse"
              active={tool === "ellipse"}
              label="Ellipse tool"
              shortcut="E"
              hint="Shift: from center. Ctrl: square or circle"
              icon={<CircleIcon size={20} />}
              onSelect={onSelect}
              onReturnFocus={onReturnFocus}
            />
            <ToolButton
              tool="polygon-star"
              active={tool === "polygon-star"}
              label="Polygon/star tool"
              shortcut="*"
              icon={<Star size={20} />}
              onSelect={onSelect}
              onReturnFocus={onReturnFocus}
            />
            {/* The next free slot of the Tools card (`specs/0074-text-objects` criterion 1):
                T is a tool letter in every state. */}
            <ToolButton
              tool="text"
              active={tool === "text"}
              label="Text tool"
              shortcut="T"
              icon={<Type size={20} />}
              onSelect={onSelect}
              onReturnFocus={onReturnFocus}
            />
          </div>
          <GroupCommands
            commands={grouping}
            selectTool={tool === "select"}
            mac={MAC}
            onReturnFocus={onReturnFocus}
          />
          {/* The Path tools card (`specs/0038-path-offset` criterion 1): tools, not commands. */}
          <div
            role="toolbar"
            aria-label="More tools"
            aria-orientation="vertical"
            className={RAIL_CARD_CLASS}
            style={RAIL_CARD_STYLE}
          >
            <ToolButton
              tool="offset"
              active={tool === "offset"}
              label="Offset tool"
              name="Offset tool"
              shortcut="O"
              hint="Drag a corner handle. Shift: both sides. Ctrl: new object."
              icon={<OffsetGlyph />}
              onSelect={onSelect}
              onReturnFocus={onReturnFocus}
            />
            {/* The Measure tool (`specs/0070-measure-and-dimension` criterion 2): M, as R, opens
                the move entry in the Select tool with a selection. */}
            <ToolButton
              tool="measure"
              active={tool === "measure"}
              label="Measure tool"
              name={`Measure tool (${letterActsOnSelection ? "Esc, M" : "M"})`}
              shortcut={letterActsOnSelection ? "Esc, M" : "M"}
              icon={<RulerDimensionLine size={20} />}
              onSelect={onSelect}
              onReturnFocus={onReturnFocus}
            />
          </div>
          <ImportCard primary={MAC ? "Cmd" : "Ctrl"} onImport={onImport} />
        </div>
        {/* Column B: the Path card and, 8px below it, the Boolean card. */}
        <div className="flex w-12 flex-col gap-2">
          <PathCommands
            commands={commands.path}
            selectTool={tool === "select"}
            onReturnFocus={onReturnFocus}
          />
          <BooleanCommands
            commands={commands.booleans}
            selectTool={tool === "select"}
            onReturnFocus={onReturnFocus}
          />
        </div>
        <BooleanNotice notice={commands.notice} />
      </div>
    </Tooltip.Provider>
  );
}

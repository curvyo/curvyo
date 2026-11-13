import type { ReactNode } from "react";

import { ToolRail } from "@/components/ToolRail";
import { SketchToolRail } from "@/components/sketch/SketchTools";
import type { EditorSession } from "@/hooks/useEditorSession";
import type { RailCommands } from "@/hooks/useRailCommands";
import type { SketchApi } from "@/hooks/useSketch";
import type { WorkbenchesApi } from "@/hooks/useWorkbenches";
import { registerWorkbench } from "@/workbenches/registry";

/** What the app hands every workbench's drawing. */
export interface WorkbenchContext {
  editor: EditorSession;
  sketch: SketchApi;
  workbenches: WorkbenchesApi;
  railCommands: RailCommands;
  /** A document preview pauses the canvas tools (`specs/0042-history-branches`). */
  paused: boolean;
  onImport: () => void;
  /** The drawing's bars and canvas notice, which the vector tools float over the canvas. */
  drawingOverlay: ReactNode;
}

/** The vector tools: the tool rail, bars and canvas notice. */
function vectorRail(ctx: WorkbenchContext) {
  return (
    <ToolRail
      tool={ctx.editor.tool}
      selectionCount={ctx.editor.selectionCount}
      onSelect={ctx.editor.setTool}
      onReturnFocus={() => ctx.editor.containerRef.current?.focus()}
      moveOnly={ctx.editor.getSession()?.selection_move_only() ?? false}
      commands={ctx.railCommands}
      grouping={ctx.editor.grouping.commands}
      paused={ctx.paused}
      onImport={ctx.onImport}
      testGridAvailable={ctx.editor.getSession()?.test_grid_tool_available() ?? false}
    />
  );
}

/** A drawing page (and an unknown kind or a laser page whose extension is off). */
registerWorkbench<WorkbenchContext, ReactNode>({
  kind: "vector",
  rail: vectorRail,
  overlay: (ctx) => ctx.drawingOverlay,
});

/** A sketch page (ADR 0026): its own tool rail, no drawing bars. */
registerWorkbench<WorkbenchContext, ReactNode>({
  kind: "sketch",
  rail: (ctx) => <SketchToolRail sketch={ctx.sketch} paused={ctx.paused} />,
});

/** An output page of any process (ADR 0042 §7): the vector tools, Text included; its machine data
 * is in the Document tab. */
registerWorkbench<WorkbenchContext, ReactNode>({
  kind: "manufacturing",
  rail: vectorRail,
  overlay: (ctx) => ctx.drawingOverlay,
});

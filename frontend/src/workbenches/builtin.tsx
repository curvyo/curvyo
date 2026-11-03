import type { ReactNode } from "react";

import { ToolRail } from "@/components/ToolRail";
import { SketchToolRail } from "@/components/sketch/SketchTools";
import { ManifestToolbox } from "@/components/workbench/ManifestToolbox";
import { ReadOnlyNotice } from "@/components/workbench/ReadOnlyNotice";
import type { EditorSession } from "@/hooks/useEditorSession";
import type { RailCommands } from "@/hooks/useRailCommands";
import type { SketchApi } from "@/hooks/useSketch";
import type { WorkbenchesApi } from "@/hooks/useWorkbenches";
import { contributionOf, registerWorkbench, type WorkbenchListEntry } from "@/workbenches/registry";

/** What the app hands every workbench's drawing. */
export interface WorkbenchContext {
  editor: EditorSession;
  sketch: SketchApi;
  workbenches: WorkbenchesApi;
  railCommands: RailCommands;
  /** A document preview pauses the canvas tools (`specs/0042-history-branches`). */
  paused: boolean;
  onImport: () => void;
  /** The drawing's bars and canvas notice, which the Vector workbench floats over the canvas. */
  drawingOverlay: ReactNode;
}

/** The Vector workbench: today's tool rail, bars and canvas notice. */
registerWorkbench<WorkbenchContext, ReactNode>({
  kind: "vector",
  icon: () => "PenTool",
  rail: (ctx) => (
    <ToolRail
      tool={ctx.editor.tool}
      selectionCount={ctx.editor.selectionCount}
      onSelect={ctx.editor.setTool}
      onReturnFocus={() => ctx.editor.containerRef.current?.focus()}
      commands={ctx.railCommands}
      grouping={ctx.editor.grouping.commands}
      paused={ctx.paused}
      onImport={ctx.onImport}
    />
  ),
  overlay: (ctx) => ctx.drawingOverlay,
});

/** The Sketch workbench (ADR 0026): its own tool rail, no drawing bars. */
registerWorkbench<WorkbenchContext, ReactNode>({
  kind: "sketch",
  icon: () => "DraftingCompass",
  rail: (ctx) => <SketchToolRail sketch={ctx.sketch} paused={ctx.paused} />,
});

/** The generic manifest workbench (ADR 0032 §3): any manufacturing of any process, drawn from its
 * extension's manifest. */
registerWorkbench<WorkbenchContext, ReactNode>({
  kind: "manufacturing",
  icon: (entry: WorkbenchListEntry) => entry.icon ?? "Factory",
  rail: (ctx) => {
    const active = ctx.workbenches.list.entries.find((e) => e.id === ctx.workbenches.list.active);
    const contribution = contributionOf(ctx.workbenches.list, active?.process);
    return contribution ? (
      <ManifestToolbox contribution={contribution} onFitPage={ctx.workbenches.fitPage} paused={ctx.paused} />
    ) : null;
  },
  overlay: (ctx) => <ReadOnlyNotice notice={ctx.workbenches.view?.read_only_notice ?? null} />,
});

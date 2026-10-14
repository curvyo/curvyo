import { TooltipProvider } from "@/components/ui/tooltip";
import { useState } from "react";

import { Canvas } from "@/components/Canvas";
import { CanvasNotice } from "@/components/CanvasNotice";
import { ErrorDialog } from "@/components/ErrorDialog";
import { NodeToolbar } from "@/components/NodeToolbar";
import { PropertiesPanel } from "@/components/panel/PropertiesPanel";
import { Rulers } from "@/components/Rulers";
import { SelectToolbar } from "@/components/SelectToolbar";
import { ShapeToolbar } from "@/components/ShapeToolbar";
import { StatusBar } from "@/components/StatusBar";
import { ToolRail } from "@/components/ToolRail";
import { HistoryPreviewFrame, HistoryPreviewLine } from "@/components/HistoryPreview";
import { usePreviewState } from "@/lib/historyPreviewStore";
import { useRailCommands } from "@/hooks/useRailCommands";
import { useEditorSession } from "@/hooks/useEditorSession";
import { useClosePathState } from "@/hooks/usePenCue";
import { useDocumentPanel } from "@/hooks/useDocumentPanel";
import { usePanelOpen } from "@/hooks/usePanelOpen";
import { useProject } from "@/hooks/useProject";
import { RULER_THICKNESS_PX } from "@/lib/rulerDraw";
import { nodeCountText } from "@/lib/nodeText";
import { isTauri, unsupportedReason } from "@/platform/host";

/** The editor, or the reason this browser cannot run it. */
function App() {
  const [problem] = useState(() => (isTauri ? null : unsupportedReason()));
  if (problem) {
    return (
      <div role="alert" className="flex h-screen w-screen items-center justify-center p-8 text-center text-destructive">
        {problem}
      </div>
    );
  }
  return <Editor />;
}

function Editor() {
  const [cursorMm, setCursorMm] = useState({ x: 0, y: 0 });
  const editor = useEditorSession(setCursorMm);
  const historyPreview = usePreviewState();
  const railCommands = useRailCommands(editor);
  const closePathState = useClosePathState(editor);
  const documentPanel = useDocumentPanel(editor);
  const panelOpen = usePanelOpen(editor.getSession);
  const [openErrorMessage, setOpenErrorMessage] = useState<string | null>(
    null,
  );
  const [saveErrorMessage, setSaveErrorMessage] = useState<string | null>(
    null,
  );
  const project = useProject({
    editor,
    panelOpen,
    panel: documentPanel,
    onOpenError: setOpenErrorMessage,
    onSaveError: setSaveErrorMessage,
  });

  return (
    // The one tooltip provider of the app: every `Tooltip` and `SliderTip` below needs it, and a
    // component that renders one without it throws (`frontend/tests/tooltipProvider.test.ts`).
    <TooltipProvider>
    <div className="flex h-screen w-screen flex-col overflow-hidden">
      <div className="flex min-h-0 flex-1">
        {/* The canvas region, a grid of the rulers' corner square, the two
         * ruler strips and the viewport (`specs/0015-document-size-and-rulers`):
         * the rulers shrink the viewport and are not drawn over it. */}
        <div
          className="grid min-h-0 min-w-0 flex-1"
          style={{
            gridTemplateColumns: `${RULER_THICKNESS_PX}px minmax(0, 1fr)`,
            gridTemplateRows: `${RULER_THICKNESS_PX}px minmax(0, 1fr)`,
          }}
        >
        <Rulers editor={editor} unit={documentPanel.view.unit} />
        {/* The viewport: the rail and the bars' overlay row are anchored to
         * it, not to the window, so no bar is ever drawn under the
         * properties panel on its right (`0007` criterion 39) or over a
         * ruler. */}
        <div className="relative flex min-h-0 min-w-0">
        <ToolRail
          tool={editor.tool}
          selectionCount={editor.selectionCount}
          onSelect={editor.setTool}
          onReturnFocus={() => editor.containerRef.current?.focus()}
          commands={railCommands}
          grouping={editor.grouping.commands}
          paused={historyPreview.kind === "document"}
        />
        <Canvas editor={editor} />
        <HistoryPreviewFrame />
        {/* Contextual tool bar: floats over the canvas, right of the tool
         * rail, so showing/hiding it never resizes the canvas
         * (`docs/design-system.md`, "no layout shift on tool switch"). */}
        <div
          inert={historyPreview.kind === "document"}
          className={`pointer-events-none absolute top-3 right-3 left-[calc(var(--rail-right)+12px)] z-20 flex ${
            historyPreview.kind === "document" ? "opacity-40" : ""
          } ${
            // The Select bar is left-aligned so its two switches never move
            // when the selection changes; the Node and Shape bars stay
            // centred (`docs/design-system.md`, "Select bar layout").
            editor.tool === "select" ? "justify-start" : "justify-center"
          }`}
        >
          {editor.tool === "select" ? (
            <SelectToolbar
              scaleStrokeWidth={editor.scaleStrokeWidth}
              onSetScaleStrokeWidth={editor.setScaleStrokeWidth}
              scaleCornerRadius={editor.scaleCornerRadius}
              onSetScaleCornerRadius={editor.setScaleCornerRadius}
              linkCorners={editor.linkCorners}
              onSetLinkCorners={editor.setLinkCorners}
              bar={editor.selectBar}
              onSetRadius={editor.setSelectedRadius}
              onRemoveRounding={editor.removeCornerRounding}
              onSetPointCount={editor.setSelectedPointCount}
              onPreviewRatio={editor.previewSelectedRatio}
              onCommitRatio={editor.commitSelectedRatio}
              onSetRatio={editor.setSelectedRatio}
              onCancelRatio={editor.cancelSelectedRatio}
              onConvertToPaths={editor.convertSelectedToPaths}
              linkedCloneShown={editor.clones.shown}
              onLinkedClone={() => editor.clones.apply("linked-clone")}
              linkedCloneNotice={editor.clones.barNotice}
              onReturnFocus={() => editor.containerRef.current?.focus()}
            />
          ) : null}
          {editor.tool === "node" ? (
            <NodeToolbar
              closePath={{
                state: closePathState,
                onClose: editor.closePath,
                onReturnFocus: () => editor.containerRef.current?.focus(),
                tool: editor.tool,
                selectionCount: editor.selectionCount,
              }}
              state={editor.nodeToolbarState}
              actions={{
                insertSelected: editor.insertSelected,
                deleteSelected: editor.deleteSelected,
                convertSelected: editor.convertSelected,
                makeLine: editor.makeLine,
                makeCurve: editor.makeCurve,
                joinSelected: editor.joinSelected,
                splitSelected: editor.splitSelected,
              }}
            />
          ) : null}
          {editor.tool === "polygon-star" ? (
            <ShapeToolbar
              tool={editor.tool}
              polyStarMode={editor.polyStarMode}
              polyStarPointCount={editor.polyStarPointCount}
              polyStarRatio={editor.polyStarRatio}
              onSetPolyStarMode={editor.setPolyStarMode}
              onSetPolyStarPointCount={editor.setPolyStarPointCount}
              onSetPolyStarRatio={editor.setPolyStarRatio}
              onReturnFocus={() => editor.containerRef.current?.focus()}
            />
          ) : null}
        </div>
        {/* After the bars in the tab order: canvas, bars, then the indicator's buttons. */}
        <CanvasNotice
          notice={editor.canvasNotice}
          groupDepth={editor.grouping.depth}
          onLeaveToLevel={(level) => {
            editor.grouping.leaveToLevel(level);
            // The pressed crumb unmounts; keep the keyboard on the canvas.
            editor.containerRef.current?.focus();
          }}
          announcement={editor.grouping.announcement}
          persistent={<HistoryPreviewLine getSession={editor.getSession} />}
        />
        </div>
        </div>
        <PropertiesPanel
          editor={editor}
          document={documentPanel}
          panelOpen={panelOpen}
          project={project}
        />
      </div>
      <StatusBar
        cursorText={documentPanel.cursorText(cursorMm.x, cursorMm.y)}
        sizeText={documentPanel.view.sizeText}
        zoomPercent={editor.zoomPercent}
        nodeCountText={editor.tool === "node" ? nodeCountText(editor.nodeToolbarState) : null}
      />
      <ErrorDialog
        title="Can't open project"
        message={openErrorMessage}
        onDismiss={() => setOpenErrorMessage(null)}
      />
      <ErrorDialog
        title="Can't save project"
        message={saveErrorMessage}
        onDismiss={() => setSaveErrorMessage(null)}
      />
    </div>
    </TooltipProvider>
  );
}

export default App;

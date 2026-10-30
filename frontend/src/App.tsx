import { useSketch } from "@/hooks/useSketch";
import { SketchToolRail, SketchToolbar } from "@/components/sketch/SketchTools";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useCallback, useEffect, useState } from "react";

import { Canvas } from "@/components/Canvas";
import { CanvasArea } from "@/components/CanvasArea";
import { FontPreview } from "@/components/fontPreview/FontPreview";
import { useFontPreview } from "@/hooks/useFontPreview";
import { CanvasNotice } from "@/components/CanvasNotice";
import { ErrorDialog } from "@/components/ErrorDialog";
import { NodeToolbar } from "@/components/NodeToolbar";
import { PropertiesPanel } from "@/components/panel/PropertiesPanel";
import { Rulers } from "@/components/Rulers";
import { SelectToolbar } from "@/components/SelectToolbar";
import { ShapeToolbar } from "@/components/ShapeToolbar";
import { TextBar } from "@/components/TextBar";
import { TraceBar } from "@/components/TraceBar";
import { useTraceInfo } from "@/hooks/useTraceInfo";
import { focusCurrentFont } from "@/lib/textText";
import { readRow2Open, writeRow2Open } from "@/lib/textBarRows";
import { setPreviewOpen } from "@/lib/fontPreviewStore";
import { CursorStatus } from "@/components/CursorStatus";
import { createCursorPosition } from "@/lib/cursorPosition";
import { ToolRail } from "@/components/ToolRail";
import { HistoryPreviewFrame, HistoryPreviewLine } from "@/components/HistoryPreview";
import { usePreviewState } from "@/lib/historyPreviewStore";
import { useRailCommands } from "@/hooks/useRailCommands";
import { useEditorSession } from "@/hooks/useEditorSession";
import { useClosePathState } from "@/hooks/usePenCue";
import { useDocumentPanel } from "@/hooks/useDocumentPanel";
import { usePanelOpen } from "@/hooks/usePanelOpen";
import { useProject } from "@/hooks/useProject";
import { useImageClipboard } from "@/hooks/useImageClipboard";
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
  const [cursor] = useState(createCursorPosition);
  const editor = useEditorSession(cursor.update);
  const sketch = useSketch(editor);
  const historyPreview = usePreviewState();
  const railCommands = useRailCommands(editor);
  const closePathState = useClosePathState(editor);
  const documentPanel = useDocumentPanel(editor);
  const panelOpen = usePanelOpen(editor.getSession);
  const setPanelOpen = panelOpen.setOpen;
  useEffect(() => {
    if (sketch.view.workbench === "sketch" && sketch.view.edit_constraint) setPanelOpen(true);
  }, [sketch.view.workbench, sketch.view.edit_constraint, setPanelOpen]);
  const traceInfo = useTraceInfo(panelOpen, sketch.view.workbench === "vector" && editor.tool === "trace");
  const [openError, setOpenError] = useState<{ message: string; title: string } | null>(null);
  const [saveErrorMessage, setSaveErrorMessage] = useState<string | null>(
    null,
  );
  // The Font button and Shift+Ctrl+T: the panel body, the Fonts tab, the current face row
  // (`specs/0074-text-objects` criterion 11).
  const showFonts = () => {
    panelOpen.setOpen(true);
    documentPanel.shortcut("fonts");
    focusCurrentFont();
  };
  // Row 2 of the Text controls, one per-user preference for the Text bar and the Select bar, read
  // once at startup (`specs/0084-text-bar-second-row` criterion 14).
  const [row2Open, setRow2Open] = useState(readRow2Open);
  const textRow2 = {
    open: row2Open,
    onToggle: () => {
      writeRow2Open(!row2Open);
      setRow2Open(!row2Open);
    },
  };
  const preview = useFontPreview({ node: editor.text.bar.sampleNode, sample: editor.text.bar.sample });
  const { getSession } = editor;
  const keepViewOrigin = useCallback(
    (delta: number) => getSession()?.keep_view_origin_for_panel_toggle(delta),
    [getSession],
  );
  const returnToText = () => {
    if (editor.text.editing) {
      editor.text.sinkRef.current?.focus({ preventScroll: true });
    } else {
      editor.containerRef.current?.focus();
    }
  };
  // The Glyphs button (`specs/0082-glyph-table` criterion 2): the pane in Glyphs mode, closed by
  // a second press; a mouse press hands the keyboard back, a key goes to the search field.
  const glyphsPressed = preview.store.open && preview.mode === "glyphs";
  const glyphs = {
    shown: editor.text.bar.sampleNode !== "",
    pressed: glyphsPressed,
    onPress: (byKeyboard: boolean) => {
      if (glyphsPressed) {
        preview.close();
      } else {
        preview.setMode("glyphs");
        setPreviewOpen(true, byKeyboard);
      }
      if (!byKeyboard) {
        returnToText();
      }
    },
  };
  // The On path button and groups (`specs/0080-text-on-path`), the same in both bars.
  const textPath = {
    bar: editor.text.bar.path,
    onPress: editor.text.path.pressOnPath,
    onRelease: editor.text.path.release,
  };
  const project = useProject({
    editor,
    panelOpen,
    panel: documentPanel,
    onOpenError: (message: string, title = "Can't open project") => setOpenError({ message, title }),
    onSaveError: setSaveErrorMessage,
  });
  // Ctrl+V and Ctrl+C for images (`specs/0087-image-clipboard-and-drop`).
  useImageClipboard(editor, project.pasteImage, project.pasteFiles);

  return (
    // The one tooltip provider of the app: every `Tooltip` and `SliderTip` below needs it, and a
    // component that renders one without it throws (`frontend/tests/tooltipProvider.test.ts`).
    <TooltipProvider>
    <div className="flex h-screen w-screen flex-col overflow-clip" data-scroll-lock="">
      <div className="flex min-h-0 flex-1">
        {/* The canvas area: the drawing, and the font preview beside it or over it
         * (`specs/0077-font-preview`). */}
        <CanvasArea
          keepViewOrigin={keepViewOrigin}
          coverDrawing={editor.coverCanvasRight}
          open={preview.store.open}
          wantFull={preview.wantFull}
          size={preview.size}
          toggleFull={preview.toggleFull}
          split={preview.split}
          setSplit={preview.setSplit}
          pane={(layout, switchView, resizing) => (
            <FontPreview
              preview={preview}
              layout={layout}
              resizing={resizing}
              onSwitchView={switchView}
              currentFace={editor.text.fonts.current?.index ?? null}
              onPick={editor.text.pickFamily}
              tileFace={editor.text.tileFace}
              onReturnFocus={returnToText}
              glyphSubject={editor.text.glyphSubject}
              onInsertGlyph={editor.text.insertGlyph}
            />
          )}
          drawing={
            <>
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
        {sketch.view.workbench === "sketch" ? <SketchToolRail sketch={sketch} paused={historyPreview.kind === "document"} /> : <ToolRail
          tool={editor.tool}
          selectionCount={editor.selectionCount}
          onSelect={editor.setTool}
          onReturnFocus={() => editor.containerRef.current?.focus()}
          commands={railCommands}
          grouping={editor.grouping.commands}
          paused={historyPreview.kind === "document"}
          onImport={() => project.importFiles()}
        />}
        <Canvas editor={editor} sketch={sketch} onDropFiles={project.importFiles} />
        <HistoryPreviewFrame />
        {/* Contextual tool bar: floats over the canvas, right of the tool
         * rail, so showing/hiding it never resizes the canvas
         * (`docs/design-system.md`, "no layout shift on tool switch"). */}
        <div
          data-bar-area
          inert={historyPreview.kind === "document"}
          className={`pointer-events-none absolute top-3 right-3 left-[calc(var(--rail-right)+12px)] z-20 flex ${
            historyPreview.kind === "document" ? "opacity-40" : ""
          } ${
            // The Select bar is left-aligned so its two switches never move
            // when the selection changes; the Node and Shape bars stay
            // centred (`docs/design-system.md`, "Select bar layout").
            editor.tool === "select" || editor.tool === "trace" ? "justify-start" : "justify-center"
          }`}
        >
          {sketch.view.workbench === "sketch" ? <SketchToolbar key={`${sketch.view.active}:${sketch.view.tool}`} sketch={sketch} /> : <>
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
              onConvertToPaths={editor.text.objectToPath}
              objectToPathNotice={editor.text.objectToPathNotice}
              textBar={editor.text.bar}
              glyphs={glyphs}
              textRow2={textRow2}
              onShowFonts={showFonts}
              onSetTextSize={editor.text.setSize}
              onPreviewTextSize={editor.text.previewSize}
              onPressTextWeight={editor.text.pressWeight}
              onPressTextItalic={editor.text.pressItalic}
              onSetTextAlign={editor.text.setAlign}
              onSetTextWritingMode={editor.text.setWritingMode}
              textPath={textPath}
              onSetTextOrientation={editor.text.setOrientation}
              strokeToPath={{
                shown: editor.flipEnabled,
                enabled: editor.selectBar.strokeToPathEnabled,
                onPress: editor.strokeToPath.apply,
                notice: editor.strokeToPath.notice,
              }}
              linkedCloneShown={editor.clones.shown}
              onLinkedClone={() => editor.clones.apply("linked-clone")}
              linkedCloneNotice={editor.clones.barNotice}
              traceShown={editor.trace.available}
              onTrace={() => editor.setTool("trace")}
              flipEnabled={editor.flipEnabled}
              flip={editor.flip}
              guideState={editor.guideState}
              onToggleGuide={editor.toggleGuide}
              arcBar={editor.arcBar}
              onReturnFocus={() => editor.containerRef.current?.focus()}
            />
          ) : null}
          {editor.tool === "trace" ? (
            <TraceBar
              trace={editor.trace}
              info={traceInfo}
              onImport={() => project.importFiles()}
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
              tool={editor.tool}
              selectionCount={editor.selectionCount}
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
          {editor.tool === "text" ? (
            <TextBar
              bar={editor.text.bar}
              onShowFonts={showFonts}
              onSetSize={editor.text.setSize}
              onPreviewSize={editor.text.previewSize}
              editing={editor.text.editing}
              onPressWeight={editor.text.pressWeight}
              onPressItalic={editor.text.pressItalic}
              onSetAlign={editor.text.setAlign}
              onSetWritingMode={editor.text.setWritingMode}
              path={textPath}
              onSetOrientation={editor.text.setOrientation}
              glyphs={glyphs}
              row2={textRow2}
              onReturnFocus={() => {
                if (editor.text.editing) {
                  editor.text.sinkRef.current?.focus({ preventScroll: true });
                } else {
                  editor.containerRef.current?.focus();
                }
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
          </>}
        </div>
        {/* After the bars in the tab order: canvas, bars, then the indicator's buttons. */}
        {sketch.view.workbench === "vector" && <CanvasNotice
          notice={editor.canvasNotice}
          groupDepth={editor.grouping.depth}
          onLeaveToLevel={(level) => {
            editor.grouping.leaveToLevel(level);
            // The pressed crumb unmounts; keep the keyboard on the canvas.
            editor.containerRef.current?.focus();
          }}
          announcement={editor.grouping.announcement}
          persistent={<HistoryPreviewLine getSession={editor.getSession} />}
        />}
        </div>
        </div>
            </>
          }
        />
        <PropertiesPanel
          sketch={sketch}
          editor={editor}
          document={documentPanel}
          panelOpen={panelOpen}
          project={project}
          traceInfo={traceInfo}
        />
      </div>
      <CursorStatus
        cursor={cursor}
        format={documentPanel.cursorText}
        sizeText={documentPanel.view.sizeText}
        zoomPercent={editor.zoomPercent}
        nodeCountText={editor.tool === "node" ? nodeCountText(editor.nodeToolbarState) : null}
      />
      <ErrorDialog
        title={openError?.title ?? "Can't open project"}
        message={openError?.message ?? null}
        onDismiss={() => setOpenError(null)}
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

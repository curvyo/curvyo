import { usePlaceOn } from "@/hooks/usePlaceOn";
import { useSketch } from "@/hooks/useSketch";
import { TooltipProvider } from "@/components/ui/tooltip";
import { type ReactNode, useCallback, useEffect, useState } from "react";

import { Canvas } from "@/components/Canvas";
import { CanvasArea } from "@/components/CanvasArea";
import { FontPreview } from "@/components/fontPreview/FontPreview";
import { useFontPreview } from "@/hooks/useFontPreview";
import { CanvasNotice } from "@/components/CanvasNotice";
import { ErrorDialog } from "@/components/ErrorDialog";
import { NodeToolbar } from "@/components/NodeToolbar";
import { PenModeBar } from "@/components/PenModeBar";
import { PropertiesPanel } from "@/components/panel/PropertiesPanel";
import { Rulers } from "@/components/Rulers";
import { CurveToBezierButton } from "@/components/CurveToBezierButton";
import { SelectToolbar } from "@/components/SelectToolbar";
import { WidthDoneButton } from "@/components/WidthDoneButton";
import { ShapeToolbar } from "@/components/ShapeToolbar";
import { TextBar } from "@/components/TextBar";
import { TraceBar } from "@/components/TraceBar";
import { TilingBar } from "@/components/TilingBar";
import { RecogniseBar } from "@/components/RecogniseBar";
import { useTraceInfo } from "@/hooks/useTraceInfo";
import { focusCurrentFont } from "@/lib/textText";
import { readRow2Open, writeRow2Open } from "@/lib/textBarRows";
import { setPreviewOpen } from "@/lib/fontPreviewStore";
import { createCursorPosition } from "@/lib/cursorPosition";
import { useWorkbenches } from "@/hooks/useWorkbenches";
import type { WorkbenchContext } from "@/workbenches/builtin";
import "@/workbenches/builtin";
import { activeKind, workbenchEntry } from "@/workbenches/registry";
import { HistoryPreviewFrame, HistoryPreviewLine } from "@/components/HistoryPreview";
import { usePreviewState } from "@/lib/historyPreviewStore";
import { useRailCommands } from "@/hooks/useRailCommands";
import { useEditorSession } from "@/hooks/useEditorSession";
import { useClosePathState } from "@/hooks/usePenCue";
import { useDocumentPanel } from "@/hooks/useDocumentPanel";
import { usePanelOpen } from "@/hooks/usePanelOpen";
import { useProject } from "@/hooks/useProject";
import { useWorkspace } from "@/hooks/useWorkspace";
import { useWorkspaceSync } from "@/hooks/useWorkspaceSync";
import { useMembership } from "@/hooks/useMembership";
import { useDevices } from "@/hooks/useDevices";
import { useImageClipboard } from "@/hooks/useImageClipboard";
import { useCollaboration } from "@/hooks/useCollaboration";
import { useFixtures } from "@/hooks/useFixtures";
import { useFixtureJobs } from "@/hooks/useFixtureJobs";
import { useOperations } from "@/hooks/useOperations";
import { CollaborationPill } from "@/components/CollaborationPill";
import { PresenceOverlay } from "@/components/PresenceOverlay";
import { RULER_THICKNESS_PX } from "@/lib/rulerDraw";
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
  const workbenches = useWorkbenches(editor);
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
  const placeOn = usePlaceOn(editor);
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
  const collab = useCollaboration(editor);
  const operations = useOperations(editor);
  // The laser fixtures of the active laser page (`specs/0102-laser-fixtures`).
  const fixtures = useFixtures(editor);
  // The fixture jobs of the active laser page (`specs/0103-fixture-jobs`).
  const fixtureJobs = useFixtureJobs(editor);
  // The workspace outlives the File tab: its scan and open document stay while other tabs show.
  const workspace = useWorkspace(editor, project);
  // Its sharing, joining and reconnecting (`specs/0095-workspace` criteria 25 to 33).
  const workspaceSync = useWorkspaceSync(editor, project, collab, workspace);
  // My devices: pairing, the device group and synced workspaces (`specs/0096-own-devices-sync`).
  const devices = useDevices(editor, collab, workspace, workspaceSync);
  // Roles, leave, add back and the notices (`specs/0097-membership-lifecycle`).
  const membership = useMembership(editor, project, collab, workspace, workspaceSync, devices);

  /** The Pen bar: centred, and at the right edge where the canvas is too narrow for it to clear
   * the page's name label at the top left (`specs/0110` UX review U7). */
  const PEN_BAR_PLACE = "justify-end min-[1100px]:justify-center";
  const workbench = workbenchEntry<WorkbenchContext, ReactNode>(activeKind(workbenches.list));
  const drawingOverlay = (
    <>
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
            editor.tool === "select" || editor.tool === "trace" ? "justify-start" : editor.tool === "pen" ? PEN_BAR_PLACE : "justify-center"
          } ${
            // The Select bar's chips (Curve to Bézier, Width) go to a second row when they do
            // not fit beside it, so they never squeeze it (`specs/0110` UX review U4).
            editor.tool === "select" ? "flex-wrap items-start gap-y-2" : ""
          }`}
        >
          {editor.tool === "select" && editor.recognise.open ? (
            <RecogniseBar recognise={editor.recognise} onReturnFocus={() => editor.containerRef.current?.focus()} />
          ) : null}
          {editor.tool === "select" && editor.tiling.open ? <TilingBar tiling={editor.tiling} /> : null}
          {/* A selected fixture has no Select bar: its handles and chips size and place it
              (`specs/0102-laser-fixtures` criterion 14, UX notes §6.4). */}
          {editor.tool === "select" && !editor.recognise.open && !editor.tiling.open && !fixtures.view?.selected ? (
            <SelectToolbar
              moveOnly={editor.getSession()?.selection_move_only() ?? false}
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
                shown: editor.flipEnabled && editor.selectBar.strokeToPathShown,
                enabled: editor.selectBar.strokeToPathEnabled,
                onPress: editor.strokeToPath.apply,
                notice: editor.strokeToPath.notice,
              }}
              linkedCloneShown={editor.clones.shown}
              onLinkedClone={() => editor.clones.apply("linked-clone")}
              linkedCloneNotice={editor.clones.barNotice}
              placeOn={placeOn}
              traceShown={editor.trace.available}
              onTrace={() => editor.setTool("trace")}
              recognise={{
                shown: editor.recognise.available,
                onEnter: editor.recognise.enter,
                notice: editor.recognise.barNotice,
              }}
              flipEnabled={editor.flipEnabled}
              flip={editor.flip}
              align={editor.align}
              guideState={editor.guideState}
              onToggleGuide={editor.toggleGuide}
              arcBar={editor.arcBar}
              tile={{ slot: editor.tiling.slot, onEnter: editor.tiling.enter, onRelease: editor.tiling.release }}
              onReturnFocus={() => editor.containerRef.current?.focus()}
            />
          ) : null}
          {editor.tool === "select" && !editor.recognise.open && !editor.tiling.open ? <CurveToBezierButton editor={editor} /> : null}
          {editor.tool === "select" || editor.tool === "pen" ? <WidthDoneButton editor={editor} /> : null}
          {editor.tool === "trace" ? (
            <TraceBar
              trace={editor.trace}
              info={traceInfo}
              onImport={() => project.importFiles()}
              onReturnFocus={() => editor.containerRef.current?.focus()}
            />
          ) : null}
          {editor.tool === "pen" ? <PenModeBar editor={editor} /> : null}
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
    </>
  );
  const workbenchContext: WorkbenchContext = {
    editor,
    sketch,
    workbenches,
    railCommands,
    paused: historyPreview.kind === "document",
    onImport: () => void project.importFiles(),
    drawingOverlay,
  };

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
        {workbench?.rail(workbenchContext)}
        <Canvas
          overOperationRow={operations.dropKey !== null}
          fitCue={fixtures.fitCue}
          editor={editor}
          sketch={sketch}
          onDropFiles={project.importFiles}
          overlay={<PresenceOverlay getSession={editor.getSession} />}
          showPageFrames
        />
        <CollaborationPill
          view={collab.view}
          onOpen={() => {
            panelOpen.setOpen(true);
            documentPanel.pressTab("share");
          }}
        />
        <HistoryPreviewFrame />
        {workbench?.overlay?.(workbenchContext)}
        </div>
        </div>
            </>
          }
        />
        <PropertiesPanel
          sketch={sketch}
          workbenches={workbenches}
          editor={editor}
          document={documentPanel}
          panelOpen={panelOpen}
          project={project}
          traceInfo={traceInfo}
          collab={collab}
          workspace={workspace}
          workspaceSync={workspaceSync}
          devices={devices}
          membership={membership}
          operations={operations}
          fixtures={fixtures}
          fixtureJobs={fixtureJobs}
        />
      </div>
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

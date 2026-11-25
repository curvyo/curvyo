import { WidthPointEntry, WidthWheelReadout } from "@/components/WidthPointEntry";
import type { FitCue } from "@/lib/fixtureRows";
import { GradientOverlay } from "@/components/GradientOverlay";
import { MeasureOverlay } from "@/components/MeasureOverlay";
import type { GradientApi } from "@/hooks/useGradient";
import type { SketchApi } from "@/hooks/useSketch";
import { SketchAnnotations } from "@/components/sketch/SketchAnnotations";
import { SketchEntryBox, SketchModeLabel } from "@/components/sketch/SketchCanvasChips";
import { Link, Unlink } from "lucide-react";
import { type PointerEvent as ReactPointerEvent, useEffect, useLayoutEffect, useRef, useState } from "react";

import { PageFrames } from "@/components/PageFrames";
import { FixtureLabels } from "@/components/FixtureLabels";
import { PrintAndCutLabels } from "@/components/PrintAndCutLabels";
import { PlacedPageTags } from "@/components/PlacedPageTags";
import { EditHintChip } from "@/components/EditHintChip";
import { HandleHintChip } from "@/components/HandleHintChip";
import { KeyHintChip } from "@/components/KeyHintChip";
import { CloneTags } from "@/components/CloneTags";
import { FontMissingTags } from "@/components/FontMissingTags";
import { TextSink } from "@/components/TextSink";
import { MoveBadges } from "@/components/MoveBadges";
import { PickSetBadge } from "@/components/PickSetBadge";
import { NudgeReadout } from "@/components/NudgeReadout";
import { liveReadoutVisible } from "@/lib/operationRows";
import { NodeContextMenu } from "@/components/NodeToolbar";
import { MoveEntryChip } from "@/components/MoveEntryChip";
import { OffsetEntryChip } from "@/components/OffsetEntryChip";
import { PenEntryChip } from "@/components/PenEntryChip";
import { PlacementChip } from "@/components/PlacementChip";
import { SelectionAnnouncer } from "@/components/SelectionAnnouncer";
import { TextPathEntryChip } from "@/components/TextPathEntryChip";
import { TilingEntryChip } from "@/components/TilingEntryChip";
import { SketchKnobEntryChip } from "@/components/SketchKnobEntryChip";
import { TransformEntryChip } from "@/components/TransformEntryChip";
import type { EditorSession } from "@/hooks/useEditorSession";
import { ColourPickChip } from "@/components/ColourPickChip";
import { PenHintChip } from "@/components/PenHintChip";
import { useFileDrop } from "@/hooks/useFileDrop";
import { clipboard_notice } from "@/wasm-bindings/curvyo_editor_wasm.js";
import { usePenCue } from "@/hooks/usePenCue";
import type { DroppedFiles } from "@/hooks/useProject";
import { cursorForHint } from "@/lib/cursors";
import { focusMode, useFocusMode } from "@/hooks/useFocusPoint";
import { isClick } from "@/lib/focusPoint";
import { isContextMenuKey } from "@/lib/centreLinesText";
import { type Rect, placeReadout } from "@/lib/readoutPlacement";

/** The readout's constant screen-space offset from its anchor, up and to
 * the right (`specs/0005-object-transform/specification.md`'s UX notes:
 * 12px, anchored to the pointer so it stays readable while a rotate drag
 * swings the handle through an arc). */
const READOUT_OFFSET_PX = 12;

/** The tool rail's clearance from the canvas's left edge, px: the readout is
 * never drawn under it. */
const TOOL_RAIL_CLEAR_PX = 120;
/** The properties panel's collapse tab sits on the canvas's right edge: a chip
 * never reaches into the last 14 px, so it cannot cover the tab when the
 * pointer leaves the canvas over the panel. */
const PANEL_TAB_CLEAR_PX = 14;

interface CanvasProps {
  sketch: SketchApi;
  editor: EditorSession;
  /** The gradient fill's handles, readout and entry box (`specs/0124-gradient-fill`). */
  gradient: GradientApi;
  /** Files dropped on the canvas (`specs/0058-bitmap-import` criterion 4). */
  onDropFiles?: (dropped: DroppedFiles) => void;
  /** Drawn over the canvas, in its coordinates: the other participants' marks (`0092`). */
  overlay?: React.ReactNode;
  /** False in a manufacturing workbench: the drawing's page borders and names are absent there (`specs/0093` UX notes §3). */
  showPageFrames?: boolean;
  /** The pointer is over an operation row of a canvas drag: the move readout is not drawn (`specs/0101` UX notes 6.3). */
  overOperationRow?: boolean;
  /** The fit cue of the move in flight (`specs/0102` 7.2). */
  fitCue?: FitCue | null;
  /** The Operations view toggle in a laser page's label (`specs/0111` criterion 15). */
  onToggleOperationsView?: (page: string) => void;
  /** The Toolpath view toggle beside it (`specs/0029` criterion 8). */
  onToggleToolpathView?: (page: string) => void;
}

/**
 * Hosts the `curvyo-editor-wasm` module's `<canvas>` (ADR 0001 §3):
 * forwards pointer/keyboard input into the attached `WasmSession` and
 * lets it render every frame — no editing logic lives here, matching
 * `curvyo-ui-core`'s own "the frontend... holds no editing logic of its
 * own" (ADR 0001 §1/§2).
 *
 * Fills the window body edge-to-edge, no margin/border/frame — the tool
 * rail and contextual toolbar dock beside/above it, they never shrink it
 * (`specs/0001-project-file-foundation`'s "chrome never frames the canvas"
 * precedent, reaffirmed by `docs/design-system.md`).
 */
export function Canvas({
  editor,
  gradient,
  sketch,
  onDropFiles,
  overlay,
  showPageFrames = true,
  overOperationRow = false,
  fitCue = null,
  onToggleOperationsView,
  onToggleToolpathView,
}: CanvasProps) {
  const penCue = usePenCue(editor);
  // The Focus point mode (`specs/0118` W57): a primary click sends a go to point and selects
  // nothing; a drag does nothing; pan and zoom stay as built. A tool change ends the mode (W56).
  const focusing = useFocusMode();
  const focusDown = useRef<{ id: number; x: number; y: number } | null>(null);
  useEffect(() => focusMode.end(), [editor.tool]);
  const takesFocusPress = (event: ReactPointerEvent<HTMLCanvasElement>) =>
    focusing && event.button === 0 && !editor.isSpaceHeld && !editor.isPanning;
  const onCanvasPointerDown = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    if (!takesFocusPress(event)) {
      editor.onPointerDown(event);
      return;
    }
    event.preventDefault();
    if (event.isPrimary) focusDown.current = { id: event.pointerId, x: event.clientX, y: event.clientY };
  };
  const onCanvasPointerUp = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    const down = focusDown.current;
    if (down === null || down.id !== event.pointerId) {
      editor.onPointerUp(event);
      return;
    }
    focusDown.current = null;
    if (!isClick(down, { x: event.clientX, y: event.clientY })) return;
    const box = event.currentTarget.getBoundingClientRect();
    const point = editor.getSession()?.screen_to_document(event.clientX - box.left, event.clientY - box.top);
    if (point === undefined) return;
    const { x, y } = point;
    point.free();
    focusMode.press(x, y);
  };
  const onCanvasPointerMove = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    // The mode draws no hover outline, no handle and no marquee; a pan goes on as built.
    if (focusing && !editor.isPanning && !editor.isSpaceHeld) return;
    editor.onPointerMove(event);
  };
  const drop = useFileDrop(editor.canvasRef, onDropFiles, () =>
    editor.showHistoryNotice(clipboard_notice("link", ""), 5000, "canvas", "alert"),
  );
  return (
    <div
      ref={editor.containerRef}
      {...drop.handlers}
      tabIndex={0}
      onKeyDown={(event) => {
        // Escape ends the Focus point mode before the selection clears (W64), one step per press.
        if (event.key === "Escape" && focusMode.get()) {
          event.preventDefault();
          event.stopPropagation();
          focusMode.end();
          return;
        }
        // Shift+F10 or the Menu key opens the Select context menu at the centre of the selection
        // box (`specs/0117-centre-lines` UX notes §1); the canvas itself never receives the key.
        // The open Centre lines bar: Enter applies, Escape cancels (`0117` UX notes §7).
        if (event.target === event.currentTarget && editor.centreLines.open && (event.key === "Enter" || event.key === "Escape")) {
          event.preventDefault();
          event.stopPropagation();
          if (event.key === "Enter") {
            editor.centreLines.apply();
          } else if (editor.centreLines.applying) {
            editor.centreLines.stop();
          } else {
            editor.centreLines.cancel();
          }
          return;
        }
        if (event.target === event.currentTarget && isContextMenuKey(event) && (editor.tool === "select" || editor.tool === "node")) {
          event.preventDefault();
          const canvas = editor.canvasRef.current;
          const [left, top, right, bottom] = editor.getSession()?.centre_lines_anchor() ?? [];
          if (canvas) {
            const box = canvas.getBoundingClientRect();
            const [x, y] =
              right === undefined
                ? [box.width / 2, box.height / 2]
                : [(left + right) / 2, (top + bottom) / 2];
            canvas.dispatchEvent(
              new MouseEvent("contextmenu", {
                bubbles: true,
                cancelable: true,
                button: 2,
                clientX: box.left + x,
                clientY: box.top + y,
              }),
            );
          }
          return;
        }
        editor.onKeyDown(event);
      }}
      onKeyUp={editor.onKeyUp}
      onBlur={editor.onContainerBlur}
      // Transform-handle cursors (`object-transform`): only the Select
      // tool ever reports a non-default hint, and a pan gesture (grab
      // cursors above) always wins.
      style={
        editor.isPanning || editor.isSpaceHeld
          ? undefined
          : focusing
            ? // The crosshair over the whole canvas while the Focus point mode is on (W57).
              { cursor: "crosshair" }
          : // Rust decides the Sketch cursor (hit, draggable, drag in progress); CSS only shows it.
            { cursor: sketch.view.workbench === "sketch" ? (sketch.view.cursor ?? "default") : cursorForHint(editor.cursorHint) }
      }
      className={`relative flex-1 outline-none ${
        // Pan cursor convention (`docs/design-system.md`): grabbing for
        // the duration of a drag-pan, open-hand from the moment Space is
        // held (even before any drag motion) — both override whatever
        // cursor the active tool would otherwise show, for exactly the
        // pan's duration (acceptance criteria 3, 4).
        editor.isPanning
          ? "cursor-grabbing"
          : editor.isSpaceHeld
            ? "cursor-grab"
            : editor.tool === "pen"
              ? penCue.kind === "close"
                ? "canvas-cursor-pen-close"
                : penCue.kind === "continue"
                  ? "canvas-cursor-pen-continue"
                  : penCue.kind === "join"
                    ? "canvas-cursor-pen-join"
                    : "canvas-cursor-pen"
              : editor.tool === "rectangle" ||
                  editor.tool === "ellipse" ||
                  editor.tool === "polygon-star" ||
                  editor.tool === "measure" ||
                  editor.tool === "test-grid" ||
                  editor.tool === "generators"
                ? // The creation tools and Measure show their crosshair everywhere, over
                  // existing objects too (`unified-object-editing` criterion 25, customer
                  // feedback on PR #243).
                  "cursor-crosshair"
                : editor.tool === "text"
                  ? // The I-beam everywhere in the Text tool (`0074`, UX notes §1).
                    "cursor-text"
                  : "cursor-default"
      }`}
    >
      {/* The context-menu wrapper stays mounted across both tools —
          see NodeContextMenu's own doc comment: swapping it in/out by
          tool would move <canvas> to a different tree position and
          React would remount it, dropping the WasmSession's attached
          wgpu surface. */}
      <NodeContextMenu
        disabled={
          sketch.view.workbench === "sketch" ||
          (editor.tool !== "node" && editor.tool !== "select") ||
          editor.cursorHint === "eyedropper" ||
          editor.cursorHint === "pick-carrier"
        }
        select={{
          shown: editor.tool === "select",
          centreLinesEnabled: editor.selectBar.centreLinesEnabled,
          onCentreLines: editor.centreLines.press,
        }}
        centreLines={
          editor.centreLines.open
            ? undefined
            : { enabled: editor.selectBar.centreLinesEnabled, onPress: editor.centreLines.press, shown: editor.selectionCount > 0 }
        }
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
      >
        <canvas
          ref={editor.canvasRef}
          // Wider than its box by the part a side pane lies over (`specs/0077-font-preview`
          // criterion 2): the split never resizes the canvas.
          className="block h-full w-[calc(100%+var(--canvas-overhang,0px))] max-w-none"
          onPointerDown={onCanvasPointerDown}
          onPointerMove={onCanvasPointerMove}
          onPointerUp={onCanvasPointerUp}
          onPointerLeave={editor.onPointerLeave}
          onPointerCancel={editor.onPointerCancel}
          onContextMenu={(event) => {
            // The pen and shape tools have no context menu of their own
            // (UX notes: the node tool's six actions are the only
            // ones); suppress the browser's native menu either way so a
            // right-click never interrupts drawing. `NodeContextMenu`
            // itself prevents default for its own (node-tool-only) menu.
            if ((editor.tool !== "node" && editor.tool !== "select") || editor.cursorHint === "eyedropper") {
              event.preventDefault();
            }
            // A right click ends pick mode of text on path without a change (`0080` UX notes §2).
            if (editor.cursorHint === "pick-carrier") {
              event.preventDefault();
              editor.cancelTextPathPick();
            }
          }}
          style={{ background: "var(--pasteboard-bg)" }}
        />

      </NodeContextMenu>
      {showPageFrames && <PageFrames getSession={editor.getSession} dropTarget={editor.moveBadges.dropTarget} onToggleOperationsView={onToggleOperationsView} onToggleToolpathView={onToggleToolpathView} />}
      {showPageFrames && <PlacedPageTags getSession={editor.getSession} />}
      {showPageFrames && <FixtureLabels getSession={editor.getSession} />}
      {showPageFrames && <PrintAndCutLabels getSession={editor.getSession} />}
      <SketchAnnotations sketch={sketch} />
      {sketch.view.workbench !== "sketch" && (
        <MeasureOverlay getSession={editor.getSession} measuring={editor.tool === "measure"} />
      )}
      {sketch.view.workbench !== "sketch" && <GradientOverlay gradient={gradient} containerRef={editor.containerRef} />}
      <SketchModeLabel sketch={sketch} containerRef={editor.containerRef} />
      <SketchEntryBox sketch={sketch} containerRef={editor.containerRef} />
      {/* The drop cue: a 2px dashed accent outline inside the viewport while files hover over the
          canvas (`specs/0058-bitmap-import` UX notes, "Drag and drop"). No text, no animation. */}
      {drop.over && (
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 z-10"
          style={{ outline: "2px dashed var(--accent)", outlineOffset: "-2px" }}
        />
      )}
      {editor.transformEntry && (
        // Keyed by the handle so a different entry starts with fresh text,
        // while a zoom or pan (same handle, new position) keeps what was typed.
        <TransformEntryChip
          key={`${editor.transformEntry.kind}:${editor.transformEntry.fields
            .map((f) => f.name + f.prefill)
            .join("|")}`}
          entry={editor.transformEntry}
          containerRef={editor.containerRef}
          onCommit={editor.commitTransformEntry}
          onCancel={editor.cancelTransformEntry}
          onLinked={editor.transformEntryLinked}
          onMirrorLine={editor.transformEntryMirrorLine}
        />
      )}
      {editor.textPathEntry && (
        // Keyed by the values it opened with: a reopen starts with fresh text, a zoom or pan
        // (same values, new position) keeps what was typed.
        <TextPathEntryChip
          key={`${editor.textPathEntry.position}|${editor.textPathEntry.distance}`}
          entry={editor.textPathEntry}
          containerRef={editor.containerRef}
          onCommit={editor.commitTextPathEntry}
          onCancel={editor.cancelTextPathEntry}
          onSide={editor.textPathEntrySide}
          onLine={editor.textPathEntryLine}
          onMirror={editor.textPathEntryMirror}
        />
      )}
      {editor.tiling.entry && editor.tiling.recipe && (
        <TilingEntryChip
          key={editor.tiling.entry.handle}
          entry={editor.tiling.entry}
          recipe={editor.tiling.recipe}
          tiling={editor.tiling}
          containerRef={editor.containerRef}
        />
      )}
      <SketchKnobEntryChip editor={editor} />
      {editor.moveEntry && (
        <MoveEntryChip
          entry={editor.moveEntry}
          containerRef={editor.containerRef}
          onCommit={editor.commitMoveEntry}
          onCancel={editor.cancelTransformEntry}
          onMode={editor.setMoveChipAbsolute}
          onReference={editor.setMoveReference}
          onRadioHover={editor.setMoveRadioHover}
          onCommitAt={editor.commitMoveEntryAt}
          markHover={editor.moveMarkHover}
          onMarkClick={editor.setMarkClickHandler}
        />
      )}
      {editor.placement.chip && (
        <PlacementChip
          placement={editor.placement}
          chip={editor.placement.chip}
          containerRef={editor.containerRef}
        />
      )}
      {editor.penEntry.chip && (
        <PenEntryChip
          penEntry={editor.penEntry}
          chip={editor.penEntry.chip}
          containerRef={editor.containerRef}
        />
      )}
      {/* The Pen entry chip's polite live region, always mounted (`0072` criterion 34). */}
      <div aria-live="polite" role="status" className="sr-only">
        <span key={editor.penEntry.announcement.id}>{editor.penEntry.announcement.text}</span>
      </div>
      {/* The placement's polite live region, always mounted (`0052` criterion 30). */}
      <div aria-live="polite" role="status" className="sr-only">
        <span key={editor.placement.announcement.id}>{editor.placement.announcement.text}</span>
      </div>
      {editor.tool === "offset" && editor.offset.view.chipOpen && (
        <OffsetEntryChip offset={editor.offset} containerRef={editor.containerRef} />
      )}
      <HandleHintChip
        // A press picks a colour while the eyedropper is on: no move to promise.
        hint={editor.cursorHint === "eyedropper" ? "" : editor.handleHint}
        cornerLines={editor.cornerHintLines}
        containerRef={editor.containerRef}
      />
      <KeyHintChip hint={editor.keyHint} containerRef={editor.containerRef} />
      <NudgeReadout feedback={editor.nudgeFeedback} containerRef={editor.containerRef} />
      <NudgeReadout
        feedback={{ readout: editor.text.kernReadout, announcement: editor.text.kernReadout?.text ?? "" }}
        containerRef={editor.containerRef}
      />
      {/* "Put on path" after a text is put on a carrier (`specs/0080-text-on-path` UX notes §2). */}
      <SelectionAnnouncer text={editor.text.pathAnnouncement.text} seq={editor.text.pathAnnouncement.seq} />
      <SelectionAnnouncer
        text={editor.selectionAnnouncement}
        seq={editor.selectionAnnouncementSeq}
      />
      <EditHintChip
        hint={editor.editHint}
        polygon={editor.selectBar.pointsShown && !editor.selectBar.ratioShown}
        containerRef={editor.containerRef}
        onDismiss={editor.dismissEditHint}
      />
      <WidthPointEntry
        entry={editor.widthEntry}
        onCommit={editor.commitWidthEntry}
        onClose={editor.closeWidthEntry}
        containerRef={editor.containerRef}
      />
      <WidthWheelReadout readout={editor.widthWheelReadout} containerRef={editor.containerRef} />
      <PenHintChip cue={penCue} containerRef={editor.containerRef} />
      <MoveBadges badges={editor.moveBadges} containerRef={editor.containerRef} fit={fitCue} />
      <PickSetBadge at={editor.text.pickBadge} />
      <CloneTags tags={editor.clones.tags} />
      <FontMissingTags marks={editor.text.missingMarks} hidden={editor.isPanning} />
      <TextSink text={editor.text} mounted={sketch.view.workbench === "vector" && editor.tool === "text"} />
      <ColourPickChip
        active={editor.cursorHint === "eyedropper"}
        getSession={editor.getSession}
        viewKey={editor.zoomKey}
        containerRef={editor.containerRef}
      />
      {editor.liveReadout && liveReadoutVisible(editor.liveReadout, overOperationRow) && (
        <ReadoutChip
          text={editor.liveReadout.text}
          second={editor.liveReadout.second}
          third={editor.liveReadout.third}
          thirdInvalid={editor.liveReadout.thirdInvalid}
          glyph={editor.liveReadout.glyph}
          x={editor.liveReadout.x}
          y={editor.liveReadout.y}
          containerRef={editor.containerRef}
        />
      )}
      {overlay}
    </div>
  );
}

/** The floating Select bar's rectangle in the canvas's own coordinates, or
 * `null` when no bar is shown. */
function measureBar(container: HTMLElement): Rect | null {
  const bar = container.parentElement?.querySelector("[data-context-bar]");
  if (!bar) {
    return null;
  }
  const outer = container.getBoundingClientRect();
  const box = bar.getBoundingClientRect();
  return {
    left: box.left - outer.left,
    top: box.top - outer.top,
    right: box.right - outer.left,
    bottom: box.bottom - outer.top,
  };
}

function sameRect(a: Rect | null, b: Rect | null): boolean {
  if (a === null || b === null) {
    return a === b;
  }
  return a.left === b.left && a.top === b.top && a.right === b.right && a.bottom === b.bottom;
}

interface ReadoutChipProps {
  text: string;
  /** A muted second line. */
  second?: string;
  /** A third line (the Offset tool's message). */
  third?: string;
  /** The third line in the invalid colour. */
  thirdInvalid?: boolean;
  /** The chain glyph before the text (`specs/0099` UX notes 12.9.4). */
  glyph?: "link" | "unlink";
  /** Canvas-relative CSS pixels, already converted by Rust
   * (`specs/0004-canvas-navigation-and-selection/adrs.md`: "Anything the
   * DOM positions... is returned already converted"). */
  x: number;
  y: number;
  containerRef: React.RefObject<HTMLDivElement | null>;
}

/**
 * The on-canvas numeric readout (`specs/0003-primitive-shapes/
 * specification.md`'s "Live creation feedback": "direct manipulation keeps
 * the number where the maker's eyes already are"; the transform readouts
 * of `specs/0005-object-transform` use the same chip). Measured after each
 * render so it can be kept fully inside the canvas — flipped left or below
 * the pointer near the right and top edges — instead of vanishing there.
 */
function ReadoutChip({ text, second, third, thirdInvalid, glyph, x, y, containerRef }: ReadoutChipProps) {
  const chipRef = useRef<HTMLDivElement>(null);
  const [sizes, setSizes] = useState({
    chip: { width: 0, height: 0 },
    canvas: {
      width: Number.POSITIVE_INFINITY,
      height: Number.POSITIVE_INFINITY,
    },
    bar: null as Rect | null,
  });

  // Both sizes are measured in a layout effect (never read from a ref
  // during render), before the browser paints, so the chip is placed
  // correctly on the frame it first shows.
  useLayoutEffect(() => {
    const chip = chipRef.current;
    const container = containerRef.current;
    if (!chip || !container) {
      return;
    }
    const next = {
      chip: { width: chip.offsetWidth, height: chip.offsetHeight },
      canvas: { width: container.clientWidth, height: container.clientHeight },
      bar: measureBar(container),
    };
    setSizes((previous) =>
      previous.chip.width === next.chip.width &&
      previous.chip.height === next.chip.height &&
      previous.canvas.width === next.canvas.width &&
      previous.canvas.height === next.canvas.height &&
      sameRect(previous.bar, next.bar)
        ? previous
        : next,
    );
  }, [text, second, third, x, y, containerRef]);

  const placement = placeReadout(
    { x, y },
    sizes.chip,
    { width: sizes.canvas.width - PANEL_TAB_CLEAR_PX, height: sizes.canvas.height },
    READOUT_OFFSET_PX,
    sizes.bar ?? undefined,
  );

  return (
    <div
      ref={chipRef}
      aria-hidden="true"
      className="pointer-events-none absolute z-40 whitespace-nowrap rounded-md px-1.5 py-0.5 text-xs tabular-nums"
      style={{
        left: Math.max(placement.left, TOOL_RAIL_CLEAR_PX),
        top: placement.top,
        background: "var(--toolbar-bg)",
        color: "var(--toolbar-icon)",
        // A hairline so the chip reads as its own surface wherever it lands,
        // also over the Select bar, which shares its ground.
        border: "1px solid color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
      }}
    >
      <div className="flex items-center gap-1">
        {glyph === "link" && <Link size={12} strokeWidth={1.5} aria-hidden />}
        {glyph === "unlink" && <Unlink size={12} strokeWidth={1.5} aria-hidden />}
        {text}
      </div>
      {second && <div className="text-muted-foreground">{second}</div>}
      {third && (
        <div style={thirdInvalid ? { color: "var(--field-invalid)" } : undefined}>{third}</div>
      )}
    </div>
  );
}

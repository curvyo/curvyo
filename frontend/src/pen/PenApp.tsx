// The pen window's page (`specs/0158-pen-display` slice 2a, 2b; ADR 0096 A10; UX 9.2 to 9.5, 10):
// the canvas under the one fixed Pen layout (rail, strip, Canvas only corner) and its own notice
// slot on the main window's project. It mounts no project, recovery, sync, workspace or connector
// hook: the main window owns the file.
import { type KeyboardEvent, useEffect, useState, useSyncExternalStore } from "react";

import { Canvas } from "@/components/Canvas";
import { CanvasNotice } from "@/components/CanvasNotice";
import { NodeToolbar } from "@/components/NodeToolbar";
import { TooltipProvider } from "@/components/ui/tooltip";
import { WidthEditToggle } from "@/components/WidthEditToggle";
import { useEditorSession } from "@/hooks/useEditorSession";
import { useGradient } from "@/hooks/useGradient";
import { useSketch } from "@/hooks/useSketch";
import { usePenWindowLink } from "@/hooks/useWindowLink";
import { createCursorPosition } from "@/lib/cursorPosition";
import { setViewTool, subscribeViewTool, toggleViewTool, viewTool } from "@/lib/viewTool";
import { invoke } from "@/platform/host";
import { PenCorner, PenRail } from "@/pen/PenRail";
import { PenStrip } from "@/pen/PenStrip";
import { penKeyAllowed } from "@/pen/penKeys";
import {
  type PenFlags,
  type PenTool,
  penControls,
  railRight,
  readPenFlags,
  savePenFlags,
  viewInSecondColumn,
} from "@/pen/penLayout";

/** The canvas the rail and the strip leave free for Fit page: right of the rail and below the
 * strip row, each with the 24 px margin (UX 9.3). */
const FIT_TOP = 12 + 56 + 24;
const FIT_INSETS = [railRight(false) + 24, FIT_TOP] as const;

const cursor = createCursorPosition();
/** The switches the hub remembered (`pen_window.json`), read once per page. */
const INITIAL_FLAGS: PenFlags = readPenFlags(typeof location === "undefined" ? "" : location.search);

/** The banner of the pen window's states (UX 9.5), `null` while live. */
function bannerText(state: string): string | null {
  if (state === "out-of-sync") return "Out of sync, reloading...";
  if (state === "held" || state === "waiting") return "Waiting for the main window";
  return null;
}

function useWindowHeight(): number {
  const [height, setHeight] = useState(() => (typeof window === "undefined" ? 1080 : window.innerHeight));
  useEffect(() => {
    const read = () => setHeight(window.innerHeight);
    window.addEventListener("resize", read);
    return () => window.removeEventListener("resize", read);
  }, []);
  return height;
}

/** The pen window. */
export function PenApp() {
  const editor = useEditorSession(cursor.update);
  const gradient = useGradient(editor);
  const sketch = useSketch(editor);
  const [flags, setFlags] = useState(INITIAL_FLAGS);
  const [eraser, setEraser] = useState(false);
  const link = usePenWindowLink(editor, FIT_INSETS, flags.followView);
  const panning = useSyncExternalStore(subscribeViewTool, viewTool) === "pan";
  const twoColumns = viewInSecondColumn(useWindowHeight());
  const banner = bannerText(link.state);
  const { getSession, canvasRef } = editor;
  const fitLeft = railRight(twoColumns) + 24;

  // The Eraser is the Pencil's mode: it shows while the Pencil is the tool.
  const penTool: PenTool = editor.tool === "pencil" ? (eraser ? "eraser" : "pencil") : editor.tool === "node" ? "node" : "select";

  const changeFlags = (next: PenFlags) => {
    setFlags(next);
    void savePenFlags(next, invoke).catch(() => undefined);
  };
  const zoom = (zoomIn: boolean) => {
    const canvas = canvasRef.current;
    const x = canvas ? (fitLeft + canvas.clientWidth) / 2 : 600;
    const y = canvas ? (FIT_TOP + canvas.clientHeight) / 2 : 360;
    getSession()?.zoom_step(zoomIn, x, y);
    editor.sync();
  };
  const controls = penControls(
    {
      tool: penTool,
      undo: editor.historyNames.undo,
      redo: editor.historyNames.redo,
      canDelete: editor.selectionCount > 0,
      panning,
      follow: flags.followView,
      canvasOnly: flags.canvasOnly,
    },
    {
      tool: (tool) => {
        setViewTool("none");
        const on = tool === "eraser";
        setEraser(on);
        getSession()?.set_pencil_eraser(on);
        editor.setTool(on ? "pencil" : tool);
      },
      undo: () => editor.pressHistory("undo"),
      redo: () => editor.pressHistory("redo"),
      delete: editor.deleteSelected,
      zoom,
      fit: () => {
        getSession()?.view_fit_page(fitLeft, FIT_TOP);
        editor.sync();
      },
      pan: () => toggleViewTool("pan"),
      follow: (on) => {
        getSession()?.link_set_follow(on);
        changeFlags({ ...flags, followView: on });
      },
      canvasOnly: (on) => changeFlags({ ...flags, canvasOnly: on }),
    },
  );
  const onKeyDownCapture = (event: KeyboardEvent) => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      event.stopPropagation();
      link.save();
      return;
    }
    if (!penKeyAllowed(event.key, event.ctrlKey || event.metaKey, event.altKey)) {
      event.preventDefault();
      event.stopPropagation();
    }
  };
  const nodeBar = (
    <NodeToolbar
      pen
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
      widthEdit={<WidthEditToggle editor={editor} inBar tall />}
    />
  );

  return (
    <TooltipProvider>
      <div className="relative flex h-screen w-screen overflow-clip" data-scroll-lock="" onKeyDownCapture={onKeyDownCapture}>
        {flags.canvasOnly ? null : (
          <div inert={banner !== null} className={banner !== null ? "opacity-40" : ""}>
            <PenRail controls={controls} twoColumns={twoColumns} />
            <div
              className="pointer-events-none absolute top-3 right-[80px] z-20 flex justify-start"
              style={{ left: railRight(twoColumns) + 12 }}
            >
              <PenStrip editor={editor} tool={penTool} node={nodeBar} />
            </div>
          </div>
        )}
        <PenCorner controls={controls} />
        <div className={`relative flex min-h-0 min-w-0 flex-1 ${banner !== null ? "pointer-events-none cursor-default" : ""}`}>
          <Canvas editor={editor} gradient={gradient} sketch={sketch} showPageFrames />
        </div>
        <CanvasNotice
          notice={editor.canvasNotice}
          groupDepth={editor.grouping.depth}
          onLeaveToLevel={editor.grouping.leaveToLevel}
          announcement={editor.grouping.announcement}
          persistent={
            banner !== null ? (
              <div role="status" className="rounded bg-[var(--toolbar-bg)] px-3 py-1 text-sm text-[var(--toolbar-icon)]">
                {banner}
              </div>
            ) : null
          }
        />
      </div>
    </TooltipProvider>
  );
}

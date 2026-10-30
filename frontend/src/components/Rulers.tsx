import { createRulerRaster } from "@/lib/rulerRaster";
import { useEffect, useRef } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import {
  type RulerColours,
  type RulerData,
  LABEL_SIZE,
  labelFont,
} from "@/lib/rulerDraw";

interface RulersProps {
  editor: EditorSession;
  /** The display unit's symbol shown in the corner square. */
  unit: string;
}

/** The design-system tokens as colour strings a canvas can paint with. */
function readColours(): RulerColours {
  const style = getComputedStyle(document.documentElement);
  const token = (name: string) => style.getPropertyValue(name).trim();
  return {
    background: token("--ruler-bg"),
    tick: token("--ruler-tick"),
    tickMinor: token("--ruler-tick-minor"),
    label: token("--ruler-label"),
    edge: token("--ruler-edge"),
    pointer: token("--ruler-pointer"),
  };
}

/** The two advances the label layout needs, CSS px: the widest digit (every
 * character of a label but its minus sign is counted at it, a point being
 * narrower) and the minus sign, which is measured on its own because the font
 * that supplies U+2212 differs per platform and is often wider than a digit. */
function measureGlyphs(): { digit: number; minus: number } {
  const ctx = document.createElement("canvas").getContext("2d");
  if (!ctx) {
    return { digit: LABEL_SIZE * 0.6, minus: LABEL_SIZE * 0.6 };
  }
  ctx.font = labelFont(1);
  const width = (c: string) => ctx.measureText(c).width;
  return {
    digit: Math.max(...[..."0123456789"].map(width)),
    minus: width("\u2212"),
  };
}

/** What the strips were last painted from; a change in any field repaints. */
interface Painted {
  scale: number;
  originX: number;
  originY: number;
  pageX: number;
  pageY: number;
  unit: string;
  dpr: number;
  width: number;
  height: number;
  horizontalThickness: number;
  verticalThickness: number;
  pointerX: number | null;
  pointerY: number | null;
}

function samePainted(a: Painted | null, b: Painted): boolean {
  return (
    a !== null &&
    a.scale === b.scale &&
    a.originX === b.originX &&
    a.originY === b.originY &&
    a.pageX === b.pageX &&
    a.pageY === b.pageY &&
    a.unit === b.unit &&
    a.dpr === b.dpr &&
    a.width === b.width &&
    a.height === b.height &&
    a.horizontalThickness === b.horizontalThickness &&
    a.verticalThickness === b.verticalThickness &&
    a.pointerX === b.pointerX &&
    a.pointerY === b.pointerY
  );
}

/**
 * The two ruler strips and the corner square beside the canvas
 * (`specs/0015-document-size-and-rulers/`): chrome docked in the canvas
 * region's grid, so they shrink the canvas and the existing
 * `getBoundingClientRect` path keeps every screen-to-document conversion
 * right. Both strips are Canvas2D, painted from the layout `curvyo-ui-core`
 * computes, by their own animation-frame callback that runs in the same frame
 * as the canvas's render loop and repaints only when the view, the unit, a
 * size or the pointer changed.
 *
 * They take no input: a press is swallowed so no focus moves and nothing
 * starts, and a wheel or pinch over a ruler is forwarded to the canvas.
 */
export function Rulers({ editor, unit }: RulersProps) {
  const horizontalRef = useRef<HTMLCanvasElement>(null);
  const verticalRef = useRef<HTMLCanvasElement>(null);
  const unitRef = useRef(unit);
  useEffect(() => {
    unitRef.current = unit;
  }, [unit]);

  const cornerRef = useRef<HTMLDivElement>(null);
  const { canvasRef, getSession, addViewResizedListener } = editor;

  useEffect(() => {
    const horizontal = horizontalRef.current;
    const vertical = verticalRef.current;
    const corner = cornerRef.current;
    const canvas = canvasRef.current;
    if (!horizontal || !vertical || !corner || !canvas) {
      return;
    }
    const strips: HTMLElement[] = [horizontal, vertical, corner];
    const colours = readColours();
    const { digit, minus } = measureGlyphs();
    const pointer: { x: number | null; y: number | null } = { x: null, y: null };
    let painted: Painted | null = null;
    const horizontalRaster = createRulerRaster(horizontal, colours);
    const verticalRaster = createRulerRaster(vertical, colours);
    let sizes = { width: 0, height: 0, horizontalThickness: 0, verticalThickness: 0 };
    const readSizes = () => {
      sizes = { width: horizontal.clientWidth, height: vertical.clientHeight,
        horizontalThickness: horizontal.clientHeight, verticalThickness: vertical.clientWidth };
    };
    readSizes();

    const onPointerMove = (event: PointerEvent) => {
      const bounds = canvas.getBoundingClientRect();
      pointer.x = event.clientX - bounds.left;
      pointer.y = event.clientY - bounds.top;
    };
    const onPointerLeave = () => {
      pointer.x = null;
      pointer.y = null;
    };
    canvas.addEventListener("pointermove", onPointerMove);
    canvas.addEventListener("pointerleave", onPointerLeave);

    // A wheel or pinch over a ruler acts on the canvas as if the pointer were
    // at the nearest point inside it, so a zoom does not stop when the
    // pointer slips onto a ruler. The canvas's own listener does the rest.
    const forwardWheel = (event: WheelEvent) => {
      event.preventDefault();
      const bounds = canvas.getBoundingClientRect();
      canvas.dispatchEvent(
        new WheelEvent("wheel", {
          bubbles: true,
          cancelable: true,
          deltaX: event.deltaX,
          deltaY: event.deltaY,
          deltaMode: event.deltaMode,
          ctrlKey: event.ctrlKey,
          shiftKey: event.shiftKey,
          altKey: event.altKey,
          metaKey: event.metaKey,
          clientX: Math.min(Math.max(event.clientX, bounds.left), bounds.right - 1),
          clientY: Math.min(Math.max(event.clientY, bounds.top), bounds.bottom - 1),
        }),
      );
    };
    for (const element of strips) {
      element.addEventListener("wheel", forwardWheel, { passive: false });
    }

    const paint = (isHorizontal: boolean, rebuild: boolean) => {
      const session = getSession();
      if (!session) return;
      const geometry = {
        horizontal: isHorizontal,
        length: isHorizontal ? sizes.width : sizes.height,
        thickness: isHorizontal ? sizes.horizontalThickness : sizes.verticalThickness,
        devicePixelRatio: window.devicePixelRatio || 1,
        pointer: isHorizontal ? pointer.x : pointer.y,
      };
      const raster = isHorizontal ? horizontalRaster : verticalRaster;
      if (rebuild) {
        const view = session.ruler_view(isHorizontal, geometry.length, digit, minus);
        const data: RulerData = {
          majors: view.majors, minors: view.minors, origin: view.origin,
          labelTicks: view.label_ticks, labelTexts: view.label_texts,
        };
        raster.rebuild(data, geometry);
        view.free();
      }
      raster.paint(geometry);
    };

    // Repaints when anything the strips are drawn from changed. Runs every
    // animation frame (pan, zoom, pointer, unit) and, through the session's
    // resize hook, right after a canvas resize reached the session: the
    // browser runs animation-frame callbacks before its resize observers, so
    // without that hook a window resize or a panel toggle would show the
    // canvas with its new view and the rulers with the old one for a frame.
    const repaintIfChanged = () => {
      const session = getSession();
      if (!session) {
        return;
      }
      const now: Painted = {
        scale: session.view_scale(),
        originX: session.view_origin_x(),
        originY: session.view_origin_y(),
        pageX: session.active_page_x(),
        pageY: session.active_page_y(),
        unit: unitRef.current,
        dpr: window.devicePixelRatio || 1,
        ...sizes,
        pointerX: pointer.x,
        pointerY: pointer.y,
      };
      if (samePainted(painted, now)) {
        return;
      }
      const rebuild = painted === null || !samePainted(
        { ...painted, pointerX: now.pointerX, pointerY: now.pointerY }, now);
      painted = now;
      paint(true, rebuild);
      paint(false, rebuild);
    };
    let frame = requestAnimationFrame(function loop() {
      frame = requestAnimationFrame(loop);
      repaintIfChanged();
    });
    const resized = () => { readSizes(); repaintIfChanged(); };
    const observer = new ResizeObserver(resized);
    observer.observe(horizontal); observer.observe(vertical);
    const removeResizeListener = addViewResizedListener(resized);

    return () => {
      cancelAnimationFrame(frame);
      removeResizeListener();
      observer.disconnect();
      canvas.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerleave", onPointerLeave);
      for (const element of strips) {
        element.removeEventListener("wheel", forwardWheel);
      }
    };
  }, [canvasRef, getSession, addViewResizedListener]);

  // A press on a ruler or the corner does nothing and keeps the focus where
  // it was (criterion 10): no guide, no selection change, no tool action.
  const swallow = {
    onPointerDown: (event: React.PointerEvent) => event.preventDefault(),
    onMouseDown: (event: React.MouseEvent) => event.preventDefault(),
    onContextMenu: (event: React.MouseEvent) => event.preventDefault(),
  };
  const edge = "1px solid var(--ruler-edge)";

  return (
    <>
      <div
        ref={cornerRef}
        aria-hidden="true"
        className="flex cursor-default items-center justify-center text-xs select-none"
        style={{
          background: "var(--ruler-bg)",
          color: "var(--panel-muted-fg)",
          borderRight: edge,
          borderBottom: edge,
          boxSizing: "border-box",
        }}
        {...swallow}
      >
        {unit}
      </div>
      <div aria-hidden="true" className="min-w-0 cursor-default" {...swallow}>
        {/* As long as the drawing's canvas, under a side pane too: the split never resizes it
         * (`specs/0077-font-preview` criterion 2). */}
        <canvas ref={horizontalRef} className="block h-full w-[calc(100%+var(--canvas-overhang,0px))] max-w-none" />
      </div>
      <div aria-hidden="true" className="min-h-0 cursor-default" {...swallow}>
        <canvas ref={verticalRef} className="block size-full" />
      </div>
    </>
  );
}

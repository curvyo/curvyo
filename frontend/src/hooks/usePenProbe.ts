// The Pen probe's wiring (`specs/0158-pen-display` slice 0, ADR 0094): the webview's Pointer Events
// and the native batches feed one `ProbeSession`; the page reads a snapshot every frame and the
// scratch canvas draws both traces. The statistics live in `lib/penProbe.ts`; this only connects.
import { type RefObject, useCallback, useEffect, useRef, useState } from "react";

import {
  type NativeBatch,
  ProbeSession,
  type ProbeSnapshot,
  type ReportContext,
  buildReport,
} from "@/lib/penProbe";
import { invoke, isTauri, messageChannel } from "@/platform/host";

export type Route = "checking" | "native" | "unavailable";

export interface PenProbeApi {
  snapshot: ProbeSnapshot;
  route: Route;
  compression: boolean;
  setCompression: (on: boolean) => void;
  clear: () => void;
  report: () => string;
}

/** A dot of the trace: radius from pressure, 1 px when there is none. */
function dot(context: CanvasRenderingContext2D, x: number, y: number, pressure: number | null, colour: string): void {
  context.fillStyle = colour;
  context.beginPath();
  context.arc(x, y, 1 + (pressure ?? 0.1) * 9, 0, Math.PI * 2);
  context.fill();
}

/** Runs while the page is mounted: the page mounts it fresh on each open. */
export function usePenProbe(canvas: RefObject<HTMLCanvasElement | null>): PenProbeApi {
  const session = useRef(new ProbeSession());
  const [snapshot, setSnapshot] = useState(() => new ProbeSession().snapshot(0));
  const [route, setRoute] = useState<Route>(isTauri ? "checking" : "unavailable");
  const [compression, setCompressionState] = useState(true);

  useEffect(() => {
    const colours = getComputedStyle(document.documentElement);
    const webColour = colours.getPropertyValue("--editor-accent") || "#2F6FEE";
    const nativeColour = colours.getPropertyValue("--toolbar-icon") || "#3A3A3F";
    const draw = (x: number, y: number, pressure: number | null, colour: string) => {
      const element = canvas.current;
      const context = element?.getContext("2d");
      if (!element || !context) {
        return;
      }
      const rect = element.getBoundingClientRect();
      const [px, py] = [x - rect.left, y - rect.top];
      if (px >= 0 && py >= 0 && px <= rect.width && py <= rect.height) {
        dot(context, px, py, pressure, colour);
      }
    };
    const onPointer = (event: PointerEvent) => {
      const coalesced = event.getCoalescedEvents?.() ?? [];
      session.current.web(
        {
          t: performance.now(),
          x: event.clientX,
          y: event.clientY,
          pointerType: event.pointerType,
          pressure: event.pressure,
          tiltX: event.tiltX,
          tiltY: event.tiltY,
          eraser: (event.buttons & 32) !== 0,
          barrel: (event.buttons & 2) !== 0,
          coalesced: Math.max(1, coalesced.length),
        },
        coalesced.map((e) => ({ x: e.clientX, y: e.clientY })),
      );
      if (event.type === "pointermove" && (event.buttons & 1) !== 0) {
        draw(event.clientX, event.clientY, event.pressure, webColour);
      }
    };
    const kinds = ["pointerdown", "pointermove", "pointerup", "pointercancel"] as const;
    kinds.forEach((kind) => window.addEventListener(kind, onPointer, true));

    let alive = true;
    let last = performance.now();
    let frame = requestAnimationFrame(function tick(now) {
      session.current.frame(now - last, now);
      last = now;
      setSnapshot(session.current.snapshot(now));
      if (alive) {
        frame = requestAnimationFrame(tick);
      }
    });

    if (isTauri) {
      const channel = messageChannel<NativeBatch>((batch) => {
        const now = performance.now();
        session.current.native(batch, now);
        for (const sample of batch.samples) {
          if (sample.phase === "contact") {
            draw(sample.x, sample.y, sample.pressure, nativeColour);
          }
        }
      });
      invoke<boolean>("pen_watch", { onBatch: channel }).then(
        (watching) => alive && setRoute(watching ? "native" : "unavailable"),
        () => alive && setRoute("unavailable"),
      );
    }
    return () => {
      alive = false;
      cancelAnimationFrame(frame);
      kinds.forEach((kind) => window.removeEventListener(kind, onPointer, true));
      if (isTauri) {
        void invoke("pen_unwatch").catch(() => undefined);
      }
    };
  }, [canvas]);

  const setCompression = useCallback((on: boolean) => {
    setCompressionState(on);
    void invoke("pen_compression", { on }).catch(() => undefined);
  }, []);

  const clear = useCallback(() => {
    const element = canvas.current;
    element?.getContext("2d")?.clearRect(0, 0, element.width, element.height);
  }, [canvas]);

  const report = useCallback(() => {
    const context: ReportContext = {
      route: route === "native" ? "native" : "unavailable",
      compression,
      platform: navigator.platform,
      agent: navigator.userAgent,
    };
    return buildReport(session.current.snapshot(performance.now()), context);
  }, [route, compression]);

  return { snapshot, route, compression, setCompression, clear, report };
}

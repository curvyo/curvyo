// Feeds the Pencil (`specs/0158-pen-display` slice 1, ADR 0095 decision 2): while the Pencil is the
// active tool, the canvas's Pointer Events and, in the app on Linux, the native pen batches go to one
// `PenSampleFeed`; the render loop flushes it into `pen_samples` once per frame, before the frame
// is drawn. Other tools are fed as before.
import { type RefObject, useCallback, useEffect, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import type { NativeBatch } from "@/lib/penProbe";
import { PenPanGate } from "@/lib/penPanGate";
import {
  afterStroke,
  applyPencilSettings,
  pencilSettings,
  setPencilSettings,
} from "@/lib/pencilSettings";
import { viewTool } from "@/lib/viewTool";
import { invoke, isTauri, messageChannel } from "@/platform/host";
import { PEN_SAMPLE_STRIDE, PenSampleFeed, type PointerLike } from "@/platform/penSamples";

export interface PencilFeed {
  /** Takes a canvas Pointer Event while the Pencil is active; false when the event is not the
   * Pencil's (another tool, or a button that is not the tip or the eraser end). */
  pointer: (event: PointerLike & { button: number }) => boolean;
  /** Hands the frame's samples to the session: `null` when none arrived, else whether a stroke or
   * an erase ended. */
  flush: (session: WasmSession) => { ended: boolean } | null;
  /** Forgets a contact the browser cancelled (the session drops it too). */
  reset: () => void;
}

export function usePencilFeed(
  canvasRef: RefObject<HTMLCanvasElement | null>,
  active: boolean,
): PencilFeed {
  // While the View card's Pan is on, no pen sample reaches the session (slice 2a).
  const [gate] = useState(() => new PenPanGate(new PenSampleFeed(), () => viewTool() === "pan"));
  const feed = gate.feed;
  const activeRef = useRef(active);
  useEffect(() => {
    activeRef.current = active;
  }, [active]);

  useEffect(() => {
    if (!active) {
      feed.reset();
      return;
    }
    if (!isTauri) return;
    // The native reader (ADR 0094); "not available" leaves the Pointer Event route alone.
    const channel = messageChannel<NativeBatch>((batch) => {
      const rect = canvasRef.current?.getBoundingClientRect();
      if (rect && activeRef.current) gate.native(batch, rect);
    });
    invoke<boolean>("pen_watch", { onBatch: channel }).catch(() => undefined);
    return () => {
      void invoke("pen_unwatch").catch(() => undefined);
    };
  }, [active, canvasRef, gate, feed]);

  const pointer = useCallback(
    (event: PointerLike & { button: number }) => {
      if (!activeRef.current) return false;
      // A right-click (often the barrel button) does nothing in the Pencil (UX notes §3).
      if (event.type === "pointerdown" && event.button !== 0 && event.button !== 5) return true;
      const rect = canvasRef.current?.getBoundingClientRect();
      if (rect) gate.pointer(event, rect);
      return true;
    },
    [canvasRef, gate],
  );

  const flush = useCallback((session: WasmSession) => {
    // Pan switched on mid-stroke drops the session's stroke as Escape does.
    const batch = gate.take(() => {
      if (activeRef.current) session.pointer_cancelled();
    });
    if (!batch || sessionStride(session) !== PEN_SAMPLE_STRIDE) return null;
    applyPencilSettings(session);
    session.pen_samples(batch.flat);
    if (feed.penSeen && !pencilSettings().penSeen) {
      setPencilSettings({ ...pencilSettings(), penSeen: true });
    }
    if (batch.ended) {
      setPencilSettings(afterStroke(pencilSettings(), batch.penEnded, session.pencil_last_had_pressure()));
    }
    return { ended: batch.ended };
  }, [gate, feed]);

  const reset = useCallback(() => feed.reset(), [feed]);

  return { pointer, flush, reset };
}

/** The session's record layout, asserted once per flush (ADR 0095 decision 1). */
function sessionStride(session: WasmSession): number {
  const ctor = session.constructor as unknown as { pen_sample_stride?: () => number };
  return ctor.pen_sample_stride?.() ?? PEN_SAMPLE_STRIDE;
}

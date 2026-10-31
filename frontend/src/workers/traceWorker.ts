// The Trace tool's worker (`specs/0025-raster-trace` criteria 17 and 58, ADR 0023 §5): it
// instantiates the same `curvyo-editor-wasm` module the page runs, from the compiled module the
// page posts, and traces off the main thread; it also makes the PNG of a copied image
// (`specs/0087-image-clipboard-and-drop` criterion 14) and the mask of the Mask view (`specs/0089`
// criterion 21). A stale or stopped job is ended by the page
// terminating this worker; a synchronous wasm call cannot be interrupted otherwise.

import { encode_png, initSync, trace_analyse, trace_auto_threshold, trace_job, trace_mask_job, trace_suggest_background } from "@/wasm-bindings/curvyo_editor_wasm.js";

import type { TraceRequest, TraceResponse } from "@/lib/traceJobs";

interface WorkerScope {
  onmessage: ((event: MessageEvent<TraceRequest>) => void) | null;
  postMessage(message: TraceResponse, transfer?: Transferable[]): void;
}

const scope = self as unknown as WorkerScope;

scope.onmessage = (event) => {
  const request = event.data;
  try {
    if (request.type === "init") {
      initSync({ module: request.module });
      return;
    }
    const started = performance.now();
    if (request.type === "threshold") {
      const threshold = trace_auto_threshold(request.rgba, request.width, request.height);
      // Remove background's Auto (`specs/0089` criterion 16): read once per image with the Threshold.
      const background = trace_suggest_background(request.rgba, request.width, request.height);
      // The automatic analysis (`specs/0089` slice F): once per image with the Threshold.
      const analysis = trace_analyse(request.rgba, request.width, request.height);
      scope.postMessage({ id: request.id, kind: "threshold", threshold, background, analysis, ms: performance.now() - started });
      return;
    }
    if (request.type === "mask") {
      // The Mask view (`specs/0089` criterion 21): the mask of the same code path as the trace.
      const mask = trace_mask_job(request.rgba, request.width, request.height, new Float64Array(request.map), request.settings);
      const reply = mask
        ? { width: mask.width, height: mask.height, bits: mask.bits }
        : { width: 0, height: 0, bits: new Uint8Array(0) };
      mask?.free();
      scope.postMessage({ id: request.id, kind: "mask", ...reply, ms: performance.now() - started }, [reply.bits.buffer]);
      return;
    }
    if (request.type === "png") {
      const png = encode_png(request.rgba, request.width, request.height);
      scope.postMessage({ id: request.id, kind: "png", png, ms: performance.now() - started }, [png.buffer]);
      return;
    }
    const wire = trace_job(
      request.rgba,
      request.width,
      request.height,
      new Float64Array(request.map),
      request.settings,
      { report: (percent: number) => scope.postMessage({ id: request.id, kind: "progress", percent }) },
    );
    scope.postMessage({ id: request.id, kind: "trace", wire, ms: performance.now() - started }, [wire.buffer]);
  } catch (error) {
    if (request.type !== "init") {
      scope.postMessage({ id: request.id, kind: "error", message: String(error) });
    }
  }
};

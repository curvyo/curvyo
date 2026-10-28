// The Trace tool's worker (`specs/0025-raster-trace` criteria 17 and 58, ADR 0023 §5): it
// instantiates the same `curvyo-editor-wasm` module the page runs, from the compiled module the
// page posts, and traces off the main thread. A stale or stopped job is ended by the page
// terminating this worker; a synchronous wasm call cannot be interrupted otherwise.

import { initSync, trace_auto_threshold, trace_job } from "@/wasm-bindings/curvyo_editor_wasm.js";

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
      scope.postMessage({ id: request.id, kind: "threshold", threshold, ms: performance.now() - started });
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

// The Trace tool's worker (`specs/0025-raster-trace` criteria 17 and 58, ADR 0023 §5): it
// instantiates the same `curvyo-editor-wasm` module the page runs, from the compiled module the
// page posts, and traces off the main thread; it also makes the PNG of a copied image
// (`specs/0087-image-clipboard-and-drop` criterion 14) and the mask of the Mask view (`specs/0089`
// criterion 21). A stale or stopped job is ended by the page
// terminating this worker; a synchronous wasm call cannot be interrupted otherwise.

import {
  encode_png,
  fit_add_face,
  fit_start,
  fit_step,
  initSync,
  ocr_job,
  ocr_load,
  trace_analyse,
  trace_auto_threshold,
  trace_job,
  trace_mask_job,
  trace_suggest_background,
  trace_text_job,
  trace_text_mask_job,
  trace_text_regions_job,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

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
      // The page waits for this before it trusts the worker (a worker that never starts would
      // otherwise leave every job waiting).
      scope.postMessage({ id: 0, kind: "ready", error: "" });
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
      const map = new Float64Array(request.map);
      const mask =
        request.lines === ""
          ? trace_mask_job(request.rgba, request.width, request.height, map, request.settings)
          : trace_text_mask_job(request.rgba, request.width, request.height, map, request.settings, request.lines);
      const reply = mask
        ? { width: mask.width, height: mask.height, bits: mask.bits }
        : { width: 0, height: 0, bits: new Uint8Array(0) };
      mask?.free();
      scope.postMessage({ id: request.id, kind: "mask", ...reply, ms: performance.now() - started }, [reply.bits.buffer]);
      return;
    }
    if (request.type === "regions") {
      // The text pass alone (`specs/0091` criterion 25); the pixels go back for the trace.
      const wire = trace_text_regions_job(request.rgba, request.width, request.height, new Float64Array(request.map), request.settings, request.lines);
      scope.postMessage({ id: request.id, kind: "regions", wire, rgba: request.rgba, ms: performance.now() - started }, [wire.buffer, request.rgba.buffer]);
      return;
    }
    if (request.type === "fit" || request.type === "fit-faces") {
      // The font fit runs here, off the main thread; the faces stay for the worker's life.
      const sink = { report: (percent: number) => scope.postMessage({ id: request.id, kind: "progress", percent }) };
      let reply: string;
      if (request.type === "fit") {
        reply = fit_start(request.request, sink);
      } else {
        for (const face of request.faces) {
          fit_add_face(face.file, face.index, face.bytes);
        }
        reply = fit_step(sink);
      }
      scope.postMessage({ id: request.id, kind: "fit", reply, ms: performance.now() - started });
      return;
    }
    if (request.type === "ocr") {
      // The weights are loaded once per worker (ADR 0028 §5); a later read skips the load.
      const error = ocr_load(request.weights.detector, request.weights.recogniser, request.weights.alphabet);
      const lines =
        error === ""
          ? ocr_job(request.rgba, request.width, request.height, {
              report: (percent: number) => scope.postMessage({ id: request.id, kind: "progress", percent }),
            })
          : "";
      // The pixels go back to the page for the trace that follows.
      scope.postMessage(
        { id: request.id, kind: "ocr", lines, error: error === "" && lines === "" ? "unreadable" : error, rgba: request.rgba, ms: performance.now() - started },
        [request.rgba.buffer],
      );
      return;
    }
    if (request.type === "png") {
      const png = encode_png(request.rgba, request.width, request.height);
      scope.postMessage({ id: request.id, kind: "png", png, ms: performance.now() - started }, [png.buffer]);
      return;
    }
    const sink = { report: (percent: number) => scope.postMessage({ id: request.id, kind: "progress", percent }) };
    const wire =
      request.lines === ""
        ? trace_job(request.rgba, request.width, request.height, new Float64Array(request.map), request.settings, sink)
        : trace_text_job(request.rgba, request.width, request.height, new Float64Array(request.map), request.settings, request.lines, sink);
    scope.postMessage({ id: request.id, kind: "trace", wire, ms: performance.now() - started }, [wire.buffer]);
  } catch (error) {
    scope.postMessage(
      request.type === "init" ? { id: 0, kind: "ready", error: String(error) } : { id: request.id, kind: "error", message: String(error) },
    );
  }
};

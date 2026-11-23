// The AI steps' worker (`specs/0135` criterion 47, ADR 0075 §3; `specs/0143`, ADR 0078 §3): the
// same `curvyo-editor-wasm` module the page runs, instantiated from the compiled module the page
// posts, with its own memory; it loads a model once from the bytes the page read from the device
// cache and runs one step, or the tiles the pool hands it, at a time, so a Run never blocks the
// page. Worker 0 of a pool also keeps every tile and assembles the picture. Cancel terminates
// the workers.

import { ai_add_tile, ai_assemble, ai_load_model, ai_run, ai_run_tile, ai_set_tile_input, initSync } from "@/wasm-bindings/curvyo_editor_wasm.js";

import type { AiRequest, AiResponse } from "@/lib/aiJobs";
import type { PoolRequest, PoolResponse } from "@/lib/aiPool";

interface WorkerScope {
  onmessage: ((event: MessageEvent<AiRequest | PoolRequest>) => void) | null;
  postMessage(message: AiResponse | PoolResponse, transfer?: Transferable[]): void;
}

const scope = self as unknown as WorkerScope;
let factor = 1;
let modelKind = "";

/** Loads `model` when given: empty when loaded (or not given), else the reason. */
function load(kind: string, model: Uint8Array | null): string {
  return model ? ai_load_model(kind, model) : "";
}

scope.onmessage = (event) => {
  const request = event.data;
  try {
    switch (request.type) {
      case "init":
        initSync({ module: request.module });
        scope.postMessage({ kind: "ready" });
        return;
      case "tiles": {
        const loaded = load(request.modelKind, request.model);
        if (loaded !== "") {
          scope.postMessage({ kind: "error", message: loaded });
          return;
        }
        modelKind = request.modelKind;
        factor = request.factor;
        ai_set_tile_input(request.rgba, request.width, request.height);
        scope.postMessage({ kind: "tilesReady" });
        return;
      }
      case "tile": {
        const tile = ai_run_tile(modelKind, factor, request.index);
        const pixels = tile.take_pixels();
        scope.postMessage({ kind: "tile", index: tile.index, width: tile.width, height: tile.height, pixels }, [pixels.buffer]);
        tile.free();
        return;
      }
      case "add":
        ai_add_tile(request.index, request.width, request.height, request.pixels);
        return;
      case "assemble": {
        const pixels = ai_assemble(request.width, request.height);
        scope.postMessage({ kind: "done", pixels }, [pixels.buffer]);
        return;
      }
      case "run": {
        const loaded = load(request.modelKind, request.model);
        if (loaded !== "") {
          scope.postMessage({ kind: "error", message: loaded });
          return;
        }
        const sink = { report: (percent: number) => scope.postMessage({ kind: "progress", percent }) };
        const pixels = ai_run(request.modelKind, request.rgba, request.width, request.height, request.factor, sink);
        scope.postMessage({ kind: "done", pixels }, [pixels.buffer]);
        return;
      }
    }
  } catch (error) {
    scope.postMessage({ kind: "error", message: error instanceof Error ? error.message : String(error) });
  }
};

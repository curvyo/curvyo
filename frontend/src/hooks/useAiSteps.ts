import { useEffect, useMemo, useState } from "react";

import { createAiRunner } from "@/lib/aiJobs";
import { AiSteps } from "@/lib/aiSteps";
import type { WasmSession } from "@/lib/editorSession";
import type { PinnedModel } from "@/lib/modelDownload";
import { indexedDbModelStore, memoryModelStore } from "@/lib/modelStore";
import { ai_models_json, ai_tile_count, ai_verify_model } from "@/wasm-bindings/curvyo_editor_wasm.js";

/**
 * The AI steps of the Adjustments group (`specs/0135` criteria 47, 63 to 65): one flow per page
 * over the device cache, the one fetch of a model and the AI worker. Nothing is fetched or read
 * until a Run; showing a step reads only whether its model is on the device.
 */
export function useAiSteps(getSession: () => WasmSession | null, sync: () => void, shown: boolean): AiSteps {
  const [, setTick] = useState(0);
  const steps = useMemo(() => {
    return new AiSteps({
      session: getSession,
      // Read on first use: this runs during the first render, before the wasm is loaded.
      models: () => JSON.parse(ai_models_json()) as PinnedModel[],
      store: typeof indexedDB === "undefined" ? memoryModelStore() : indexedDbModelStore,
      fetch: (url, init) => fetch(url, init),
      verify: (kind, bytes) => ai_verify_model(kind, bytes),
      runner: createAiRunner(ai_tile_count),
      storage: globalThis.navigator?.storage,
      changed: () => {
        setTick((n) => n + 1);
        sync();
      },
    });
  }, [getSession, sync]);
  useEffect(() => {
    if (shown) void steps.refresh();
  }, [steps, shown]);
  return steps;
}

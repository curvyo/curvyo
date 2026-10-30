/** Fetches the mask of the Trace tool's Mask view from the worker (`specs/0089` criterion 21). */
import { useCallback, useRef } from "react";

import type { WasmSession } from "@/lib/editorSession";
import type { TraceRunner } from "@/lib/traceJobs";
import { pixelsOf } from "@/lib/traceSource";

/** The pixels the last preview step traced: the image and whether it was the reduced copy. */
export interface MaskSource {
  id: string;
  reduced: boolean;
}

/**
 * The mask jobs of the Mask view: each preview step asks for the mask of the same pixels and
 * settings before it traces them, while the Mask view shows; a view change to Mask asks for the
 * pixels of the last step. The mask comes from the same code path as the trace in the worker, so
 * it is exactly what the tracer reads; an answer whose job became stale is dropped.
 */
export function useTraceMask(getSession: () => WasmSession | null, onChange: () => void, runner: () => TraceRunner) {
  const last = useRef<MaskSource | null>(null);

  const maskStep = useCallback(
    async (id: string, reduced: boolean, settings: string, stale: () => boolean) => {
      last.current = { id, reduced };
      const session = getSession();
      if (!session || session.trace_view_mode() !== "mask") {
        return;
      }
      const pixels = pixelsOf(session, id, reduced);
      if (!pixels) {
        return;
      }
      const mask = await runner().mask(pixels, settings);
      if (stale() || !mask) {
        return;
      }
      session.set_trace_mask(id, mask.width, mask.height, mask.bits);
      onChange();
    },
    [getSession, onChange, runner],
  );

  return { maskStep, last };
}

// Dev-only GPU-path variants of the perf build (`tools/perf/measure.py --gpu-variants`): splits a
// variant name like `ctx-no-depth+samples-1` into what the page applies (the editor canvas's
// context attributes before wgpu asks for it, a draw-timing switch, a page ablation, the input)
// and what the wasm applies (`curvyo-editor-wasm/src/gpu_variant.rs`). Pure: no DOM, no wasm.
import { type Ablation, ABLATIONS } from "./perfAblate.ts";

/** The parts the wasm applies (`GpuVariant::parse`). */
const WASM_PARTS = /^(baseline|samples-(1|2|4|8|16)|no-fences|reuse-vertex-buffer|cache-vertices)$/;

/** The context attributes wgpu asks for (wgpu-hal `gles/web.rs`: only `antialias: false`). A
 * context created first with more attributes is the one `getContext` hands wgpu. */
const WGPU_ATTRIBUTES: WebGLContextAttributes = { antialias: false };

/** The pre-created editor contexts, by part name. */
export const CONTEXT_PARTS: Record<string, WebGLContextAttributes> = {
  "ctx-no-depth": { depth: false },
  "ctx-opaque": { alpha: false },
  "ctx-desync": { desynchronized: true },
  "ctx-all": {
    alpha: false, antialias: false, depth: false, stencil: false, premultipliedAlpha: true,
    powerPreference: "high-performance", desynchronized: true, preserveDrawingBuffer: false,
  },
};

/** One variant, split by who applies it. */
export type GpuPlan = {
  /** The parts for `perf_set_gpu_variant`, joined by `+` ("" for none). */
  wasm: string;
  /** The attributes to create the editor canvas's context with before attach, or null. */
  context: WebGLContextAttributes | null;
  /** The page ablation applied after attach (`page-<ablation>`). */
  page: Ablation;
  /** `draw-after-input`: `render()` right after each input burst instead of in the render loop. */
  drawAfterInput: boolean;
  /** The input of the measured window: a pan, or none with a forced redraw every frame
   * (`redraw-static`: the whole present path without any page change). */
  input: "pan" | "redraw";
  /** Parts nobody knows. */
  unknown: string[];
};

/** Splits `name` (parts joined by `+`) by who applies each part. */
export function planGpuVariant(name: string): GpuPlan {
  const plan: GpuPlan = { wasm: "", context: null, page: "baseline", drawAfterInput: false, input: "pan", unknown: [] };
  const wasm: string[] = [];
  for (const part of name.split("+").map((p) => p.trim()).filter((p) => p.length > 0)) {
    if (WASM_PARTS.test(part)) {
      if (part !== "baseline") wasm.push(part);
    } else if (part in CONTEXT_PARTS) {
      plan.context = { ...WGPU_ATTRIBUTES, ...plan.context, ...CONTEXT_PARTS[part] };
    } else if (part === "draw-after-input") {
      plan.drawAfterInput = true;
    } else if (part === "redraw-static") {
      plan.input = "redraw";
    } else if (part.startsWith("page-") && (ABLATIONS as readonly string[]).includes(part.slice(5))) {
      plan.page = part.slice(5) as Ablation;
    } else {
      plan.unknown.push(part);
    }
  }
  plan.wasm = wasm.join("+");
  return plan;
}

/** The attributes of `asked` the context did not take (`actual` from `getContextAttributes`):
 * `name=asked->actual`, empty when it took them all. */
export function attributesNotTaken(asked: WebGLContextAttributes | null, actual: WebGLContextAttributes | null): string[] {
  if (!asked) return [];
  if (!actual) return ["no context"];
  const record = actual as Record<string, unknown>;
  return Object.entries(asked)
    .filter(([key, value]) => record[key] !== value)
    .map(([key, value]) => `${key}=${String(value)}->${String(record[key])}`);
}

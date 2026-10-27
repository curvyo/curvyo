/**
 * The page's side of the trace worker (`specs/0025-raster-trace` criteria 10, 17 and 58, ADR 0023
 * §5): one worker at a time, fed the compiled wasm module once; a job is a promise; cancelling
 * terminates the worker and the next job starts a fresh one from the cached module.
 */

/** A message to the worker. */
export type TraceRequest =
  | { type: "init"; module: WebAssembly.Module }
  | { type: "threshold"; id: number; rgba: Uint8Array; width: number; height: number }
  | {
      type: "trace";
      id: number;
      rgba: Uint8Array;
      width: number;
      height: number;
      map: number[];
      threshold: number;
      smooth: number;
      specks: number;
    };

/** A message from the worker. */
export type TraceResponse =
  | { id: number; kind: "threshold"; threshold: number; ms: number }
  | { id: number; kind: "trace"; wire: Float64Array; ms: number }
  | { id: number; kind: "error"; message: string };

/** The pixels of one job. */
export interface TracePixels {
  width: number;
  height: number;
  rgba: Uint8Array;
  map: number[];
}

/** A finished trace: the worker's numbers and the compute time. */
export interface TraceOutput {
  wire: Float64Array;
  seconds: number;
}

/** Thrown into a job's promise when it was cancelled. */
export class TraceCancelled extends Error {
  constructor() {
    super("trace cancelled");
    this.name = "TraceCancelled";
  }
}

/** The minimal worker the runner drives, so a test can hand in a fake. */
export interface TraceWorkerLike {
  onmessage: ((event: MessageEvent<TraceResponse>) => void) | null;
  onerror: ((event: ErrorEvent) => void) | null;
  postMessage(message: TraceRequest, transfer?: Transferable[]): void;
  terminate(): void;
}

type Pending = { resolve: (response: TraceResponse) => void; reject: (error: Error) => void };

/** Runs trace jobs in a worker; `cancel` ends every running job by terminating it. */
export class TraceRunner {
  private worker: TraceWorkerLike | null = null;
  private nextId = 1;
  private readonly pending = new Map<number, Pending>();
  private readonly makeWorker: () => TraceWorkerLike;
  private readonly module: () => Promise<WebAssembly.Module>;

  constructor(makeWorker: () => TraceWorkerLike, module: () => Promise<WebAssembly.Module>) {
    this.makeWorker = makeWorker;
    this.module = module;
  }

  /** Whether a job is running. */
  get busy(): boolean {
    return this.pending.size > 0;
  }

  private async ready(): Promise<TraceWorkerLike> {
    if (this.worker) {
      return this.worker;
    }
    const module = await this.module();
    if (this.worker) {
      return this.worker;
    }
    const worker = this.makeWorker();
    worker.onmessage = (event) => {
      const response = event.data;
      const pending = this.pending.get(response.id);
      if (pending) {
        this.pending.delete(response.id);
        pending.resolve(response);
      }
    };
    worker.onerror = (event) => {
      const failure = new Error(event.message || "trace worker failed");
      for (const pending of this.pending.values()) {
        pending.reject(failure);
      }
      this.pending.clear();
      worker.terminate();
      if (this.worker === worker) {
        this.worker = null;
      }
    };
    worker.postMessage({ type: "init", module });
    this.worker = worker;
    return worker;
  }

  private async send(make: (id: number) => TraceRequest, transfer: Transferable[]): Promise<TraceResponse> {
    const worker = await this.ready();
    const id = this.nextId++;
    return new Promise<TraceResponse>((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      worker.postMessage(make(id), transfer);
    });
  }

  /** The automatic Threshold of `pixels` (criterion 9). */
  async threshold(pixels: TracePixels): Promise<number> {
    const response = await this.send(
      (id) => ({ type: "threshold", id, rgba: pixels.rgba, width: pixels.width, height: pixels.height }),
      [],
    );
    if (response.kind !== "threshold") {
      throw new Error(response.kind === "error" ? response.message : "unexpected reply");
    }
    return response.threshold;
  }

  /** One trace of `pixels` with the bar's values; the pixels are handed over to the worker. */
  async trace(pixels: TracePixels, threshold: number, smooth: number, specks: number): Promise<TraceOutput> {
    const response = await this.send(
      (id) => ({
        type: "trace",
        id,
        rgba: pixels.rgba,
        width: pixels.width,
        height: pixels.height,
        map: pixels.map,
        threshold,
        smooth,
        specks,
      }),
      [pixels.rgba.buffer],
    );
    if (response.kind !== "trace") {
      throw new Error(response.kind === "error" ? response.message : "unexpected reply");
    }
    return { wire: response.wire, seconds: response.ms / 1000 };
  }

  /** Ends every running job: the worker is terminated and their promises reject with
   * [`TraceCancelled`]; the next job starts a fresh worker. */
  cancel(): void {
    if (this.pending.size === 0) {
      return;
    }
    this.worker?.terminate();
    this.worker = null;
    for (const pending of this.pending.values()) {
      pending.reject(new TraceCancelled());
    }
    this.pending.clear();
  }

  /** Ends the worker for good (the editor unmounts). */
  dispose(): void {
    this.cancel();
    this.worker?.terminate();
    this.worker = null;
  }
}

let compiled: Promise<WebAssembly.Module> | null = null;

/** The editor's wasm module, compiled once for every worker (ADR 0023 §5). */
export function editorModule(): Promise<WebAssembly.Module> {
  compiled ??= (async () => {
    const url = new URL("../wasm-bindings/curvyo_editor_wasm_bg.wasm", import.meta.url);
    const response = await fetch(url);
    try {
      return await WebAssembly.compileStreaming(response.clone());
    } catch {
      // A server without the `application/wasm` type: compile the bytes.
      return WebAssembly.compile(await response.arrayBuffer());
    }
  })();
  return compiled;
}

/** A runner over the real worker. */
export function createTraceRunner(): TraceRunner {
  return new TraceRunner(
    () => new Worker(new URL("../workers/traceWorker.ts", import.meta.url), { type: "module" }),
    editorModule,
  );
}

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
      /** The settings JSON of `settingsJson` in `traceText`. */
      settings: string;
      /** The OCR lines JSON of the picture (`specs/0091`), empty without text recognition. */
      lines: string;
    }
  /** The mask the same trace would walk, for the Mask view (`specs/0089` criterion 21). */
  | { type: "mask"; id: number; rgba: Uint8Array; width: number; height: number; map: number[]; settings: string; lines: string }
  /** The text pass alone (`specs/0091` criterion 25): the read lines' own traces, before the fit. */
  | { type: "regions"; id: number; rgba: Uint8Array; width: number; height: number; map: number[]; settings: string; lines: string }
  /** The font fit of the read lines (`specs/0091` decision 4): the session's request. */
  | { type: "fit"; id: number; request: string }
  /** The faces a fit asked for, as `[file, index, bytes]`; the fit goes on. */
  | { type: "fit-faces"; id: number; faces: FitFace[] }
  /** The text lines of a picture, read once (`specs/0091` decision 1), with the weights. */
  | { type: "ocr"; id: number; rgba: Uint8Array; width: number; height: number; weights: OcrWeights }
  /** The PNG of a copied image (`specs/0087-image-clipboard-and-drop` criterion 14). */
  | { type: "png"; id: number; rgba: Uint8Array; width: number; height: number };

/** A message from the worker. */
export type TraceResponse =
  /** The automatic Threshold, the suggested background `#RRGGBB` (empty: transparent) and the
   * analysis JSON (`specs/0089` slice F; empty when it failed). */
  | { id: number; kind: "threshold"; threshold: number; background: string; analysis: string; ms: number }
  | { id: number; kind: "trace"; wire: Float64Array; ms: number }
  /** The text pass's numbers and, handed back, the pixels (`specs/0091` criterion 25). */
  | { id: number; kind: "regions"; wire: Float64Array; rgba: Uint8Array; ms: number }
  /** A running trace reached a higher whole percent (criterion 10). */
  | { id: number; kind: "progress"; percent: number }
  | { id: number; kind: "png"; png: Uint8Array; ms: number }
  /** A fit's answer: `{kind: "need", faces}` or `{kind: "finished", lines}` as JSON. */
  | { id: number; kind: "fit"; reply: string; ms: number }
  /** The OCR lines JSON; empty with `error` set when the reader could not be used. */
  | { id: number; kind: "ocr"; lines: string; error: string; rgba: Uint8Array; ms: number }
  /** A mask: 255 for content, 0 for the rest, row by row; `width` 0 when the picture cannot be traced. */
  | { id: number; kind: "mask"; width: number; height: number; bits: Uint8Array; ms: number }
  | { id: number; kind: "error"; message: string }
  /** The answer to `init` (id 0): `error` is empty when the module was instantiated. */
  | { id: 0; kind: "ready"; error: string };

/** The text reader's three files (ADR 0028 §5), fetched same-origin once per page. */
export interface OcrWeights {
  detector: Uint8Array;
  recogniser: Uint8Array;
  alphabet: string;
}

/** A font file face handed to the worker's fit. */
export interface FitFace {
  file: string;
  index: number;
  bytes: Uint8Array;
}

/** The OCR read failed (the weights could not be loaded or read). */
export class OcrFailed extends Error {
  constructor(reason: string) {
    super(`text recognition failed: ${reason}`);
    this.name = "OcrFailed";
  }
}

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

/** The mask of the Mask view: `width` x `height` mask pixels, 255 for content. */
export interface TraceMask {
  width: number;
  height: number;
  bits: Uint8Array;
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

type Pending = {
  resolve: (response: TraceResponse) => void;
  reject: (error: Error) => void;
  onProgress?: (percent: number) => void;
};

/** How long a new worker may take to answer `init`, and the module to compile, before the jobs
 * waiting for it fail (Proposal, customer 2026-10-31: a webview whose worker never starts showed
 * "Analysing..." for good). A 15 MB module compiles in well under a second. */
export const WORKER_START_MS = 20_000;

/** Thrown into the jobs of a worker that did not start: it did not answer `init` in time, or its
 * module could not be instantiated. */
export class TraceWorkerFailed extends Error {
  constructor(reason: string) {
    super(`the trace worker did not start: ${reason}`);
    this.name = "TraceWorkerFailed";
  }
}

/** Runs trace jobs in a worker; `cancel` ends every running job by terminating it. */
export class TraceRunner {
  private worker: TraceWorkerLike | null = null;
  private startTimer: ReturnType<typeof setTimeout> | null = null;
  private nextId = 1;
  private readonly pending = new Map<number, Pending>();
  private readonly makeWorker: () => TraceWorkerLike;
  private readonly module: () => Promise<WebAssembly.Module>;
  private readonly startMs: number;

  constructor(makeWorker: () => TraceWorkerLike, module: () => Promise<WebAssembly.Module>, startMs = WORKER_START_MS) {
    this.makeWorker = makeWorker;
    this.module = module;
    this.startMs = startMs;
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
      if (response.kind === "ready") {
        this.clearStartTimer();
        if (response.error !== "") {
          this.fail(worker, new TraceWorkerFailed(response.error));
        }
        return;
      }
      const pending = this.pending.get(response.id);
      if (pending && response.kind === "progress") {
        pending.onProgress?.(response.percent);
      } else if (pending) {
        this.pending.delete(response.id);
        pending.resolve(response);
      }
    };
    worker.onerror = (event) => this.fail(worker, new Error(event.message || "trace worker failed"));
    worker.postMessage({ type: "init", module });
    this.worker = worker;
    this.clearStartTimer();
    this.startTimer = setTimeout(() => this.fail(worker, new TraceWorkerFailed(`no answer in ${this.startMs} ms`)), this.startMs);
    return worker;
  }

  private clearStartTimer(): void {
    if (this.startTimer !== null) {
      clearTimeout(this.startTimer);
      this.startTimer = null;
    }
  }

  /** `worker` cannot answer: its jobs fail with `failure` and the next job starts a fresh one. */
  private fail(worker: TraceWorkerLike, failure: Error): void {
    if (this.worker !== worker) {
      return;
    }
    this.clearStartTimer();
    for (const pending of this.pending.values()) {
      pending.reject(failure);
    }
    this.pending.clear();
    worker.terminate();
    this.worker = null;
  }

  private async send(
    make: (id: number) => TraceRequest,
    transfer: Transferable[],
    onProgress?: (percent: number) => void,
  ): Promise<TraceResponse> {
    const worker = await this.ready();
    const id = this.nextId++;
    return new Promise<TraceResponse>((resolve, reject) => {
      this.pending.set(id, { resolve, reject, onProgress });
      worker.postMessage(make(id), transfer);
    });
  }

  /** The automatic Threshold of `pixels` (criterion 9). */
  async threshold(pixels: TracePixels): Promise<number> {
    return (await this.defaults(pixels)).threshold;
  }

  /** What a new image starts with: the automatic Threshold (criterion 9), the background colour
   * Remove background's Auto suggests (`specs/0089` criterion 16; empty: transparent) and the
   * analysis JSON (`specs/0089` slice F; empty when it failed). */
  async defaults(pixels: TracePixels): Promise<{ threshold: number; background: string; analysis: string }> {
    const response = await this.send(
      (id) => ({ type: "threshold", id, rgba: pixels.rgba, width: pixels.width, height: pixels.height }),
      [],
    );
    if (response.kind !== "threshold") {
      throw new Error(response.kind === "error" ? response.message : "unexpected reply");
    }
    return { threshold: response.threshold, background: response.background ?? "", analysis: response.analysis ?? "" };
  }

  /** One trace of `pixels` with `settings` (the JSON of `settingsJson`); the pixels are handed
   * over to the worker; `onProgress` hears each higher whole percent. */
  async trace(pixels: TracePixels, settings: string, onProgress?: (percent: number) => void, lines = ""): Promise<TraceOutput> {
    const response = await this.send(
      (id) => ({
        type: "trace",
        id,
        rgba: pixels.rgba,
        width: pixels.width,
        height: pixels.height,
        map: pixels.map,
        settings,
        lines,
      }),
      [pixels.rgba.buffer],
      onProgress,
    );
    if (response.kind !== "trace") {
      throw new Error(response.kind === "error" ? response.message : "unexpected reply");
    }
    return { wire: response.wire, seconds: response.ms / 1000 };
  }

  /** The text pass alone of `pixels` with `settings` and the OCR `lines` (`specs/0091` criterion
   * 25): the numbers the fit request is made of; the pixels are handed over and come back for the
   * trace that follows. */
  async regions(pixels: TracePixels, settings: string, lines: string): Promise<Float64Array> {
    const response = await this.send(
      (id) => ({ type: "regions", id, rgba: pixels.rgba, width: pixels.width, height: pixels.height, map: pixels.map, settings, lines }),
      [pixels.rgba.buffer],
    );
    if (response.kind !== "regions") {
      throw new Error(response.kind === "error" ? response.message : "unexpected reply");
    }
    pixels.rgba = response.rgba;
    return response.wire;
  }

  /** The mask a trace of `pixels` with `settings` would walk (`specs/0089` criterion 21), or
   * `null` when the picture cannot be traced; the pixels are handed over. */
  async mask(pixels: TracePixels, settings: string, lines = ""): Promise<TraceMask | null> {
    const response = await this.send(
      (id) => ({ type: "mask", id, rgba: pixels.rgba, width: pixels.width, height: pixels.height, map: pixels.map, settings, lines }),
      [pixels.rgba.buffer],
    );
    if (response.kind !== "mask") {
      throw new Error(response.kind === "error" ? response.message : "unexpected reply");
    }
    return response.width > 0 ? { width: response.width, height: response.height, bits: response.bits } : null;
  }

  /** The text lines of `pixels` read with `weights` (`specs/0091` decision 1): the OCR lines
   * JSON and, handed back, the pixels (so the trace that follows needs no second copy out of the
   * session, 0025 criterion 58); `error` is set, and `lines` empty, when the reader could not be
   * used. `onProgress` hears each higher whole percent. */
  async ocr(pixels: TracePixels, weights: OcrWeights, onProgress?: (percent: number) => void): Promise<{ lines: string; error: string; rgba: Uint8Array }> {
    const response = await this.send(
      (id) => ({ type: "ocr", id, rgba: pixels.rgba, width: pixels.width, height: pixels.height, weights }),
      [pixels.rgba.buffer],
      onProgress,
    );
    if (response.kind !== "ocr") {
      throw new OcrFailed(response.kind === "error" ? response.message : "unexpected reply");
    }
    return { lines: response.lines, error: response.error, rgba: response.rgba };
  }

  /** The font fit of `request` (the session's `trace_fit_request`) in the worker, off the main
   * thread (`specs/0091` decision 4): the faces it asks for are read by `loadFaces` and handed
   * in, as often as it asks; the finished result JSON. Cancelled like every job. */
  async fit(
    request: string,
    loadFaces: (faces: [string, number][]) => Promise<FitFace[]>,
    onProgress?: (percent: number) => void,
  ): Promise<string> {
    let response = await this.send((id) => ({ type: "fit", id, request }), [], onProgress);
    for (;;) {
      if (response.kind !== "fit") {
        throw new Error(response.kind === "error" ? response.message : "unexpected reply");
      }
      const reply = JSON.parse(response.reply) as { kind: string; faces?: [string, number][] };
      if (reply.kind !== "need") {
        return response.reply;
      }
      const faces = await loadFaces(reply.faces ?? []);
      response = await this.send((id) => ({ type: "fit-faces", id, faces }), [], onProgress);
    }
  }

  /** The PNG of `rgba` (`width` x `height`, straight alpha), made in the worker; the pixels are
   * handed over (`specs/0087-image-clipboard-and-drop` criterion 14). */
  async png(rgba: Uint8Array, width: number, height: number): Promise<Uint8Array> {
    const response = await this.send(
      (id) => ({ type: "png", id, rgba, width, height }),
      [rgba.buffer],
    );
    if (response.kind !== "png") {
      throw new Error(response.kind === "error" ? response.message : "unexpected reply");
    }
    return response.png;
  }

  /** Ends every running job: the worker is terminated and their promises reject with
   * [`TraceCancelled`]; the next job starts a fresh worker. */
  cancel(): void {
    if (this.pending.size === 0) {
      return;
    }
    this.clearStartTimer();
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
    this.clearStartTimer();
    this.worker?.terminate();
    this.worker = null;
  }
}

let compiled: Promise<WebAssembly.Module> | null = null;

/** `promise`, or a [`TraceWorkerFailed`] after `ms`. */
function withinStart<T>(promise: Promise<T>, ms: number, what: string): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const late = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new TraceWorkerFailed(`${what} took over ${ms} ms`)), ms);
  });
  return Promise.race([promise, late]).finally(() => clearTimeout(timer));
}

/** The editor's wasm module, compiled once for every worker (ADR 0023 §5); a compile that fails
 * or does not finish in [`WORKER_START_MS`] is tried again by the next job. */
export function editorModule(): Promise<WebAssembly.Module> {
  compiled ??= withinStart(
    (async () => {
      const url = new URL("../wasm-bindings/curvyo_editor_wasm_bg.wasm", import.meta.url);
      const response = await fetch(url);
      try {
        return await WebAssembly.compileStreaming(response.clone());
      } catch {
        // A server without the `application/wasm` type: compile the bytes.
        return WebAssembly.compile(await response.arrayBuffer());
      }
    })(),
    WORKER_START_MS,
    "compiling the module",
  ).catch((error: unknown) => {
    compiled = null;
    throw error;
  });
  return compiled;
}

let weights: Promise<OcrWeights> | null = null;

/** The folder of the shipped weights (ADR 0028 §5): the app's own files, no download. */
export const OCR_MODEL_DIR = "models/paddleocr-latin/";

/** The text reader's files, fetched same-origin once per page (ADR 0028 §5); a failed fetch is
 * tried again by the next call. */
export function ocrWeights(fetchFile: (path: string) => Promise<Response> = (path) => fetch(`${import.meta.env.BASE_URL}${path}`)): Promise<OcrWeights> {
  weights ??= (async () => {
    const read = async (name: string) => {
      const response = await fetchFile(`${OCR_MODEL_DIR}${name}`);
      if (!response.ok) {
        throw new OcrFailed(`${name}: ${response.status}`);
      }
      return new Uint8Array(await response.arrayBuffer());
    };
    const [detector, recogniser, alphabet] = await Promise.all([read("det.onnx"), read("rec.onnx"), read("dict.txt")]);
    return { detector, recogniser, alphabet: new TextDecoder().decode(alphabet) };
  })().catch((error: unknown) => {
    weights = null;
    throw error;
  });
  return weights;
}

/** A runner over the real worker. */
export function createTraceRunner(): TraceRunner {
  return new TraceRunner(
    () => new Worker(new URL("../workers/traceWorker.ts", import.meta.url), { type: "module" }),
    editorModule,
  );
}

/**
 * The page's side of the AI workers (`specs/0135` criterion 47, ADR 0075 §3; `specs/0143`, ADR
 * 0078): worker 0, fed the compiled wasm module once, and for Upscale and AI denoise a pool of
 * extra workers for the Run's tiles; a Run is a promise with progress; Cancel terminates every
 * worker and the next Run starts a fresh one (which loads its model again).
 */
import { type PoolRequest, type PoolResponse, type PoolWorker, TILED_KINDS, poolSize, runPool } from "./aiPool.ts";
import { editorModule } from "./traceJobs.ts";

/** A message to the worker. */
export type AiRequest =
  | { type: "init"; module: WebAssembly.Module }
  | {
      type: "run";
      modelKind: string;
      /** The model file, when this worker has not loaded it yet. */
      model: Uint8Array | null;
      rgba: Uint8Array;
      width: number;
      height: number;
      factor: number;
    };

/** A message from the worker. */
export type AiResponse =
  | { kind: "ready" }
  | { kind: "progress"; percent: number }
  | { kind: "done"; pixels: Uint8Array }
  | { kind: "error"; message: string };

/** The output of a Run and its size (`320 x 320` grey for a mask). */
export interface AiOutput {
  pixels: Uint8Array;
  width: number;
  height: number;
  grey: boolean;
}

/** The output size of a run of `modelKind` on `width` x `height` (ADR 0075 §2). */
export function outputSize(modelKind: string, width: number, height: number, factor: number): AiOutput {
  if (modelKind === "background") return { pixels: new Uint8Array(), width: 320, height: 320, grey: true };
  const f = modelKind === "upscale" ? factor : 1;
  return { pixels: new Uint8Array(), width: width * f, height: height * f, grey: false };
}

/** The worker a runner uses (the tests pass a fake). */
export interface AiWorkerLike {
  postMessage(message: AiRequest | PoolRequest, transfer?: Transferable[]): void;
  onmessage: ((event: MessageEvent<AiResponse | PoolResponse>) => void) | null;
  terminate(): void;
}

/** The rejection of a Run that was cancelled (the row is already idle). */
export class AiCancelled extends Error {
  constructor() {
    super("cancelled");
    this.name = "AiCancelled";
  }
}

/** How long worker 0 stays after a Run before it ends (criterion 6, Proposal). */
export const IDLE_MS = 60_000;

/** The extras of a Run beyond the plain `run` call (`specs/0143`). */
export interface RunExtras {
  /** The model file for the pool's extra workers (worker 0 may hold it already). */
  readModel?: () => Promise<Uint8Array>;
  /** The pool size of a tiled Run, once it is known (1: "Running on one core"). */
  workers?: (n: number) => void;
}

/** What a runner is built from beyond the worker and module (the tests pass fakes). */
export interface RunnerDeps {
  /** Logical cores (`navigator.hardwareConcurrency`). */
  cores: () => number | undefined;
  /** The number of tiles of `width` x `height` (`ai_tile_count`). */
  tileCount: (width: number, height: number) => number;
  now: () => number;
  /** Calls `run` after `ms`; the returned call cancels it. */
  later: (run: () => void, ms: number) => () => void;
}

/** Worker 0 and, during a tiled Run, its pool; the models worker 0 loaded, by kind. */
export class AiRunner {
  private worker: AiWorkerLike | null = null;
  private pool = new Set<AiWorkerLike>();
  private loaded = new Set<string>();
  private idle: (() => void) | null = null;
  private abort: AbortController | null = null;
  private readonly spawn: () => AiWorkerLike;
  private readonly module: () => Promise<WebAssembly.Module>;
  private readonly deps: RunnerDeps;

  constructor(spawn: () => AiWorkerLike, module: () => Promise<WebAssembly.Module>, deps: RunnerDeps) {
    this.spawn = spawn;
    this.module = module;
    this.deps = deps;
  }

  /** A fresh worker with the module instantiated, or the reason it did not start. */
  private async started(): Promise<AiWorkerLike> {
    const worker = this.spawn();
    const module = await this.module();
    await new Promise<void>((resolve, reject) => {
      worker.onmessage = (event) => (event.data.kind === "ready" ? resolve() : reject(new Error("worker failed")));
      worker.postMessage({ type: "init", module });
    }).catch((error: unknown) => {
      worker.terminate();
      throw error;
    });
    return worker;
  }

  private async ready(): Promise<AiWorkerLike> {
    this.idle?.();
    this.idle = null;
    if (this.worker) return this.worker;
    const worker = await this.started();
    this.worker = worker;
    this.loaded.clear();
    return worker;
  }

  /** Whether the worker holds model `kind` (so the bytes need not be read from the cache). */
  hasModel(kind: string): boolean {
    return this.worker !== null && this.loaded.has(kind);
  }

  /** Runs `modelKind` on the input; `model` the file bytes when not loaded. Upscale and AI
   * denoise run in tiles on a pool (ADR 0078); `progress` hears whole percents. */
  async run(
    modelKind: string,
    model: Uint8Array | null,
    input: { rgba: Uint8Array; width: number; height: number; factor: number },
    progress: (percent: number) => void,
    extras: RunExtras = {},
  ): Promise<AiOutput> {
    const worker = await this.ready();
    const size = outputSize(modelKind, input.width, input.height, input.factor);
    const abort = new AbortController();
    this.abort = abort;
    try {
      const pixels = TILED_KINDS.has(modelKind)
        ? await this.tiled(worker, modelKind, model, input, progress, extras, abort.signal)
        : await this.whole(worker, modelKind, model, input, progress, abort.signal);
      this.loaded.add(modelKind);
      return { ...size, pixels };
    } finally {
      if (this.abort === abort) this.abort = null;
      if (this.worker === worker) this.rest();
    }
  }

  private whole(
    worker: AiWorkerLike,
    modelKind: string,
    model: Uint8Array | null,
    input: { rgba: Uint8Array; width: number; height: number; factor: number },
    progress: (percent: number) => void,
    signal: AbortSignal,
  ): Promise<Uint8Array> {
    return new Promise((resolve, reject) => {
      signal.addEventListener(
        "abort",
        () => {
          worker.onmessage = null;
          reject(new AiCancelled());
        },
        { once: true },
      );
      worker.onmessage = (event) => {
        const message = event.data;
        if (message.kind === "progress") progress(message.percent);
        else if (message.kind === "done") resolve(message.pixels);
        else if (message.kind === "error") reject(new Error(message.message));
      };
      worker.postMessage({ type: "run", modelKind, model, ...input }, [input.rgba.buffer]);
    });
  }

  private tiled(
    worker: AiWorkerLike,
    modelKind: string,
    model: Uint8Array | null,
    input: { rgba: Uint8Array; width: number; height: number; factor: number },
    progress: (percent: number) => void,
    extras: RunExtras,
    signal: AbortSignal,
  ): Promise<Uint8Array> {
    const n = extras.readModel ? poolSize(this.deps.cores(), input.rgba.byteLength) : 1;
    extras.workers?.(n);
    const spawned = Array.from({ length: n - 1 }, () =>
      this.started().then(
        (w) => {
          if (this.worker !== worker) {
            w.terminate();
            return null;
          }
          this.pool.add(w);
          return w as PoolWorker;
        },
        () => null,
      ),
    );
    const firstHasModel = this.loaded.has(modelKind);
    const readModel = model ? async () => model : (extras.readModel ?? (() => Promise.reject(new Error("no model file"))));
    let last = -1;
    return runPool({
      modelKind,
      input,
      output: outputSize(modelKind, input.width, input.height, input.factor),
      tiles: this.deps.tileCount(input.width, input.height),
      first: worker as PoolWorker,
      firstHasModel,
      extras: spawned,
      readModel,
      workers: extras.workers,
      signal,
      cancelled: () => new AiCancelled(),
      progress: (done, total) => {
        const percent = Math.floor((done * 100) / Math.max(1, total));
        if (percent !== last) {
          last = percent;
          progress(percent);
        }
      },
      now: this.deps.now,
    });
  }

  /** The Run ended: the pool is gone, worker 0 ends after [`IDLE_MS`] unless another Run comes. */
  private rest(): void {
    for (const w of this.pool) w.terminate();
    this.pool.clear();
    this.idle = this.deps.later(() => this.cancel(), IDLE_MS);
  }

  /** Cancel: every worker is terminated and the pending Run rejects with [`AiCancelled`]; the
   * next Run starts a fresh one. */
  cancel(): void {
    this.abort?.abort();
    this.abort = null;
    this.idle?.();
    this.idle = null;
    this.worker?.terminate();
    this.worker = null;
    for (const w of this.pool) w.terminate();
    this.pool.clear();
    this.loaded.clear();
  }

  /** The workers alive now (worker 0 and the pool), for the tests and the task-manager check. */
  get alive(): number {
    return (this.worker ? 1 : 0) + this.pool.size;
  }
}

/** A runner over the real worker; `tileCount` is the wasm's `ai_tile_count`. */
export function createAiRunner(tileCount: (width: number, height: number) => number): AiRunner {
  return new AiRunner(
    () => new Worker(new URL("../workers/aiWorker.ts", import.meta.url), { type: "module" }) as unknown as AiWorkerLike,
    editorModule,
    {
      cores: () => globalThis.navigator?.hardwareConcurrency,
      tileCount,
      now: () => performance.now(),
      later: (run, ms) => {
        const id = setTimeout(run, ms);
        return () => clearTimeout(id);
      },
    },
  );
}

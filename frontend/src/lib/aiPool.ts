/**
 * How the tiles of one Upscale or AI denoise Run are shared over a pool of AI workers and put
 * back together (`specs/0143`, ADR 0078 §3, §4): each worker is the editor wasm with its own
 * memory; the next free worker takes the next tile, every tile goes to worker 0, which assembles
 * the picture by index, so the result is the same for any pool size and any finishing order.
 * No React and no real workers, so node tests drive it with fakes.
 */

/** The most workers in a pool (Frage 302 A: the cap of the measured scaling). */
export const MAX_POOL = 8;

/** The memory the extra workers of a pool may take together (ADR 0078 §4). */
export const POOL_MEMORY = 2 ** 30;

/** The peak memory of one AI worker during a tile, without its input picture: 62 to 73 MB
 * measured in Chromium (ADR 0078, note 2026-11-21). */
export const WORKER_MEMORY = 64 * 2 ** 20;

/** The shortest time between two progress reports (at most 10 a second, criterion 7). */
export const PROGRESS_MS = 100;

/** The model kinds that run in tiles on a pool (Remove background runs once on worker 0). */
export const TILED_KINDS: ReadonlySet<string> = new Set(["upscale", "denoise"]);

/** The pool size for `cores` logical processors and an input of `inputBytes`:
 * `min(cores - 1, 8, floor(1 GiB / M))`, at least 1, where `M` is one worker's peak with the
 * input picture it holds (criterion 3). */
export function poolSize(cores: number | undefined, inputBytes: number): number {
  const c = Math.floor((cores ?? 2) - 1);
  const byMemory = Math.floor(POOL_MEMORY / (WORKER_MEMORY + Math.max(0, inputBytes)));
  return Math.max(1, Math.min(MAX_POOL, Number.isFinite(c) ? c : 1, byMemory));
}

/** A message to a pool worker (beside `init` and `run` of `AiRequest`). */
export type PoolRequest =
  | { type: "tiles"; modelKind: string; model: Uint8Array | null; rgba: Uint8Array; width: number; height: number; factor: number }
  | { type: "tile"; index: number }
  | { type: "add"; index: number; width: number; height: number; pixels: Uint8Array }
  | { type: "assemble"; width: number; height: number };

/** A message from a pool worker. */
export type PoolResponse =
  | { kind: "tilesReady" }
  | { kind: "tile"; index: number; width: number; height: number; pixels: Uint8Array }
  | { kind: "done"; pixels: Uint8Array }
  | { kind: "error"; message: string };

/** The worker the pool talks to (the tests pass a fake). */
export interface PoolWorker {
  postMessage(message: PoolRequest, transfer?: Transferable[]): void;
  onmessage: ((event: MessageEvent<PoolResponse>) => void) | null;
  terminate(): void;
}

/** One tiled Run. */
export interface PoolRun {
  modelKind: string;
  input: { rgba: Uint8Array; width: number; height: number; factor: number };
  /** The output size (`assemble`'s picture). */
  output: { width: number; height: number };
  /** The number of tiles (`ai_tile_count` of the input). */
  tiles: number;
  /** Worker 0, ready; it assembles. */
  first: PoolWorker;
  /** Whether worker 0 holds the model already. */
  firstHasModel: boolean;
  /** The extra workers as they become ready; a worker that fails to start is never among them. */
  extras: Promise<PoolWorker | null>[];
  /** The model file, read once when a worker needs it. */
  readModel: () => Promise<Uint8Array>;
  /** `done` of `total` tiles, at most every [`PROGRESS_MS`] and once at the end. */
  progress: (done: number, total: number) => void;
  /** The pool size again whenever a worker is dropped or fails to start (criterion 10: a pool
   * that falls to one says "Running on one core"). */
  workers?: (n: number) => void;
  /** Cancel: the Run rejects with `cancelled()` and holds nothing more. */
  signal?: AbortSignal;
  cancelled?: () => Error;
  now: () => number;
}

/** Runs the tiles of `run` on worker 0 and the extras: the assembled RGBA pixels. Every extra
 * worker is terminated when it has no tile left; one that fails is dropped and its tile handed
 * out again; worker 0 failing fails the Run. */
export function runPool(run: PoolRun): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    const queue = Array.from({ length: run.tiles }, (_, i) => i);
    const busy = new Map<PoolWorker, number>();
    const live = new Set<PoolWorker>([run.first]);
    let done = 0;
    let reported = -Infinity;
    let ended = false;
    let model: Promise<Uint8Array> | null = null;
    let size = 1 + run.extras.length;
    const lost = () => {
      size -= 1;
      if (!ended) run.workers?.(size);
    };
    const readModel = () => (model ??= run.readModel());

    const end = () => {
      ended = true;
      for (const w of live) if (w !== run.first) w.terminate();
      live.clear();
    };
    const fail = (error: Error) => {
      if (ended) return;
      end();
      reject(error);
    };
    const give = (w: PoolWorker) => {
      const index = queue.shift();
      if (index !== undefined) {
        busy.set(w, index);
        w.postMessage({ type: "tile", index });
      } else if (w !== run.first) {
        live.delete(w);
        w.terminate();
      }
    };
    const report = () => {
      const t = run.now();
      if (done === run.tiles || t - reported >= PROGRESS_MS) {
        reported = t;
        run.progress(done, run.tiles);
      }
    };
    const drop = (w: PoolWorker) => {
      if (!live.has(w)) return;
      lost();
      live.delete(w);
      w.terminate();
      const index = busy.get(w);
      busy.delete(w);
      if (index !== undefined) {
        queue.unshift(index);
        if (!busy.has(run.first)) give(run.first);
      }
    };
    const listen = (w: PoolWorker) => {
      w.onmessage = (event) => {
        if (ended) return;
        const m = event.data;
        if (m.kind === "error") {
          if (w === run.first) fail(new Error(m.message));
          else drop(w);
        } else if (m.kind === "tilesReady") {
          give(w);
        } else if (m.kind === "tile") {
          busy.delete(w);
          run.first.postMessage({ type: "add", index: m.index, width: m.width, height: m.height, pixels: m.pixels }, [m.pixels.buffer]);
          done += 1;
          report();
          if (done === run.tiles) run.first.postMessage({ type: "assemble", ...run.output });
          else give(w);
        } else if (m.kind === "done" && w === run.first) {
          end();
          resolve(m.pixels);
        }
      };
    };
    const start = async (w: PoolWorker, needsModel: boolean, rgba: Uint8Array, transfer: boolean) => {
      const bytes = needsModel ? await readModel() : null;
      if (ended) return;
      listen(w);
      // A copy of the model per worker: a transfer would leave the next worker without it.
      w.postMessage({ type: "tiles", modelKind: run.modelKind, model: bytes && bytes.slice(), ...run.input, rgba }, transfer ? [rgba.buffer] : []);
    };

    run.signal?.addEventListener("abort", () => fail(run.cancelled?.() ?? new Error("cancelled")), { once: true });
    report();
    if (run.tiles === 0) {
      listen(run.first);
      run.first.postMessage({ type: "assemble", ...run.output });
      return;
    }
    for (const extra of run.extras) {
      void extra.then(
        (w) => {
          if (!w) {
            lost();
            return;
          }
          if (ended || queue.length === 0) {
            w.terminate();
            return;
          }
          live.add(w);
          start(w, true, run.input.rgba.slice(), false).catch(() => drop(w));
        },
        () => lost(),
      );
    }
    // Worker 0 gets a copy too: the extras that start later still copy from the original.
    start(run.first, !run.firstHasModel, run.input.rgba.slice(), true).catch((e: unknown) => fail(e instanceof Error ? e : new Error(String(e))));
  });
}

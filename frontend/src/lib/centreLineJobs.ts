/**
 * The page's side of the Find centre lines worker (`specs/0117-centre-lines` criterion 15b, ADR
 * 0063 amendment 2026-11-18, W): a second worker from the trace worker's script and compiled
 * module, never the trace worker itself. One job is in flight and one pending slot holds the
 * newest request, so a slider drag never queues and never terminates the worker; a reply whose
 * generation is not the newest request's is dropped. The worker is terminated only by `close`
 * (Cancel, Escape, deselecting, the bar closing) and by the watchdog: a job that goes
 * `STAGE_TIMEOUT_MS` without a word from the worker (a hang in the kernel) ends it as a failure.
 */

import { type TraceRequest, type TraceResponse, type TraceWorkerLike, editorModule } from "./traceJobs.ts";

/** How long the worker may stay silent on a job (no ready, progress, preview or result)
 * before it counts as hung: well above the 15 s wasm budget of Apply on 100,000 nodes. */
export const STAGE_TIMEOUT_MS = 60_000;

/** The bar's values as the worker reads them: overlap (fraction), Thick parts (mm), optimise
 * (fraction), keep inner areas (0 or 1), offset (mm), join tips (0 or 1). */
export type CentreOptionsWire = [number, number, number, number, number, number];

/** A request of the newest values: the subjects when they changed, and the options. */
interface Request {
  gen: number;
  subjects: Float64Array | null;
  options: CentreOptionsWire;
}

/** What the page hears from the jobs. */
export interface CentreLineListener {
  /** The preview of generation `gen` (always the newest one asked for). */
  preview: (gen: number, wire: Float64Array, ms: number) => void;
  /** The subjects' skeletons are being built: the share done, 0 to 100. */
  progress: (percent: number) => void;
  /** The worker failed; it is ended and the next request starts a fresh one. */
  failed: (message: string) => void;
}

/** Runs the Find centre lines jobs in their own worker. */
export class CentreLineJobs {
  private worker: TraceWorkerLike | null = null;
  private started = false;
  /** Posted before the worker started, in order. */
  private waiting: { message: TraceRequest; transfer: Transferable[] }[] = [];
  /** The generation of the newest request. */
  private latest = 0;
  /** The request in flight, and the one waiting for it. */
  private inFlight: Request | null = null;
  private pending: Request | null = null;
  /** The subjects of the last request, sent again to a fresh worker. */
  private subjects: Float64Array | null = null;
  /** The newest generation's preview arrived. */
  private answered = 0;
  /** Apply waits for the newest preview, then for its result. */
  private resultWaiters: { gen: number; resolve: (wire: Float64Array | null) => void }[] = [];
  /** The watchdog of the job the worker owes an answer to. */
  private watchdog: ReturnType<typeof setTimeout> | null = null;
  private readonly timeoutMs: number;
  private readonly makeWorker: () => TraceWorkerLike;
  private readonly module: () => Promise<WebAssembly.Module>;
  private readonly listener: CentreLineListener;

  constructor(
    makeWorker: () => TraceWorkerLike,
    module: () => Promise<WebAssembly.Module>,
    listener: CentreLineListener,
    timeoutMs: number = STAGE_TIMEOUT_MS,
  ) {
    this.makeWorker = makeWorker;
    this.module = module;
    this.listener = listener;
    this.timeoutMs = timeoutMs;
  }

  /** Whether a request newer than the shown preview is in flight or waiting. */
  get busy(): boolean {
    return this.answered !== this.latest;
  }

  /** The generation of the newest request. */
  get generation(): number {
    return this.latest;
  }

  /** Asks for the preview of `options`; with `subjects` the worker takes them first (the bar
   * opened, or the document or selection changed). Returns the request's generation. */
  request(options: CentreOptionsWire, subjects: Float64Array | null = null): number {
    this.latest += 1;
    if (subjects) {
      this.subjects = subjects;
    }
    // The pending slot keeps subjects it has not sent yet, whatever newer options overwrite it.
    const carried = subjects ?? this.pending?.subjects ?? null;
    this.pending = { gen: this.latest, subjects: carried, options };
    this.pump();
    return this.latest;
  }

  /** The result of the newest generation for Apply, once its preview is in: `null` when the
   * worker failed or was closed meanwhile. */
  result(): Promise<Float64Array | null> {
    return new Promise((resolve) => {
      this.resultWaiters.push({ gen: this.latest, resolve });
      this.pumpResult();
    });
  }

  /** Ends the worker (memo and memory freed); every waiting Apply hears `null`. */
  close(): void {
    this.disarm();
    this.worker?.terminate();
    this.worker = null;
    this.started = false;
    this.waiting = [];
    this.inFlight = null;
    this.pending = null;
    this.subjects = null;
    this.answered = this.latest;
    for (const waiter of this.resultWaiters.splice(0)) {
      waiter.resolve(null);
    }
  }

  /** Sends the pending request when nothing is in flight. */
  private pump(): void {
    if (this.inFlight || !this.pending) {
      return;
    }
    const job = this.pending;
    this.pending = null;
    this.inFlight = job;
    void this.ready().then((worker) => {
      if (this.inFlight !== job || !worker) {
        return;
      }
      this.arm(worker);
      if (job.subjects) {
        // The worker keeps a copy only for a fresh start; the transferred buffer goes.
        const copy = job.subjects.slice();
        this.post(worker, { type: "centre-subjects", id: job.gen, wire: copy }, [copy.buffer]);
      }
      this.post(worker, { type: "centre-finish", id: job.gen, options: job.options }, []);
    });
  }

  /** Asks the worker for the result once the newest preview is in and nothing is in flight. */
  private pumpResult(): void {
    if (this.inFlight || this.pending || this.resultWaiters.length === 0 || !this.worker) {
      if (!this.worker && !this.inFlight && !this.pending) {
        for (const waiter of this.resultWaiters.splice(0)) {
          waiter.resolve(null);
        }
      }
      return;
    }
    const gen = this.latest;
    this.arm(this.worker);
    this.post(this.worker, { type: "centre-result", id: gen }, []);
  }

  /** (Re)starts the watchdog for `worker`: silent for the timeout, it is ended as failed. */
  private arm(worker: TraceWorkerLike): void {
    this.disarm();
    this.watchdog = setTimeout(() => {
      this.watchdog = null;
      this.fail("centre lines worker timed out", worker);
    }, this.timeoutMs);
  }

  private disarm(): void {
    if (this.watchdog !== null) {
      clearTimeout(this.watchdog);
      this.watchdog = null;
    }
  }

  /** Whether the worker still owes an answer: a job in flight or a result asked for. */
  private owed(): boolean {
    return this.inFlight !== null || this.resultWaiters.length > 0;
  }

  private async ready(): Promise<TraceWorkerLike | null> {
    if (this.worker) {
      return this.worker;
    }
    let module: WebAssembly.Module;
    try {
      module = await this.module();
    } catch (error) {
      this.fail(String(error));
      return null;
    }
    if (this.worker) {
      return this.worker;
    }
    const worker = this.makeWorker();
    this.worker = worker;
    this.started = false;
    worker.onmessage = (event) => this.hear(worker, event.data);
    worker.onerror = (event) => this.fail(event.message || "centre lines worker failed", worker);
    worker.onmessageerror = () => this.fail("centre lines worker reply unreadable", worker);
    // The watchdog covers a worker that never answers its init too.
    this.arm(worker);
    worker.postMessage({ type: "init", module });
    // A fresh worker needs the subjects of the request in flight even when only options moved.
    if (this.inFlight && !this.inFlight.subjects && this.subjects) {
      this.inFlight.subjects = this.subjects;
    }
    return worker;
  }

  private post(worker: TraceWorkerLike, message: TraceRequest, transfer: Transferable[]): void {
    if (this.started && this.worker === worker) {
      worker.postMessage(message, transfer);
    } else {
      this.waiting.push({ message, transfer });
    }
  }

  private hear(worker: TraceWorkerLike, response: TraceResponse): void {
    if (this.worker !== worker) {
      return;
    }
    if (response.kind === "ready") {
      if (response.error !== "") {
        this.fail(response.error, worker);
        return;
      }
      this.started = true;
      if (this.owed()) {
        this.arm(worker);
      } else {
        this.disarm();
      }
      for (const job of this.waiting.splice(0)) {
        worker.postMessage(job.message, job.transfer);
      }
      return;
    }
    if (response.kind === "error") {
      this.fail(response.message, worker);
      return;
    }
    if (response.kind === "progress") {
      // The worker is alive: each stage's progress restarts the watchdog.
      this.arm(worker);
      if (response.id === this.latest) {
        this.listener.progress(response.percent);
      }
      return;
    }
    if (response.kind === "centre-preview") {
      const job = this.inFlight;
      this.inFlight = null;
      this.disarm();
      if (job && response.id === this.latest) {
        this.answered = response.id;
        this.listener.preview(response.id, response.wire, response.ms);
      }
      // A stale reply is dropped; the newest request goes next.
      this.pump();
      this.pumpResult();
      return;
    }
    if (response.kind === "centre-result") {
      const ready = this.resultWaiters.filter((w) => w.gen <= response.id);
      this.resultWaiters = this.resultWaiters.filter((w) => w.gen > response.id);
      if (this.owed()) {
        this.arm(worker);
      } else {
        this.disarm();
      }
      for (const waiter of ready) {
        waiter.resolve(response.id === this.latest ? response.wire : null);
      }
    }
  }

  private fail(message: string, worker?: TraceWorkerLike): void {
    if (worker && this.worker !== worker) {
      return;
    }
    this.close();
    this.listener.failed(message);
  }
}

/** The jobs over the real worker: the trace worker's script, its own instance. */
export function createCentreLineJobs(listener: CentreLineListener): CentreLineJobs {
  return new CentreLineJobs(
    () => new Worker(new URL("../workers/traceWorker.ts", import.meta.url), { type: "module" }),
    editorModule,
    listener,
  );
}

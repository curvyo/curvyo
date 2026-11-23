// The messages between the page and the recovery worker (`specs/0136-autosave-and-restore`
// criterion 2, ADR 0079 decision 4), and the page's end of it: a worker with its own wasm
// instance holds a replica of the open document, takes the deltas the session exports and packs
// the copy, so the canvas thread never waits for the packing. Every answer settles one promise.

import { editorModule } from "@/lib/traceJobs";

/** What the page asks of the worker. */
export type RecoveryRequest =
  | { type: "init"; module: WebAssembly.Module; peer: bigint }
  /** A delta (`full`: a snapshot that seeds the replica). */
  | { type: "apply"; id: number; full: boolean; bytes: Uint8Array }
  | { type: "pack"; id: number }
  /** A new session key: forget the replica. */
  | { type: "reset" };

/** What the worker answers; `result` of an `applied` is empty, `reseed` or `rejected`. */
export type RecoveryResponse =
  | { type: "ready" }
  | { type: "applied"; id: number; result: string }
  | { type: "packed"; id: number; bytes: Uint8Array | null; pages: number }
  | { type: "error"; id?: number; message: string };

/** A packed copy and its page count. */
export interface Packed {
  bytes: Uint8Array;
  pages: number;
}

/** The two calls the recovery hook makes of the worker. */
export interface RecoveryWorker {
  /** Hands over a delta; `""` when taken, `reseed` when a snapshot must follow. */
  apply(full: boolean, bytes: Uint8Array): Promise<string>;
  /** The replica packed; `null` while it holds nothing. */
  pack(): Promise<Packed | null>;
  reset(): void;
}

/** The worker over a real `Worker`, started once with the editor's compiled module. */
export class RecoveryWorkerLink implements RecoveryWorker {
  private worker: Worker | null = null;
  private starting: Promise<void> | null = null;
  private next = 0;
  private readonly waiting = new Map<number, { resolve: (answer: RecoveryResponse) => void; reject: (error: Error) => void }>();

  private readonly peer: bigint;
  private readonly create: () => Worker;

  constructor(
    peer: bigint,
    create: () => Worker = () => new Worker(new URL("../workers/recoveryWriter.ts", import.meta.url), { type: "module" }),
  ) {
    this.peer = peer;
    this.create = create;
  }

  private start(): Promise<void> {
    this.starting ??= editorModule().then((module) => {
      const worker = this.create();
      worker.onmessage = (event: MessageEvent<RecoveryResponse>) => this.onAnswer(event.data);
      worker.onerror = () => this.fail("the recovery worker stopped");
      worker.postMessage({ type: "init", module, peer: this.peer } satisfies RecoveryRequest);
      this.worker = worker;
    });
    return this.starting;
  }

  private onAnswer(answer: RecoveryResponse): void {
    if (answer.type === "ready") {
      return;
    }
    const id = answer.id;
    const waiter = id === undefined ? undefined : this.waiting.get(id);
    if (!waiter) {
      return;
    }
    this.waiting.delete(id as number);
    if (answer.type === "error") {
      waiter.reject(new Error(answer.message));
    } else {
      waiter.resolve(answer);
    }
  }

  /** The worker is gone: every waiting call fails, and the next call starts a new one. */
  private fail(message: string): void {
    for (const waiter of this.waiting.values()) {
      waiter.reject(new Error(message));
    }
    this.waiting.clear();
    this.worker?.terminate();
    this.worker = null;
    this.starting = null;
  }

  private async ask(make: (id: number) => RecoveryRequest, transfer: Transferable[] = []): Promise<RecoveryResponse> {
    await this.start();
    const id = (this.next += 1);
    return new Promise<RecoveryResponse>((resolve, reject) => {
      this.waiting.set(id, { resolve, reject });
      this.worker?.postMessage(make(id), transfer);
    });
  }

  async apply(full: boolean, bytes: Uint8Array): Promise<string> {
    const answer = await this.ask((id) => ({ type: "apply", id, full, bytes }), [bytes.buffer]);
    return answer.type === "applied" ? answer.result : "rejected";
  }

  async pack(): Promise<Packed | null> {
    const answer = await this.ask((id) => ({ type: "pack", id }));
    return answer.type === "packed" && answer.bytes ? { bytes: answer.bytes, pages: answer.pages } : null;
  }

  reset(): void {
    void this.start().then(() => this.worker?.postMessage({ type: "reset" } satisfies RecoveryRequest));
  }
}

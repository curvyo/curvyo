// The messages between the page and the background writer's worker (`specs/0094-workspace`
// criteria 31 and 32), and the page's end of it: the worker started once per page with the
// editor's compiled module.

import { editorModule } from "@/lib/traceJobs";

/** One record of a document that is not open. */
export interface WriterItem {
  document: string;
  seq: number;
  kind: string;
  bytes: Uint8Array;
}

/** What the page asks of the worker. */
export type WriterRequest =
  | { type: "init"; module: WebAssembly.Module; peer: bigint }
  | { type: "records"; items: WriterItem[] }
  | { type: "file"; document: string; bytes: Uint8Array }
  | { type: "skip"; document: string }
  /** The document has no file here yet: build it from its records (criterion 35). */
  | { type: "create"; document: string; workspace: string }
  /** The document's file cannot be had now: ask again after the interval. */
  | { type: "later"; document: string };

/** What the worker answers. */
export type WriterResponse =
  | { type: "ready" }
  | { type: "need"; document: string }
  | { type: "written"; document: string; bytes: Uint8Array; through: number; created: boolean }
  | { type: "skipped"; document: string; through: number }
  | { type: "failed"; document: string }
  | { type: "error"; message: string };

/** The page's end of the worker: messages go out once it started; answers go to `onAnswer`. */
export class BackgroundWriterLink {
  private worker: Worker | null = null;
  private starting: Promise<void> | null = null;
  private readonly onAnswer: (answer: WriterResponse) => void;
  private readonly peer: bigint;

  constructor(peer: bigint, onAnswer: (answer: WriterResponse) => void) {
    this.peer = peer;
    this.onAnswer = onAnswer;
  }

  /** Starts the worker once. */
  private start(): Promise<void> {
    this.starting ??= editorModule().then((module) => {
      const worker = new Worker(new URL("../workers/backgroundWriter.ts", import.meta.url), { type: "module" });
      worker.onmessage = (event: MessageEvent<WriterResponse>) => this.onAnswer(event.data);
      worker.postMessage({ type: "init", module, peer: this.peer } satisfies WriterRequest);
      this.worker = worker;
    });
    return this.starting;
  }

  /** Sends `request`, starting the worker first when needed. */
  send(request: WriterRequest, transfer: Transferable[] = []): void {
    void this.start().then(() => this.worker?.postMessage(request, transfer));
  }

  /** Ends the worker (the page closes the workspace). */
  stop(): void {
    this.worker?.terminate();
    this.worker = null;
    this.starting = null;
  }
}

/**
 * The flow of a Run of an AI step (`specs/0135` criteria 47, 63 to 65; ADR 0076 §3, §6, §7),
 * kept free of React so the tests drive it with a fake session, store, fetch and worker: the
 * model's state on the device, the consent line before a first download (a partial file counts
 * as consent given), the one download, the Run in the worker and the stored result.
 */
import { type DownloadOutcome, type Fetch, type PinnedModel, downloadModel, hasRoom } from "./modelDownload.ts";
import type { RunExtras } from "./aiJobs.ts";
import type { ModelStore } from "./modelStore.ts";

/** The session calls the flow needs (the `WasmSession` bindings of `wasm_image_ai.rs`). */
export interface AiSession {
  set_ai_model_state(kind: string, state: string, bytes: number): boolean;
  ai_run_input_json(row: number): string;
  take_ai_run_pixels(): Uint8Array;
  set_ai_progress(row: number, done: number, total: number): void;
  /** The Run has a pool of one worker (`specs/0143` criterion 10); older fakes leave it out. */
  set_ai_one_core?(row: number): void;
  end_ai_run(): void;
  store_ai_result(row: number, hash: string, width: number, height: number, pixels: Uint8Array, grey: boolean): string;
  remove_ai_result(row: number): boolean;
}

/** The worker side of a Run (`AiRunner`). */
export interface AiRun {
  hasModel(kind: string): boolean;
  run(
    kind: string,
    model: Uint8Array | null,
    input: { rgba: Uint8Array; width: number; height: number; factor: number },
    progress: (percent: number) => void,
    extras?: RunExtras,
  ): Promise<{ pixels: Uint8Array; width: number; height: number; grey: boolean }>;
  cancel(): void;
}

/** What the flow is built from. */
export interface AiDeps {
  session: () => AiSession | null;
  /** The pinned models, or a call that reads them (the wasm's `ai_models_json`), made on first
   * use only: the flow is built during the first render, before the wasm is loaded. */
  models: PinnedModel[] | (() => PinnedModel[]);
  store: ModelStore;
  fetch: Fetch;
  verify: (kind: string, bytes: Uint8Array) => boolean;
  runner: AiRun;
  storage: StorageManager | undefined;
  /** The view must be read again. */
  changed: () => void;
}

/** The AI steps' flow of one page. */
export class AiSteps {
  private downloads = new Map<string, AbortController>();
  private readonly deps: AiDeps;
  private pinned: PinnedModel[] | null = null;

  constructor(deps: AiDeps) {
    this.deps = deps;
  }

  private models(): PinnedModel[] {
    this.pinned ??= typeof this.deps.models === "function" ? this.deps.models() : this.deps.models;
    return this.pinned;
  }

  private model(kind: string): PinnedModel | undefined {
    return this.models().find((m) => m.kind === kind);
  }

  /** The pinned model `kind` for its licence lines (ADR 0076 §9); `undefined` before the wasm. */
  pinnedModel(kind: string): PinnedModel | undefined {
    try {
      return this.model(kind);
    } catch {
      return undefined;
    }
  }

  private report(kind: string, state: string, bytes = 0): void {
    this.deps.session()?.set_ai_model_state(kind, state, bytes);
    this.deps.changed();
  }

  /** Reads which models are on the device (a name lookup only, ADR 0076 §3) for the rows. */
  async refresh(): Promise<void> {
    for (const model of this.models()) {
      if (this.downloads.has(model.kind)) continue;
      const status = await this.deps.store.status(model.sha256);
      this.report(model.kind, status.state === "complete" ? "ready" : status.state === "partial" ? "paused" : "absent", status.state === "absent" ? 0 : status.bytes);
    }
  }

  /** A press on Run (or Try again) of row `row`, whose model is `kind`. */
  async run(row: number, kind: string): Promise<void> {
    const model = this.model(kind);
    if (!model) return;
    const status = await this.deps.store.status(model.sha256);
    if (status.state === "complete" || this.deps.runner.hasModel(kind)) {
      await this.execute(row, model);
      return;
    }
    if (status.state === "absent") {
      // The consent line first (criterion 63); nothing is fetched.
      this.report(kind, "asking");
      return;
    }
    if ((await this.download(model)) === "ready") await this.execute(row, model);
  }

  /** Download on the consent line, then Run. */
  async consent(row: number, kind: string): Promise<void> {
    const model = this.model(kind);
    if (!model) return;
    if ((await this.download(model)) === "ready") await this.execute(row, model);
  }

  /** Cancel on the consent line: nothing was downloaded. */
  dismiss(kind: string): void {
    this.report(kind, "absent");
  }

  /** Cancel of a download (kept as a partial file) or of a Run (the worker ends). */
  cancel(kind: string): void {
    const download = this.downloads.get(kind);
    if (download) {
      download.abort();
      return;
    }
    this.deps.runner.cancel();
    this.deps.session()?.end_ai_run();
    this.deps.changed();
  }

  /** Remove of a cached model (the Storage line, criterion 64). */
  async removeModel(kind: string): Promise<void> {
    const model = this.model(kind);
    if (!model) return;
    await this.deps.store.remove(model.sha256);
    this.deps.runner.cancel();
    this.report(kind, "absent");
  }

  private async download(model: PinnedModel): Promise<DownloadOutcome> {
    const status = await this.deps.store.status(model.sha256);
    const stored = status.state === "absent" ? 0 : status.bytes;
    if (!(await hasRoom(model.bytes - stored, this.deps.storage))) {
      this.report(model.kind, "storage");
      return "storage";
    }
    try {
      void this.deps.storage?.persist?.().catch(() => false);
    } catch {
      // A browser without the call keeps the default.
    }
    const abort = new AbortController();
    this.downloads.set(model.kind, abort);
    this.report(model.kind, "downloading", stored);
    const outcome = await downloadModel(
      model,
      this.deps.store,
      this.deps.fetch,
      (bytes) => this.deps.verify(model.kind, bytes),
      (bytes) => this.report(model.kind, "downloading", bytes),
      abort.signal,
    );
    this.downloads.delete(model.kind);
    const after = await this.deps.store.status(model.sha256);
    this.report(model.kind, outcome, after.state === "absent" ? 0 : after.bytes);
    return outcome;
  }

  private async execute(row: number, model: PinnedModel): Promise<void> {
    const session = this.deps.session();
    if (!session) return;
    const input = JSON.parse(session.ai_run_input_json(row)) as {
      refusal?: string;
      kind: string;
      factor: number;
      width: number;
      height: number;
      hash: string;
    };
    this.deps.changed();
    if (input.refusal !== undefined) return;
    const rgba = session.take_ai_run_pixels();
    const bytes = this.deps.runner.hasModel(model.kind) ? null : await this.deps.store.read(model.sha256);
    try {
      const out = await this.deps.runner.run(
        model.kind,
        bytes,
        { rgba, width: input.width, height: input.height, factor: input.factor },
        (percent) => {
          this.deps.session()?.set_ai_progress(row, percent, 100);
          this.deps.changed();
        },
        {
          readModel: async () => {
            const file = await this.deps.store.read(model.sha256);
            if (!file) throw new Error("the model is not on this device");
            return file;
          },
          workers: (n) => {
            if (n === 1) this.deps.session()?.set_ai_one_core?.(row);
            this.deps.changed();
          },
        },
      );
      this.deps.session()?.store_ai_result(row, input.hash, out.width, out.height, out.pixels, out.grey);
    } catch (error) {
      // A cancelled Run's row was made idle by Cancel already (a new Run may have begun since).
      if (!(error instanceof Error && error.name === "AiCancelled")) this.deps.session()?.end_ai_run();
    }
    this.deps.changed();
  }

  /** Remove stored result of row `row`: one step. */
  removeResult(row: number): void {
    this.deps.session()?.remove_ai_result(row);
    this.deps.changed();
  }
}

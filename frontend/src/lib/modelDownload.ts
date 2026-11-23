/**
 * The one network use of the app (`specs/0135` criteria 63 to 65, ADR 0076 §3 to §8): the download
 * of one pinned AI model on demand, after the consent line, from its one fixed URL. The request
 * carries no credentials, no referrer, no query and no body, only a `Range` header on a resume;
 * the bytes stream into the device cache in chunks, capped at the pinned size, and are committed
 * only when their SHA-256 matches the pin compiled into the app.
 */
import { CHUNK_BYTES, type ModelStore } from "./modelStore.ts";

/** One pinned model as the wasm lists it (`ai_models_json`). */
export interface PinnedModel {
  kind: string;
  name: string;
  version: string;
  bytes: number;
  sha256: string;
  url: string;
  licence_url: string;
  licence: string;
  copyright: string;
  upstream: string;
  training_data: string;
  placeholder: boolean;
}

/** How a download ended; the step row shows each (ADR 0076 §7). */
export type DownloadOutcome =
  | "ready"
  | "paused"
  | "unreachable"
  | "withdrawn"
  | "storage"
  | "checksum";

/** The `fetch` the download uses (the tests pass a fake). */
export type Fetch = (url: string, init: RequestInit) => Promise<Response>;

/** The request of a download from `stored` bytes on (ADR 0076 §4): exactly these options. */
export function downloadRequest(stored: number, signal?: AbortSignal): RequestInit {
  const init: RequestInit = {
    method: "GET",
    credentials: "omit",
    referrerPolicy: "no-referrer",
    cache: "no-store",
    redirect: "error",
    mode: "cors",
  };
  if (stored > 0) init.headers = { Range: `bytes=${stored}-` };
  if (signal) init.signal = signal;
  return init;
}

/** Whether the browser has room for `bytes` more (ADR 0076 §8: the size plus 16 MiB). */
export async function hasRoom(bytes: number, storage: StorageManager | undefined): Promise<boolean> {
  try {
    const estimate = await storage?.estimate?.();
    if (!estimate || estimate.quota === undefined || estimate.usage === undefined) return true;
    return estimate.quota - estimate.usage >= bytes + 16 * 1024 * 1024;
  } catch {
    return true;
  }
}

/** The start offset a `Content-Range` answer names, `null` when it names none. */
function rangeStart(response: Response): number | null {
  const match = /^bytes (\d+)-/.exec(response.headers.get("Content-Range") ?? "");
  return match ? Number(match[1]) : null;
}

/**
 * Downloads `model` into `store`, resuming a partial file; `progress(bytes)` hears the stored
 * length; `verify(bytes)` checks the pin (the wasm's `ai_verify_model`). A cancel (the signal)
 * or a broken connection keeps the partial file: "paused". Never throws.
 */
export async function downloadModel(
  model: PinnedModel,
  store: ModelStore,
  fetchFn: Fetch,
  verify: (bytes: Uint8Array) => boolean,
  progress: (bytes: number) => void,
  signal?: AbortSignal,
): Promise<DownloadOutcome> {
  let status = await store.status(model.sha256);
  if (status.state === "complete") return "ready";
  let stored = status.state === "partial" ? status.bytes : 0;
  let response: Response;
  try {
    response = await fetchFn(model.url, downloadRequest(stored, signal));
  } catch {
    return signal?.aborted ? "paused" : stored > 0 ? "paused" : "unreachable";
  }
  if (response.status === 404 || response.status === 410) return "withdrawn";
  if (response.status === 206 && rangeStart(response) === stored) {
    // Resume: append from the stored length.
  } else if (response.status === 206 && stored > 0) {
    // Another start (ADR 0076 §5): the partial file is discarded and the download starts at zero.
    await response.body?.cancel().catch(() => undefined);
    await store.remove(model.sha256);
    return downloadModel(model, store, fetchFn, verify, progress, signal);
  } else if (response.status === 200) {
    if (stored > 0) {
      await store.remove(model.sha256);
      stored = 0;
    }
  } else {
    return "unreachable";
  }
  const reader = response.body?.getReader();
  if (!reader) return "unreachable";
  let pending: Uint8Array[] = [];
  let pendingBytes = 0;
  const flush = async () => {
    if (pendingBytes === 0) return;
    const chunk = new Uint8Array(pendingBytes);
    let at = 0;
    for (const part of pending) {
      chunk.set(part, at);
      at += part.length;
    }
    await store.append(model.sha256, stored, chunk);
    stored += pendingBytes;
    pending = [];
    pendingBytes = 0;
    progress(stored);
  };
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      if (stored + pendingBytes + value.length > model.bytes) {
        // Over the pinned size: never the pinned file.
        await reader.cancel();
        await store.remove(model.sha256);
        return "checksum";
      }
      pending.push(value);
      pendingBytes += value.length;
      if (pendingBytes >= CHUNK_BYTES) await flush();
    }
    await flush();
  } catch {
    await flush().catch(() => undefined);
    return "paused";
  }
  status = await store.status(model.sha256);
  if (status.state === "absent" || status.bytes < model.bytes) {
    // The host ended the file cleanly before the pinned size: not the pinned file (a broken
    // connection errors the stream above and keeps the partial file).
    await store.remove(model.sha256);
    return "checksum";
  }
  const bytes = await store.read(model.sha256);
  if (!bytes || !verify(bytes)) {
    await store.remove(model.sha256);
    return "checksum";
  }
  await store.commit(model.sha256);
  return "ready";
}

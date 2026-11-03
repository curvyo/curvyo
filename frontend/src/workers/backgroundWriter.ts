// The background writer's worker (`specs/0094-workspace` criteria 31 and 32, ADR 0036 decision 8,
// the trace worker's pattern of ADR 0023 §5): its own instance of the editor's wasm module keeps
// the records of documents that are not open and writes each document's file at most every 5 s.
// The page reads and writes the files; this worker only unpacks, imports and packs.

import { WasmBackgroundWriter, initSync } from "@/wasm-bindings/curvyo_editor_wasm.js";

import type { WriterRequest, WriterResponse } from "@/lib/backgroundWriterJobs";

interface WorkerScope {
  onmessage: ((event: MessageEvent<WriterRequest>) => void) | null;
  postMessage(message: WriterResponse, transfer?: Transferable[]): void;
}

const scope = self as unknown as WorkerScope;
/** How often the worker looks for due documents (ms). */
const DUE_MS = 500;

let writer: WasmBackgroundWriter | null = null;

scope.onmessage = (event) => {
  const request = event.data;
  try {
    if (request.type === "init") {
      initSync({ module: request.module });
      writer = new WasmBackgroundWriter(request.peer);
      setInterval(() => {
        if (!writer) {
          return;
        }
        for (const document of JSON.parse(writer.due(Date.now())) as string[]) {
          scope.postMessage({ type: "need", document });
        }
      }, DUE_MS);
      scope.postMessage({ type: "ready" });
      return;
    }
    if (!writer) {
      return;
    }
    if (request.type === "records") {
      for (const item of request.items) {
        writer.add(item.document, item.seq, item.kind, item.bytes);
      }
      return;
    }
    if (request.type === "skip") {
      scope.postMessage({ type: "skipped", document: request.document, through: writer.forget(request.document) });
      return;
    }
    if (request.type === "later") {
      writer.postpone(request.document, Date.now());
      return;
    }
    const created = request.type === "create";
    const written = created
      ? writer.create(request.document, request.workspace, Date.now())
      : writer.write(request.document, request.bytes, Date.now());
    if (!written) {
      scope.postMessage({ type: "failed", document: request.document });
      return;
    }
    const reply: WriterResponse = {
      type: "written",
      document: request.document,
      bytes: written.bytes,
      through: written.through,
      created,
    };
    written.free();
    scope.postMessage(reply, [reply.bytes.buffer]);
  } catch (error) {
    scope.postMessage({ type: "error", message: String(error) });
  }
};

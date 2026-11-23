// The recovery worker (`specs/0136-autosave-and-restore` criterion 2, ADR 0079 decision 4): its own
// instance of the editor's wasm module holds a replica of the open document, takes the deltas the
// page exports and packs the recovery container. The page writes the bytes through the host; this
// worker only imports and packs, off the canvas thread.

import { WasmRecoveryWriter, initSync } from "@/wasm-bindings/curvyo_editor_wasm.js";

import type { RecoveryRequest, RecoveryResponse } from "@/lib/recoveryWriterJobs";

interface WorkerScope {
  onmessage: ((event: MessageEvent<RecoveryRequest>) => void) | null;
  postMessage(message: RecoveryResponse, transfer?: Transferable[]): void;
}

const scope = self as unknown as WorkerScope;

let writer: WasmRecoveryWriter | null = null;
let peer = 0n;

scope.onmessage = (event) => {
  const request = event.data;
  const id = "id" in request ? request.id : undefined;
  try {
    if (request.type === "init") {
      initSync({ module: request.module });
      peer = request.peer;
      writer = new WasmRecoveryWriter(peer);
      scope.postMessage({ type: "ready" });
      return;
    }
    if (!writer) {
      return;
    }
    if (request.type === "reset") {
      writer.reset();
      return;
    }
    if (request.type === "apply") {
      scope.postMessage({ type: "applied", id: request.id, result: writer.apply(request.full, request.bytes) });
      return;
    }
    const packed = writer.pack();
    if (!packed) {
      scope.postMessage({ type: "packed", id: request.id, bytes: null, pages: 0 });
      return;
    }
    const reply: RecoveryResponse = { type: "packed", id: request.id, bytes: packed.bytes, pages: packed.pages };
    packed.free();
    scope.postMessage(reply, [(reply.bytes as Uint8Array).buffer]);
  } catch (error) {
    scope.postMessage({ type: "error", id, message: String(error) });
  }
};

// The files of a shared workspace's documents that are not open (`specs/0095-workspace`
// criteria 31 and 32, ADR 0037 decision 8): the session's background records go to the writer's
// worker, which asks for a due document's file and hands back the written bytes; a workspace
// snapshot the room asked for is built from the files once they hold every record it covers.

import { type MutableRefObject, useCallback, useEffect, useRef } from "react";

import { BackgroundWriterLink, type WriterItem, type WriterResponse, sendLocal } from "@/lib/backgroundWriterJobs";
import { mintPeerId } from "@/lib/editorSession";
import { type DocumentPlace, parsePlace } from "@/lib/sharedTree";
import { answerBytes } from "@/lib/binaryArgs";
import { invoke } from "@/platform/host";
import type { WasmSession } from "@/wasm-bindings/curvyo_editor_wasm.js";

function random(length: number): Uint8Array {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  return bytes;
}

/** A document's file at its place, `null` when there is none. */
async function readPlace(place: DocumentPlace, document: string): Promise<Uint8Array<ArrayBuffer> | null> {
  const read =
    place.kind === "file"
      ? invoke<ArrayBuffer>("read_workspace_document", { path: place.path })
      : invoke<ArrayBuffer>("read_deleted_document", { document });
  return read.then(answerBytes, () => null);
}

/** Writes a document's bytes at its place; whether it was written. A created file is new (it
 * never replaces another one). */
async function writePlace(place: DocumentPlace, document: string, bytes: Uint8Array, created: boolean): Promise<boolean> {
  const write =
    place.kind === "deleted"
      ? invoke("write_deleted_document", { document }, bytes)
      : place.kind === "file" && created
        ? invoke("apply_workspace_op", { op: { kind: "create-document", path: place.path } }, bytes)
        : place.kind === "file"
          ? invoke("replace_workspace_document", { path: place.path }, bytes)
          : Promise.reject(new Error("no place"));
  return write.then(
    () => true,
    () => false,
  );
}

export interface WorkspaceWriterApi {
  /** Hands `session`'s background records on and builds a snapshot that is ready. */
  pump: (session: WasmSession) => void;
  /** Ends the worker (the workspace closed). */
  stop: () => void;
}

export function useWorkspaceWriter(
  getSession: () => WasmSession | null,
  drive: (session: WasmSession) => void,
  writingRef: MutableRefObject<Promise<void>>,
  notice: (text: string) => void,
): WorkspaceWriterApi {
  /** The background writer's worker (criterion 32), started with the first record. */
  const writerRef = useRef<BackgroundWriterLink | null>(null);
  /** A workspace snapshot being built from the files. */
  const buildingRef = useRef(false);
  const failedRef = useRef(new Set<string>());

  /** The worker's answers: it asks for a due document's file and hands back the written bytes.
   * A document with no file here yet is built from its records (criterion 35); a deleted one
   * keeps its copy in the marker directory (criterion 40). */
  const onWriterAnswer = useCallback(
    (answer: WriterResponse) => {
      const link = writerRef.current;
      if (!link) {
        return;
      }
      const { document } = answer as { document?: string };
      if (answer.type === "need" && document) {
        const session = getSession();
        // The open document is never written in the background (its session holds the records).
        if (!session?.workspace_may_write(document)) {
          link.send({ type: "skip", document });
          return;
        }
        const place = parsePlace(session.workspace_document_place(document));
        if (place.kind === "unknown") {
          // The list does not name it yet: its records wait.
          link.send({ type: "later", document });
          return;
        }
        const workspace = session.workspace_id_hex();
        void readPlace(place, document).then((file) => {
          if (file) {
            link.send({ type: "file", document, workspace, bytes: file }, [file.buffer]);
          } else {
            link.send({ type: "create", document, workspace });
          }
        });
      } else if (answer.type === "written") {
        failedRef.current.delete(answer.document);
        writingRef.current = writingRef.current.then(async () => {
          const session = getSession();
          if (!session?.workspace_may_write(answer.document)) {
            return;
          }
          const place = parsePlace(session.workspace_document_place(answer.document));
          const bytes = sendLocal(session, answer.document, answer.bytes, answer.outgoing,
            () => notice("A workspace update could not be sent; local changes remain unsent."));
          if (await writePlace(place, answer.document, bytes, answer.created)) {
            getSession()?.background_written(answer.document, answer.through);
          }
        });
      } else if (answer.type === "skipped" && answer.through >= 0) {
        // The open document's session holds these records itself.
        getSession()?.background_written(answer.document, answer.through);
      } else if (answer.type === "failed" && answer.message && !failedRef.current.has(answer.document)) {
        failedRef.current.add(answer.document);
        notice(answer.message);
      }
    },
    [getSession, writingRef, notice],
  );

  /** Hands the session's background records to the worker. */
  const feedWriter = useCallback(
    (session: WasmSession) => {
      const items: WriterItem[] = [];
      for (let item = session.next_background_item(); item; item = session.next_background_item()) {
        items.push({ document: item.document, seq: item.seq, kind: item.kind, bytes: item.bytes });
        item.free();
      }
      if (items.length === 0) {
        return;
      }
      writerRef.current ??= new BackgroundWriterLink(mintPeerId(), (answer) => onWriterAnswerRef.current(answer));
      writerRef.current.send(
        { type: "records", items },
        items.map((item) => item.bytes.buffer),
      );
    },
    [],
  );
  const onWriterAnswerRef = useRef(onWriterAnswer);

  /** A workspace snapshot the room asked for, built from the files once they hold every record
   * it covers (ADR 0037 decision 8). */
  const buildSnapshot = useCallback(
    async (session: WasmSession) => {
      const wanted = session.workspace_snapshot_documents_json();
      buildingRef.current = true;
      try {
        const files: [string, Uint8Array][] = [];
        for (const entry of JSON.parse(wanted) as { document: string; path?: string[]; deleted?: boolean }[]) {
          const place: DocumentPlace = entry.path ? { kind: "file", path: entry.path } : { kind: "deleted" };
          const bytes = await readPlace(place, entry.document);
          if (bytes) {
            files.push([entry.document, bytes]);
          }
        }
        const current = getSession();
        // The maker opened another document meanwhile: the set changed; the next tick builds anew.
        if (!current || current.workspace_snapshot_ready() < 0 || current.workspace_snapshot_documents_json() !== wanted) {
          return;
        }
        for (const [document, bytes] of files) {
          current.add_workspace_snapshot_document(document, bytes);
        }
        current.finish_workspace_snapshot(random(16));
        drive(current);
      } finally {
        buildingRef.current = false;
      }
    },
    [drive, getSession],
  );

  useEffect(() => {
    onWriterAnswerRef.current = onWriterAnswer;
  });

  const pump = useCallback(
    (session: WasmSession) => {
      feedWriter(session);
      if (!buildingRef.current && session.workspace_snapshot_ready() >= 0) {
        void buildSnapshot(session);
      }
    },
    [buildSnapshot, feedWriter],
  );

  const stop = useCallback(() => {
    writerRef.current?.stop();
    writerRef.current = null;
  }, []);

  return { pump, stop };
}

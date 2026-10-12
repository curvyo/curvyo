// Mirrors the payloads curvyo-app (src-tauri) sends across the IPC
// boundary: the payload and outcome types in curvyo-app/src/project.rs.
// Keep these in sync with that file — there is no shared schema generator
// for this slice.
//
// `path-node-editing`'s PR review ("the host does byte I/O only") moved
// the actual document to the frontend's own `WasmSession`
// (`@/lib/editorSession`); the host now only ever hands this side raw
// bytes to parse (`WasmSession.open`) or already-packed bytes to write
// (`WasmSession.pack`, then the `save_project_bytes` command) — it never
// builds or reads a document itself.

/**
 * A host-level failure to even read a file's bytes (missing, permissions,
 * not a file) — what an `open-error` event carries. Distinct from a
 * `.curvyo` whose *content* is invalid: that refusal is the frontend's own,
 * thrown by `WasmSession.open` itself once it has the bytes.
 */
export interface OpenErrorPayload {
  message: string;
}

/** A file the host could read, for `WasmSession.open` to try parsing —
 * what an `open-bytes` event carries (a file the operating system handed to the running app). */
export interface OpenBytesPayload {
  path: string;
  bytes: number[];
}

/**
 * A project file the host read, or failed to read: what `pick_project` returns (`null` when the
 * maker cancelled the dialog) and what `take_pending_open` returns (`null` if nothing is pending)
 * for the file-association launch race. Tagged by `kind` so one handler takes both live events
 * and these answers.
 */
export type OpenPayload =
  | ({ kind: "bytes" } & OpenBytesPayload)
  | ({ kind: "io_error" } & OpenErrorPayload);

/**
 * How `save_project_bytes` ended: written (with the file's name and path), cancelled (the maker
 * closed the dialog) or failed (with the sentence for the error dialog). There is no separate
 * error event.
 */
export type SaveOutcome =
  | { kind: "saved"; file_name: string; path: string }
  | { kind: "cancelled" }
  | { kind: "failed"; message: string };

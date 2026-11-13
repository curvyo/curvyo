// Performs the material writes a session hands out through the host (ADR 0055 §1, ADR 0058 §1,
// §4, §5; `specs/0112-test-grid-and-materials` criteria 17, 25, 27, 31): records as text, photo
// bytes taken from the session by handle and sent as bytes, the retired ids and the preference
// files as text. The host is passed in, so the page and the tests use the same rules.
import { type MaterialWrite, failedWriteName } from "./materialWrites.ts";

/** The host's `invoke`: a command, its arguments and, for a photo, its bytes. */
export type HostInvoke = (command: string, args?: Record<string, unknown>, bytes?: Uint8Array) => Promise<unknown>;

/** The record reader's limit in UTF-8 bytes (`MAX_RECORD_TEXT` of `curvyo-laser-core`). */
export const MAX_RECORD_TEXT = 512 * 1024;

/** Why a record text is not handed to the host: over 512 KiB it would not read back. */
export function recordTextRefusal(text: string): string | null {
  return new TextEncoder().encode(text).byteLength > MAX_RECORD_TEXT ? "it is over 512 KiB" : null;
}

/** The notice of a photo the host could not keep; the record stays as the session has it. */
export function photoFailureText(reason: string): string {
  return `Could not save the photo: ${reason}.`;
}

/** What went wrong: the first refused record write with its material's name, and the first
 * refused photo's reason. */
export interface MaterialWritesResult {
  record: { reason: string; name: string | null } | null;
  photo: string | null;
}

/** A refusal as words: an `Error`'s message, else the host's text. */
function reasonText(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

/** Performs one write; a refusal rejects with the reason. */
function perform(write: MaterialWrite, invoke: HostInvoke, takeBytes: (handle: number) => Uint8Array): Promise<unknown> {
  switch (write.op) {
    case "write": {
      const refusal = recordTextRefusal(write.text);
      return refusal === null ? invoke("write_material", { id: write.id, text: write.text }) : Promise.reject(refusal);
    }
    case "remove":
      return invoke("remove_material", { id: write.id });
    case "order":
      return invoke("write_material_order", { text: write.text });
    case "grid_choice":
      return invoke("write_test_grid_choice", { text: write.text });
    case "retired":
      return invoke("write_retired_test_ids", { text: write.text });
    case "write_photo": {
      const bytes = takeBytes(write.handle);
      return bytes.byteLength === 0
        ? Promise.reject("its bytes are gone")
        : invoke("write_material_photo", { name: write.name }, bytes);
    }
    case "remove_photo":
      return invoke("remove_material_photo", { name: write.name });
  }
}

/** Performs `writes` in order, each tried even after a refusal. `known` holds the record files as
 * the host listed them, to name a material whose write was refused. A refused order, grid choice
 * or retired file is silent (no step, rewritten at the next change). */
export async function runMaterialWrites(
  writes: readonly MaterialWrite[],
  invoke: HostInvoke,
  takeBytes: (handle: number) => Uint8Array,
  known: readonly { name: string; text: string }[],
): Promise<MaterialWritesResult> {
  const result: MaterialWritesResult = { record: null, photo: null };
  for (const write of writes) {
    try {
      await perform(write, invoke, takeBytes);
    } catch (reason) {
      if (write.op === "write" || write.op === "remove") {
        result.record ??= { reason: reasonText(reason), name: failedWriteName(write, known) };
      } else if (write.op === "write_photo" || write.op === "remove_photo") {
        result.photo ??= reasonText(reason);
      }
    }
  }
  return result;
}

/** The bytes of the photo file `name`; `"missing"` when the host has none or cannot read it
 * ("Photo not found", criterion 31). */
export async function readMaterialPhoto(invoke: HostInvoke, name: string): Promise<Uint8Array | "missing"> {
  try {
    const answer = await invoke("read_material_photo", { name });
    if (answer instanceof ArrayBuffer && answer.byteLength > 0) {
      return new Uint8Array(answer);
    }
  } catch {
    // A missing or unreadable photo is shown as missing; the test stays usable.
  }
  return "missing";
}

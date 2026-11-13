// The per-user material records, their list order, the retired test ids, the test photos and the
// remembered test grid values (ADR 0055 §1, §5, §6, §7; ADR 0058 §1, §4, §5). The host only moves
// text and photo bytes (Tauri commands on the desktop; `localStorage` and IndexedDB in the
// browser); `curvyo-laser-core` parses and writes inside each session. Read once before the first
// session; every new session gets them; each library change the session makes comes back as
// writes the host performs here. Nothing here uses the network (spec `0112` criterion 33).
import type { WasmSession } from "@/lib/editorSession";
import { photoFailureText, readMaterialPhoto, runMaterialWrites } from "@/lib/materialHostWrites";
import { decodeMaterialWrites } from "@/lib/materialWrites";
import { invoke } from "@/platform/host";

export { photoFailureText };

/** One record file as the host lists it. */
export interface MaterialRecordText {
  name: string;
  text: string;
}

/** What the host reads once before the first session. */
export interface MaterialFiles {
  records: MaterialRecordText[];
  order: string | null;
  retired: string | null;
  gridChoice: string | null;
}

let files: MaterialFiles = { records: [], order: null, retired: null, gridChoice: null };
let loading: Promise<void> | null = null;

/** Reads the records, the order, the retired ids and the remembered grid values; an unreadable
 * folder or file means none. */
export async function readMaterialFiles(): Promise<MaterialFiles> {
  const records = await invoke<MaterialRecordText[]>("list_materials").catch(() => []);
  const order = await invoke<string | null>("read_material_order").catch(() => null);
  const retired = await invoke<string | null>("read_retired_test_ids").catch(() => null);
  const gridChoice = await invoke<string | null>("read_test_grid_choice").catch(() => null);
  return { records, order, retired, gridChoice };
}

/** Reads the files once; every later session gets what was read (and written) since. */
export function materialFilesReady(): Promise<void> {
  loading ??= readMaterialFiles().then((read) => {
    files = read;
  });
  return loading;
}

/** Gives a session the kept records, the order, the retired ids and the remembered grid values. */
export function applyMaterials(session: WasmSession): void {
  session.load_materials(JSON.stringify(files.records), files.order ?? "");
  session.load_retired_test_ids(files.retired ?? "");
  if (files.gridChoice !== null) {
    session.load_test_grid_choice(files.gridChoice);
  }
}

/** The bytes of the photo file `name` to show, also handed to `session` so an undo of a later
 * photo step can write them back (ADR 0058 §8); `"missing"` shows "Photo not found". */
export async function loadMaterialPhoto(session: WasmSession, name: string): Promise<Uint8Array | "missing"> {
  const bytes = await readMaterialPhoto(invoke, name);
  if (bytes !== "missing") {
    session.load_material_photo(name, bytes);
  }
  return bytes;
}

/** A write the host could not perform: its reason and, for a record, the material's name when
 * known. `photo` marks a photo the host did not keep: the record stays and the notice is
 * [`photoFailureText`]. */
export interface MaterialWriteFailure {
  reason: string;
  name: string | null;
  photo: boolean;
}

/** Performs the session's pending writes in order. A failed record write (also one over 512 KiB,
 * refused before the host) reloads the library from storage into the session, so it shows what is
 * kept, and returns the host's reason with the material's name; `resync` redraws. A failed photo
 * keeps the record and returns its reason with `photo` set. A preference or retired file failing
 * to save is silent. `null` when all were kept. */
export async function flushMaterialWrites(
  session: WasmSession,
  resync?: () => void,
): Promise<MaterialWriteFailure | null> {
  const writes = decodeMaterialWrites(session.take_material_writes());
  if (writes.length === 0) {
    return null;
  }
  const result = await runMaterialWrites(
    writes,
    invoke,
    (handle) => session.take_material_photo_bytes(handle),
    files.records,
  );
  files = await readMaterialFiles();
  if (result.record !== null) {
    applyMaterials(session);
    resync?.();
    return { ...result.record, photo: false };
  }
  return result.photo === null ? null : { reason: result.photo, name: null, photo: true };
}

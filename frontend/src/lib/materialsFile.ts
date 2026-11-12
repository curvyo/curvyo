// The per-user material records, their list order and the remembered test grid values (ADR 0055
// §1, §5, §6, §7). The host only moves text (Tauri commands on the desktop, `localStorage` in the
// browser); `curvyo-laser-core` parses and writes inside each session. Read once before the first
// session; every new session gets them; each library change the session makes comes back as
// writes the host performs here. Nothing here uses the network (spec `0028` criterion 31).
import type { WasmSession } from "@/lib/editorSession";
import { type MaterialWrite, decodeMaterialWrites, failedWriteName } from "@/lib/materialWrites";
import { invoke } from "@/platform/host";

/** One record file as the host lists it. */
export interface MaterialRecordText {
  name: string;
  text: string;
}

/** What the host reads once before the first session. */
export interface MaterialFiles {
  records: MaterialRecordText[];
  order: string | null;
  gridChoice: string | null;
}

let files: MaterialFiles = { records: [], order: null, gridChoice: null };
let loading: Promise<void> | null = null;

/** Reads the records, the order and the remembered grid values; an unreadable folder or file
 * means none. */
export async function readMaterialFiles(): Promise<MaterialFiles> {
  const records = await invoke<MaterialRecordText[]>("list_materials").catch(() => []);
  const order = await invoke<string | null>("read_material_order").catch(() => null);
  const gridChoice = await invoke<string | null>("read_test_grid_choice").catch(() => null);
  return { records, order, gridChoice };
}

/** Reads the files once; every later session gets what was read (and written) since. */
export function materialFilesReady(): Promise<void> {
  loading ??= readMaterialFiles().then((read) => {
    files = read;
  });
  return loading;
}

/** Gives a session the kept records, the order and the remembered grid values. */
export function applyMaterials(session: WasmSession): void {
  session.load_materials(JSON.stringify(files.records), files.order ?? "");
  if (files.gridChoice !== null) {
    session.load_test_grid_choice(files.gridChoice);
  }
}

/** Performs one write; a failure rejects with the host's reason. */
export function performMaterialWrite(write: MaterialWrite): Promise<unknown> {
  switch (write.op) {
    case "write":
      return invoke("write_material", { id: write.id, text: write.text });
    case "remove":
      return invoke("remove_material", { id: write.id });
    case "order":
      return invoke("write_material_order", { text: write.text });
    case "grid_choice":
      return invoke("write_test_grid_choice", { text: write.text });
  }
}

/** A record write the host could not perform: its reason and the material's name when known. */
export interface MaterialWriteFailure {
  reason: string;
  name: string | null;
}

/** Performs the session's pending writes in order. A failed record write reloads the library
 * from storage into the session, so it shows what is kept, and returns the host's reason with
 * the material's name; `resync` redraws. A preference file failing to save is silent. `null`
 * when all were kept. */
export async function flushMaterialWrites(
  session: WasmSession,
  resync?: () => void,
): Promise<MaterialWriteFailure | null> {
  const writes = decodeMaterialWrites(session.take_material_writes());
  if (writes.length === 0) {
    return null;
  }
  let failure: MaterialWriteFailure | null = null;
  for (const write of writes) {
    try {
      await performMaterialWrite(write);
    } catch (reason) {
      if (write.op === "write" || write.op === "remove") {
        failure ??= { reason: String(reason), name: failedWriteName(write, files.records) };
      }
    }
  }
  files = await readMaterialFiles();
  if (failure !== null) {
    applyMaterials(session);
    resync?.();
  }
  return failure;
}

// The per-user machine profile records and the last pick (ADR 0036 §1, §6, §7). The host only
// moves text (Tauri commands on the desktop, `localStorage` in the browser); `curvyo-laser-core`
// parses and writes inside each session. Read once before the first session; every new session
// gets them; each library change the session makes comes back as writes the host performs here.
import type { WasmSession } from "@/lib/editorSession";
import { invoke } from "@/platform/host";

interface RecordText {
  name: string;
  text: string;
}

/** One write the session asks for (`take_machine_writes`). */
type MachineWrite =
  | { kind: "write"; id: string; text: string }
  | { kind: "remove"; id: string }
  | { kind: "choice"; text: string };

let records: RecordText[] = [];
let choice: string | null = null;
let loading: Promise<void> | null = null;
let onFailure: (() => void) | null = null;

async function readAll(): Promise<void> {
  records = await invoke<RecordText[]>("list_machine_profiles").catch(() => []);
  choice = await invoke<string | null>("read_machine_choice").catch(() => null);
}

/** Reads the records and the last pick once; an unreadable folder means the built-in only. */
export function machineProfilesReady(): Promise<void> {
  loading ??= readAll();
  return loading;
}

/** Gives a session the kept records and the last pick. */
export function applyMachineProfiles(session: WasmSession): void {
  session.load_machine_profiles(JSON.stringify(records));
  if (choice !== null) {
    session.load_machine_choice(choice);
  }
}

/** Who shows "Could not save the profile. Nothing was changed." when a write fails. */
export function setMachineWriteFailureHandler(handler: (() => void) | null): void {
  onFailure = handler;
}

function perform(write: MachineWrite): Promise<unknown> {
  switch (write.kind) {
    case "write":
      return invoke("write_machine_profile", { id: write.id, text: write.text });
    case "remove":
      return invoke("remove_machine_profile", { id: write.id });
    case "choice":
      choice = write.text;
      return invoke("write_machine_choice", { text: write.text });
  }
}

/** Performs the session's pending writes in order. A failed record write reloads the library
 * from storage into the session, so it shows what is kept, and reports the failure; `resync`
 * redraws. The last pick failing to save is silent: it only affects the next Add manufacturing. */
export async function flushMachineWrites(session: WasmSession, resync: () => void): Promise<void> {
  const writes = JSON.parse(session.take_machine_writes()) as MachineWrite[];
  if (writes.length === 0) {
    return;
  }
  let failed = false;
  for (const write of writes) {
    try {
      await perform(write);
    } catch {
      failed ||= write.kind !== "choice";
    }
  }
  await readAll();
  if (failed) {
    session.load_machine_profiles(JSON.stringify(records));
    resync();
    onFailure?.();
  }
}

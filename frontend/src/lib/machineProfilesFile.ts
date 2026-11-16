// The per-user machine profile records and the last pick (ADR 0038 §1, §6, §7). The host only
// moves text (Tauri commands on the desktop, `localStorage` in the browser); `curvyo-laser-core`
// parses and writes inside each session. Read once before the first session; every new session
// gets them; each library change the session makes comes back as writes the host performs here.
import type { WasmSession } from "@/lib/editorSession";
import { MACHINES_PREFIX, invoke } from "@/platform/host";

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
let onRecordNotice: ((text: string) => void) | null = null;
let writing = 0;

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
  // The core has no random source: the ids of new sets and fixtures mix these bytes in (`0113` ADR 0060).
  session.seed_record_ids(crypto.getRandomValues(new Uint8Array(16)));
  session.load_machine_profiles(JSON.stringify(records));
  if (choice !== null) {
    session.load_machine_choice(choice);
  }
}

/** Who shows "Could not save the profile. Nothing was changed." when a write fails. */
export function setMachineWriteFailureHandler(handler: (() => void) | null): void {
  onFailure = handler;
}

/** Who shows the notice of a set edit whose profile write failed and was undone (`0113` criterion 45). */
export function setMachineRecordNoticeHandler(handler: ((text: string) => void) | null): void {
  onRecordNotice = handler;
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
  writing += 1;
  try {
    for (const write of writes) {
      try {
        await perform(write);
      } catch {
        if (write.kind === "write") {
          // A set edit that wrote this record is undone by the session, which names it (criterion 45).
          const notice = session.machine_record_write_failed(write.id);
          if (notice) {
            session.take_fixture_notice();
            onRecordNotice?.(notice);
            resync();
            continue;
          }
        }
        failed ||= write.kind !== "choice";
      }
    }
    await readAll();
  } finally {
    writing -= 1;
  }
  if (failed) {
    session.load_machine_profiles(JSON.stringify(records));
    resync();
    onFailure?.();
  }
}

/** Reads the records again and hands them to `session` when another window or process changed
 * them (`0113` criterion 44): the pages then follow by alignment. Nothing while this window writes. */
export async function reloadMachineRecords(session: WasmSession): Promise<boolean> {
  if (writing > 0) return false;
  const before = JSON.stringify(records);
  const next = await invoke<RecordText[]>("list_machine_profiles").catch(() => null);
  if (next === null || writing > 0) return false;
  const text = JSON.stringify(next);
  if (text === before) return false;
  records = next;
  session.load_machine_profiles(text);
  return true;
}

/** Calls `onChange` when the window gets the focus back and, in the browser, when another window
 * writes a `curvyo.machines/` key; returns the cleanup. */
export function watchMachineRecords(onChange: () => void): () => void {
  const onStorage = (event: StorageEvent) => {
    if (event.key === null || event.key.startsWith(MACHINES_PREFIX)) onChange();
  };
  window.addEventListener("focus", onChange);
  window.addEventListener("storage", onStorage);
  return () => {
    window.removeEventListener("focus", onChange);
    window.removeEventListener("storage", onStorage);
  };
}

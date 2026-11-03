// The text of the per-user extension settings, `extensions.toml` (ADR 0034 §6): which compiled-in
// extensions are switched off. The host only moves text (a Tauri command on the desktop,
// `localStorage` in the browser); `curvyo-ui-core` parses and writes it inside each session. Read
// once before the first session; every new session gets it; kept current after each switch.
import type { WasmSession } from "@/lib/editorSession";
import { invoke } from "@/platform/host";

let text: string | null = null;
let loading: Promise<void> | null = null;

/** Reads the file once; a missing or unreadable file means every extension is on (permit). */
export function extensionsFileReady(): Promise<void> {
  loading ??= invoke<string | null>("read_extensions_file").then(
    (read) => {
      text = read;
    },
    () => {
      text = null;
    },
  );
  return loading;
}

/** Gives a new session the kept settings. */
export function applyExtensions(session: WasmSession): void {
  if (text !== null) {
    session.load_extension_settings(text);
  }
}

/** Keeps `newText` and writes it; a failed write keeps the switch for this run only. */
export function storeExtensions(newText: string): void {
  text = newText;
  invoke("write_extensions_file", { text: newText }).catch(() => undefined);
}

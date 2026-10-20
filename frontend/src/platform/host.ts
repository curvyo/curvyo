// The one place the frontend talks to its host. Inside the Tauri desktop
// shell `invoke`/`listen` are Tauri's own, unchanged. In a plain browser (the
// public demo build, docs/deploy-demo.md) there is no host: the same two
// functions are served by a tiny in-page implementation, so `App` runs
// the same code on both. Browser Open reads a picked file, Save downloads it
// and counts as saved: the page cannot see whether the browser's own save prompt was cancelled.
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import {
  listen as tauriListen,
  type EventCallback,
  type UnlistenFn,
} from "@tauri-apps/api/event";

import {
  browserFontBytes,
  listBrowserFonts,
  removeBrowserFontFiles,
} from "@/lib/browserFonts";
import { detectPlatform, type PlatformKind } from "@/lib/platformKind";
import type { OpenPayload, SaveOutcome } from "@/lib/projectState";

/** True inside the Tauri desktop shell, false in a plain browser. */
export const isTauri = "__TAURI_INTERNALS__" in window;

/** The build and system the page runs on: which file chords exist and how they are named. */
export const PLATFORM: PlatformKind = detectPlatform(navigator.platform, isTauri);

/** The name a download gets when the project has none. */
const DEFAULT_FILE_NAME = "Untitled.curvyo";

/** Browser Save: downloads the bytes as `fileName`. */
function download(bytes: number[], fileName: string): SaveOutcome {
  const url = URL.createObjectURL(new Blob([new Uint8Array(bytes)]));
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  link.click();
  URL.revokeObjectURL(url);
  return { kind: "saved", file_name: fileName, path: fileName };
}

/** Browser Open: the picked `.curvyo` file's name and bytes, `null` when the picker was
 * cancelled, or the read error. The answer returns to the caller, as the desktop command's does. */
function pickProject(): Promise<OpenPayload | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".curvyo";
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) {
        resolve(null);
        return;
      }
      file.arrayBuffer().then(
        (buffer) =>
          resolve({ kind: "bytes", path: file.name, bytes: Array.from(new Uint8Array(buffer)) }),
        () => resolve({ kind: "io_error", message: "This file couldn't be read." }),
      );
    };
    input.addEventListener("cancel", () => resolve(null));
    // An engine that sends no `cancel` would leave this open for good, and the dialog gate with
    // it. When the window gets the focus back and no file arrived, the picker was dismissed.
    window.addEventListener(
      "focus",
      () => window.setTimeout(() => (input.files?.length ? undefined : resolve(null)), 1500),
      { once: true },
    );
    input.click();
  });
}

/** Where the browser build keeps the text of the formats file
 * (`specs/0045-document-formats-library/` criterion 31). Every access is
 * guarded: a failure means the built-in formats only. */
const FORMATS_KEY = "curvyo.document-formats";
/** The font library's text and the check cache in the browser build (`0075` decisions 1, 5). */
const FONT_LIBRARY_KEY = "curvyo.font-library";
const FONT_CHECKS_KEY = "curvyo.font-checks";

function readStored(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

/** Browser Import: a picked `.toml` file's name and text, or `null` when the
 * picker was cancelled. */
function pickTextFile(): Promise<{ name: string; text: string } | null> {
  return new Promise((resolve, reject) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".toml";
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) {
        resolve(null);
        return;
      }
      file.text().then((text) => resolve({ name: file.name, text }), reject);
    };
    input.addEventListener("cancel", () => resolve(null));
    input.click();
  });
}

function downloadText(name: string, text: string) {
  const url = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  URL.revokeObjectURL(url);
}

const browserCommands: Record<string, (args?: Record<string, unknown>) => unknown> = {
  take_pending_open: () => null,
  pick_project: () => pickProject(),
  confirm_project_opened: () => undefined,
  reset_project: () => undefined,
  set_dirty: () => undefined,
  save_project_bytes: (args) =>
    download(args?.bytes as number[], String(args?.fileName ?? DEFAULT_FILE_NAME)),
  read_formats_file: () => readStored(FORMATS_KEY),
  write_formats_file: (args) => window.localStorage.setItem(FORMATS_KEY, String(args?.text)),
  set_formats_file_aside: () => {
    const text = readStored(FORMATS_KEY);
    if (text !== null) {
      window.localStorage.setItem(`${FORMATS_KEY}.broken`, text);
    }
    window.localStorage.removeItem(FORMATS_KEY);
  },
  import_formats_file: () => pickTextFile(),
  export_formats_file: (args) => {
    downloadText("curvyo-formats.toml", String(args?.text));
    return true;
  },
  // The browser has no system fonts (`specs/0074-text-objects` criterion 16); its Curvyo fonts
  // are the files added in this browser, kept in IndexedDB (`specs/0075-font-library`
  // criterion 28).
  font_folder_list: () => listBrowserFonts(),
  system_font_list: () => [],
  font_bytes: (args) => browserFontBytes(String(args?.id)),
  // The browser picks and adds font files through `browserFonts` (File objects, no paths).
  pick_font_files: () => {
    throw new Error("the browser picks font files through browserFonts");
  },
  add_font_files: () => {
    throw new Error("the browser adds font files through browserFonts");
  },
  remove_font_files: (args) => removeBrowserFontFiles(args?.faces as { id: string; index: number }[]),
  // The font library file and the check cache sit beside the formats file (`0075` decisions 1, 5).
  read_font_library: () => readStored(FONT_LIBRARY_KEY),
  write_font_library: (args) => window.localStorage.setItem(FONT_LIBRARY_KEY, String(args?.text)),
  set_font_library_aside: () => {
    const text = readStored(FONT_LIBRARY_KEY);
    if (text !== null) {
      window.localStorage.setItem(`${FONT_LIBRARY_KEY}.broken`, text);
    }
    window.localStorage.removeItem(FONT_LIBRARY_KEY);
  },
  read_font_checks: () => readStored(FONT_CHECKS_KEY),
  write_font_checks: (args) => window.localStorage.setItem(FONT_CHECKS_KEY, String(args?.text)),
};

/** Same contract as Tauri's `invoke`. */
export function invoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (isTauri) {
    return tauriInvoke<T>(command, args);
  }
  const handler = browserCommands[command];
  if (!handler) {
    return Promise.reject(new Error(`${command} is not available in the browser`));
  }
  try {
    return Promise.resolve(handler(args) as T);
  } catch (error) {
    // A guarded storage access that fails is a rejected command, not a throw.
    return Promise.reject(error);
  }
}

/** Same contract as Tauri's `listen`. The browser has no host, so no event ever arrives. */
export function listen<T>(
  event: string,
  handler: EventCallback<T>,
): Promise<UnlistenFn> {
  if (isTauri) {
    return tauriListen<T>(event, handler);
  }
  return Promise.resolve(() => undefined);
}

/** Why this browser cannot run the editor, or `null` if it can. */
export function unsupportedReason(): string | null {
  if (typeof WebAssembly === "undefined") {
    return "This browser has no WebAssembly support, which Curvyo needs.";
  }
  if (!document.createElement("canvas").getContext("webgl2")) {
    return "This browser or device has no WebGL 2, which Curvyo needs to draw. Try a current Chrome, Edge, Firefox or Safari with hardware acceleration on.";
  }
  return null;
}

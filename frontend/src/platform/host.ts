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
  browserDocumentBytes,
  browserFontBytes,
  listBrowserFonts,
  removeBrowserFontFiles,
} from "@/lib/browserFonts";
import { ARGS_HEADER, asciiJson } from "@/lib/binaryArgs";
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

/** What the browser's picker offers per dialog kind (`specs/0024-svg-import-export` criterion 1):
 * Open… projects and SVG drawings, Template projects. */
const PICKER_ACCEPT: Record<string, string> = {
  open: ".curvyo,.svg,image/svg+xml",
  project: ".curvyo",
};

/** What the Import button's picker offers (`specs/0058-bitmap-import` criterion 1). */
const IMPORT_ACCEPT = ".svg,.png,.jpg,.jpeg,.webp,image/svg+xml,image/png,image/jpeg,image/webp";

/** Browser Import: every picked file's name and bytes in the picker's order, none when the
 * picker was cancelled; a file that cannot be read is its read error. */
function pickImportFiles(): Promise<OpenPayload[]> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = true;
    input.accept = IMPORT_ACCEPT;
    input.onchange = () => {
      const files = Array.from(input.files ?? []);
      void Promise.all(
        files.map((file) =>
          file.arrayBuffer().then(
            (buffer): OpenPayload => ({
              kind: "bytes",
              path: file.name,
              bytes: Array.from(new Uint8Array(buffer)),
            }),
            (): OpenPayload => ({ kind: "io_error", message: "This file couldn't be read." }),
          ),
        ),
      ).then(resolve);
    };
    input.addEventListener("cancel", () => resolve([]));
    window.addEventListener(
      "focus",
      () => window.setTimeout(() => (input.files?.length ? undefined : resolve([])), 1500),
      { once: true },
    );
    input.click();
  });
}

/** Browser Open: the picked file's name and bytes, `null` when the picker was cancelled, or the
 * read error. The answer returns to the caller, as the desktop command's does. */
function pickProject(kind: string): Promise<OpenPayload | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = PICKER_ACCEPT[kind] ?? PICKER_ACCEPT.project;
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
/** The browser keeps `extensions.toml` here (ADR 0034 §6). */
const EXTENSIONS_KEY = "curvyo.extensions";
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

/** Browser Export SVG… (`specs/0024-svg-import-export` criterion 4): one Blob, downloaded as
 * `name`. */
export function downloadSvg(name: string, text: string): void {
  const url = URL.createObjectURL(new Blob([text], { type: "image/svg+xml" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  URL.revokeObjectURL(url);
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
  pick_project: (args) => pickProject(String(args?.kind ?? "project")),
  pick_import_files: () => pickImportFiles(),
  confirm_project_opened: () => undefined,
  reset_project: () => undefined,
  set_dirty: () => undefined,
  // The browser downloads an export without a dialog (`specs/0024-svg-import-export` criterion 4);
  // the page calls these two only on the desktop.
  pick_svg_export_path: (args) => String(args?.suggestedName ?? "Untitled.svg"),
  write_svg_export: () => {
    throw new Error("the browser downloads an SVG export");
  },
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
  read_extensions_file: () => readStored(EXTENSIONS_KEY),
  write_extensions_file: (args) => window.localStorage.setItem(EXTENSIONS_KEY, String(args?.text)),
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
  // The browser cannot read a local path (`specs/0087-image-clipboard-and-drop` criterion 6).
  read_local_files: () => [],
  add_dropped_fonts: () => {
    throw new Error("the browser adds dropped font files through browserFonts");
  },
  remove_font_files: (args) => removeBrowserFontFiles(args?.faces as { id: string; index: number }[]),
  // A font folder's documents from IndexedDB (`specs/0083-font-info-and-zip` criterion 4): the
  // first 1 MiB of a text, a PDF in a new tab from the stored bytes (no network request).
  read_font_document: async (args) => {
    const bytes = await browserDocumentBytes(String(args?.folder), String(args?.name));
    const limit = 1024 * 1024;
    return { text: new TextDecoder("utf-8").decode(bytes.slice(0, limit)), cut: bytes.byteLength > limit };
  },
  open_font_document: async (args) => {
    const bytes = await browserDocumentBytes(String(args?.folder), String(args?.name));
    const url = URL.createObjectURL(new Blob([bytes], { type: "application/pdf" }));
    window.open(url, "_blank", "noopener");
    window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
  },
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
  // Browser key custody is undecided (`specs/0092-cooperative-editing/adrs.md`, debt): the
  // device identity and the epoch keys live in this page's memory only and end with it.
  read_device_keys: (args) => ({
    status: "ok",
    identity: browserIdentity(),
    keys: browserRoomKeys.get(String(args?.room)) ?? [],
  }),
  // The browser's tab title names no share (the desktop build is slice 1).
  set_shared: () => undefined,
  write_document_keys: (args) => {
    browserRoomKeys.set(String(args?.room), args?.keys as [number, string][]);
  },
  // A workspace is a folder, which the browser cannot open (`specs/0094-workspace`, desktop app
  // only): no recent workspaces, and the section is not shown, so nothing else is called.
  recent_workspaces: () => [],
  remove_recent_workspace: () => [],
  pick_workspace: () => ({ kind: "cancelled" }),
  open_recent_workspace: () => ({ kind: "refused", code: "not-found" }),
  close_workspace: () => undefined,
  scan_open_workspace: () => noWorkspace(),
  apply_workspace_op: () => noWorkspace(),
  read_workspace_document: () => noWorkspace(),
  confirm_workspace_document_opened: () => undefined,
  probe_workspace_document: () => noWorkspace(),
  workspace_marker_share: () => noWorkspace(),
  write_workspace_marker_share: () => noWorkspace(),
  write_workspace_id: () => noWorkspace(),
  replace_workspace_document: () => noWorkspace(),
  write_workspace_index: () => noWorkspace(),
  pick_join_folder: () => ({ kind: "cancelled" }),
  start_joined_workspace: () => ({ kind: "refused", code: "not-found" }),
  scan_shared_workspace: () => noWorkspace(),
  keep_workspace_document: () => noWorkspace(),
  restore_workspace_document: () => noWorkspace(),
  read_deleted_document: () => noWorkspace(),
  write_deleted_document: () => noWorkspace(),
  clear_workspace_marker_share: () => noWorkspace(),
};

/** What a workspace command answers in the browser: there is no folder. */
function noWorkspace(): never {
  throw "not-found";
}

/** The browser page's device identity: 64 random bytes as hex, made once per page. */
let pageIdentity: string | null = null;
function browserIdentity(): string {
  if (pageIdentity === null) {
    const bytes = new Uint8Array(64);
    crypto.getRandomValues(bytes);
    pageIdentity = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  }
  return pageIdentity;
}
const browserRoomKeys = new Map<string, [number, string][]>();

/** Same contract as Tauri's `invoke`. With `bytes`, they travel as a raw binary body and `args`
 * in a header (`curvyo-app`'s `raw_ipc`), so a file never crosses as a JSON number array; the
 * browser stand-in gets `{ ...args, bytes }`. */
export function invoke<T>(
  command: string,
  args?: Record<string, unknown>,
  bytes?: Uint8Array,
): Promise<T> {
  if (isTauri) {
    return bytes
      ? tauriInvoke<T>(command, bytes, { headers: { [ARGS_HEADER]: asciiJson(args ?? {}) } })
      : tauriInvoke<T>(command, args);
  }
  const handler = browserCommands[command];
  if (!handler) {
    return Promise.reject(new Error(`${command} is not available in the browser`));
  }
  try {
    return Promise.resolve(handler(bytes ? { ...args, bytes } : args) as T);
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

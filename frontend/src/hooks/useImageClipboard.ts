import { useEffect, useRef } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import type { ImportFile } from "@/hooks/useProject";
import { isMac } from "@/lib/historyText";
import {
  type ClipboardFile,
  copyFailureReason,
  isCopyChord,
  keepsClipboard,
  pastedName,
  pickPasted,
} from "@/lib/imageClipboard";
import { type TraceRunner, createTraceRunner } from "@/lib/traceJobs";
import { resolveLocalFiles } from "@/platform/localFiles";
import { clipboard_notice, paste_rank } from "@/wasm-bindings/curvyo_editor_wasm.js";

const MAC = isMac(navigator.platform);

/** After this long a copy shows "Copying..." (criterion 14). */
const COPYING_AFTER_MS = 400;

/** The files of a paste event: its file items, else its file list. */
function clipboardFiles(data: DataTransfer | null): File[] {
  if (!data) {
    return [];
  }
  const items = Array.from(data.items)
    .filter((item) => item.kind === "file")
    .map((item) => item.getAsFile())
    .filter((file): file is File => file !== null);
  return items.length > 0 ? items : Array.from(data.files);
}

/** Whether the page lets the session take a chord now: no dialog, nothing typed into. */
function pageAllows(): boolean {
  return !document.querySelector('[role="alertdialog"]') && !keepsClipboard(document.activeElement);
}

/**
 * Ctrl+V and Ctrl+C for images (`specs/0087-image-clipboard-and-drop` criteria 1 to 16, `adrs.md`
 * decisions 1 and 2). A paste is read from the page's `paste` event, which needs no permission:
 * the best image file goes through the import under the last pointer position over the canvas
 * (else the view centre). Ctrl+C writes the topmost selected image as PNG with
 * `navigator.clipboard.write`, the `ClipboardItem` made inside the key press and its PNG made in
 * the trace worker. A text field and the text editor keep both keys. On the desktop, a clipboard
 * without an image whose text names local files (a file manager's copy) pastes those files, read
 * by the host (criterion 6, lead decision 2026-10-29).
 */
export function useImageClipboard(
  editor: EditorSession,
  pasteImage: (file: ImportFile, at: { x: number; y: number } | null) => void,
  pasteFiles: (files: ImportFile[], at: { x: number; y: number } | null) => void,
) {
  const depsRef = useRef({ editor, pasteImage, pasteFiles });
  useEffect(() => {
    depsRef.current = { editor, pasteImage, pasteFiles };
  });
  const runnerRef = useRef<TraceRunner | null>(null);

  useEffect(() => {
    const notice = (text: string, alert = false) =>
      depsRef.current.editor.showHistoryNotice(text, alert ? 5000 : undefined, "canvas", alert ? "alert" : "status");
    const blocked = () => document.documentElement.hasAttribute("aria-busy");

    const onPaste = (event: ClipboardEvent) => {
      const { editor } = depsRef.current;
      const session = editor.getSession();
      if (!session || keepsClipboard(event.target) || !pageAllows() || !session.clipboard_allowed(blocked())) {
        return;
      }
      event.preventDefault();
      // Read now: the pointer may move while the files are read.
      const at = editor.lastCanvasPointer();
      const files = clipboardFiles(event.clipboardData);
      void Promise.all(
        files.map(
          async (file): Promise<ClipboardFile> => ({
            name: file.name,
            type: file.type,
            bytes: new Uint8Array(await file.arrayBuffer().catch(() => new ArrayBuffer(0))),
          }),
        ),
      ).then((read) => {
        const picked = pickPasted(read, (file) => paste_rank(file.bytes, file.type));
        if (picked) {
          depsRef.current.pasteImage({ name: pastedName(picked.name), bytes: picked.bytes }, at);
          return;
        }
        void resolveLocalFiles("paste").then((local) => {
          if (local.length > 0) {
            depsRef.current.pasteFiles(local, at);
          } else {
            notice(clipboard_notice("nothing", ""));
          }
        });
      });
    };

    const onKey = (event: KeyboardEvent) => {
      const { editor } = depsRef.current;
      const session = editor.getSession();
      if (!session || !isCopyChord(event, MAC) || event.isComposing || !pageAllows()) {
        return;
      }
      if (!session.clipboard_allowed(blocked())) {
        return;
      }
      const copy = session.image_copy();
      if (!copy) {
        // No image selected: the key does nothing and the browser keeps its own copy.
        return;
      }
      event.preventDefault();
      const { width, height } = copy;
      const rgba = copy.rgba;
      copy.free();
      if (typeof ClipboardItem === "undefined" || !navigator.clipboard?.write) {
        notice(clipboard_notice("newer-browser", ""), true);
        return;
      }
      runnerRef.current ??= createTraceRunner();
      const png = runnerRef.current.png(rgba, width, height).then((bytes) => new Blob([bytes as BlobPart], { type: "image/png" }));
      // The item is made inside the key press with a promise, as Safari requires.
      let item: ClipboardItem;
      try {
        item = new ClipboardItem({ "image/png": png });
      } catch (error) {
        notice(clipboard_notice("failed", copyFailureReason(error)), true);
        return;
      }
      let done = false;
      let copyingShown = false;
      const slow = window.setTimeout(() => {
        if (!done) {
          copyingShown = true;
          notice(clipboard_notice("copying", ""));
        }
      }, COPYING_AFTER_MS);
      navigator.clipboard
        .write([item])
        .then(
          // No notice for a successful copy (customer rule of 2026-10-31); only the busy line goes.
          () => {
            if (copyingShown) {
              depsRef.current.editor.showHistoryNotice("", 1);
            }
          },
          (error: unknown) => notice(clipboard_notice("failed", copyFailureReason(error)), true),
        )
        .finally(() => {
          done = true;
          window.clearTimeout(slow);
        });
    };

    document.addEventListener("paste", onPaste);
    window.addEventListener("keydown", onKey, true);
    return () => {
      document.removeEventListener("paste", onPaste);
      window.removeEventListener("keydown", onKey, true);
      runnerRef.current?.dispose();
      runnerRef.current = null;
    };
  }, []);
}

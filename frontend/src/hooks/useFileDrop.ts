import { useEffect, useRef, useState } from "react";
import type { DragEvent, RefObject } from "react";

import type { DroppedFiles, ImportFile } from "@/hooks/useProject";
import { dragKind, dropContent } from "@/lib/dropData";
import { isTauri } from "@/platform/host";
import { resolveLocalFiles } from "@/platform/localFiles";

/** Whether a drag carries files or a link (on the desktop also plain text, which may name local
 * files): what the canvas takes and the page refuses elsewhere. */
function carriesFiles(event: { dataTransfer: DataTransfer | null }): boolean {
  return dragKind(Array.from(event.dataTransfer?.types ?? []), isTauri) !== null;
}

/**
 * Files dropped on the canvas (`specs/0058-bitmap-import` criterion 4, UX notes "Drag and drop";
 * `specs/0087-image-clipboard-and-drop` criteria 19 to 23): while a drag with files, image data or
 * a link is over the canvas the dashed outline shows; a drop hands the files (image data from a
 * browser comes as a file item) and the drop point (canvas CSS pixels) to `onDrop`, and a drop of
 * only a link to `onLink`, which fetches nothing. On the desktop, a drop whose text names local
 * files (a file manager's `text/uri-list` or paths) hands those files, read by the host, to `onDrop`
 * (criterion 21, lead decision 2026-10-29). Anywhere else in the page such a drag shows the
 * "none" effect and a drop is cancelled, so it never navigates the webview; a drop target that
 * took the drag itself (the Fonts tab) keeps it.
 */
export function useFileDrop(
  canvasRef: RefObject<HTMLCanvasElement | null>,
  onDrop: ((dropped: DroppedFiles) => void) | undefined,
  onLink?: () => void,
) {
  const [over, setOver] = useState(false);
  const depth = useRef(0);

  useEffect(() => {
    const refuse = (event: globalThis.DragEvent) => {
      // Plain text is left alone: it never navigates the webview, and a field may take it.
      if (dragKind(Array.from(event.dataTransfer?.types ?? [])) === null || event.defaultPrevented) {
        return;
      }
      event.preventDefault();
      if (event.dataTransfer) {
        event.dataTransfer.dropEffect = "none";
      }
    };
    window.addEventListener("dragover", refuse);
    window.addEventListener("drop", refuse);
    return () => {
      window.removeEventListener("dragover", refuse);
      window.removeEventListener("drop", refuse);
    };
  }, []);

  const handlers = {
    onDragEnter: (event: DragEvent) => {
      if (!carriesFiles(event) || !onDrop) {
        return;
      }
      depth.current += 1;
      setOver(true);
    },
    onDragOver: (event: DragEvent) => {
      if (!carriesFiles(event) || !onDrop) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      event.dataTransfer.dropEffect = "copy";
    },
    onDragLeave: (event: DragEvent) => {
      if (!carriesFiles(event)) {
        return;
      }
      depth.current = Math.max(0, depth.current - 1);
      if (depth.current === 0) {
        setOver(false);
      }
    },
    onDrop: (event: DragEvent) => {
      if (!carriesFiles(event) || !onDrop) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      depth.current = 0;
      setOver(false);
      const box = canvasRef.current?.getBoundingClientRect();
      const at = { x: event.clientX - (box?.left ?? 0), y: event.clientY - (box?.top ?? 0) };
      const content = dropContent(event.dataTransfer);
      if (content?.kind !== "files") {
        // A uri list or text may name local files; the host reads the dropped drag's list itself
        // (WebKitGTK hides it from the page).
        const types = Array.from(event.dataTransfer.types);
        const named = types.includes("text/uri-list") || types.includes("text/plain");
        void (named ? resolveLocalFiles("drop") : Promise.resolve([])).then((local) => {
          if (local.length > 0) {
            onDrop({ files: local, at });
          } else if (content?.kind === "link") {
            onLink?.();
          }
        });
        return;
      }
      const files = content.files;
      void Promise.all(
        files.map(
          (file): Promise<ImportFile> =>
            file.arrayBuffer().then(
              (buffer) => ({ name: file.name, bytes: new Uint8Array(buffer) }),
              () => ({ name: file.name, bytes: new Uint8Array() }),
            ),
        ),
      ).then((read) => onDrop({ files: read, at }));
    },
  };
  return { over, handlers };
}

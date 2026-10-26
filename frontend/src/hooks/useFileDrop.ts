import { useEffect, useRef, useState } from "react";
import type { DragEvent, RefObject } from "react";

import type { DroppedFiles, ImportFile } from "@/hooks/useProject";

/** Whether a drag carries files from the file manager. */
function carriesFiles(event: { dataTransfer: DataTransfer | null }): boolean {
  return event.dataTransfer?.types.includes("Files") ?? false;
}

/**
 * Files dropped on the canvas (`specs/0058-bitmap-import` criterion 4, UX notes "Drag and drop"):
 * while a drag with files is over the canvas the dashed outline shows; a drop hands the files and
 * the drop point (canvas CSS pixels) to `onDrop`. Anywhere else in the page a file drag shows the
 * "none" effect and a drop is cancelled, so a file never navigates the webview.
 */
export function useFileDrop(
  canvasRef: RefObject<HTMLCanvasElement | null>,
  onDrop: ((dropped: DroppedFiles) => void) | undefined,
) {
  const [over, setOver] = useState(false);
  const depth = useRef(0);

  useEffect(() => {
    const refuse = (event: globalThis.DragEvent) => {
      if (!carriesFiles(event)) {
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
      const files = Array.from(event.dataTransfer.files);
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

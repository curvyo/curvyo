// Font files dropped on the Fonts tab (`specs/0075-font-library` criterion 24, UX notes §4): the
// desktop hears the window's drag-drop event and hit-tests the tab's body; the browser uses the
// page's drop events. A drag that carries no files shows nothing.
import { useEffect, useState } from "react";

import { isTauri } from "@/platform/host";

export interface FontDrop {
  /** Files are dragged over the tab's body. */
  over: boolean;
  /** The browser's drag handlers for the body (none on the desktop). */
  handlers: {
    onDragOver?: (event: React.DragEvent) => void;
    onDragLeave?: (event: React.DragEvent) => void;
    onDrop?: (event: React.DragEvent) => void;
  };
}

function hasFiles(event: React.DragEvent): boolean {
  return Array.from(event.dataTransfer.types).includes("Files");
}

export function useFontDrop(
  bodyRef: React.RefObject<HTMLElement | null>,
  onDrop: (dropped: { paths?: string[]; files?: File[] }) => void,
): FontDrop {
  const [over, setOver] = useState(false);

  useEffect(() => {
    if (!isTauri) {
      return undefined;
    }
    const inside = (x: number, y: number) => {
      const rect = bodyRef.current?.getBoundingClientRect();
      const scale = window.devicePixelRatio || 1;
      const [cx, cy] = [x / scale, y / scale];
      return !!rect && cx >= rect.left && cx <= rect.right && cy >= rect.top && cy <= rect.bottom;
    };
    let unlisten: (() => void) | null = null;
    let live = true;
    void import("@tauri-apps/api/webview").then(({ getCurrentWebview }) =>
      getCurrentWebview()
        .onDragDropEvent((event) => {
          const payload = event.payload;
          if (payload.type === "leave") {
            setOver(false);
            return;
          }
          const hit = inside(payload.position.x, payload.position.y);
          if (payload.type === "drop") {
            setOver(false);
            if (hit && payload.paths.length > 0) {
              onDrop({ paths: payload.paths });
            }
            return;
          }
          setOver(hit);
        })
        .then((stop) => {
          if (live) {
            unlisten = stop;
          } else {
            stop();
          }
        }),
    );
    return () => {
      live = false;
      unlisten?.();
    };
  }, [bodyRef, onDrop]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOver(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (isTauri) {
    return { over, handlers: {} };
  }
  return {
    over,
    handlers: {
      onDragOver: (event) => {
        if (hasFiles(event)) {
          event.preventDefault();
          setOver(true);
        }
      },
      onDragLeave: (event) => {
        if (!bodyRef.current?.contains(event.relatedTarget as Node | null)) {
          setOver(false);
        }
      },
      onDrop: (event) => {
        if (!hasFiles(event)) {
          return;
        }
        event.preventDefault();
        setOver(false);
        onDrop({ files: Array.from(event.dataTransfer.files) });
      },
    },
  };
}

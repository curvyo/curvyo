// Font files dropped on the Fonts tab (`specs/0075-font-library` criterion 24, UX notes §4): the
// page's drop events in both builds; the desktop window hands every drop to the page
// (`dragDropEnabled: false`, `specs/0087-image-clipboard-and-drop` criterion 19). A drag that
// carries no files shows nothing.
import { useEffect, useState } from "react";

export interface FontDrop {
  /** Files are dragged over the tab's body. */
  over: boolean;
  /** The drag handlers for the body. */
  handlers: {
    onDragOver: (event: React.DragEvent) => void;
    onDragLeave: (event: React.DragEvent) => void;
    onDrop: (event: React.DragEvent) => void;
  };
}

function hasFiles(event: React.DragEvent): boolean {
  return Array.from(event.dataTransfer.types).includes("Files");
}

export function useFontDrop(
  bodyRef: React.RefObject<HTMLElement | null>,
  onDrop: (dropped: { files: File[] }) => void,
): FontDrop {
  const [over, setOver] = useState(false);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOver(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return {
    over,
    handlers: {
      onDragOver: (event) => {
        if (hasFiles(event)) {
          // Taken here: the page's refusal of drops outside the canvas does not see it.
          event.preventDefault();
          event.stopPropagation();
          event.dataTransfer.dropEffect = "copy";
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
        event.stopPropagation();
        setOver(false);
        onDrop({ files: Array.from(event.dataTransfer.files) });
      },
    },
  };
}

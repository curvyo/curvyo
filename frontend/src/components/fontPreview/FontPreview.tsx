// The font preview pane (`specs/0077-font-preview`, UX notes §1): the header and the tile grid of
// the Fonts tab's listed families; Escape anywhere inside closes it, and so does an Escape outside
// that nothing else handled; the focus goes back to the Preview button when it came from inside
// and the Fonts tab is shown, else to the canvas.
import { useCallback, useEffect, useRef } from "react";

import { PreviewGrid } from "@/components/fontPreview/PreviewGrid";
import { PreviewHeader } from "@/components/fontPreview/PreviewHeader";
import type { FontPreviewApi } from "@/hooks/useFontPreview";
import { escapeClosesPreview } from "@/lib/fontPreviewKeys";
import { type PaneLayout, headerRows } from "@/lib/fontPreviewLayout";
import { PREVIEW_BUTTON_ID, sampleFocused } from "@/lib/fontPreviewStore";
import { MAX_SAMPLE_CHARS } from "@/lib/fontPreviewState";
import type { FontFace as ListedFace } from "@/lib/textText";

interface FontPreviewProps {
  preview: FontPreviewApi;
  layout: PaneLayout;
  /** The divider is being dragged. */
  resizing: boolean;
  /** The Full view/Split view switch. */
  onSwitchView: () => void;
  currentFace: number | null;
  onPick: (face: ListedFace, byKeyboard: boolean) => void;
  /** The canvas, or the text sink in edit mode. */
  onReturnFocus: () => void;
}

export function FontPreview({ preview, layout, resizing, onSwitchView, currentFace, onPick, onReturnFocus }: FontPreviewProps) {
  const rootRef = useRef<HTMLElement | null>(null);
  const sampleRef = useRef<HTMLInputElement | null>(null);
  const { store } = preview;

  // Opened by a key: the sample field takes the focus (UX notes §1).
  useEffect(() => {
    if (store.focusSample) {
      sampleRef.current?.focus();
      sampleFocused();
    }
  }, [store.focusSample]);

  // Escape outside the preview closes it once the canvas, the text edit or a control had nothing
  // of its own to cancel (criterion 5): those handle it first and mark it handled.
  const closeRef = useRef<() => void>(() => {});
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (escapeClosesPreview(event.key, event.defaultPrevented, true)) {
        event.preventDefault();
        closeRef.current();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const close = () => {
    const inside = rootRef.current?.contains(document.activeElement) ?? false;
    preview.close();
    if (inside) {
      window.requestAnimationFrame(() => {
        const button = document.getElementById(PREVIEW_BUTTON_ID);
        if (button) {
          button.focus();
        } else {
          onReturnFocus();
        }
      });
    }
  };

  useEffect(() => {
    closeRef.current = close;
  });

  // Stable, so a divider drag does not re-render the grid (`PreviewGrid` is memoised).
  const sampleNow = useRef(preview.sample);
  useEffect(() => {
    sampleNow.current = preview.sample;
  });
  const { typeSample } = preview;
  const typeIntoSample = useCallback(
    (character: string) => {
      typeSample((sampleNow.current + character).slice(0, MAX_SAMPLE_CHARS));
      sampleRef.current?.focus();
    },
    [typeSample],
  );

  return (
    <section
      ref={rootRef}
      role="region"
      aria-label="Font preview"
      className="flex min-h-0 min-w-0 flex-1 flex-col"
      onKeyDown={(event) => {
        if (event.key === "Escape" && !event.defaultPrevented) {
          event.preventDefault();
          event.stopPropagation();
          close();
        }
      }}
    >
      <PreviewHeader
        rows={headerRows(layout.width)}
        sample={preview.sample}
        onSample={preview.typeSample}
        sampleRef={sampleRef}
        size={preview.size}
        onSize={preview.setSize}
        subject={store.listing.subject}
        canSplit={layout.canSplit}
        full={layout.full}
        onFull={onSwitchView}
        onClose={close}
        onReturnFocus={onReturnFocus}
      />
      <PreviewGrid
        listing={store.listing}
        sample={preview.sample}
        size={preview.size}
        currentFace={currentFace}
        resizing={resizing}
        onPick={onPick}
        onType={typeIntoSample}
      />
    </section>
  );
}

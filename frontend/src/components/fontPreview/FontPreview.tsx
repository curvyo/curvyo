// The font preview pane (`specs/0077-font-preview`, UX notes §1; `specs/0082-glyph-table`): the
// header and, in Fonts mode, the tile grid of the Fonts tab's listed families, in Glyphs mode the
// glyph table of the subject face and its detail bar; Escape anywhere inside closes it, and so
// does an Escape outside that nothing else handled; the focus goes back to the Preview button when
// it came from inside and the Fonts tab is shown, else to the canvas.
import { useCallback, useEffect, useRef, useState } from "react";

import { GlyphDetailBar } from "@/components/fontPreview/GlyphDetailBar";
import { GlyphGrid } from "@/components/fontPreview/GlyphGrid";
import { PreviewGrid } from "@/components/fontPreview/PreviewGrid";
import { PreviewHeader } from "@/components/fontPreview/PreviewHeader";
import type { FontPreviewApi } from "@/hooks/useFontPreview";
import { useGlyphTable } from "@/hooks/useGlyphTable";
import type { GlyphSubject } from "@/hooks/useText";
import { charOf, glyphSubjectLine, insertedText } from "@/lib/glyphTableText";
import { focusStaysOnCell } from "@/lib/glyphTableLayout";
import { escapeClosesPreview } from "@/lib/fontPreviewKeys";
import { type PaneLayout, headerRows } from "@/lib/fontPreviewLayout";
import { PREVIEW_BUTTON_ID, sampleFocused } from "@/lib/fontPreviewStore";
import { MAX_GLYPH_SEARCH_CHARS, MAX_SAMPLE_CHARS } from "@/lib/fontPreviewState";
import type { FontFace as ListedFace } from "@/lib/textText";

interface FontPreviewProps {
  preview: FontPreviewApi;
  layout: PaneLayout;
  /** The divider is being dragged. */
  resizing: boolean;
  /** The Full view/Split view switch. */
  onSwitchView: () => void;
  currentFace: number | null;
  /** A press on a tile: a family pick (`specs/0078-font-style-buttons` criterion 24). */
  onPick: (face: ListedFace, byKeyboard: boolean) => void;
  /** The face a tile of `face`'s family shows (criterion 24). */
  tileFace: (face: ListedFace) => { id: string; faceIndex: number };
  /** The canvas, or the text sink in edit mode. */
  onReturnFocus: () => void;
  /** The face the glyph table shows (`0082` "Terms"). */
  glyphSubject: GlyphSubject | null;
  /** A character from the glyph table inserted (`0082` criteria 12 to 14). */
  onInsertGlyph: (character: string, keepFocus: boolean) => void;
}

export function FontPreview({
  preview,
  layout,
  resizing,
  onSwitchView,
  currentFace,
  onPick,
  tileFace,
  onReturnFocus,
  glyphSubject,
  onInsertGlyph,
}: FontPreviewProps) {
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

  // Glyphs mode (`specs/0082-glyph-table`).
  const glyphs = preview.mode === "glyphs";
  const glyphState = useGlyphTable(glyphSubject, preview.glyphSearch, store.open && glyphs);
  const [hovered, setHovered] = useState<number | null>(null);
  const [current, setCurrent] = useState<number | null>(null);
  const [announcement, setAnnouncement] = useState("");
  const focusFirstCell = useRef<(() => void) | null>(null);
  const searchNow = useRef(preview.glyphSearch);
  useEffect(() => {
    searchNow.current = preview.glyphSearch;
  });
  const { setGlyphSearch, addRecent } = preview;
  const typeIntoSearch = useCallback(
    (key: string) => {
      const before = searchNow.current;
      const next = key === "Backspace" ? [...before].slice(0, -1).join("") : before + key;
      setGlyphSearch([...next].slice(0, MAX_GLYPH_SEARCH_CHARS).join(""));
      sampleRef.current?.focus();
    },
    [setGlyphSearch],
  );
  const insertGlyph = useCallback(
    (cp: number, byKeyboard: boolean) => {
      const character = charOf(cp);
      onInsertGlyph(character, focusStaysOnCell(byKeyboard, layout.full));
      addRecent(character);
      setAnnouncement(insertedText(character));
    },
    [onInsertGlyph, addRecent, layout.full],
  );
  const searching = preview.glyphSearch.trim() !== "";
  const subjectLine = glyphs
    ? glyphSubjectLine(
        glyphSubject?.name ?? "",
        glyphState.table?.total ?? 0,
        glyphState.table?.shown ?? 0,
        searching,
        glyphState.status === "missing",
      )
    : store.listing.subject;

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
        mode={preview.mode}
        onMode={preview.setMode}
        sample={glyphs ? preview.glyphSearch : preview.sample}
        onSample={glyphs ? preview.setGlyphSearch : preview.typeSample}
        sampleRef={sampleRef}
        onFieldDown={() => focusFirstCell.current?.()}
        size={preview.size}
        onSize={preview.setSize}
        subject={subjectLine}
        canSplit={layout.canSplit}
        full={layout.full}
        onFull={onSwitchView}
        onClose={close}
        onReturnFocus={onReturnFocus}
      />
      {glyphs ? (
        <>
          <GlyphGrid
            state={glyphState}
            face={glyphSubject?.face ?? null}
            faceName={glyphSubject?.name ?? ""}
            family={glyphSubject?.family ?? ""}
            query={preview.glyphSearch}
            size={preview.size}
            recent={preview.recent}
            onInsert={insertGlyph}
            onHover={setHovered}
            onCurrent={setCurrent}
            onType={typeIntoSearch}
            focusFirstRef={focusFirstCell}
          />
          <GlyphDetailBar
            face={glyphState.status === "ready" ? (glyphSubject?.face ?? null) : null}
            cp={hovered ?? current}
            singleLine={glyphState.table?.single_line ?? false}
            onInsert={insertGlyph}
          />
          <div aria-live="polite" className="sr-only">
            {announcement}
          </div>
        </>
      ) : (
        <PreviewGrid
          listing={store.listing}
          sample={preview.sample}
          size={preview.size}
          currentFace={currentFace}
          resizing={resizing}
          onPick={onPick}
          tileFace={tileFace}
          onType={typeIntoSample}
        />
      )}
    </section>
  );
}

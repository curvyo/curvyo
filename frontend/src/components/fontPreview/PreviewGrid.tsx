// The font preview's tile grid (`specs/0077-font-preview` criteria 11, 13, 16, 17, 19 to 23):
// one tile per listed family in equal columns, virtualised by rows (the rows in view and one
// screen ahead), faces loaded after 100 ms in the page, scrolled to the top when the Fonts tab's
// filter changes; roving focus with arrows, Home, End and pages, Enter or Space to pick, Ctrl+D
// for the star, F2 and Shift+F2 for the tag line, printable characters into the sample field.
import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

import { PreviewTile } from "@/components/fontPreview/PreviewTile";
import { FontTagLine } from "@/components/panel/FontTagLine";
import { favouritesAmong } from "@/hooks/fontLibraryPage";
import { useFontTagLine } from "@/hooks/useFontTagLine";
import {
  GRID_GAP_PX,
  GRID_PAD_PX,
  gridColumns,
  rowsInPage,
  sameGridArea,
  tileColumnCss,
  tileHeight,
  tileTagRoom,
} from "@/lib/fontPreviewLayout";
import { gridMove, tileTagKey, typesIntoSample } from "@/lib/fontPreviewKeys";
import { familyKey, shownTags } from "@/lib/fontRows";
import type { PreviewFamily, PreviewListing } from "@/lib/fontPreviewStore";
import { prerenderTiles, previewTile } from "@/lib/previewFaces";
import type { FontFace as ListedFace } from "@/lib/textText";

/** A tile loads its face once it has been in the page this long, so fast scrolling loads nothing. */
const LOAD_AFTER_MS = 100;

interface PreviewGridProps {
  listing: PreviewListing;
  sample: string;
  size: number;
  /** The listed face index of the current face, for the current mark. */
  currentFace: number | null;
  onPick: (face: ListedFace, byKeyboard: boolean) => void;
  /** A printable character typed in the grid goes to the sample field. */
  onType: (character: string) => void;
  /** The divider is being dragged. */
  resizing: boolean;
}

/** The pixel size of the scroll area, measured. While the divider is dragged (`resizing`) a new
 * width is taken only when it changes the column count at `size`: the tiles stretch in CSS, so a
 * drag re-renders the grid only when a column comes or goes (customer report 2026-10-23: the drag
 * lagged). */
function useAreaSize(ref: React.RefObject<HTMLDivElement | null>, resizing: boolean, size: number) {
  const [area, setArea] = useState({ width: 480, height: 400 });
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) {
      return undefined;
    }
    const read = () => {
      const next = { width: el.clientWidth, height: el.clientHeight };
      setArea((before) => (sameGridArea(before, next, resizing, size) ? before : next));
    };
    read();
    const observer = new ResizeObserver(read);
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref, resizing, size]);
  return area;
}

export const PreviewGrid = memo(function PreviewGrid({
  listing,
  sample,
  size,
  currentFace,
  resizing,
  onPick,
  onType,
}: PreviewGridProps) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const area = useAreaSize(scrollRef, resizing, size);
  const [scrollTop, setScrollTop] = useState(0);
  const [revision, setRevision] = useState(0);
  const bump = useCallback(() => setRevision((r) => r + 1), []);
  const families = listing.families;

  const columns = gridColumns(area.width, size);
  const tile = tileHeight(size);
  const rowHeight = tile + GRID_GAP_PX;
  const rowCount = Math.ceil(families.length / columns.count);
  const page = rowsInPage(scrollTop, area.height, rowHeight, rowCount);
  const firstIndex = page.first * columns.count;
  const lastIndex = Math.min(families.length - 1, (page.last + 1) * columns.count - 1);

  const holdsCurrent = useCallback(
    (entry: PreviewFamily) => entry.family.faces.some((face) => face.index === currentFace),
    [currentFace],
  );
  const currentIndex = families.findIndex(holdsCurrent);
  const [active, setActive] = useState<number | null>(null);
  const activeIndex = active !== null && active < families.length ? active : Math.max(0, currentIndex);

  // A new strip cell or search starts at the top (criterion 16).
  useLayoutEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = 0;
    }
    setScrollTop(0);
    setActive(null);
  }, [listing.filter]);

  // Faces of tiles that stay in the page load after 100 ms (criterion 17), these first and the
  // next families after them, in the background; each drawn into the tile cache for the sample.
  const pageKey = `${listing.filter}|${firstIndex}|${lastIndex}|${families.length}`;
  useEffect(() => {
    const timer = window.setTimeout(() => {
      prerenderTiles(
        families.map((entry) => entry.family.faces[0]),
        (face) => sample || face.family,
        { first: firstIndex, last: lastIndex },
        bump,
      );
    }, LOAD_AFTER_MS);
    return () => window.clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pageKey, sample]);

  const inPage = families.slice(firstIndex, lastIndex + 1);
  const favourites = useMemo(
    () => favouritesAmong(inPage.map((entry) => entry.family.family)),
    // `revision` reads the stars again after a toggle.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [pageKey, revision],
  );

  const focusTile = (index: number) => {
    setActive(index);
    const list = scrollRef.current;
    if (list) {
      const top = GRID_PAD_PX + Math.floor(index / columns.count) * rowHeight;
      if (top < list.scrollTop) {
        list.scrollTop = top - GRID_PAD_PX;
      } else if (top + tile > list.scrollTop + list.clientHeight) {
        list.scrollTop = top + tile + GRID_PAD_PX - list.clientHeight;
      }
      setScrollTop(list.scrollTop);
    }
    window.requestAnimationFrame(() =>
      scrollRef.current?.querySelector<HTMLElement>(`[data-tile-index="${index}"]`)?.focus(),
    );
  };

  // The list's tag line logic, looked up in this grid (criterion 14). The listing's edits keep the
  // family listed until the filter changes, so no tile moves while its tags change.
  const focusFamily = (key: string) => {
    const index = families.findIndex((entry) => familyKey(entry.family) === key);
    if (index >= 0) {
      focusTile(index);
    }
  };
  const tags = useFontTagLine(
    { removeTag: (family, id) => listing.removeTag(family, id) },
    focusFamily,
    () => {},
    scrollRef,
  );
  const tagRoom = tileTagRoom(columns.width);
  /** The family key whose tile shows all its tags over the sample ("+N", the list's show all). */
  const [allTags, setAllTags] = useState<string | null>(null);
  const tagLine = (entry: PreviewFamily, index: number, wrap: boolean) => {
    const key = familyKey(entry.family);
    return (
      <FontTagLine
        family={entry.family.family}
        tags={entry.tags}
        shown={shownTags(entry.tags, tagRoom)}
        wrap={wrap}
        inset={false}
        adding={tags.adding === key}
        completions={(prefix) => listing.completions(entry.family.family, prefix)}
        onAdd={(name) => listing.addTag(entry.family.family, name)}
        onStartAdd={(byKeyboard) => tags.openAddTag(entry.family, byKeyboard)}
        onCloseAdd={() => tags.closeAddTag(entry.family)}
        onRemove={(tag, byKeyboard) => tags.removeTag(entry.family, tag, byKeyboard, entry.tags)}
        onMore={() => {
          setAllTags(key);
          window.requestAnimationFrame(() => tags.focusTagLine(entry.family));
        }}
        onLeave={() => {
          setAllTags(null);
          focusTile(index);
        }}
      />
    );
  };

  const star = (entry: PreviewFamily) => {
    listing.star(entry.family);
    bump();
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const entry = families[activeIndex];
    if (!entry) {
      return;
    }
    const pageRows = Math.max(1, Math.floor(area.height / rowHeight));
    const next = gridMove(event.key, activeIndex, families.length, columns.count, pageRows);
    if (next !== null) {
      event.preventDefault();
      focusTile(next);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onPick(entry.family.faces[0], true);
    } else if (tileTagKey(event.key, event.shiftKey) !== null && event.target === event.currentTarget.querySelector(`[data-tile-index="${activeIndex}"]`)) {
      event.preventDefault();
      if (tileTagKey(event.key, event.shiftKey) === "add") {
        tags.openAddTag(entry.family, true);
      } else {
        tags.focusTagLine(entry.family);
      }
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "d") {
      event.preventDefault();
      star(entry);
    } else if (typesIntoSample(event.key, event.ctrlKey, event.altKey, event.metaKey)) {
      event.preventDefault();
      onType(event.key);
    }
  };

  return (
    <div
      ref={scrollRef}
      role="listbox"
      aria-label="Fonts"
      aria-multiselectable="false"
      onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
      onKeyDown={onKeyDown}
      className="relative min-h-0 flex-1 overflow-y-auto bg-[var(--canvas-bg)] [scrollbar-gutter:stable]"
    >
      {families.length === 0 ? (
        <p className="pt-6 text-center text-sm text-[var(--panel-muted-fg)]">{listing.empty ?? ""}</p>
      ) : (
        <div style={{ position: "relative", height: 2 * GRID_PAD_PX + rowCount * rowHeight - GRID_GAP_PX }}>
          {inPage.map((entry, offset) => {
            const index = firstIndex + offset;
            const face = entry.family.faces[0];
            const text = sample || entry.family.family;
            const drawing = previewTile(face, text);
            const column = index % columns.count;
            return (
              <PreviewTile
                key={`${entry.family.block}:${entry.family.family}`}
                index={index}
                rowKey={familyKey(entry.family)}
                family={entry.family.family}
                text={text}
                drawing={drawing}
                size={size}
                cuttable={entry.cuttable}
                singleLine={entry.singleLine}
                favourite={favourites.has(entry.family.family)}
                current={index === currentIndex}
                focused={index === activeIndex}
                tagLineStyle={tags.adding === familyKey(entry.family) ? { width: tagRoom } : { maxWidth: tagRoom }}
                tagLine={allTags === familyKey(entry.family) ? null : tagLine(entry, index, false)}
                allTags={allTags === familyKey(entry.family) ? tagLine(entry, index, true) : null}
                onLeaveTags={() => setAllTags(null)}
                style={{
                  top: GRID_PAD_PX + Math.floor(index / columns.count) * rowHeight,
                  ...tileColumnCss(column, columns.count),
                  height: tile,
                }}
                onFocus={() => setActive(index)}
                onPick={(byKeyboard) => onPick(face, byKeyboard)}
                onStar={() => star(entry)}
              />
            );
          })}
        </div>
      )}
    </div>
  );
});

// The Fonts tab (`specs/0074-text-objects` criteria 15 to 19 and 29; `specs/0075-font-library`;
// UX notes and tags amendment): the tag strip, a search row with Add font, and one
// virtualised tree of the blocks Missing, Curvyo fonts, Included and This computer, each family
// set in its own face once it has been in view for 100 ms, with its star and its tag line.
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { FontTagStrip } from "@/components/panel/FontTagStrip";
import { TextLayoutBlock } from "@/components/panel/TextLayoutBlock";
import type { TextLayoutApi } from "@/hooks/useTextLayout";
import { FontTagLine } from "@/components/panel/FontTagLine";
import {
  FAMILY_PX,
  HEADER_PX,
  type Row,
  buildRows,
  familyKey,
  pinnedHeader,
  rowTops,
  shownTags,
  tagLineRoom,
} from "@/lib/fontRows";
import { useFontLibrary } from "@/hooks/useFontLibrary";
import { useDeleteArm } from "@/hooks/useDeleteArm";
import { useFontTagLine } from "@/hooks/useFontTagLine";
import { useFontDrop } from "@/hooks/useFontDrop";
import { isTauri } from "@/platform/host";
import { armedRemoveLabel, checkingText, emptyTagText, removedText } from "@/lib/fontLibraryText";
import { documentsGoing } from "@/lib/fontInfoText";
import { hostFolders, hostListing } from "@/hooks/useText";
import { useFontTreeScroll } from "@/hooks/useFontTreeScroll";
import { useFontPreviewState } from "@/hooks/useFontPreviewState";
import { type PreviewListing, previewState, publishListing, setPreviewOpen } from "@/lib/fontPreviewStore";
import { subjectLine } from "@/lib/fontPreviewText";
import { onFontTreeKey } from "@/lib/fontTreeKeys";
import { TabNotice } from "@/components/panel/FontTabNotice";
import { FontSearchRow } from "@/components/panel/FontSearchRow";
import { BlockHeader, FaceRow, FamilyRow, MissingRow } from "@/components/panel/FontTreeRows";
import {
  type FontFace as ListedFace,
  type FontFamily,
  type FontsTabData,
  FOCUS_CURRENT_FONT_EVENT,
  blockCount,
  fontBlocks,
} from "@/lib/textText";


interface FontsTabProps {
  data: FontsTabData;
  onPick: (face: ListedFace, byKeyboard: boolean) => void;
  /** A family row: the subject texts keep their style where the family has it
   * (`specs/0078-font-style-buttons` criterion 22); `face` is the family's Regular or first. */
  onPickFamily: (face: ListedFace, byKeyboard: boolean) => void;
  onReturnFocus: () => void;
  /** The tab is shown: the Curvyo folder is read again (criterion 19); resolves with its list. */
  onShown: () => Promise<void>;
  /** The texts of the project using a Curvyo family (`0075` criterion 25). */
  textsUsingFamily: (family: string) => number;
  /** The text layout block over the strip (`specs/0079-text-layout-controls` criterion 6). */
  layout: TextLayoutApi;
}

/** The drop outline (UX notes §4): an overlay, so the strip, the search field and the pinned
 * block headers do not paint over it. */
const DROP_OUTLINE =
  "after:pointer-events-none after:absolute after:inset-0 after:z-20 after:rounded-[5px] after:border-2 after:border-dashed after:border-[var(--editor-accent)] after:content-['']";

export function FontsTab({
  data,
  onPick,
  onPickFamily,
  onReturnFocus,
  onShown,
  textsUsingFamily,
  layout,
}: FontsTabProps) {
  const [search, setSearch] = useState("");
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  const [collapsedCurrent, setCollapsedCurrent] = useState(false);
  const [focusKey, setFocusKey] = useState<string | null>(null);
  /** Families that left the chosen cell by the maker's own press: they stay until the cell or
   * the search changes (UX notes §3). */
  const [kept, setKept] = useState<Set<string>>(() => new Set());
  const searchRef = useRef<HTMLInputElement | null>(null);
  const bodyRef = useRef<HTMLDivElement | null>(null);
  const lib = useFontLibrary(data, onShown);
  /** The one armed trash of the tab: a Curvyo family's Remove or an own tag's Delete (`0081`). */
  const arm = useDeleteArm();

  useEffect(() => {
    void onShown();
  }, [onShown]);

  const chosen = lib.chosen;
  const chosenTag = lib.tags.find((tag) => tag.id === chosen);
  const chosenName = chosen === "favourites" ? "Favourites" : (chosenTag?.name ?? chosen);
  const checking = lib.progress !== null;
  useEffect(() => {
    setKept(new Set());
  }, [chosen, search]);

  const inCell = useCallback(
    (family: FontFamily) => lib.isMember(chosen, family.family) || kept.has(family.family),
    [lib, chosen, kept],
  );
  const blocks = useMemo(
    () => fontBlocks(data, search, isTauri, chosen === "all" ? undefined : inCell, chosen === "cuttable" && checking),
    [data, search, chosen, inCell, checking],
  );
  const currentFace = data.current?.index ?? null;
  
  const currentFamily = useMemo(() => {
    for (const block of blocks) {
      for (const family of block.families) {
        if (family.faces.some((face) => face.index === currentFace)) {
          return familyKey(family);
        }
      }
    }
    return null;
  }, [blocks, currentFace]);

  /** The tree's inner width (the 244 px column less the gutter until measured). */
  const [treeWidth, setTreeWidth] = useState(236);
  const rows = useMemo(
    () =>
      buildRows({
        data,
        blocks,
        isOpen: (key) => expanded.has(key) || (key === currentFamily && !collapsedCurrent && !search),
        tags: (family) => lib.tagsOf(family),
        treeWidth,
      }),
    // `lib.tags` changes with every library change, so the tag lines are read again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [blocks, data, expanded, currentFamily, collapsedCurrent, search, lib.tags, treeWidth],
  );

  const tops = useMemo(() => rowTops(rows), [rows]);
  const { setList, height, scrollTop, visible, onScroll, focusRow, reveal } = useFontTreeScroll(
    rows,
    tops,
    `${chosen}|${search}`,
    setFocusKey,
    setTreeWidth,
  );




  const focusable = rows.filter((row) => row.kind !== "header");
  const activeKey =
    focusKey && focusable.some((row) => row.key === focusKey)
      ? focusKey
      : (rows.find((row) => row.kind === "face" && row.face.index === currentFace)?.key ?? focusable[0]?.key ?? null);



  // The Font button and Shift+Ctrl+T focus the current face row.
  useEffect(() => {
    const onFocusCurrent = () => {
      const current = rows.find((row) => row.kind === "face" && row.face.index === currentFace);
      const missing = rows.find((row) => row.kind === "missing");
      const target = missing ?? current ?? focusable[0];
      if (target) {
        focusRow(target.key);
      }
    };
    window.addEventListener(FOCUS_CURRENT_FONT_EVENT, onFocusCurrent);
    return () => window.removeEventListener(FOCUS_CURRENT_FONT_EVENT, onFocusCurrent);
  }, [rows, currentFace, focusable, focusRow]);

  const toggle = (family: FontFamily, open?: boolean) => {
    const key = familyKey(family);
    if (key === currentFamily && !expanded.has(key)) {
      setCollapsedCurrent((collapsed) => (open === undefined ? !collapsed : !open));
      return;
    }
    setExpanded((before) => {
      const next = new Set(before);
      const willOpen = open ?? !next.has(key);
      if (willOpen) {
        next.add(key);
      } else {
        next.delete(key);
      }
      return next;
    });
  };

  const pickFamily = (family: FontFamily, byKeyboard: boolean) => {
    toggle(family, true);
    onPickFamily(family.faces[0], byKeyboard);
  };

  /** A maker's press keeps the row in view until the strip cell or the search changes. */
  const keep = (family: string) => setKept((before) => new Set(before).add(family));

  const star = (family: FontFamily) => {
    keep(family.family);
    lib.toggleFavourite(family.family);
  };

  // The list changing under an armed trash disarms it (`0081` criterion 5).
  const { disarm } = arm;
  useEffect(disarm, [disarm, search, chosen, expanded, collapsedCurrent]);

  /** The documents a Remove of `family` takes with an emptied font folder (`0083` criterion 14). */
  const documentsOf = (family: FontFamily) => documentsGoing(family.family, hostListing("curvyo"), hostFolders());

  /** A press on a Curvyo family's trash: arms it, or removes the family when armed. */
  const pressRemove = (family: FontFamily, byKeyboard: boolean) => {
    const key = familyKey(family);
    const label = armedRemoveLabel(family.family, textsUsingFamily(family.family), documentsOf(family));
    arm.press(`font:${key}`, label, () => {
      const at = focusable.findIndex((r) => r.key === key);
      const after = focusable.slice(at + 1).find((r) => r.kind === "family");
      const before = focusable.slice(0, at).reverse().find((r) => r.kind === "family");
      // By mouse the focus stays, unless it was inside the removed row (never to `body`).
      const inRow = document.activeElement?.closest(`[data-arm-zone="${CSS.escape(`font:${key}`)}"]`);
      void lib.removeFamily(family).then(() => {
        const target = after ?? before;
        if (target && (byKeyboard || inRow)) {
          focusRow(target.key);
        }
      });
      return removedText(family.family);
    });
  };

  const addFonts = async (dropped: { paths?: string[]; files?: File[] } | undefined, byKeyboard: boolean) => {
    const family = await lib.addFonts(dropped);
    if (!family) {
      return;
    }
    const needle = search.trim().toLowerCase();
    // An own tag or Initials chosen in the strip went on the new families (criterion 23).
    const tagged = chosenTag !== undefined && (!chosenTag.builtin || chosenTag.id === "initials");
    const inChosen = chosen === "all" || tagged || lib.isMember(chosen, family);
    const shown = inChosen && (!needle || family.toLowerCase().includes(needle));
    if (!shown) {
      lib.choose("all");
      setSearch("");
    }
    const key = `curvyo:${family}`;
    setExpanded((before) => new Set(before).add(key));
    reveal(key, byKeyboard);
  };
  const addFontsRef = useRef(addFonts);
  addFontsRef.current = addFonts;
  const onDrop = useCallback((dropped: { paths?: string[]; files?: File[] }) => {
    void addFontsRef.current(dropped, false);
  }, []);
  const drop = useFontDrop(bodyRef, onDrop);

  const { adding, openAddTag, closeAddTag, focusTagLine, removeTag } = useFontTagLine(lib, focusRow, keep, bodyRef);

  const onListKey = (event: React.KeyboardEvent<HTMLDivElement>) =>
    onFontTreeKey(event, {
      rows,
      focusable,
      activeKey,
      pageRows: Math.max(1, Math.floor(height / FAMILY_PX) - 1),
      focusRow,
      toggle,
      pickFamily,
      pickFace: (face) => onPick(face, true),
      star,
      pressRemove: (family) => pressRemove(family, true),
      isArmed: (family) => arm.armed === `font:${familyKey(family)}`,
      openAddTag: (family) => openAddTag(family, true),
      focusTagLine,
      returnFocus: onReturnFocus,
      typeInSearch: (character) => {
        setSearch((text) => text + character);
        searchRef.current?.focus();
      },
    });

  /** The trash of a Curvyo family row, idle or armed (`0081` criteria 1 and 3). */
  const trashOf = (family: FontFamily) => {
    const target = `font:${familyKey(family)}`;
    const armed = arm.armed === target;
    const label = armed
      ? armedRemoveLabel(family.family, textsUsingFamily(family.family), documentsOf(family))
      : null;
    return {
      target,
      armed,
      label: label ?? `Remove ${family.family} from Curvyo fonts`,
      tip: label ?? "Remove from Curvyo fonts",
      onPress: () => pressRemove(family, false),
    };
  };

  const searching = search.trim().length > 0;
  const filtered = searching || chosen !== "all";
  const skipped = data.skipped.length;
  const headerRow = (row: Extract<Row, { kind: "header" }>, style: React.CSSProperties) => (
    <BlockHeader
      key={row.key}
      title={row.block.title}
      count={
        row.key === "missing-header"
          ? ""
          : row.block.block === "system" && isTauri && !data.systemRead
            ? "Reading…"
            : row.block.block === "system" && lib.progress
              ? checkingText(lib.progress.done, lib.progress.total)
              : blockCount(row.block, filtered, skipped)
      }
      tip={row.block.block === "curvyo" && skipped > 0 ? data.skipped.slice(0, 10).join("\n") : undefined}
      style={style}
    />
  );
  const pinned = pinnedHeader(rows, tops, scrollTop);
  const firstFamily = blocks.flatMap((block) => block.families)[0];
  const shownFamilies = blocks.reduce((sum, block) => sum + block.families.length, 0);

  // The preview shows the same families (`specs/0077-font-preview` criteria 16 and 26).
  const preview = useFontPreviewState();
  const starRef = useRef(star);
  starRef.current = star;
  const tagEditsRef = useRef({ lib, keep });
  tagEditsRef.current = { lib, keep };
  const total = useMemo(
    () =>
      searching
        ? fontBlocks(data, "", isTauri, chosen === "all" ? undefined : inCell).reduce((sum, b) => sum + b.families.length, 0)
        : shownFamilies,
    [searching, data, chosen, inCell, shownFamilies],
  );
  useEffect(() => {
    const member = (id: string) => lib.tags.find((tag) => tag.id === id)?.members;
    const cuttable = member("cuttable");
    const singleLine = member("single_line");
    const empty =
      shownFamilies > 0
        ? null
        : data.faces.length === 0 && !(isTauri && !data.systemRead)
          ? "Loading fonts…"
          : chosen !== "all"
            ? emptyTagText(chosenName, search, chosen === "cuttable" && checking)
            : searching
              ? `No font matches "${search.trim()}".`
              : "Loading fonts…";
    const next: PreviewListing = {
      families: blocks
        .flatMap((block) => block.families)
        .map((family) => ({
          family,
          tags: tagEditsRef.current.lib.tagsOf(family.family),
          cuttable: cuttable?.has(family.family) ?? false,
          singleLine: singleLine?.has(family.family) ?? false,
        })),
      subject: subjectLine(chosen === "all" ? "All" : chosenName, shownFamilies, total, searching),
      empty,
      filter: `${chosen}|${search}`,
      star: (family) => starRef.current(family),
      addTag: (family, name) => {
        tagEditsRef.current.keep(family);
        return tagEditsRef.current.lib.addTag(family, name);
      },
      removeTag: (family, id) => {
        tagEditsRef.current.keep(family);
        tagEditsRef.current.lib.removeTag(family, id);
      },
      completions: (family, prefix) => tagEditsRef.current.lib.completions(family, prefix),
    };
    if (!sameListing(previewState().listing, next)) {
      publishListing(next);
    }
  }, [blocks, lib.tags, chosen, chosenName, search, searching, shownFamilies, total, checking, data]);

  const list =
    data.faces.length === 0 && !(isTauri && !data.systemRead) ? (
      <p className="text-sm text-[var(--toolbar-icon)]">Loading fonts…</p>
    ) : shownFamilies === 0 && chosen !== "all" ? (
      <div>
        {chosen === "cuttable" && lib.progress ? (
          // The run's count stays visible while nothing was found yet (UX notes §5).
          <BlockHeader
            title="This computer"
            count={checkingText(lib.progress.done, lib.progress.total)}
            style={{ height: HEADER_PX }}
          />
        ) : null}
        <p className="text-sm text-[var(--toolbar-icon)]">
          {emptyTagText(chosenName, search, chosen === "cuttable" && checking)}
        </p>
      </div>
    ) : searching && blocks.length === 0 ? (
      <p className="text-sm text-[var(--toolbar-icon)]">No font matches "{search.trim()}".</p>
    ) : (
      <div
        ref={setList}
        role="tree"
        aria-label="Fonts"
        onScroll={onScroll}
        onKeyDown={onListKey}
        className="relative min-h-[120px] flex-1 overflow-y-auto [scrollbar-gutter:stable]"
      >
        {pinned ? (
          // The header of the block at the scroll top stays pinned while its rows scroll
          // (UX notes §4); the next header pushes it up.
          <div
            aria-hidden
            className="sticky top-0 z-10 mr-4 -mb-6 h-6"
            style={{ transform: pinned.push < 0 ? `translateY(${pinned.push}px)` : undefined }}
          >
            {headerRow(pinned.row, { position: "absolute", top: 0, left: 0, right: 0, height: HEADER_PX })}
          </div>
        ) : null}
        <div style={{ height: tops.total, position: "relative", marginRight: 16 }}>
          {visible.map((i) => {
            const row = rows[i];
            const style = { position: "absolute" as const, top: tops.at[i], left: 0, right: 0, height: row.height };
            if (row.kind === "header") {
              return headerRow(row, style);
            }
            const focused = row.key === activeKey;
            if (row.kind === "missing") {
              return (
                <MissingRow
                  key={row.key}
                  rowKey={row.key}
                  name={row.name}
                  focused={focused}
                  style={style}
                  onFocus={() => setFocusKey(row.key)}
                />
              );
            }
            if (row.kind === "family") {
              return (
                <FamilyRow
                  key={row.key}
                  rowKey={row.key}
                  family={row.family}
                  open={rows[i + 1]?.kind === "face"}
                  focused={focused}
                  holdsCurrent={row.family.faces.some((f) => f.index === currentFace)}
                  favourite={lib.isFavourite(row.family.family)}
                  tagNames={row.tags.map((tag) => tag.name)}
                  tagLine={
                    <FontTagLine
                      family={row.family.family}
                      tags={row.tags}
                      shown={shownTags(row.tags, tagLineRoom(treeWidth))}
                      wrap={row.open}
                      adding={adding === row.key}
                      completions={(prefix) => lib.completions(row.family.family, prefix)}
                      onAdd={(name) => {
                        keep(row.family.family);
                        return lib.addTag(row.family.family, name);
                      }}
                      onStartAdd={(byKeyboard) => openAddTag(row.family, byKeyboard)}
                      onCloseAdd={() => closeAddTag(row.family)}
                      onRemove={(tag, byKeyboard) => removeTag(row.family, tag, byKeyboard, row.tags)}
                      onMore={() => toggle(row.family, true)}
                      onLeave={() => focusRow(row.key)}
                    />
                  }
                  zone={row.family.block === "curvyo" ? `font:${row.key}` : undefined}
                  trash={row.family.block === "curvyo" ? trashOf(row.family) : undefined}
                  notice={<TabNotice notice={lib.notice} anchor="row" row={row.key} rowHeight={row.height} />}
                  style={style}
                  onFocus={() => setFocusKey(row.key)}
                  onPick={(byKeyboard) => pickFamily(row.family, byKeyboard)}
                  onToggle={() => toggle(row.family)}
                  onStar={() => star(row.family)}
                />
              );
            }
            return (
              <FaceRow
                key={row.key}
                rowKey={row.key}
                zone={row.family.block === "curvyo" ? `font:${familyKey(row.family)}` : undefined}
                face={row.face}
                current={row.face.index === currentFace}
                focused={focused}
                style={style}
                onFocus={() => setFocusKey(row.key)}
                onPick={(byKeyboard) => onPick(row.face, byKeyboard)}
              />
            );
          })}
        </div>
      </div>
    );



  return (
    <div
      ref={bodyRef}
      {...drop.handlers}
      className={`relative flex min-h-0 flex-1 flex-col ${drop.over ? DROP_OUTLINE : ""}`}
    >
      <p role="status" aria-live="polite" className="sr-only">
        {arm.status}
      </p>
      {layout.fields ? (
        <TextLayoutBlock layout={layout} fields={layout.fields} onReturnFocus={onReturnFocus} />
      ) : null}
      <div className="relative">
        <FontTagStrip
          tags={lib.tags}
          chosen={chosen}
          progress={lib.progress}
          onChoose={(id) => lib.choose(id)}
          onRename={lib.renameTag}
          onDelete={lib.deleteTag}
          arm={arm}
          onReturnFocus={onReturnFocus}
        />
        <TabNotice notice={lib.notice} anchor="strip" />
      </div>
      <>
          <FontSearchRow
            searchRef={searchRef}
            search={search}
            onSearch={setSearch}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown" && focusable[0]) {
                event.preventDefault();
                focusRow(focusable[0].key);
              } else if (event.key === "Enter" && firstFamily) {
                event.preventDefault();
                pickFamily(firstFamily, false);
              } else if (event.key === "Escape") {
                event.preventDefault();
                event.stopPropagation();
                if (search) {
                  setSearch("");
                } else {
                  onReturnFocus();
                }
              }
            }}
            busy={lib.busy}
            previewOpen={preview.open}
            onPreview={(byKeyboard) => {
              setPreviewOpen(!preview.open, byKeyboard);
              if (!byKeyboard) {
                onReturnFocus();
              }
            }}
            dropOver={drop.over}
            notice={lib.notice}
            onAdd={(byKeyboard) => void addFonts(undefined, byKeyboard)}
          />
          {list}
      </>
    </div>
  );
}

/** Whether two listings show the same tiles, words and filter (the star is always the latest). */
function sameListing(a: PreviewListing, b: PreviewListing): boolean {
  return (
    a.subject === b.subject &&
    a.empty === b.empty &&
    a.filter === b.filter &&
    a.families.length === b.families.length &&
    a.families.every(
      (entry, at) =>
        sameFamily(entry.family, b.families[at].family) &&
        entry.tags.map((tag) => `${tag.id}:${tag.source}`).join("|") ===
          b.families[at].tags.map((tag) => `${tag.id}:${tag.source}`).join("|"),
    )
  );
}

/** The same family with the same face for its tile. */
function sameFamily(a: FontFamily, b: FontFamily): boolean {
  return (
    familyKey(a) === familyKey(b) &&
    a.faces[0]?.id === b.faces[0]?.id &&
    a.faces[0]?.index === b.faces[0]?.index &&
    a.faces[0]?.loaded === b.faces[0]?.loaded
  );
}

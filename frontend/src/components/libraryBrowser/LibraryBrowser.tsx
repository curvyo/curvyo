// The library browser (`specs/0152-library-browser`, ADR 0086, UX notes): the canvas area's side
// pane with two views, Items (big previews of everything the Library tab holds) and Sets (one card
// per set, catalogue, folder or source), filters as strips and lists, the fuzzy search that runs in
// the page, a history of places with Back, and the detail bar. A press selects; a double-click,
// Enter or a drag loads an item; a double-click or Enter opens a card (Part G). The search gains
// the keyword file on the first text (`specs/0153-library-keywords`).
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { BrowserDetailBar } from "@/components/libraryBrowser/BrowserDetailBar";
import { BrowserFacets } from "@/components/libraryBrowser/BrowserFacets";
import { BrowserGrid, type GridHandle } from "@/components/libraryBrowser/BrowserGrid";
import { BrowserHeader } from "@/components/libraryBrowser/BrowserHeader";
import { GuideView } from "@/components/libraryBrowser/GuideView";
import { KeywordLicencesView } from "@/components/libraryBrowser/KeywordLicencesView";
import { SetsGrid, type SetsHandle } from "@/components/libraryBrowser/SetsGrid";
import type { LibraryBrowserApi } from "@/hooks/useLibraryBrowser";
import type { PaneLayout } from "@/lib/fontPreviewLayout";
import { FACET_COLUMN_PX, isWide } from "@/lib/libraryBrowserLayout";
import {
  type BrowserQuery,
  type CardWords,
  type Place,
  type TileWords,
  cardWords,
  narrowedItems,
  runQuery,
  runSetsQuery,
  tileWords,
} from "@/lib/libraryBrowserQuery";
import type { BrowserSort } from "@/lib/libraryBrowserState";
import {
  BROWSE_BUTTON_ID,
  type SetsChoice,
  backName,
  browserFetched,
  closeBrowser,
  escapeGoesBack,
  goBack,
  guideOpened,
  placeApplied,
  pushPlace,
  refreshItems,
  searchFocused,
  setBrowserSession,
  tileFocused,
  useBrowserStore,
} from "@/lib/libraryBrowserStore";
import { watchCanvasDrag } from "@/lib/libraryDrag";
import { useKeywords, wantKeywords, whyMatched } from "@/lib/libraryKeywords";
import { type LoadContext, importIcon, importPage, loadTemplate } from "@/lib/libraryLoad";
import { openPreview } from "@/lib/libraryStore";
import {
  addReadySource,
  beginOnlineSearch,
  openLicence,
  openOnlineList,
  openOnlineSet,
  searchOnlineSource,
} from "@/lib/onlineApi";
import { isTauri } from "@/platform/host";
import { backApplies, isBackKey, isTextField } from "@/lib/browserBack";

interface LibraryBrowserProps extends LoadContext {
  api: LibraryBrowserApi;
  layout: PaneLayout;
  onSwitchView: () => void;
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  /** After a mouse load and on close, the keyboard goes back to the canvas. */
  onReturnFocus: () => void;
}

/** The catalogue's or source's name for Came from: the first part of the place. */
const sourceName = (words: TileWords) => words.place.split(" › ")[0] ?? words.place;

const EMPTY_SETS: SetsChoice = { text: "", sources: [], categories: [], colour: "all" };

export function LibraryBrowser({ api, layout, onSwitchView, canvasRef, onReturnFocus, ...ctx }: LibraryBrowserProps) {
  const store = useBrowserStore();
  const { session } = store;
  const { prefs, keep } = api;
  const wide = isWide(layout.width);
  const view = session.guide ? "guide" : session.licences ? "licences" : prefs.view;
  /** The guide or the keyword licences stand in for the views. */
  const standIn = view === "guide" || view === "licences";
  const keywords = useKeywords();
  const searchRef = useRef<HTMLInputElement>(null);
  const gridRef = useRef<GridHandle>(null);
  const setsRef = useRef<SetsHandle>(null);
  const [textSort, setTextSort] = useState<BrowserSort | null>(null);
  const [hovered, setHovered] = useState<number | null>(null);
  const [busyCards, setBusyCards] = useState<ReadonlySet<number>>(new Set());
  const [announce, setAnnounce] = useState("");
  const searchTicket = useRef(0);
  /** After a place change: what to scroll to and focus once the new view is drawn. */
  /** What Back still has to do once the place is drawn; `drawn` is false until the render that
   * shows it (the effect that sets it runs in the same flush as the one that reads it). */
  const settle = useRef<{ anchor: number | null; focus: number | null; drawn: boolean } | null>(null);

  const searching = prefs.text.trim() !== "";
  const sort: BrowserSort = searching ? (textSort ?? "best") : prefs.sort;
  const query: BrowserQuery = {
    text: prefs.text,
    sort,
    own_sort: prefs.sort,
    sources: session.sources,
    categories: session.categories,
    colour: prefs.colour,
    customizable: prefs.customizable,
    level: session.level,
    narrowed: session.narrowed,
  };
  const queryKey = JSON.stringify(query);
  const { answer, order } = useMemo(
    () => runQuery(query),
    // The query's text, the list's version and the keyword file decide the answer.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [queryKey, store.version, keywords.version],
  );
  // The keyword file is fetched on the first search text, and after a failure again when the
  // field next goes from empty to text (ADR 0087 decision 5, UX K3).
  useEffect(() => {
    if (searching && view === "items") {
      wantKeywords();
    }
  }, [searching, view]);
  const setsKey = JSON.stringify(session.sets);
  const sets = useMemo(
    () => runSetsQuery(session.sets),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [setsKey, store.version],
  );
  const levelKey = session.level ?? -1;
  const current = session.current.get(levelKey) ?? null;
  const shownIndex = hovered ?? (view === "sets" ? session.setsCurrent : current);
  const shownWords = useMemo(
    () => (view !== "items" || shownIndex === null ? null : (tileWords([shownIndex])[0] ?? null)),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [view, shownIndex, store.version],
  );
  const shownCard = useMemo(
    () => (view !== "sets" || shownIndex === null ? null : (cardWords([shownIndex])[0] ?? null)),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [view, shownIndex, store.version],
  );
  // Why the shown and the current item matched (criterion 14), one call each, not per keystroke.
  const shownWhy = useMemo(
    () => (view === "items" && searching ? whyMatched(shownIndex, prefs.text) : null),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [view, searching, shownIndex, prefs.text, store.version, keywords.version],
  );
  const currentWhy = useMemo(
    () => (view === "items" && searching ? (whyMatched(current, prefs.text)?.line ?? "") : ""),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [view, searching, current, prefs.text, store.version, keywords.version],
  );
  const back = backName();
  void store.history;

  // Back from anywhere in the app while the pane is open, except a text field outside it
  // (criterion 30): Alt+Left, Cmd+[ and the mouse's back button.
  useEffect(() => {
    const inPane = (target: EventTarget | null) =>
      target instanceof Element && target.closest("[data-library-browser]") !== null;
    const onKey = (event: KeyboardEvent) => {
      if (isBackKey(event) && backApplies(true, inPane(event.target), isTextField(event.target))) {
        event.preventDefault();
        event.stopPropagation();
        goBack();
      }
    };
    const onMouse = (event: MouseEvent) => {
      if (event.button === 3 && backApplies(true, inPane(event.target), isTextField(event.target))) {
        event.preventDefault();
        goBack();
      }
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mouseup", onMouse, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mouseup", onMouse, true);
    };
  }, []);

  useEffect(() => {
    if (store.focusSearch) {
      searchRef.current?.focus();
      searchFocused();
    }
  }, [store.focusSearch]);

  /** The place shown now (criterion 30). */
  const snapshot = (opened: number | null): string => {
    const place: Place = {
      view,
      items: query,
      sets: session.sets,
      current: view === "sets" ? session.setsCurrent : current,
      anchor: view === "sets" ? (setsRef.current?.anchor() ?? null) : view === "items" ? (gridRef.current?.anchor() ?? null) : null,
      opened,
    };
    return JSON.stringify(place);
  };

  const applyItems = (items: BrowserQuery) => {
    keep({
      view: "items",
      text: items.text,
      colour: items.colour,
      sort: items.own_sort,
    });
    setTextSort(items.text.trim() && items.sort !== "best" ? items.sort : null);
  };

  // Back brought a place: everything that makes it comes back (UX notes 0.3).
  useEffect(() => {
    if (store.pendingPlace === null) {
      return;
    }
    const place = JSON.parse(store.pendingPlace) as Place;
    placeApplied();
    applyItems(place.items);
    const level = place.items.level ?? -1;
    setHovered(null);
    setBrowserSession({
      guide: place.view === "guide",
      licences: place.view === "licences",
      sources: place.items.sources,
      categories: place.items.categories,
      level: place.items.level,
      narrowed: place.items.narrowed,
      current:
        place.view === "items" && place.current !== null
          ? new Map(session.current).set(level, place.current)
          : session.current,
      sets: place.sets,
      setsCurrent: place.view === "sets" ? place.current : session.setsCurrent,
    });
    if (place.view === "sets") {
      keep({ view: "sets" });
    }
    settle.current = { anchor: place.anchor, focus: place.opened ?? place.current, drawn: false };
    // Every place change is applied in one go; the effect reads the session of this render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [store.pendingPlace]);

  // Once the place is drawn: its scroll anchor, then the focus on what was opened from it.
  useEffect(() => {
    const target = settle.current;
    if (!target) {
      if (store.focusTile) {
        tileFocused();
      }
      return;
    }
    if (!target.drawn) {
      // This flush still shows the place that was left; the next render shows the new one.
      target.drawn = true;
      return;
    }
    settle.current = null;
    tileFocused();
    if (view === "sets") {
      if (target.anchor !== null) {
        setsRef.current?.reveal(target.anchor);
      }
      setsRef.current?.focusCard(target.focus);
    } else if (view === "items") {
      if (target.anchor !== null) {
        gridRef.current?.reveal(target.anchor);
      }
      gridRef.current?.focusItem(target.focus);
    }
  });

  /** Leaves this place for another: it goes on the history (criterion 30). */
  const leave = (opened: number | null, byOpening: boolean) => {
    pushPlace(snapshot(opened), byOpening);
    setHovered(null);
  };

  // The hosting guide asked for from the Add source form opens as a place.
  useEffect(() => {
    if (store.pendingGuide) {
      guideOpened();
      if (!session.guide) {
        leave(null, true);
        setBrowserSession({ guide: true });
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [store.pendingGuide]);

  const setText = (text: string) => {
    if (view === "sets") {
      setBrowserSession({ sets: { ...session.sets, text } });
      return;
    }
    if (!prefs.text.trim() && text.trim()) {
      setTextSort(null);
    }
    keep({ text });
  };
  const select = (index: number) => setBrowserSession({ current: new Map(session.current).set(levelKey, index) });

  const close = useCallback(() => {
    const inside = document.activeElement?.closest("[data-library-browser]");
    closeBrowser();
    if (inside) {
      const browse = document.getElementById(BROWSE_BUTTON_ID);
      if (browse) {
        browse.focus();
      } else {
        onReturnFocus();
      }
    }
  }, [onReturnFocus]);

  const switchView = (next: "items" | "sets") => {
    if (next === view) {
      return;
    }
    leave(null, false);
    keep({ view: next });
    setBrowserSession({ guide: false, licences: false });
  };

  const load = async (words: TileWords, byKeyboard: boolean, drop: { x: number; y: number } | null = null) => {
    if (words.refusal) {
      return;
    }
    let done = false;
    let name = words.name;
    if (words.kind === "icon") {
      done = await importIcon(ctx, words.host, words.prefix, words.name, sourceName(words), drop);
    } else if (words.kind === "template") {
      // A template loads all its pages, also one with several (UX U46).
      const loaded = await loadTemplate(ctx, words.host, words.path, words.meta);
      done = loaded !== null;
      name = loaded ?? name;
      browserFetched();
    } else if (words.kind === "page") {
      done = await importPage(
        ctx,
        {
          source: words.host,
          path: words.path,
          pageId: words.page_id,
          pageName: words.page_name,
          documentName: words.document,
          sourceName: sourceName(words),
        },
        drop,
      );
    }
    if (done) {
      setAnnounce(`Loaded ${name}`);
      if (!byKeyboard) {
        onReturnFocus();
      }
    }
  };

  const showPages = (index: number) => {
    leave(index, true);
    setBrowserSession({ level: index });
    window.setTimeout(() => gridRef.current?.focusFirst(), 0);
  };

  /** A double-click or Enter on a tile: a multi-page document opens its pages, else it loads. */
  const activate = (index: number, byKeyboard: boolean) => {
    const words = tileWords([index])[0];
    if (!words) {
      return;
    }
    select(index);
    if (words.kind === "document") {
      showPages(index);
    } else {
      void load(words, byKeyboard);
    }
  };

  const action = (name: TileWords["actions"][number], words: TileWords) => {
    if (name === "show_pages") {
      showPages(words.index);
      return;
    }
    leave(words.index, true);
    void openPreview({
      source: words.host,
      path: words.path,
      pageId: words.page_id,
      pageName: words.page_name || words.name,
      chip: "",
      subject: words.place,
      documentName: words.document,
      back: true,
    });
  };

  /** Opening a card from the Sets view: Items on it alone (criterion 27). */
  const narrowTo = (opened: number, key: string) => {
    leave(opened, true);
    const items = narrowedItems(key);
    applyItems(items);
    setBrowserSession({ guide: false, licences: false, sources: items.sources, categories: [], level: null, narrowed: items.narrowed });
    // The opened card leaves the page: the keyboard goes on in the new place (Escape, Alt+Left).
    window.setTimeout(() => gridRef.current?.focusFirst(), 0);
  };

  /** A double-click or Enter on a card is its one request (criterion 28). */
  const openCard = (card: CardWords) => {
    const busy = (on: boolean) =>
      setBusyCards((now) => {
        const next = new Set(now);
        if (on) {
          next.add(card.index);
        } else {
          next.delete(card.index);
        }
        return next;
      });
    const after = (then: () => void) => () => {
      busy(false);
      browserFetched();
      then();
    };
    setBrowserSession({ setsCurrent: card.index });
    if (card.kind === "ready") {
      busy(true);
      void addReadySource().then((answer) => {
        if (answer.kind !== "added") {
          busy(false);
          return;
        }
        void openOnlineList(answer.key, false).then(
          after(() => narrowTo(card.index, answer.key)),
          after(() => undefined),
        );
      }, () => busy(false));
    } else if (card.kind === "set" && !card.loaded) {
      busy(true);
      void openOnlineSet(card.key, card.prefix).then(
        after(() => {
          setAnnounce(`${card.name}: loaded`);
          narrowTo(card.index, card.narrow);
        }),
        after(() => undefined),
      );
    } else if (card.kind === "source") {
      busy(true);
      // An Iconify source's set cards replace it in place; a catalogue opens on its templates.
      void openOnlineList(card.key, false).then(
        after(() =>
          card.narrow
            ? narrowTo(card.index, card.narrow)
            : // The source's set cards take its place; the first of them takes the focus.
              window.setTimeout(() => setsRef.current?.focusCard(card.index), 0),
        ),
        after(() => undefined),
      );
    } else if (card.narrow) {
      narrowTo(card.index, card.narrow);
    }
  };

  // Enter also sends the words to the added Iconify sources (Frage 340 B), only in the Items view
  // (H9); typing sends nothing.
  const searchOnline =
    isTauri && store.hasIconify && view === "items"
      ? () => {
          const words = prefs.text.trim();
          if (!words) {
            return;
          }
          const mine = ++searchTicket.current;
          void beginOnlineSearch(prefs.text).then((begun) => {
            if (mine !== searchTicket.current || !begun) {
              return;
            }
            void refreshItems();
            for (const key of begun.keys) {
              // The outcome is announced once, by the subject line's status region.
              void searchOnlineSource(key, begun.words).then(() => {
                if (mine === searchTicket.current) {
                  browserFetched();
                }
              }, () => undefined);
            }
          }, () => undefined);
        }
      : null;

  const dragStart = (event: React.PointerEvent<HTMLDivElement>, words: TileWords) => {
    // The drag of the tab's row, split view only (the drawing is hidden in full view).
    if (layout.full || event.button !== 0 || words.refusal || words.kind === "document") {
      return;
    }
    const picture = event.currentTarget.querySelector<HTMLImageElement>("img")?.src ?? null;
    watchCanvasDrag(
      event,
      () => canvasRef.current,
      words.name,
      () => (words.kind === "icon" ? "copy" : "import"),
      (at) => void load(words, false, words.kind === "template" ? null : at),
      picture,
    );
  };

  const clearAll = () => {
    if (view === "sets") {
      setBrowserSession({ sets: EMPTY_SETS });
    } else {
      keep({ text: "", colour: "all", customizable: false });
      setBrowserSession({ sources: [], categories: [], narrowed: false });
    }
    // Both Clear all buttons leave (the empty state's) or hide: the keyboard goes to the search.
    searchRef.current?.focus();
  };
  const facetsActive = view === "sets" ? sets.active : answer.active;
  const canClear = (view === "sets" ? session.sets.text.trim() !== "" : searching) || facetsActive > 0;

  const facets =
    standIn ? null : view === "sets" ? (
      <BrowserFacets
        answer={sets}
        sets
        searching={session.sets.text.trim() !== ""}
        sort="name"
        onSort={() => undefined}
        sources={session.sets.sources}
        onSources={(sources) => setBrowserSession({ sets: { ...session.sets, sources } })}
        colour={session.sets.colour}
        onColour={(colour) => setBrowserSession({ sets: { ...session.sets, colour } })}
        customizable={false}
        onCustomizable={() => undefined}
        categories={session.sets.categories}
        onCategories={(categories) => setBrowserSession({ sets: { ...session.sets, categories } })}
        canClear={canClear}
        onClearAll={clearAll}
      />
    ) : (
      <BrowserFacets
        answer={answer}
        sets={false}
        searching={searching}
        sort={sort}
        onSort={(next) => (searching ? setTextSort(next) : next !== "best" && keep({ sort: next }))}
        sources={session.sources}
        onSources={(sources) => setBrowserSession({ sources, narrowed: false })}
        colour={prefs.colour}
        onColour={(colour) => keep({ colour })}
        customizable={prefs.customizable}
        onCustomizable={(customizable) => keep({ customizable })}
        categories={session.categories}
        onCategories={(categories) => setBrowserSession({ categories })}
        canClear={canClear}
        onClearAll={clearAll}
        onKeywordLicences={() => {
          leave(null, true);
          setBrowserSession({ licences: true });
        }}
      />
    );

  const goBackNow = () => {
    goBack();
  };

  const subject =
    view === "guide"
      ? "Host your own library"
      : view === "licences"
        ? "Search keyword licences"
        : view === "sets"
          ? sets.subject
          : keywords.slow && searching && !answer.subject.includes(" · ")
            ? `${answer.subject} · Loading keywords…`
            : answer.subject;
  const itemsEmpty =
    // The browser build lists its own documents only (`0137` UX notes 2.1).
    !isTauri && answer.empty === "Nothing in the library yet." ? "No documents in this browser." : answer.empty;

  return (
    <section
      data-library-browser=""
      role="region"
      aria-label="Library browser"
      className="flex min-h-0 flex-1 flex-col"
      onKeyDown={(event) => {
        // Back (Alt+Left, Cmd+[, mouse back) listens on the window, below.
        if (event.key === "Escape") {
          // Escape goes Back while the newest place was left by opening something (H6).
          event.stopPropagation();
          if (escapeGoesBack()) {
            goBackNow();
          } else {
            close();
          }
        }
      }}
    >
      <span className="sr-only" role="status">
        {announce}
      </span>
      <BrowserHeader
        wide={wide}
        view={view === "licences" ? "guide" : view}
        onView={switchView}
        backName={back}
        onBack={goBackNow}
        text={view === "sets" ? session.sets.text : prefs.text}
        onText={setText}
        searchRef={searchRef}
        onFieldDown={() => (view === "sets" ? setsRef.current?.focusFirst() : gridRef.current?.focusFirst())}
        onSearchOnline={searchOnline}
        size={prefs.size}
        onSize={(size) => keep({ size })}
        subject={subject}
        canSplit={layout.canSplit}
        full={layout.full}
        onFull={onSwitchView}
        onClose={close}
        filtersOpen={prefs.filtersOpen}
        onFilters={() => keep({ filtersOpen: !prefs.filtersOpen })}
        activeFilters={facetsActive}
        onReturnFocus={() => searchRef.current?.focus()}
      />
      {!wide && prefs.filtersOpen && facets ? (
        <div
          id="library-browser-filters"
          className="max-h-[50%] shrink-0 overflow-y-auto border-b border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] bg-[var(--panel-bg)] p-3 [overscroll-behavior:contain]"
        >
          {facets}
        </div>
      ) : null}
      <div className="flex min-h-0 flex-1">
        {wide && facets ? (
          <div
            className="shrink-0 overflow-y-auto border-r border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] bg-[var(--panel-bg)] p-3 [overscroll-behavior:contain]"
            style={{ width: FACET_COLUMN_PX }}
          >
            {facets}
          </div>
        ) : null}
        {view === "guide" ? (
          <GuideView />
        ) : view === "licences" ? (
          <KeywordLicencesView />
        ) : view === "sets" ? (
          <SetsGrid
            ref={setsRef}
            answer={sets}
            version={store.version}
            current={session.setsCurrent}
            busy={busyCards}
            onSelect={(index) => setBrowserSession({ setsCurrent: index })}
            onOpen={openCard}
            onHover={setHovered}
            onType={(key) => {
              const text = session.sets.text;
              setText(key === "\b" ? text.slice(0, -1) : text + key);
              searchRef.current?.focus();
            }}
            onGuide={
              isTauri
                ? () => {
                    leave(null, true);
                    setBrowserSession({ guide: true });
                  }
                : null
            }
            onClearAll={sets.empty && canClear ? clearAll : null}
          />
        ) : (
          <BrowserGrid
            ref={gridRef}
            order={order}
            empty={itemsEmpty}
            size={prefs.size}
            filterKey={JSON.stringify({ ...query, sort: undefined, own_sort: undefined })}
            version={store.version}
            current={current}
            currentWhy={currentWhy}
            onSelect={select}
            onLoad={activate}
            onShowPages={showPages}
            onHover={setHovered}
            onDragStart={dragStart}
            onType={(key) => {
              setText(key === "\b" ? prefs.text.slice(0, -1) : prefs.text + key);
              searchRef.current?.focus();
            }}
            onClearAll={answer.empty && canClear ? clearAll : null}
            onShowSets={
              answer.empty && answer.show_sets && !canClear
                ? () => {
                    switchView("sets");
                    // The button leaves with the empty state: the first card takes the focus.
                    window.setTimeout(() => setsRef.current?.focusFirst(), 0);
                  }
                : null
            }
          />
        )}
      </div>
      {standIn ? null : (
        <BrowserDetailBar
          words={shownWords}
          card={shownCard}
          isCurrent={shownWords !== null && shownWords.index === current}
          wide={wide}
          why={shownWhy}
          onAction={action}
          onLicence={(url) => void openLicence(url).catch(() => undefined)}
        />
      )}
    </section>
  );
}

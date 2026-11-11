import { ChevronDown, ChevronRight } from "lucide-react";
import { useEffect, useId, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent, type ReactNode } from "react";

import { FixtureGlyph } from "@/components/workbench/FixtureGlyphs";
import { FixtureJobParts } from "@/components/panel/FixtureJobParts";
import { FixtureJobRowView, type JobRowActions } from "@/components/panel/FixtureJobRow";
import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import type { FixtureJobsApi } from "@/hooks/useFixtureJobs";
import { useDeleteArm } from "@/hooks/useDeleteArm";
import type { FixtureJobsView, JobGroup, JobKindText, JobRow, PartsView } from "@/lib/fixtureJobsView";
import {
  ADD_KINDS,
  addLabel,

  armKey,
  flatJobs,
  jobNeighbour,
  landingPosition,
  partsFor,
  sectionTitle,
} from "@/lib/fixtureJobRows";
import { insertionEdge, nextFocusIndex } from "@/lib/operationRows";
import { visibleRange } from "@/lib/operationWindow";
import { entriesOf, HEADING_PX, WINDOW_ABOVE, OVERSCAN, type Entry } from "@/lib/fixtureJobRows";

/** The nearest ancestor that scrolls, or `null`: the panel's column (the tab scrolls as one). */
function scrollParent(element: HTMLElement | null): HTMLElement | null {
  for (let at = element?.parentElement ?? null; at; at = at.parentElement) {
    const overflow = getComputedStyle(at).overflowY;
    if (overflow === "auto" || overflow === "scroll") return at;
  }
  return null;
}

interface FixtureJobsSectionProps {
  api: FixtureJobsApi;
  /** A press on a group's heading selects the fixture on the canvas (the mouse route). */
  selectFixture: (id: string) => boolean;
}

/**
 * Whether the section was open the last time: kept for the session while the tab is left and
 * entered again, never stored in the file (`ux-notes.md` 3.1, criterion 1).
 */
let rememberedExpanded = false;

/** Where focus goes after the next render: a job's name, the header, or the add toolbar. */
type FocusTarget = { job: string } | "header" | "add" | null;

/** The drag in flight (`ux-notes.md` 4.6): the job, its group and where it started. */
interface DragState {
  id: string;
  group: string;
  from: number;
  startY: number;
  offset: number;
  active: boolean;
}

/**
 * The Fixture jobs section of the Operations tab (`specs/0103-fixture-jobs` `ux-notes.md`
 * section 3): a collapsed header with the count, the add toolbar of the one selected fixture, the
 * jobs grouped under their fixtures, and the open job's Parts list. Nothing here decides a rule;
 * the session does, and the view is re-read after each call.
 */
export function FixtureJobsSection({ api, selectFixture }: FixtureJobsSectionProps) {
  const { view } = api;
  const [expanded, setExpanded] = useState(rememberedExpanded);
  const bodyId = useId();
  const rootRef = useRef<HTMLElement>(null);
  const headerRef = useRef<HTMLButtonElement>(null);
  const arm = useDeleteArm();
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [stop, setStop] = useState(0);
  const [addStop, setAddStop] = useState(0);
  /** The job the last add made: its status line names it, "Engrave 1 added." (ux-notes 11). */
  const [addedId, setAddedId] = useState<string | null>(null);
  const [dropAt, setDropAt] = useState<number | null>(null);
  const [dragId, setDragId] = useState<string | null>(null);
  const [ghost, setGhost] = useState<{ y: number; label: string; left: number; top: number; bottom: number; width: number; offset: number } | null>(null);
  const drag = useRef<DragState | null>(null);
  /** The viewport in the groups' coordinates (px) while the rows are windowed; `null` unwindowed. */
  const [scroll, setScroll] = useState<{ top: number; height: number } | null>(null);
  const groupsRef = useRef<HTMLDivElement>(null);
  const windowed = (view?.count ?? 0) > WINDOW_ABOVE;
  const dropRef = useRef<number | null>(null);
  const pending = useRef<FocusTarget>(null);

  // The list changing or the section collapsing disarms an armed trash (`0081`; ux-notes 4.5).
  const order = view ? view.groups.map((group) => `${group.fixture}:${group.jobs.map((job) => job.id).join(",")}`).join("|") : "";
  const disarm = arm.disarm;
  useEffect(() => {
    disarm();
  }, [disarm, order, expanded]);

  // The window follows the panel's scroll: the viewport's top, in the groups' coordinates.
  useEffect(() => {
    const groups = groupsRef.current;
    const container = scrollParent(groups);
    if (!windowed || !groups || !container || !expanded) {
      setScroll(null);
      return;
    }
    const measure = () => {
      const offset = groups.getBoundingClientRect().top - container.getBoundingClientRect().top;
      setScroll({ top: Math.max(0, -offset), height: container.clientHeight });
    };
    measure();
    container.addEventListener("scroll", measure, { passive: true });
    window.addEventListener("resize", measure);
    return () => {
      container.removeEventListener("scroll", measure);
      window.removeEventListener("resize", measure);
    };
  }, [windowed, expanded, view?.count]);

  useEffect(() => {
    const target = pending.current;
    pending.current = null;
    const root = rootRef.current;
    if (target === "header") headerRef.current?.focus();
    else if (target === "add") root?.querySelector<HTMLButtonElement>("[data-add-kind]")?.focus();
    else if (target && typeof target === "object") {
      root?.querySelector<HTMLButtonElement>(`[data-job-row="${target.job}"] [data-row-name]`)?.focus();
    }
  });

  useEffect(() => {
    dropRef.current = dropAt;
  });

  // The reorder drag (ux-notes 4.6): starts after 4 px, lands inside the job's group only, and is
  // one step on release. A release outside the group puts the job back; Escape cancels.
  useEffect(() => {
    if (!view) return;
    const groupRows = (fixture: string) =>
      Array.from(rootRef.current?.querySelectorAll<HTMLElement>(`[data-group="${fixture}"] li[data-job-row]`) ?? []);
    const onMove = (event: PointerEvent) => {
      const current = drag.current;
      if (!current) return;
      if (!current.active) {
        if (Math.abs(event.clientY - current.startY) < 4) return;
        current.active = true;
        setDragId(current.id);
        const box = rootRef.current?.querySelector(`[data-group="${current.group}"]`)?.getBoundingClientRect();
        const label = flatJobs(view.groups).find((job) => job.id === current.id)?.name ?? "";
        setGhost({ y: event.clientY, label, left: box?.left ?? 0, top: box?.top ?? 0, bottom: box?.bottom ?? 0, width: box?.width ?? 0, offset: current.offset });
      }
      const box = rootRef.current?.querySelector(`[data-group="${current.group}"]`)?.getBoundingClientRect();
      const outside = box !== undefined && (event.clientY < box.top || event.clientY > box.bottom);
      const middles = groupRows(current.group).map((element) => {
        const rect = element.getBoundingClientRect();
        return rect.top + rect.height / 2;
      });
      setGhost((previous) => (previous ? { ...previous, y: event.clientY } : previous));
      setDropAt(outside ? null : landingPosition(current.from, middles, event.clientY));
    };
    const finish = (apply: boolean) => {
      const current = drag.current;
      drag.current = null;
      setDropAt(null);
      setDragId(null);
      setGhost(null);
      if (!current || !current.active || !apply) return;
      const target = dropRef.current;
      if (target !== null && target !== current.from) {
        if (api.move(current.id, target)) pending.current = { job: current.id };
      }
    };
    const onUp = () => finish(true);
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && drag.current?.active) {
        event.preventDefault();
        event.stopPropagation();
        finish(false);
      }
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("keydown", onKey, true);
    };
  }, [view, api]);

  if (view === null) return null;

  const jobs = flatJobs(view.groups);
  const stopAt = Math.min(stop, Math.max(jobs.length - 1, 0));
  const flatIndex = new Map(jobs.map((job, index) => [job.id, index]));
  const selectedRow = jobs.find((job) => job.id === view.selected) ?? null;
  const parts = selectedRow ? partsFor(view, selectedRow) : null;
  const { entries, rowAt } = entriesOf(view.groups, parts, view.selected);
  const range =
    windowed && scroll ? visibleRange(entries.map((entry) => entry.height), scroll.top, scroll.height, OVERSCAN, null, null) : null;
  // The focused and the open row stay in the DOM wherever they scroll to (`ux-notes.md` 14); the
  // window itself is not stretched to reach them, so the rows between are not drawn.
  const pinned = new Set([rowAt.get(jobs[stopAt]?.id ?? ""), view.selected ? rowAt.get(view.selected) : undefined]);
  /** Whether entry `at` is drawn: everything unwindowed, else the window's entries and the pinned. */
  const drawn = (at: number) => range === null || (at >= range.first && at < range.last) || pinned.has(at);
  const addedName = addedId ? jobs.find((job) => job.id === addedId)?.name ?? null : null;

  const actions: JobRowActions = {
    select: (row, rename = false) => {
      api.select(row.id);
      setStop(flatIndex.get(row.id) ?? 0);
      // An unknown job is renamed like any other (criterion 21: rename and delete only).
      if (rename) setRenamingId(row.id);
    },
    rename: (row, name) => {
      api.rename(row.id, name);
    },
    endRename: (row, refocus) => {
      setRenamingId(null);
      if (refocus) pending.current = { job: row.id };
    },
    trash: (row) => {
      const index = flatIndex.get(row.id) ?? 0;
      arm.press(armKey(row.id), `Press again to delete ${row.name}.`, () => {
        const remaining = jobs.filter((job) => job.id !== row.id);
        if (api.remove(row.id)) {
          if (remaining.length === 0) pending.current = "header";
          else {
            const next = remaining[Math.min(index, remaining.length - 1)];
            pending.current = { job: next.id };
            setStop(Math.min(index, remaining.length - 1));
          }
        }
        return `${row.name} deleted.`;
      });
    },
    setShow: (row, on) => {
      api.setShow(row.id, on);
    },
    setOutput: (row, on) => {
      api.setOutput(row.id, on);
    },
    step: (row, step) => {
      const to = jobNeighbour(view.groups, row.id, step);
      if (to !== null && api.move(row.id, to)) pending.current = { job: row.id };
    },
    edge: (row, edge) => {
      const group = view.groups.find((g) => g.jobs.some((job) => job.id === row.id));
      if (!group) return;
      const at = group.jobs.findIndex((job) => job.id === row.id);
      const to = edge === "first" ? 0 : group.jobs.length - 1;
      if (to !== at && api.move(row.id, to)) pending.current = { job: row.id };
    },
    dragStart: (row, event: ReactPointerEvent<HTMLElement>) => {
      if (event.button !== 0 || row.kind === null) return;
      const group = view.groups.find((g) => g.jobs.some((job) => job.id === row.id));
      if (!group) return;
      const rowTop = (event.currentTarget as HTMLElement).closest("li")?.getBoundingClientRect().top ?? event.clientY;
      drag.current = {
        id: row.id,
        group: group.fixture,
        from: group.jobs.findIndex((job) => job.id === row.id),
        startY: event.clientY,
        offset: event.clientY - rowTop,
        active: false,
      };
    },
  };

  const onListKey = (event: ReactKeyboardEvent<HTMLUListElement>) => {
    if (event.altKey) return;
    const element = (event.target as HTMLElement).closest<HTMLElement>("li[data-row-index]");
    const index = element ? Number(element.dataset.rowIndex) : stopAt;
    const next = nextFocusIndex(jobs.length, index, event.key);
    if (next !== null) {
      event.preventDefault();
      setStop(next);
      // Every row is in the tree (no windowing), so the focus moves at once.
      const id = jobs[next]?.id;
      rootRef.current?.querySelector<HTMLButtonElement>(`[data-job-row="${id}"] [data-row-name]`)?.focus();
    }
  };

  const onAddKey = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>("button"));
    const at = (addStop + step + buttons.length) % buttons.length;
    setAddStop(at);
    buttons[at]?.focus();
  };

  const press = (kind: JobKindText) => {
    if (!view.addTo) return;
    const id = api.add(kind);
    if (id) setAddedId(id);
  };

  const leave = (job: JobRow) => {
    rootRef.current?.querySelector<HTMLButtonElement>(`[data-job-row="${job.id}"] [data-row-name]`)?.focus();
  };

  const expandedBody = (
    <div id={bodyId} role="region" aria-label="Fixture jobs" className="flex flex-col gap-1 pt-1">
      {view.damageNotices.map((text) => (
        <p key={text} role="status" className="text-xs text-[var(--toolbar-icon)]">
          {text}
        </p>
      ))}
      <div
        role="toolbar"
        aria-label="Add fixture job"
        onKeyDown={onAddKey}
        className="flex h-7 items-center gap-1"
      >
        {view.addTo
          ? ADD_KINDS.map(([kind], at) => (
              <button
                key={kind}
                type="button"
                data-add-kind={kind}
                aria-label={addLabel(kind, view.addTo?.name ?? "")}
                title={addLabel(kind, view.addTo?.name ?? "")}
                tabIndex={at === addStop ? 0 : -1}
                onFocus={() => setAddStop(at)}
                onClick={() => press(kind)}
                className="flex h-7 w-7 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              >
                <OperationGlyph kind={kind} />
              </button>
            ))
          : null}
      </div>
      {jobs.length === 0 ? (
        <p className="py-2 text-sm text-[var(--panel-muted-fg)]">No fixture jobs. Select a fixture to add one.</p>
      ) : (
        <>
          <p className="h-6 text-[11px] font-semibold leading-6 text-[var(--panel-muted-fg)]">Runs first</p>
          <div ref={groupsRef} className="flex flex-col">
          {view.groups.map((group, groupAt) => (
            <JobGroupView
              key={group.fixture}
              group={group}
              first={groupAt === 0}
              flatIndex={flatIndex}
              rowAt={rowAt}
              entries={entries}
              drawn={drawn}
              total={jobs.length}
              view={view}
              stopAt={stopAt}
              armed={arm.armed}
              renamingId={renamingId}
              dragId={dragId}
              dropAt={dropAt}
              parts={parts}
              selectFixture={selectFixture}
              actions={actions}
              api={api}
              onListKey={onListKey}
              onFocusRow={(index) => setStop(index)}
              leave={leave}
            />
          ))}
          </div>
        </>
      )}
    </div>
  );

  return (
    <section ref={rootRef} aria-label="Fixture jobs section" className="flex flex-col">
      <button
        ref={headerRef}
        type="button"
        aria-expanded={expanded}
        aria-controls={expanded ? bodyId : undefined}
        onClick={() => {
          rememberedExpanded = !expanded;
          setExpanded(rememberedExpanded);
        }}
        className="flex h-7 w-full items-center gap-1 rounded text-left text-sm text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        {expanded ? <ChevronDown size={12} aria-hidden /> : <ChevronRight size={12} aria-hidden />}
        <span>{sectionTitle(view.count)}</span>
      </button>
      {expanded ? expandedBody : null}
      {api.refusal ? (
        <p role="alert" className="text-xs text-[var(--toolbar-icon)]">
          {api.refusal}
        </p>
      ) : null}
      {ghost && (
        <div
          aria-hidden
          data-job-ghost=""
          className="pointer-events-none fixed z-50 flex h-10 items-center rounded bg-[var(--value-fill)] px-2 text-[13px] text-[var(--toolbar-icon)] shadow"
          style={{
            left: ghost.left,
            top: Math.min(Math.max(ghost.y - ghost.offset, ghost.top), ghost.bottom - 40),
            width: ghost.width,
            opacity: 0.6,
          }}
        >
          {ghost.label}
        </div>
      )}
      <div role="status" aria-atomic="true" className="sr-only">
        {arm.status || (addedName ? `${addedName} added.` : "")}
      </div>
    </section>
  );
}

/** One fixture's group: its heading (symbol and name) and its jobs with their Parts lists. */
function JobGroupView({
  group,
  first,
  flatIndex,
  rowAt,
  entries,
  drawn,
  total,
  view,
  stopAt,
  armed,
  renamingId,
  dragId,
  dropAt,
  parts,
  selectFixture,
  actions,
  api,
  onListKey,
  onFocusRow,
  leave,
}: {
  group: JobGroup;
  first: boolean;
  flatIndex: Map<string, number>;
  rowAt: Map<string, number>;
  entries: Entry[];
  drawn: (at: number) => boolean;
  total: number;
  view: FixtureJobsView;
  stopAt: number;
  armed: string | null;
  renamingId: string | null;
  dragId: string | null;
  dropAt: number | null;
  parts: PartsView | null;
  selectFixture: (id: string) => boolean;
  actions: JobRowActions;
  api: FixtureJobsApi;
  onListKey: (event: ReactKeyboardEvent<HTMLUListElement>) => void;
  onFocusRow: (index: number) => void;
  leave: (job: JobRow) => void;
}) {
  const headingId = useId();
  const sourceAt = dragId ? group.jobs.findIndex((job) => job.id === dragId) : -1;
  const headingAt = entries.findIndex((entry) => entry.kind === "heading" && entry.fixture === group.fixture);
  // Rows out of the window are spacers of their height, runs of them one spacer (windowing).
  const rows: ReactNode[] = [];
  let gap = 0;
  const flush = (key: string) => {
    if (gap > 0) rows.push(<li key={key} aria-hidden style={{ height: gap }} />);
    gap = 0;
  };
  group.jobs.forEach((row, at) => {
    const entry = rowAt.get(row.id) ?? 0;
    if (!drawn(entry)) {
      gap += entries[entry]?.height ?? 0;
      return;
    }
    flush(`gap-${row.id}`);
    const index = flatIndex.get(row.id) ?? 0;
    const insertion = sourceAt >= 0 && dropAt !== null && dropAt === at ? insertionEdge(sourceAt, dropAt) : null;
    const open = parts !== null && parts.job === row.id && view.selected === row.id;
    rows.push(
      <FixtureJobRowView
        key={row.id}
        row={row}
        index={index}
        total={total}
        selected={view.selected === row.id}
        renaming={renamingId === row.id}
        armed={armed === armKey(row.id)}
        canUp={jobNeighbour(view.groups, row.id, -1) !== null}
        canDown={jobNeighbour(view.groups, row.id, 1) !== null}
        tabStop={index === stopAt}
        dragSource={dragId === row.id}
        insertion={insertion}
        onFocusRow={() => onFocusRow(index)}
        actions={actions}
      >
        {open ? (
          <FixtureJobParts
            job={row}
            parts={parts}
            actions={{
              setPart: (job, token, ticked) => {
                api.setPart(job.id, token, ticked);
              },
              selectPart: (job, token) => {
                api.selectPart(job.id, token);
              },
              leave,
            }}
          />
        ) : null}
      </FixtureJobRowView>,
    );
  });
  flush("gap-end");
  return (
    <div
      role="group"
      aria-labelledby={headingId}
      data-group={group.fixture}
      className={`flex flex-col ${first ? "" : "mt-1 border-t border-[var(--divider,transparent)] pt-1"}`}
    >
      {headingAt >= 0 && drawn(headingAt) ? (
        <div
          id={headingId}
          title={group.name}
          onClick={() => selectFixture(group.fixture)}
          className="flex h-7 items-center gap-1.5 text-[13px] font-semibold text-[var(--toolbar-icon)]"
        >
          <FixtureGlyph kind={group.kind} className="shrink-0" />
          <span className="min-w-0 truncate">{group.name}</span>
        </div>
      ) : (
        <div id={headingId} aria-hidden style={{ height: HEADING_PX }} />
      )}
      <ul role="list" onKeyDown={onListKey} className="flex flex-col">
        {rows}
      </ul>
    </div>
  );
}

import { Eye, EyeOff, Plus, Trash2, TriangleAlert } from "lucide-react";
import { type KeyboardEvent, useEffect, useRef, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import type { ManufacturingView, WorkbenchesApi } from "@/hooks/useWorkbenches";
import { fixtureRowKey, focusAfterRemove, tabStopRow } from "@/lib/fixtureRowKeys";
import { WorkbenchIcon } from "@/workbenches/icons";
import type { ManifestBlock, ManifestParam, ManifestWorkbenchContribution } from "@/workbenches/registry";

interface ManifestTabProps {
  contribution: ManifestWorkbenchContribution;
  view: ManufacturingView;
  workbenches: WorkbenchesApi;
  onReturnFocus: () => void;
}

/**
 * A manufacturing workbench's tab, drawn from its extension's manifest (ADR 0034 §4,
 * `specs/0093-laser-workbench-shell` UX notes §4): the three block kinds `Notice`, `Params` and
 * `FixtureList` in the manifest's order. Nothing here names a process.
 */
export function ManifestTab({ contribution, view, workbenches, onReturnFocus }: ManifestTabProps) {
  const blocks = contribution.tabs[0]?.blocks ?? [];
  return (
    <div className="flex flex-col gap-4">
      {blocks.map((block, index) => (
        <Block
          key={`${block.kind}-${index}`}
          block={block}
          contribution={contribution}
          view={view}
          workbenches={workbenches}
          onReturnFocus={onReturnFocus}
        />
      ))}
    </div>
  );
}

function Block({
  block,
  contribution,
  view,
  workbenches,
  onReturnFocus,
}: ManifestTabProps & { block: ManifestBlock }) {
  switch (block.kind) {
    case "notice":
      return <NoticeBlock text={view.notices[block.id]} />;
    case "params":
      return (
        <section aria-label={block.label} className="flex flex-col gap-2">
          <h3 className="text-sm text-[var(--toolbar-icon)]">{block.label}</h3>
          <HostParams params={block.params} view={view} workbenches={workbenches} />
        </section>
      );
    case "fixture_list":
      return (
        <FixtureList
          label={block.label}
          contribution={contribution}
          view={view}
          workbenches={workbenches}
          onReturnFocus={onReturnFocus}
        />
      );
  }
}

/** A `Notice` block: shown only while its text applies (UX notes §6). */
function NoticeBlock({ text }: { text: string | undefined }) {
  if (!text) {
    return null;
  }
  return (
    <div
      role="status"
      className="flex items-start gap-2 rounded-md p-2 text-xs text-[var(--toolbar-icon)]"
      style={{
        background: "var(--toolbar-bg)",
        border: "1px solid color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
      }}
    >
      <TriangleAlert size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="shrink-0" />
      <span>{text}</span>
    </div>
  );
}

/** The fields of a `Params` block whose keys bind host values (machine name, work area). */
function HostParams({
  params,
  view,
  workbenches,
}: {
  params: ManifestParam[];
  view: ManufacturingView;
  workbenches: WorkbenchesApi;
}) {
  const width = Number(view.values["host.work_width"]);
  const height = Number(view.values["host.work_height"]);
  const commit = (key: string, mm: number) => {
    if (key === "host.work_width") workbenches.setWorkArea(mm, height);
    if (key === "host.work_height") workbenches.setWorkArea(width, mm);
  };
  const lengths = params.filter((param) => param.kind.kind === "length");
  return (
    <>
      {params
        .filter((param) => param.kind.kind === "display")
        .map((param) => (
          <label key={param.key} className="flex h-7 items-center justify-between gap-2 text-sm">
            <span className="text-[var(--toolbar-icon)]">{param.label}</span>
            <input
              readOnly
              value={String(view.values[param.key] ?? "")}
              className="h-7 w-[150px] rounded-[5px] border border-[var(--field-border)] bg-white px-2 text-sm opacity-70 outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            />
          </label>
        ))}
      <div className="grid grid-cols-2 gap-2">
        {lengths.map((param) => (
          <MmField
            key={`${param.key}:${String(view.values[param.key])}`}
            param={param}
            value={Number(view.values[param.key])}
            onCommit={(mm) => commit(param.key, mm)}
          />
        ))}
      </div>
    </>
  );
}

/** "Read-only" here, three decimals with trailing zeros trimmed (UX notes §4). */
function mmText(mm: number): string {
  return Number.isFinite(mm) ? String(Number(mm.toFixed(3))) : "";
}

/** A length field: the label inside the left edge, "mm" inside the right; Enter or blur
 * commits (the session clamps into the schema's bounds), Escape reverts. */
function MmField({
  param,
  value,
  onCommit,
}: {
  param: ManifestParam;
  value: number;
  onCommit: (mm: number) => void;
}) {
  // The parent keys the field on its value, so a new value starts a fresh draft.
  const [text, setText] = useState(mmText(value));
  const commit = () => {
    const mm = Number(text.replace(",", "."));
    if (text.trim() === "" || !Number.isFinite(mm)) {
      setText(mmText(value));
      return;
    }
    if (mm !== value) {
      onCommit(mm);
    }
  };
  return (
    <Tooltip side="left" content={param.tooltip}>
      <label className="flex h-7 items-center gap-1 rounded-[5px] border border-[var(--field-border)] bg-white px-2 text-sm focus-within:ring-2 focus-within:ring-[var(--editor-accent)]">
        <span className="shrink-0 text-[var(--toolbar-icon)]">{param.label}</span>
        <input
          aria-label={`${param.label} (mm)`}
          inputMode="decimal"
          value={text}
          onChange={(event) => setText(event.target.value)}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              commit();
            } else if (event.key === "Escape") {
              event.preventDefault();
              setText(mmText(value));
            }
          }}
          className="min-w-0 flex-1 bg-transparent text-right tabular-nums outline-none"
        />
        <span aria-hidden className="shrink-0 text-xs text-[var(--panel-muted-fg)]">
          mm
        </span>
      </label>
    </Tooltip>
  );
}

const ICON_BUTTON =
  "inline-flex h-7 w-6 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

const KIND_BUTTON =
  "flex h-7 items-center gap-2 rounded-md px-2 text-sm outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** A `FixtureList` block (UX notes §4): the installed fixtures with add, hide and remove; the
 * selected row expands in place with the kind's value fields. The rows are one Tab stop (keys in
 * `fixtureRowKeys`). */
function FixtureList({
  label,
  contribution,
  view,
  workbenches,
  onReturnFocus,
}: {
  label: string;
  contribution: ManifestWorkbenchContribution;
  view: ManufacturingView;
  workbenches: WorkbenchesApi;
  onReturnFocus: () => void;
}) {
  const [open, setOpen] = useState<string | null>(null);
  const [choosing, setChoosing] = useState(false);
  const [active, setActive] = useState<string | null>(null);
  // Where the focus goes once the list has re-rendered: a row id, the Add fixture button
  // ("add") or the first kind button ("kinds").
  const pendingFocus = useRef<string | null>(null);
  const sectionRef = useRef<HTMLElement>(null);
  const kinds = contribution.fixture_kinds;
  const ids = view.fixtures.map((fixture) => fixture.id);
  const tabStop = tabStopRow(ids, active);
  const focusTarget = (target: string) => {
    const section = sectionRef.current;
    const selector =
      target === "add"
        ? "[data-add-fixture]"
        : target === "kinds"
          ? "[data-fixture-kind]"
          : `[data-fixture-row="${CSS.escape(target)}"]`;
    section?.querySelector<HTMLElement>(selector)?.focus();
  };
  useEffect(() => {
    const target = pendingFocus.current;
    if (target !== null) {
      pendingFocus.current = null;
      focusTarget(target);
    }
  });
  const closeKinds = () => {
    setChoosing(false);
    pendingFocus.current = "add";
  };
  const onRowKey = (event: KeyboardEvent<HTMLElement>, index: number) => {
    const fixture = view.fixtures[index];
    const action = fixtureRowKey(event.key, index, view.fixtures.length);
    if (!fixture || action === null || event.altKey || event.ctrlKey || event.metaKey) {
      return;
    }
    event.preventDefault();
    if (action.kind === "focus") {
      const id = ids[action.index];
      setActive(id);
      focusTarget(id);
    } else if (action.kind === "toggle_visible") {
      workbenches.setFixtureVisible(fixture.id, !fixture.visible);
    } else {
      const next = focusAfterRemove(index, ids.length);
      const remaining = ids.filter((id) => id !== fixture.id);
      const nextId = next === null ? null : remaining[next];
      workbenches.deleteFixture(fixture.id);
      setActive(nextId);
      pendingFocus.current = nextId ?? "add";
    }
  };
  const add = () => {
    if (kinds.length === 1) {
      workbenches.addFixture(kinds[0].id);
      onReturnFocus();
    } else if (choosing) {
      closeKinds();
    } else {
      setChoosing(true);
      pendingFocus.current = "kinds";
    }
  };
  return (
    <section ref={sectionRef} aria-label={label} className="flex flex-col gap-1">
      <div className="flex h-7 items-center justify-between">
        <h3 className="text-sm text-[var(--toolbar-icon)]">{label}</h3>
        <Tooltip side="left" content="Add fixture">
          <button type="button" data-add-fixture aria-label="Add fixture" aria-expanded={kinds.length > 1 ? choosing : undefined} className={`${ICON_BUTTON} w-7`} onClick={add}>
            <Plus size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          </button>
        </Tooltip>
      </div>
      {choosing ? (
        <div
          role="group"
          aria-label="Add fixture"
          className="flex flex-col gap-1 p-2"
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              event.stopPropagation();
              closeKinds();
            }
          }}
        >
          {kinds.map((kind) => (
            <button
              key={kind.id}
              type="button"
              data-fixture-kind
              className={KIND_BUTTON}
              onClick={() => {
                workbenches.addFixture(kind.id);
                closeKinds();
              }}
            >
              <WorkbenchIcon name={kind.icon} />
              {kind.label}
            </button>
          ))}
          <button type="button" className={KIND_BUTTON} onClick={closeKinds}>
            Cancel
          </button>
        </div>
      ) : null}
      <ul className="flex flex-col">
        {view.fixtures.map((fixture, index) => {
          const kind = kinds.find((k) => k.id === fixture.kind);
          const expanded = open === fixture.id;
          return (
            <li key={fixture.id} className="flex flex-col">
              <div className="flex h-9 items-center gap-2 rounded-md px-1 hover:bg-[var(--editor-accent-hover)]">
                <button
                  type="button"
                  data-fixture-row={fixture.id}
                  tabIndex={index === tabStop ? 0 : -1}
                  aria-expanded={expanded}
                  onFocus={() => setActive(fixture.id)}
                  onKeyDown={(event) => onRowKey(event, index)}
                  className={`flex min-w-0 flex-1 items-center gap-2 text-left text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${fixture.visible ? "" : "opacity-60"}`}
                  onClick={() => setOpen(expanded ? null : fixture.id)}
                >
                  <WorkbenchIcon name={fixture.icon ?? "Box"} />
                  <span className="truncate">{fixture.name}</span>
                </button>
                <Tooltip side="left" content={`${fixture.visible ? "Hide" : "Show"} ${fixture.name}`}>
                  <button
                    type="button"
                    aria-label={`${fixture.visible ? "Hide" : "Show"} ${fixture.name}`}
                    aria-pressed={!fixture.visible}
                    tabIndex={-1}
                    className={ICON_BUTTON}
                    onClick={() => workbenches.setFixtureVisible(fixture.id, !fixture.visible)}
                  >
                    {fixture.visible ? (
                      <Eye size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                    ) : (
                      <EyeOff size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                    )}
                  </button>
                </Tooltip>
                <Tooltip side="left" content={`Delete ${fixture.name}`}>
                  <button
                    type="button"
                    aria-label={`Delete ${fixture.name}`}
                    tabIndex={-1}
                    className={ICON_BUTTON}
                    onClick={() => workbenches.deleteFixture(fixture.id)}
                  >
                    <Trash2 size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                  </button>
                </Tooltip>
              </div>
              {expanded && kind ? (
                <div className="flex flex-col gap-2 py-2 pl-7">
                  {kind.params
                    .filter((param) => param.kind.kind === "length")
                    .map((param) => (
                      <MmField
                        key={`${param.key}:${String(fixture.values[param.key])}`}
                        param={param}
                        value={Number(fixture.values[param.key])}
                        onCommit={(mm) => workbenches.setFixtureLength(fixture.id, param.key, mm)}
                      />
                    ))}
                </div>
              ) : null}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

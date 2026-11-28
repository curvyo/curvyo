import { useEffect, useRef, useState } from "react";
import { BookmarkCheck, BookmarkPlus, ChevronDown, ChevronRight, Copy, Ellipsis, Plus, Trash2 } from "lucide-react";

import { InlinePickList } from "@/components/InlinePickList";
import { LibrarySyncToggle } from "@/components/LibrarySyncToggle";
import { EntryField } from "@/components/panel/EntryField";
import { PanelSwitch } from "@/components/ui/switch";
import { Tooltip } from "@/components/ui/tooltip";
import { ConnectorGroup } from "@/components/workbench/ConnectorGroup";
import { MachineSizeOffer } from "@/components/workbench/MachineSizeOffer";
import type { WorkbenchesApi } from "@/hooks/useWorkbenches";
import type { LibrarySyncApi } from "@/lib/devicesApi";
import { profileRowKey } from "@/lib/machineBlockKeys";
import { librarySyncOn } from "@/lib/librarySyncView";
import { setMachineWriteFailureHandler } from "@/lib/machineProfilesFile";
import type { MachineConnection, MachineField, MachinePicker, MachineRow } from "@/lib/machinesView";

const HEADING = "text-[11px] font-semibold text-[var(--panel-muted-fg)]";
const ICON =
  "flex size-7 shrink-0 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:size-9";
const TEXT_BUTTON =
  "flex h-7 items-center gap-1 rounded border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] px-2 text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:min-h-9";
const CORNERS = [
  { id: "top_left", name: "Top left" },
  { id: "top_right", name: "Top right" },
  { id: "bottom_left", name: "Bottom left" },
  { id: "bottom_right", name: "Bottom right" },
];

/** The family whose row the "…" of the Document tab asked to open, across the tab switch. */
let requestedRow: string | null = null;

/** The write-failure alert (UX unify 4.4): the library is read back from storage. */
const WRITE_FAILED = "Could not save the machine. Nothing was changed.";
/** How long the alert stays. */
const NOTICE_MS = 3000;

/**
 * The Machine group of an output page's Document tab (ADR 0098 decision 4, UX unify 4.1): the
 * profile pick of the page's family, the "…" that opens the machine in the Machines tab, and the
 * state row while one applies ("Differs from profile" with Reset, "Not in your machines" with
 * Save as machine). The machine's fields live in the Machines tab.
 */
export function MachineGroup({
  picker,
  workbenches,
  onOpenMachines,
  onReturnFocus,
}: {
  picker: MachinePicker;
  workbenches: WorkbenchesApi;
  onOpenMachines: () => void;
  onReturnFocus: () => void;
}) {
  return (
    <section aria-label="Machine" className="flex flex-col gap-1">
      <h3 className={HEADING}>Machine</h3>
      <div className="flex items-center gap-1">
        <div className="min-w-0 flex-1">
          <InlinePickList
            label={picker.name}
            className="flex h-7 w-full items-center justify-between rounded border border-[var(--field-border)] bg-[var(--field-bg,white)] px-2 text-sm pointer-coarse:min-h-9"
            rows={picker.profiles.map((p) => ({ id: p.id, name: p.name, detail: p.detail }))}
            onPick={(id) => {
              if (workbenches.output.pick(id)) onReturnFocus();
            }}
          />
        </div>
        <Tooltip content="Edit machine" side="bottom">
          <button
            type="button"
            aria-label={`Edit machine ${picker.name}`}
            className={ICON}
            onClick={() => {
              requestedRow = picker.id;
              onOpenMachines();
            }}
          >
            <Ellipsis size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          </button>
        </Tooltip>
      </div>
      {picker.state === "differs" ? (
        <div className="flex items-center justify-between gap-2 text-xs text-[var(--panel-muted-fg)]">
          <span>Differs from profile</span>
          <Tooltip content="Reset from profile" side="bottom">
            <button type="button" aria-label="Reset from profile" className={ICON} onClick={() => workbenches.output.reset()}>
              <BookmarkCheck size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            </button>
          </Tooltip>
        </div>
      ) : picker.state === "missing" ? (
        <div className="flex items-center justify-between gap-2 text-xs text-[var(--panel-muted-fg)]">
          <span>Not in your machines</span>
          <Tooltip content="Save as machine" side="bottom">
            <button type="button" aria-label="Save as machine" className={ICON} onClick={() => workbenches.output.saveAs()}>
              <BookmarkPlus size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            </button>
          </Tooltip>
        </div>
      ) : null}
    </section>
  );
}

/** One body field: the laser's white field, a switch, the corner pick, or a plain line. */
function FieldView({
  row,
  field,
  workbenches,
  onReturnFocus,
}: {
  row: MachineRow;
  field: MachineField;
  workbenches: WorkbenchesApi;
  onReturnFocus: () => void;
}) {
  const [notice, setNotice] = useState<string | null>(null);
  const [fieldError, setFieldError] = useState("");
  const free = field.kind === "entry";
  const set = (text: string) => {
    const result = workbenches.machines.set(row.id, field.key, text);
    setNotice(result.notice ?? null);
    return result.error;
  };
  if (field.greyed !== undefined) {
    // Greyed in place at 40 % with its cause, its value kept (`specs/0118` W62).
    return (
      <Tooltip side="left" content={<><div>{field.label}</div><div className="text-muted-foreground">{field.greyed}</div></>}>
        <div aria-disabled="true" aria-description={field.greyed} className="flex items-baseline justify-between gap-2 opacity-40">
          <span className="text-xs text-[var(--panel-muted-fg)]">{field.label}</span>
          <span className="text-[13px] tabular-nums text-[var(--toolbar-icon)]">
            {field.kind === "switch" ? (field.value === "on" ? "On" : "Off") : field.value}
            {field.unit && field.kind !== "switch" ? ` ${field.unit}` : ""}
          </span>
        </div>
      </Tooltip>
    );
  }
  if (field.kind === "text") {
    return (
      <div className="flex items-baseline justify-between gap-2">
        <span className="text-xs text-[var(--panel-muted-fg)]">{field.label}</span>
        <span className="text-[13px] tabular-nums text-[var(--toolbar-icon)]">
          {field.value}
          {field.unit && field.value ? ` ${field.unit}` : ""}
        </span>
      </div>
    );
  }
  if (field.kind === "switch") {
    return (
      <PanelSwitch label={field.label} checked={field.value === "on"} onCheckedChange={(on) => set(on ? "on" : "off")} onReturnFocus={onReturnFocus} />
    );
  }
  if (field.kind === "corner") {
    return (
      <InlinePickList
        label={`Origin: ${CORNERS.find((c) => c.id === field.value)?.name ?? field.value}`}
        className={TEXT_BUTTON + " w-full justify-between"}
        rows={CORNERS.map((c) => ({ id: c.id, name: c.name, detail: "" }))}
        onPick={(id) => set(id)}
      />
    );
  }
  return (
    <div className="flex flex-col gap-0.5">
      <EntryField
        label={field.unit ? `${field.label}, ${field.unit}` : field.label}
        prefix={field.label}
        shown={field.value}
        mixed={false}
        suffix={field.unit || undefined}
        width={228}
        onSubmit={(text) => {
          const error = set(text);
          if (error === undefined) return "committed";
          setFieldError(error || "Enter a number.");
          return "invalid:field";
        }}
        messages={{ field: fieldError }}
        onReturnFocus={onReturnFocus}
        commitOnBlur
        align={free ? "left" : "right"}
        inputMode={free ? "text" : "decimal"}
      />
      {notice ? <p className="text-[11px] text-[var(--panel-muted-fg)]">{notice}</p> : null}
    </div>
  );
}

/** A body field, and under a laser's Height the Machine size offer row (`specs/0131`). */
function FieldRow(props: { row: MachineRow; field: MachineField; workbenches: WorkbenchesApi; onReturnFocus: () => void }) {
  const { row, field, workbenches } = props;
  const number = (key: string) => Number(row.fields.find((f) => f.key === key)?.value ?? "0");
  return (
    <>
      <FieldView {...props} />
      {field.key === "height" && row.connection ? (
        <MachineSizeOffer
          profileId={row.id}
          address={row.connection.connector_address.trim()}
          unit="mm"
          api={workbenches.machines.sizeApi(row.id)}
          width={number("width")}
          height={number("height")}
          onFocusProfile={() => {}}
          onUsed={() => {}}
        />
      ) : null}
    </>
  );
}

/** The Connection group of a maker's laser (`0064`, UX unify 4.2): a library write on commit,
 * no undo step; the connector's state under it. */
function ConnectionGroup({
  row,
  connection,
  onCommit,
}: {
  row: MachineRow;
  connection: MachineConnection;
  onCommit: (address: string, port: string, baud: number) => string;
}) {
  const [notice, setNotice] = useState("");
  const { connector_address: address, controller_port: port, baud_rate: baud } = connection;
  const commit = (a: string, p: string, b: number) => {
    setNotice(onCommit(a, p, b));
    return "committed" as const;
  };
  return (
    <section aria-label="Connection" className="mt-1 flex flex-col gap-2">
      <h3 className={HEADING}>Connection</h3>
      <EntryField
        label="Connector address"
        prefix="Connector"
        shown={address}
        mixed={false}
        width={228}
        align="left"
        inputMode="text"
        onSubmit={(text) => commit(text, port, baud)}
        messages={{}}
        onReturnFocus={() => {}}
        commitOnBlur
      />
      <EntryField
        label="Controller port"
        prefix="Port"
        shown={port}
        mixed={false}
        width={228}
        align="left"
        inputMode="text"
        onSubmit={(text) => commit(address, text, baud)}
        messages={{}}
        onReturnFocus={() => {}}
        commitOnBlur
      />
      <EntryField
        label="Baud rate"
        prefix="Baud rate"
        shown={String(baud)}
        mixed={false}
        width={228}
        inputMode="decimal"
        onSubmit={(text) => {
          const value = Number(text.replace(/,/g, ""));
          return Number.isFinite(value) ? commit(address, port, value) : "invalid:field";
        }}
        messages={{ field: "Enter a number." }}
        onReturnFocus={() => {}}
        commitOnBlur
      />
      {notice ? <p className="text-[11px] text-[var(--panel-muted-fg)]">{notice}</p> : null}
      <ConnectorGroup profileId={row.id} address={address.trim()} />
    </section>
  );
}

/** One machine row with its actions and, unfolded, its body. */
function MachineRowView({
  row,
  open,
  onToggle,
  workbenches,
  onReturnFocus,
  follow,
  library,
}: {
  row: MachineRow;
  open: boolean;
  onToggle: () => void;
  workbenches: WorkbenchesApi;
  onReturnFocus: () => void;
  follow: { sentence: string } | null;
  library?: LibrarySyncApi;
}) {
  const syncOn = library ? librarySyncOn(library.view, "machines", row.id) : false;
  const toggleSync = () => {
    if (library && !row.builtin) library.switch("machine", row.id, !syncOn);
  };
  const [armed, setArmed] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const nameRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (open && requestedRow === row.id) {
      requestedRow = null;
      nameRef.current?.focus();
    }
  }, [open, row.id]);
  const shownSince = useRef(0);
  useEffect(() => {
    if (follow) shownSince.current = Date.now();
  }, [follow]);
  return (
    <li
      className="flex flex-col rounded-md"
      style={armed ? { background: "color-mix(in srgb, var(--destructive) 12%, transparent)" } : undefined}
    >
      <div className="flex min-h-10 items-center gap-1 px-1 pointer-coarse:min-h-12">
        {renaming ? (
          <EntryField
            label="Name"
            shown={row.name}
            mixed={false}
            width={160}
            align="left"
            autoFocus
            onSubmit={(text) => (workbenches.machines.rename(row.id, text) ? "committed" : "unchanged")}
            messages={{}}
            onReturnFocus={() => setRenaming(false)}
            onClose={() => setRenaming(false)}
            commitOnBlur
          />
        ) : (
          <button
            ref={nameRef}
            type="button"
            aria-label={`${row.name}, ${row.line2}`}
            aria-expanded={open}
            onClick={onToggle}
            onDoubleClick={() => !row.builtin && setRenaming(true)}
            onKeyDown={(event) => {
              // The `0026` §3 row keys: F2 renames, Ctrl+D duplicates, S syncs, Delete arms.
              const action = profileRowKey(event.key, event.ctrlKey || event.metaKey, 0, 1);
              if (action?.kind === "rename" && !row.builtin) setRenaming(true);
              else if (action?.kind === "duplicate") {
                event.preventDefault();
                workbenches.machines.duplicate(row.id);
              } else if (action?.kind === "sync") toggleSync();
              else if (action?.kind === "delete" && !row.builtin) {
                if (armed) workbenches.machines.remove(row.id);
                setArmed(!armed);
              }
            }}
            className="flex min-w-0 flex-1 flex-col items-start text-left outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          >
            <span className="truncate text-sm text-[var(--toolbar-icon)]">{row.name}</span>
            <span className="truncate text-[11px] text-[var(--panel-muted-fg)]">{row.line2}</span>
          </button>
        )}
        <Tooltip content="Duplicate" side="bottom">
          <button type="button" aria-label={`Duplicate ${row.name}`} className={ICON} onClick={() => workbenches.machines.duplicate(row.id)}>
            <Copy size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          </button>
        </Tooltip>
        {library && !row.builtin ? (
          <LibrarySyncToggle name={row.name} on={syncOn} view={library.view} onToggle={toggleSync} slot="flex size-7" />
        ) : null}
        {row.builtin ? null : (
          <button
            type="button"
            aria-label={armed ? `Press again to delete ${row.name}. Pages keep their copy.` : `Delete ${row.name}`}
            className={ICON}
            onBlur={() => setArmed(false)}
            onClick={() => {
              if (armed) {
                workbenches.machines.remove(row.id);
                setArmed(false);
              } else setArmed(true);
            }}
          >
            <Trash2 size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          </button>
        )}
      </div>
      {open ? (
        <div className="flex flex-col gap-2 border-l-2 border-[var(--editor-accent)] py-2 pl-4" data-machine-body>
          {row.fields.map((field) => (
            <FieldRow key={field.key} row={row} field={field} workbenches={workbenches} onReturnFocus={onReturnFocus} />
          ))}
          {follow ? (
            // Frage 434 C (UX unify 4.5): the inline offer after the fields, no focus taken.
            <div
              role="group"
              aria-label={follow.sentence}
              className="flex flex-col gap-2 rounded-md p-2"
              style={{ background: "var(--toolbar-bg)" }}
              onKeyDown={(event) => event.key === "Escape" && workbenches.machines.keep()}
            >
              <p role="status" className="text-xs text-[var(--toolbar-icon)]">
                {follow.sentence}
              </p>
              <div className="grid grid-cols-2 gap-2">
                <button type="button" className={TEXT_BUTTON + " justify-center"} onClick={() => workbenches.machines.keep()}>
                  Keep pages
                </button>
                <button
                  type="button"
                  className="flex h-7 items-center justify-center rounded bg-[var(--toolbar-icon-active-bg)] px-2 text-xs text-[var(--toolbar-icon-active-fg)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:min-h-9"
                  onClick={() => {
                    if (Date.now() - shownSince.current >= 400) workbenches.machines.follow();
                  }}
                >
                  Update pages
                </button>
              </div>
            </div>
          ) : null}
          {row.connection ? (
            <ConnectionGroup
              row={row}
              connection={row.connection}
              onCommit={(address, port, baud) => workbenches.machines.setConnection(row.id, address, port, baud)}
            />
          ) : null}
        </div>
      ) : null}
    </li>
  );
}

/**
 * The Machines tab (ADR 0098 decision 4, UX unify 4.2 to 4.5): one section per enabled family,
 * the active page's family first and unfolded, "+ Machine", rows with Duplicate, rename and the
 * armed trash, one body open at a time, and the inline offer to update the pages.
 */
export function MachinesTab({
  workbenches,
  onReturnFocus,
  library,
}: {
  workbenches: WorkbenchesApi;
  onReturnFocus: () => void;
  /** The library sync's toggles (`specs/0116-library-sync`). */
  library?: LibrarySyncApi;
}) {
  const view = workbenches.machines.view;
  const [folded, setFolded] = useState<Record<string, boolean>>({});
  const [open, setOpen] = useState<string | null>(requestedRow);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    setMachineWriteFailureHandler(() => setFailed(true));
    return () => setMachineWriteFailureHandler(null);
  }, []);
  useEffect(() => {
    if (!failed) return;
    const timer = window.setTimeout(() => setFailed(false), NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [failed]);
  if (!view) return null;
  return (
    <div className="flex flex-col gap-4">
      <h2 className="text-sm font-semibold text-[var(--toolbar-icon)]">Machines</h2>
      {failed ? (
        <p role="alert" className="text-xs text-[var(--field-invalid)]">
          {WRITE_FAILED}
        </p>
      ) : null}
      {library !== undefined && library.view === null ? (
        <p role="status" className="text-xs text-[var(--panel-muted-fg)]">
          Pair a device to sync machine profiles.
        </p>
      ) : null}
      {view.sections.map((section) => {
        const isFolded = folded[section.family] ?? !section.active;
        return (
          <section key={section.family} aria-label={`${section.word} machines`} className="flex flex-col gap-1">
            <div className="flex items-center justify-between">
              <button
                type="button"
                aria-expanded={!isFolded}
                aria-label={`${section.word} machines, ${section.rows.length}`}
                className="flex h-7 items-center gap-1 text-[11px] font-semibold text-[var(--panel-muted-fg)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
                onClick={() => setFolded({ ...folded, [section.family]: !isFolded })}
              >
                {isFolded ? (
                  <ChevronRight size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                ) : (
                  <ChevronDown size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                )}
                {section.word} · {section.rows.length}
              </button>
              {isFolded ? null : (
                <button
                  type="button"
                  className={TEXT_BUTTON}
                  onClick={() => {
                    const id = workbenches.machines.add(section.family);
                    if (id) setOpen(id);
                  }}
                >
                  <Plus size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                  Machine
                </button>
              )}
            </div>
            {isFolded ? null : (
              <>
                {section.notices.map((notice) => (
                  <p key={notice} role="status" className="rounded-md p-2 text-xs text-[var(--toolbar-icon)]" style={{ background: "var(--toolbar-bg)" }}>
                    {notice}
                  </p>
                ))}
                <ul role="list" className="flex flex-col">
                  {section.rows.map((row) => (
                    <MachineRowView
                      key={row.id}
                      row={row}
                      open={open === row.id}
                      onToggle={() => {
                        if (open === row.id) workbenches.machines.keep();
                        setOpen(open === row.id ? null : row.id);
                      }}
                      workbenches={workbenches}
                      onReturnFocus={onReturnFocus}
                      follow={view.follow && view.follow.id === row.id ? view.follow : null}
                      library={library}
                    />
                  ))}
                </ul>
              </>
            )}
          </section>
        );
      })}
    </div>
  );
}

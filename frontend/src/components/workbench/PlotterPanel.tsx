import { Check, ChevronDown, Trash2 } from "lucide-react";
import { type KeyboardEvent, useState } from "react";

import { PanelSwitch } from "@/components/ui/switch";
import { type OperationId, operationIdText } from "@/lib/operationsView";
import type { PlotterRow, PlotterView } from "@/lib/plotterView";

/** The commands the panel calls; each is one step in the session (`specs/0159`). */
export interface PlotterApi {
  pick: (id: string) => boolean;
  saveAs: () => string;
  setDefault: (field: string, text: string) => string;
  setMat: (size: string) => boolean;
  setGrip: (grip: string) => boolean;
  setGrid: (on: boolean) => boolean;
  press: (kind: string, shift: boolean) => string;
  removeSelection: (id: string) => boolean;
  remove: (id: string) => boolean;
  setNumber: (id: string, field: string, text: string) => string;
}

interface PlotterPanelProps {
  view: PlotterView;
  api: PlotterApi;
  /** Export job file, the laser's flow (`useJobExport`). */
  onExport: () => void;
  /** The Toolpath view toggle of page `page`. */
  onToggleToolpath: (page: string) => void;
  /** Selects the items of a check. */
  onSelectItems: (ids: OperationId[]) => void;
  onReturnFocus: () => void;
}

const HEADING = "text-xs font-medium text-[var(--panel-muted-fg)]";
const ROW = "flex items-center justify-between gap-2 text-sm text-[var(--toolbar-icon)]";
const SEGMENT =
  "flex h-7 min-w-0 flex-1 basis-0 items-center justify-center rounded-md px-1 text-[13px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] aria-checked:bg-[var(--toolbar-icon-active-bg)] aria-checked:text-[var(--toolbar-icon-active-fg)] [@media(pointer:coarse)]:h-9";
const BUTTON =
  "flex h-7 w-full items-center justify-between rounded border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] px-2 text-xs text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] disabled:opacity-50 pointer-coarse:min-h-9";
const FIELD =
  "h-7 w-full rounded border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] bg-transparent px-2 text-sm tabular-nums text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/**
 * A plotter page's panel (`specs/0159-cutting-plotter` slice 1, UX notes 4 to 9): Machine (profile
 * pick, cut area, numbers on an HPGL profile, Output), Mat (size, grip, Mat grid), Operations
 * (Cut and Draw, rows with their numbers) and Output (checks, total, Toolpath view, Export). Laser
 * rows are absent, not disabled.
 */
export function PlotterPanel({ view, api, onExport, onToggleToolpath, onSelectItems, onReturnFocus }: PlotterPanelProps) {
  return (
    <div className="flex flex-col gap-4" data-plotter-panel="">
      <MachineSection view={view} api={api} onReturnFocus={onReturnFocus} />
      {view.mat ? <MatSection view={view} api={api} onReturnFocus={onReturnFocus} /> : null}
      <OperationsSection view={view} api={api} onReturnFocus={onReturnFocus} />
      <OutputSection
        view={view}
        onExport={onExport}
        onToggleToolpath={onToggleToolpath}
        onSelectItems={onSelectItems}
      />
      {view.notice ? (
        <p role="status" className="text-xs text-[var(--panel-muted-fg)]">
          {view.notice}
        </p>
      ) : null}
    </div>
  );
}

function MachineSection({ view, api, onReturnFocus }: { view: PlotterView; api: PlotterApi; onReturnFocus: () => void }) {
  const [open, setOpen] = useState(false);
  const { profile } = view;
  return (
    <section aria-label="Machine" className="flex flex-col gap-2">
      <h3 className={HEADING}>Machine</h3>
      <button type="button" className={BUTTON} aria-expanded={open} onClick={() => setOpen(!open)}>
        <span className="truncate">{profile.name}</span>
        <ChevronDown size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
      </button>
      {open ? (
        <div role="listbox" aria-label="Machine profile" className="flex flex-col">
          {view.profiles.map((p) => (
            <button
              key={p.id}
              type="button"
              role="option"
              aria-selected={p.id === profile.id}
              className="flex h-7 items-center justify-between rounded px-2 text-[13px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
              onClick={() => {
                api.pick(p.id);
                setOpen(false);
                onReturnFocus();
              }}
            >
              <span className="truncate">{p.name}</span>
              {p.id === profile.id ? <Check size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden /> : null}
            </button>
          ))}
          <button
            type="button"
            className="h-7 rounded px-2 text-left text-[13px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
            onClick={() => {
              api.saveAs();
              setOpen(false);
            }}
          >
            Save as profile
          </button>
        </div>
      ) : null}
      <div className={ROW}>
        <span>{profile.matless ? "Work area" : "Cut area"}</span>
        <span className="tabular-nums">{profile.area}</span>
      </div>
      {profile.setsNumbers ? null : (
        <>
          <NumberField label="Max speed (mm/s)" value={profile.maxSpeed} onCommit={(t) => api.setDefault("max_speed", t)} />
          <NumberField
            label="Default pressure (percent)"
            value={profile.defaultPressure}
            onCommit={(t) => api.setDefault("default_pressure", t)}
          />
          <NumberField label="Default speed (mm/s)" value={profile.defaultSpeed} onCommit={(t) => api.setDefault("default_speed", t)} />
        </>
      )}
      <div className={ROW} aria-label={`Output: ${profile.output}`}>
        <span>Output</span>
        <span>{profile.output}</span>
      </div>
    </section>
  );
}

/** A whole-number field: Enter or leaving commits; a refused value shows the session's message. */
function NumberField({ label, value, onCommit }: { label: string; value: number; onCommit: (text: string) => string }) {
  const [text, setText] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const commit = () => {
    if (text === null) return;
    const refused = onCommit(text);
    setMessage(refused);
    if (!refused) setText(null);
  };
  return (
    <label className="flex flex-col gap-1 text-xs text-[var(--panel-muted-fg)]">
      <span>{label}</span>
      <input
        className={FIELD}
        inputMode="numeric"
        aria-label={label}
        aria-invalid={message !== ""}
        value={text ?? String(value)}
        onChange={(event) => setText(event.target.value)}
        onBlur={commit}
        onKeyDown={(event: KeyboardEvent<HTMLInputElement>) => {
          if (event.key === "Enter") commit();
          if (event.key === "Escape") {
            setText(null);
            setMessage("");
          }
        }}
      />
      {message ? (
        <span role="alert" className="text-[var(--destructive)]">
          {message}
        </span>
      ) : null}
    </label>
  );
}

function MatSection({ view, api, onReturnFocus }: { view: PlotterView; api: PlotterApi; onReturnFocus: () => void }) {
  const mat = view.mat;
  const [gripOpen, setGripOpen] = useState(false);
  if (!mat) return null;
  return (
    <section aria-label="Mat" className="flex flex-col gap-2">
      <h3 className={HEADING}>Mat</h3>
      <div role="radiogroup" aria-label="Mat size" className="flex gap-1">
        {mat.sizes.map((size) => (
          <button
            key={size.key}
            type="button"
            role="radio"
            aria-checked={size.key === mat.size}
            tabIndex={size.key === mat.size ? 0 : -1}
            className={SEGMENT}
            onClick={() => {
              api.setMat(size.key);
              onReturnFocus();
            }}
          >
            {size.label}
          </button>
        ))}
      </div>
      <button type="button" className={BUTTON} aria-label="Grip" aria-expanded={gripOpen} onClick={() => setGripOpen(!gripOpen)}>
        <span>Grip</span>
        <span className="flex items-center gap-1">
          <Swatch colour={mat.gripColour} />
          {mat.gripWord}
          <ChevronDown size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        </span>
      </button>
      {gripOpen ? (
        <div role="listbox" aria-label="Grip" className="flex flex-col">
          {mat.grips.map((grip) => (
            <button
              key={grip.key}
              type="button"
              role="option"
              aria-selected={grip.key === mat.grip}
              className="flex h-7 items-center gap-1 rounded px-2 text-[13px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
              onClick={() => {
                api.setGrip(grip.key);
                setGripOpen(false);
                onReturnFocus();
              }}
            >
              <Swatch colour={grip.colour} />
              <span className="flex-1 text-left">{grip.word}</span>
              {grip.key === mat.grip ? <Check size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden /> : null}
            </button>
          ))}
        </div>
      ) : null}
      <div className="flex justify-between">
        <PanelSwitch label="Mat grid" checked={mat.grid} onCheckedChange={(on) => api.setGrid(on)} onReturnFocus={onReturnFocus} />
      </div>
    </section>
  );
}

function Swatch({ colour }: { colour: string }) {
  return <span aria-hidden className="inline-block h-3 w-3 rounded-sm border border-[var(--lane-line)]" style={{ background: colour }} />;
}

function OperationsSection({ view, api, onReturnFocus }: { view: PlotterView; api: PlotterApi; onReturnFocus: () => void }) {
  const [unfolded, setUnfolded] = useState<string | null>(null);
  const key = (row: PlotterRow) => operationIdText(row.id);
  return (
    <section aria-label="Operations" className="flex flex-col gap-2">
      <h3 className={HEADING}>Operations</h3>
      <div className="flex gap-1" role="group" aria-label="Operation">
        {view.kinds.map((kind) => (
          <button
            key={kind.kind}
            type="button"
            className={SEGMENT}
            aria-description={kind.description}
            title={kind.description}
            onClick={(event) => {
              api.press(kind.kind, event.shiftKey);
              onReturnFocus();
            }}
          >
            {kind.label}
          </button>
        ))}
      </div>
      {view.operations.map((row) => (
        <OperationRow
          key={key(row)}
          row={row}
          api={api}
          unfolded={unfolded === key(row)}
          onUnfold={() => setUnfolded(unfolded === key(row) ? null : key(row))}
        />
      ))}
    </section>
  );
}

function OperationRow({
  row,
  api,
  unfolded,
  onUnfold,
}: {
  row: PlotterRow;
  api: PlotterApi;
  unfolded: boolean;
  onUnfold: () => void;
}) {
  const name = row.unfolds ? row.name : `${row.name}, run ${row.number}, set in Design Space`;
  return (
    <div className="flex flex-col gap-1 rounded-md p-1 hover:bg-[var(--editor-accent-hover)]" data-plotter-row={row.kind ?? ""}>
      <div className="flex items-center gap-2">
        <button
          type="button"
          aria-label={name}
          aria-expanded={row.unfolds ? unfolded : undefined}
          className="flex min-w-0 flex-1 flex-col items-start text-left outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          onClick={() => {
            if (row.unfolds) onUnfold();
          }}
        >
          <span className="truncate text-sm text-[var(--toolbar-icon)]">{row.name}</span>
          <span className="text-xs text-[var(--panel-muted-fg)]">
            <span className="tabular-nums">{row.number}</span> · {row.line2}
          </span>
          <span className="text-[11px] text-[var(--panel-muted-fg)]">{row.line3}</span>
        </button>
        <button
          type="button"
          aria-label={`Remove selection from ${row.name}`}
          className="h-7 rounded px-1 text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          onClick={() => api.removeSelection(operationIdText(row.id))}
        >
          −
        </button>
        <button
          type="button"
          aria-label={`Delete ${row.name}`}
          className="flex h-7 w-7 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          onClick={() => api.remove(operationIdText(row.id))}
        >
          <Trash2 size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        </button>
      </div>
      {unfolded && row.unfolds && row.numbers ? (
        <div className="grid grid-cols-2 gap-1">
          <NumberField label="Pressure (%)" value={row.numbers.pressure} onCommit={(t) => api.setNumber(operationIdText(row.id), "pressure", t)} />
          {row.kind === "cut" ? (
            <NumberField label="Passes (×)" value={row.numbers.passes} onCommit={(t) => api.setNumber(operationIdText(row.id), "passes", t)} />
          ) : null}
          <div className="col-span-2">
            <NumberField label="Speed (mm/s)" value={row.numbers.speed} onCommit={(t) => api.setNumber(operationIdText(row.id), "speed", t)} />
          </div>
        </div>
      ) : null}
    </div>
  );
}

function OutputSection({
  view,
  onExport,
  onToggleToolpath,
  onSelectItems,
}: {
  view: PlotterView;
  onExport: () => void;
  onToggleToolpath: (page: string) => void;
  onSelectItems: (ids: OperationId[]) => void;
}) {
  return (
    <section aria-label="Output" className="flex flex-col gap-2">
      <h3 className={HEADING}>Output</h3>
      <p className="text-xs text-[var(--toolbar-icon)]">{view.machineLine}</p>
      {view.checks.map((check, index) => (
        <div
          key={`${check.severity}-${index}`}
          role={check.severity === "block" ? "alert" : "status"}
          data-severity={check.severity}
          className="rounded-md p-2 text-xs text-[var(--toolbar-icon)]"
          style={{ border: "1px solid color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
        >
          {check.items.length > 0 ? (
            <button type="button" className="text-left underline-offset-2 hover:underline" onClick={() => onSelectItems(check.items)}>
              {check.text}
            </button>
          ) : (
            check.text
          )}
        </div>
      ))}
      <div className="flex flex-col text-xs text-[var(--panel-muted-fg)]">
        {view.total.time ? <span className="text-[var(--toolbar-icon)]">{view.total.time}</span> : null}
        <span>{view.total.runs}</span>
        <span className="tabular-nums">{view.total.lengths}</span>
      </div>
      <PanelSwitchRow label="Toolpath view" checked={view.toolpath} onChange={() => onToggleToolpath(view.page)} />
      <button type="button" className={BUTTON} disabled={!view.exportAllowed} onClick={onExport}>
        <span>Export job file</span>
        <span className="text-[var(--panel-muted-fg)]">{view.fileType}</span>
      </button>
    </section>
  );
}

function PanelSwitchRow({ label, checked, onChange }: { label: string; checked: boolean; onChange: () => void }) {
  return (
    <div className="flex justify-between">
      <PanelSwitch label={label} checked={checked} onCheckedChange={onChange} onReturnFocus={() => undefined} />
    </div>
  );
}

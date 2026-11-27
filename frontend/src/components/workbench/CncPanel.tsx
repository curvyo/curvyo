import { Check, ChevronDown, Pause, Play, Plus, Trash2 } from "lucide-react";
import { type KeyboardEvent, useEffect, useRef, useState } from "react";

import { PanelSwitch } from "@/components/ui/switch";
import { useLinkEntry } from "@/hooks/useLinkEntry";
import { type CncRow, type CncTool, type CncView, cncPositionAt } from "@/lib/cncView";
import { sendLinkOf } from "@/lib/connectorLink";
import { type OperationId, operationIdText } from "@/lib/operationsView";
import { isTauri } from "@/platform/host";

/** The commands the panel calls; each is one step in the session (`specs/0161`), except `play`,
 * `setConnector` (a record edit) and `send`. */
export interface CncApi {
  pick: (id: string) => boolean;
  saveAs: () => string;
  /** The connector address of the page's user profile; the message or "". */
  setConnector: (address: string) => string;
  /** `z_travel_mm`, `spindle_max_rpm`, `safe_height_mm`; the message or "". */
  setMachine: (field: string, text: string) => string;
  /** A stock field; the message or "". */
  setStock: (field: string, text: string) => string;
  addTool: () => string;
  setTool: (id: string, field: string, text: string) => string;
  removeTool: (id: string) => boolean;
  press: (side: string, shift: boolean) => string;
  removeSelection: (id: string) => boolean;
  remove: (id: string) => boolean;
  /** A Profile field by its stored key; the message or "". */
  setNumber: (id: string, field: string, text: string) => string;
  /** The play position in seconds (session state, no step). */
  play: (seconds: number) => void;
  /** Send to machine: the job goes to the page's connector. */
  send: () => void;
}

interface CncPanelProps {
  view: CncView;
  api: CncApi;
  onExport: () => void;
  onToggleToolpath: (page: string) => void;
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
const ICON = "flex h-7 w-7 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]";

/** The play speeds of the Toolpath view. */
const SPEEDS = [1, 4, 16];

/**
 * A CNC page's panel (`specs/0161-cnc-output` slice 1, UX notes 3 to 8): Machine (profile pick,
 * work area, Z travel, spindle, safe height, Output, connector), Stock, Tools, Operations (Profile
 * outside and inside; rows with tool, side, depth, direction and feeds) and Output (checks, total,
 * Toolpath view with its play bar, Export, Send). Laser rows are absent.
 */
export function CncPanel({ view, api, onExport, onToggleToolpath, onSelectItems, onReturnFocus }: CncPanelProps) {
  return (
    <div className="flex flex-col gap-4" data-cnc-panel="">
      <MachineSection view={view} api={api} onReturnFocus={onReturnFocus} />
      <StockSection view={view} api={api} />
      <ToolsSection view={view} api={api} />
      <OperationsSection view={view} api={api} onReturnFocus={onReturnFocus} />
      <OutputSection view={view} api={api} onExport={onExport} onToggleToolpath={onToggleToolpath} onSelectItems={onSelectItems} />
      {view.notice ? (
        <p role="status" className="text-xs text-[var(--panel-muted-fg)]">
          {view.notice}
        </p>
      ) : null}
    </div>
  );
}

/** A field: Enter or leaving commits; a refused value stays with the session's message. */
function TextField({ label, shown, unit, value, onCommit, numeric = true }: {
  label: string;
  shown: string;
  unit: string;
  value: string;
  onCommit: (text: string) => string;
  numeric?: boolean;
}) {
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
      <span className="flex justify-between">
        <span>{shown}</span>
        <span>{unit}</span>
      </span>
      <input
        className={FIELD}
        inputMode={numeric ? "decimal" : "text"}
        aria-label={label}
        aria-invalid={message !== ""}
        value={text ?? value}
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

function Segments({ label, options, value, onPick }: {
  label: string;
  options: { key: string; label: string }[];
  value: string;
  onPick: (key: string) => void;
}) {
  return (
    <div role="radiogroup" aria-label={label} className="flex gap-1">
      {options.map((option) => (
        <button
          key={option.key}
          type="button"
          role="radio"
          aria-checked={option.key === value}
          tabIndex={option.key === value ? 0 : -1}
          className={SEGMENT}
          onClick={() => onPick(option.key)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

function MachineSection({ view, api, onReturnFocus }: { view: CncView; api: CncApi; onReturnFocus: () => void }) {
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
        <span>Work area</span>
        <span className="tabular-nums">{profile.workArea}</span>
      </div>
      <div className="grid grid-cols-2 gap-1">
        <TextField label="Z travel, millimetres" shown="Z travel" unit="mm" value={String(profile.zTravel)} onCommit={(t) => api.setMachine("z_travel_mm", t)} />
        <TextField label="Safe height, millimetres" shown="Safe height" unit="mm" value={String(profile.safeHeight)} onCommit={(t) => api.setMachine("safe_height_mm", t)} />
      </div>
      <TextField label="Spindle max, rpm" shown="Spindle max" unit="rpm" value={String(profile.spindleMax)} onCommit={(t) => api.setMachine("spindle_max_rpm", t)} />
      <div className={ROW} aria-label={`Output: ${profile.output}`}>
        <span>Output</span>
        <span>{profile.output}</span>
      </div>
      {profile.connector !== null ? (
        <TextField label="Connector address" shown="Connector" unit="" numeric={false} value={profile.connector} onCommit={api.setConnector} />
      ) : null}
    </section>
  );
}

function StockSection({ view, api }: { view: CncView; api: CncApi }) {
  const { stock } = view;
  return (
    <section aria-label="Stock" className="flex flex-col gap-2">
      <h3 className={HEADING}>Stock</h3>
      <div className="grid grid-cols-2 gap-1">
        <TextField label="Stock width, millimetres" shown="Width" unit="mm" value={String(stock.width)} onCommit={(t) => api.setStock("width_mm", t)} />
        <TextField label="Stock height, millimetres" shown="Height" unit="mm" value={String(stock.height)} onCommit={(t) => api.setStock("height_mm", t)} />
      </div>
      <div className="grid grid-cols-2 gap-1">
        <TextField label="Stock thickness, millimetres" shown="Thickness" unit="mm" value={String(stock.thickness)} onCommit={(t) => api.setStock("thickness_mm", t)} />
      </div>
      <TextField label="Stock material" shown="Material" unit="" numeric={false} value={stock.material} onCommit={(t) => api.setStock("material", t)} />
    </section>
  );
}

/** The numbers of a tool (criterion 4): label, unit, stored key. */
const TOOL_FIELDS: { label: string; unit: string; key: keyof CncTool["numbers"]; name: string }[] = [
  { label: "Diameter", unit: "mm", key: "diameter_mm", name: "Diameter, millimetres" },
  { label: "Flutes", unit: "", key: "flutes", name: "Flutes" },
  { label: "Feed", unit: "mm/min", key: "feed_mm_min", name: "Feed, millimetres per minute" },
  { label: "Plunge", unit: "mm/min", key: "plunge_mm_min", name: "Plunge, millimetres per minute" },
  { label: "Spindle", unit: "rpm", key: "rpm", name: "Spindle, rpm" },
  { label: "Step-down", unit: "mm", key: "step_down_mm", name: "Step-down, millimetres" },
];

function ToolsSection({ view, api }: { view: CncView; api: CncApi }) {
  const [unfolded, setUnfolded] = useState<string | null>(null);
  const [armed, setArmed] = useState<string | null>(null);
  const last = view.tools.length === 1;
  return (
    <section aria-label="Tools" className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <h3 className={HEADING}>Tools</h3>
        <button
          type="button"
          className="flex h-7 items-center gap-1 rounded border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] px-2 text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          onClick={() => setUnfolded(api.addTool() || unfolded)}
        >
          <Plus size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          Tool
        </button>
      </div>
      {view.tools.map((tool) => (
        <div key={tool.id} className="flex flex-col gap-1 rounded-md p-1" data-cnc-tool="" style={armed === tool.id ? { background: "color-mix(in srgb, var(--destructive) 12%, transparent)" } : undefined}>
          <div className="flex items-center gap-2">
            <button
              type="button"
              aria-label={`${tool.name}, ${tool.line2}`}
              aria-expanded={unfolded === tool.id}
              className="flex min-w-0 flex-1 flex-col items-start text-left outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              onClick={() => setUnfolded(unfolded === tool.id ? null : tool.id)}
            >
              <span className="truncate text-sm text-[var(--toolbar-icon)]">{tool.name}</span>
              <span className="text-[11px] text-[var(--panel-muted-fg)]">{tool.line2}</span>
            </button>
            {last ? null : (
              <button
                type="button"
                aria-label={armed === tool.id ? `Press again to delete ${tool.name}` : `Delete ${tool.name}`}
                className={ICON}
                onBlur={() => setArmed(null)}
                onClick={() => {
                  if (armed === tool.id) {
                    api.removeTool(tool.id);
                    setArmed(null);
                  } else setArmed(tool.id);
                }}
              >
                <Trash2 size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
              </button>
            )}
          </div>
          {unfolded === tool.id ? (
            <div className="grid grid-cols-2 gap-1 pl-4">
              {TOOL_FIELDS.map((field) => (
                <TextField
                  key={field.key}
                  label={field.name}
                  shown={field.label}
                  unit={field.unit}
                  value={String(tool.numbers[field.key])}
                  onCommit={(text) => api.setTool(tool.id, field.key, text)}
                />
              ))}
            </div>
          ) : null}
        </div>
      ))}
    </section>
  );
}

function OperationsSection({ view, api, onReturnFocus }: { view: CncView; api: CncApi; onReturnFocus: () => void }) {
  const [unfolded, setUnfolded] = useState<string | null>(null);
  const key = (row: CncRow) => operationIdText(row.id);
  return (
    <section aria-label="Operations" className="flex flex-col gap-2">
      <h3 className={HEADING}>Operations</h3>
      <div className="flex gap-1" role="group" aria-label="Operation">
        {view.kinds.map((kind) => (
          <button
            key={kind.side}
            type="button"
            className={SEGMENT}
            aria-description={kind.description}
            title={kind.description}
            onClick={(event) => {
              api.press(kind.side, event.shiftKey);
              onReturnFocus();
            }}
          >
            {kind.side === "outside" ? "Profile outside" : "Profile inside"}
          </button>
        ))}
      </div>
      {view.operations.map((row) => (
        <OperationRow
          key={key(row)}
          row={row}
          view={view}
          api={api}
          unfolded={unfolded === key(row)}
          onUnfold={() => setUnfolded(unfolded === key(row) ? null : key(row))}
        />
      ))}
    </section>
  );
}

function OperationRow({ row, view, api, unfolded, onUnfold }: { row: CncRow; view: CncView; api: CncApi; unfolded: boolean; onUnfold: () => void }) {
  const id = operationIdText(row.id);
  return (
    <div className="flex flex-col gap-1 rounded-md p-1 hover:bg-[var(--editor-accent-hover)]" data-cnc-row={row.side ?? ""}>
      <div className="flex items-center gap-2">
        <button
          type="button"
          aria-label={`${row.name}, run ${row.number}${row.time ? `, ${row.time}` : ""}`}
          aria-expanded={row.unfolds ? unfolded : undefined}
          className="flex min-w-0 flex-1 flex-col items-start text-left outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          onClick={() => {
            if (row.unfolds) onUnfold();
          }}
        >
          <span className="truncate text-sm text-[var(--toolbar-icon)]">{row.name}</span>
          <span className="flex w-full justify-between text-xs text-[var(--panel-muted-fg)]">
            <span>
              <span className="inline-flex h-4 min-w-4 items-center justify-center rounded-full border border-[var(--lane-line)] px-1 tabular-nums">{row.number}</span>{" "}
              {row.line2}
            </span>
            {row.time ? <span className="tabular-nums">{row.time}</span> : null}
          </span>
          <span className="text-[11px] text-[var(--panel-muted-fg)]">{row.line3}</span>
        </button>
        <button type="button" aria-label={`Remove selection from ${row.name}`} className="h-7 rounded px-1 text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]" onClick={() => api.removeSelection(id)}>
          −
        </button>
        <button type="button" aria-label={`Delete ${row.name}`} className={ICON} onClick={() => api.remove(id)}>
          <Trash2 size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        </button>
      </div>
      {unfolded && row.numbers ? <RowBody row={row} view={view} api={api} /> : null}
    </div>
  );
}

function RowBody({ row, view, api }: { row: CncRow; view: CncView; api: CncApi }) {
  const id = operationIdText(row.id);
  const n = row.numbers;
  if (!n) return null;
  const tool = view.tools.find((t) => t.id === n.tool);
  return (
    <div className="flex flex-col gap-1">
      <span className={HEADING}>Tool</span>
      <Segments label="Tool" options={view.tools.map((t) => ({ key: t.id, label: t.name }))} value={n.tool} onPick={(key) => api.setNumber(id, "tool", key)} />
      <Segments
        label="Side"
        options={[
          { key: "outside", label: "Outside" },
          { key: "inside", label: "Inside" },
        ]}
        value={n.side}
        onPick={(key) => api.setNumber(id, "side", key)}
      />
      <div className="grid grid-cols-2 items-end gap-1">
        <TextField label="Depth, millimetres" shown="Depth" unit="mm" value={String(n.depth_mm)} onCommit={(t) => api.setNumber(id, "depth_mm", t)} />
        <span className="pb-1 text-xs text-[var(--panel-muted-fg)]">
          {row.passes} {row.passes === 1 ? "pass" : "passes"}
        </span>
      </div>
      <Segments
        label="Direction"
        options={[
          { key: "conventional", label: "Conventional" },
          { key: "climb", label: "Climb" },
        ]}
        value={n.direction}
        onPick={(key) => api.setNumber(id, "direction", key)}
      />
      <TextField label="Feed, millimetres per minute" shown="Feed" unit="mm/min" value={String(n.feed_mm_min)} onCommit={(t) => api.setNumber(id, "feed_mm_min", t)} />
      {n.feedTyped ? <span className="text-[11px] text-[var(--panel-muted-fg)]">Tool {n.toolFeed}</span> : null}
      <TextField label="Plunge, millimetres per minute" shown="Plunge" unit="mm/min" value={String(n.plunge_mm_min)} onCommit={(t) => api.setNumber(id, "plunge_mm_min", t)} />
      {n.plungeTyped ? <span className="text-[11px] text-[var(--panel-muted-fg)]">Tool {n.toolPlunge}</span> : null}
      {(n.feedTyped || n.plungeTyped) && tool ? (
        <button
          type="button"
          className="self-start text-xs text-[var(--editor-accent)] hover:underline"
          onClick={() => {
            api.setNumber(id, "feed_mm_min", "");
            api.setNumber(id, "plunge_mm_min", "");
          }}
        >
          Reset to tool
        </button>
      ) : null}
    </div>
  );
}

function PlayBar({ view, api }: { view: CncView; api: CncApi }) {
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(16);
  const { play } = view;
  const latest = useRef({ position: play.position, length: play.length, api });
  useEffect(() => {
    latest.current = { position: play.position, length: play.length, api };
  });
  useEffect(() => {
    if (!playing) return;
    const { position, length } = latest.current;
    const start = position >= length ? 0 : position;
    const began = performance.now();
    let frame = requestAnimationFrame(function tick(now) {
      const { length: total, api: commands } = latest.current;
      const next = cncPositionAt(start, (now - began) / 1000, speed, total);
      commands.play(next);
      if (next >= total) {
        setPlaying(false);
        return;
      }
      frame = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(frame);
  }, [playing, speed]);
  const Glyph = playing ? Pause : Play;
  return (
    <div className="flex min-w-0 flex-col gap-1" data-cnc-play="">
      <div className="flex items-center gap-2">
        <button type="button" aria-label={playing ? "Pause" : "Play"} className={ICON} onClick={() => setPlaying(!playing)}>
          <Glyph size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        </button>
        <input
          type="range"
          className="min-w-0 flex-1"
          aria-label="Run position"
          aria-valuetext={play.text.replace("−", "minus ")}
          min={0}
          max={play.length}
          step={0.1}
          value={play.position}
          onChange={(event) => {
            setPlaying(false);
            api.play(Number(event.target.value));
          }}
        />
      </div>
      <div className="flex items-center justify-between gap-2 text-xs tabular-nums text-[var(--toolbar-icon)]">
        <span>{play.text}</span>
        <Segments label="Speed" options={SPEEDS.map((s) => ({ key: String(s), label: `${s}×` }))} value={String(speed)} onPick={(key) => setSpeed(Number(key))} />
      </div>
    </div>
  );
}

/** Send to machine and the connector's state line (`0125`, `0118`): only in the desktop app. */
function SendRow({ send, api }: { send: NonNullable<CncView["send"]>; api: CncApi }) {
  const entry = useLinkEntry(send.profileId, send.address, isTauri, true);
  const paired = isTauri && sendLinkOf(entry) === "paired";
  const status = entry.status;
  const job = status?.job ?? null;
  return (
    <div className="flex flex-col gap-1">
      <button type="button" className={BUTTON} disabled={!send.enabled || !paired || entry.pageSend !== null} onClick={api.send}>
        <span>Send to machine</span>
        <span className="text-[var(--panel-muted-fg)]">{send.address}</span>
      </button>
      {status ? (
        <span role="status" className="text-xs tabular-nums text-[var(--toolbar-icon)]">
          {status.state.name}
          {job ? ` · line ${job.line} of ${job.lines}` : ""}
        </span>
      ) : null}
    </div>
  );
}

function OutputSection({ view, api, onExport, onToggleToolpath, onSelectItems }: {
  view: CncView;
  api: CncApi;
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
        {view.total.time ? <span className="text-[13px] font-semibold text-[var(--toolbar-icon)]">{view.total.time}</span> : null}
        <span>{view.total.runs}</span>
        <span className="tabular-nums">{view.total.lengths}</span>
      </div>
      <div className="flex justify-between">
        <PanelSwitch label="Toolpath view" checked={view.toolpath} onCheckedChange={() => onToggleToolpath(view.page)} onReturnFocus={() => undefined} />
      </div>
      {view.toolpath && view.play.length > 0 ? <PlayBar view={view} api={api} /> : null}
      <button type="button" className={BUTTON} disabled={!view.exportAllowed} onClick={onExport}>
        <span>Export job file</span>
        <span className="text-[var(--panel-muted-fg)]">{view.fileType}</span>
      </button>
      {view.send ? <SendRow send={view.send} api={api} /> : null}
    </section>
  );
}

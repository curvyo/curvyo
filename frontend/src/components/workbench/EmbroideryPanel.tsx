import { Check, ChevronDown, Pause, Play, Trash2 } from "lucide-react";
import { type KeyboardEvent, useEffect, useRef, useState } from "react";

import { ColourPicker } from "@/components/panel/ColourPicker";
import { PanelSwitch } from "@/components/ui/switch";
import type { HsvTriple } from "@/hooks/useStylePanel";
import { type EmbroideryRow, type EmbroideryView, hexOf, packedOf, positionAt } from "@/lib/embroideryView";
import { type OperationId, operationIdText } from "@/lib/operationsView";

/** The commands the panel calls; each is one step in the session (`specs/0160`), except `play`. */
export interface EmbroideryApi {
  pick: (id: string) => boolean;
  saveAs: () => string;
  /** The Speed field; the field message or "". */
  setSpeed: (text: string) => string;
  setHoop: (key: string) => boolean;
  press: (kind: string, shift: boolean) => string;
  removeSelection: (id: string) => boolean;
  remove: (id: string) => boolean;
  /** A number by its stored key; the field message or "". */
  setNumber: (id: string, field: string, text: string) => string;
  setThread: (id: string, hex: string) => boolean;
  /** Moves the simulation (session state, no step). */
  play: (position: number) => void;
  hsvOf: (rgb: number) => HsvTriple;
  rgbOf: (hue: number, saturation: number, value: number) => number;
}

interface EmbroideryPanelProps {
  view: EmbroideryView;
  api: EmbroideryApi;
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

/** The play speeds: 1× is the profile's machine speed (UX E13). */
const SPEEDS = [1, 4, 16];

/**
 * An embroidery page's panel (`specs/0160-embroidery` slice 1, UX notes 3 to 7): Machine (profile
 * pick, hoop, speed, max stitch, Output), Hoop (the profile's hoops), Operations (Running stitch,
 * Tatami fill, Satin; rows with thread and numbers) and Output (checks, total, Toolpath view with
 * its play bar, Export). No font controls (Fragen 422 C, 423); laser rows are absent.
 */
export function EmbroideryPanel({ view, api, onExport, onToggleToolpath, onSelectItems, onReturnFocus }: EmbroideryPanelProps) {
  return (
    <div className="flex flex-col gap-4" data-embroidery-panel="">
      <MachineSection view={view} api={api} onReturnFocus={onReturnFocus} />
      <HoopSection view={view} api={api} onReturnFocus={onReturnFocus} />
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

function MachineSection({ view, api, onReturnFocus }: { view: EmbroideryView; api: EmbroideryApi; onReturnFocus: () => void }) {
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
        <span>Hoop</span>
        <span className="tabular-nums">{profile.hoop}</span>
      </div>
      <TextField label="Speed (stitches per minute)" shown="Speed" unit="stitches/min" value={String(profile.speed)} onCommit={api.setSpeed} />
      <div className={ROW}>
        <span>Max stitch</span>
        <span className="tabular-nums">{profile.maxStitch}</span>
      </div>
      <div className={ROW} aria-label={`Output: ${profile.output}`}>
        <span>Output</span>
        <span>{profile.output}</span>
      </div>
    </section>
  );
}

/** A number field: Enter or leaving commits; a refused value shows the session's message. */
function TextField({
  label,
  shown,
  unit,
  value,
  onCommit,
}: {
  label: string;
  shown: string;
  unit: string;
  value: string;
  onCommit: (text: string) => string;
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
        inputMode="decimal"
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

function Segments({
  label,
  options,
  value,
  onPick,
}: {
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

function HoopSection({ view, api, onReturnFocus }: { view: EmbroideryView; api: EmbroideryApi; onReturnFocus: () => void }) {
  return (
    <section aria-label="Hoop" className="flex flex-col gap-2">
      <h3 className={HEADING}>Hoop</h3>
      <Segments
        label="Hoop"
        options={view.hoop.sizes}
        value={view.hoop.key}
        onPick={(key) => {
          api.setHoop(key);
          onReturnFocus();
        }}
      />
    </section>
  );
}

function Swatch({ colour }: { colour: string | null }) {
  return (
    <span
      aria-hidden
      className="inline-block h-3 w-3 shrink-0 rounded-sm border border-[var(--swatch-border,var(--lane-line))]"
      style={{ background: colour ?? "transparent" }}
    />
  );
}

function OperationsSection({ view, api, onReturnFocus }: { view: EmbroideryView; api: EmbroideryApi; onReturnFocus: () => void }) {
  const [unfolded, setUnfolded] = useState<string | null>(null);
  const key = (row: EmbroideryRow) => operationIdText(row.id);
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

/** The numbers of a kind (criterion 6): label, unit, stored key. */
const NUMBERS: Record<string, { label: string; unit: string; key: keyof NonNullable<EmbroideryRow["numbers"]>; name: string }[]> = {
  running: [{ label: "Stitch length", unit: "mm", key: "stitch_length_mm", name: "Stitch length, millimetres" }],
  tatami: [
    { label: "Angle", unit: "°", key: "angle_deg", name: "Angle, degrees" },
    { label: "Row spacing", unit: "mm", key: "row_spacing_mm", name: "Row spacing, millimetres" },
    { label: "Max stitch", unit: "mm", key: "max_stitch_mm", name: "Max stitch, millimetres" },
  ],
  satin: [{ label: "Zigzag spacing", unit: "mm", key: "zigzag_spacing_mm", name: "Zigzag spacing, millimetres" }],
};

function OperationRow({ row, api, unfolded, onUnfold }: { row: EmbroideryRow; api: EmbroideryApi; unfolded: boolean; onUnfold: () => void }) {
  const id = operationIdText(row.id);
  const name = `${row.name}, ${row.thread ?? ""}, run ${row.number}${row.time ? `, ${row.time}` : ""}`;
  return (
    <div className="flex flex-col gap-1 rounded-md p-1 hover:bg-[var(--editor-accent-hover)]" data-embroidery-row={row.kind ?? ""}>
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
          <span className="flex min-w-0 items-center gap-1 text-sm text-[var(--toolbar-icon)]">
            <Swatch colour={row.thread} />
            <span className="truncate">{row.name}</span>
          </span>
          <span className="flex w-full justify-between text-xs text-[var(--panel-muted-fg)]">
            <span>
              <span
                className="inline-flex h-4 min-w-4 items-center justify-center rounded-full border border-[var(--lane-line)] px-1 tabular-nums"
                style={row.startsColour && row.thread ? { background: row.thread, color: "var(--selection-casing, #fff)" } : undefined}
              >
                {row.number}
              </span>{" "}
              {row.line2}
            </span>
            {row.time ? <span className="tabular-nums">{row.time}</span> : null}
          </span>
          <span className="text-[11px] text-[var(--panel-muted-fg)]">{row.line3}</span>
        </button>
        <button
          type="button"
          aria-label={`Remove selection from ${row.name}`}
          className="h-7 rounded px-1 text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          onClick={() => api.removeSelection(id)}
        >
          −
        </button>
        <button
          type="button"
          aria-label={`Delete ${row.name}`}
          className="flex h-7 w-7 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          onClick={() => api.remove(id)}
        >
          <Trash2 size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        </button>
      </div>
      {unfolded && row.unfolds && row.numbers && row.kind ? <RowBody row={row} api={api} /> : null}
    </div>
  );
}

function RowBody({ row, api }: { row: EmbroideryRow; api: EmbroideryApi }) {
  const id = operationIdText(row.id);
  const [picking, setPicking] = useState(false);
  const preview = useRef<number | null>(null);
  const numbers = row.numbers;
  if (!numbers || !row.kind) return null;
  return (
    <div className="flex flex-col gap-1">
      <button type="button" className={BUTTON} aria-label={`Thread, ${row.thread ?? ""}`} aria-expanded={picking} onClick={() => setPicking(!picking)}>
        <span>Thread</span>
        <span className="flex items-center gap-1 tabular-nums">
          <Swatch colour={row.thread} />
          {row.thread}
        </span>
      </button>
      {picking ? (
        <div
          onKeyDown={(event) => {
            if (event.key === "Escape") setPicking(false);
          }}
        >
          <ColourPicker
            name="Thread"
            rgb={packedOf(row.thread)}
            mixed={false}
            hsvOf={api.hsvOf}
            rgbOf={api.rgbOf}
            width={228}
            onPreview={(hue, saturation, value) => {
              preview.current = api.rgbOf(hue, saturation, value);
            }}
            onSettle={() => {
              if (preview.current !== null) api.setThread(id, hexOf(preview.current));
              preview.current = null;
            }}
            onOpenEntry={() => undefined}
          />
          <TextField label="Thread, hex" shown="Hex" unit="" value={row.thread ?? ""} onCommit={(text) => (api.setThread(id, text.trim()) ? "" : "")} />
        </div>
      ) : null}
      {row.kind === "running" ? (
        <Segments
          label="Repeats"
          options={[1, 3, 5].map((n) => ({ key: String(n), label: `${n}×` }))}
          value={String(numbers.repeats)}
          onPick={(key) => api.setNumber(id, "repeats", key)}
        />
      ) : null}
      <div className="grid grid-cols-2 gap-1">
        {NUMBERS[row.kind].map((field) => (
          <TextField
            key={field.key}
            label={field.name}
            shown={field.label}
            unit={field.unit}
            value={String(numbers[field.key])}
            onCommit={(text) => api.setNumber(id, field.key, text)}
          />
        ))}
      </div>
    </div>
  );
}

function PlayBar({ view, api }: { view: EmbroideryView; api: EmbroideryApi }) {
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(16);
  const { play } = view;
  // The latest values for the animation; the effect itself restarts only on Play and Speed.
  const latest = useRef({ position: play.position, length: play.length, api, rate: view.stitchesPerSecond });
  useEffect(() => {
    latest.current = { position: play.position, length: play.length, api, rate: view.stitchesPerSecond };
  });
  useEffect(() => {
    if (!playing) return;
    const { position, length } = latest.current;
    const start = position >= length ? 0 : position;
    const began = performance.now();
    let frame = requestAnimationFrame(function tick(now) {
      const { length: total, api: commands, rate } = latest.current;
      const next = positionAt(start, (now - began) / 1000, speed, rate, total);
      commands.play(next);
      if (next >= total) {
        setPlaying(false);
        return;
      }
      frame = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(frame);
  }, [playing, speed]);
  const valueText = `Stitch ${play.text}, colour ${play.colourText}`;
  const Glyph = playing ? Pause : Play;
  return (
    <div className="flex min-w-0 flex-col gap-1" data-embroidery-play="">
      <div className="flex items-center gap-2">
        <button
          type="button"
          aria-label={playing ? "Pause" : "Play"}
          className="flex h-7 w-7 shrink-0 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          onClick={() => setPlaying(!playing)}
        >
          <Glyph size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        </button>
        <input
          type="range"
          className="min-w-0 flex-1"
          aria-label="Run position"
          aria-valuetext={valueText}
          min={0}
          max={play.length}
          step={1}
          value={play.position}
          onChange={(event) => {
            setPlaying(false);
            api.play(Number(event.target.value));
          }}
        />
      </div>
      <div className="flex items-center justify-between gap-2 text-xs tabular-nums text-[var(--toolbar-icon)]">
        <span className="flex items-center gap-1">
          {play.text} · <Swatch colour={play.colour} /> {play.colourText}
        </span>
        <Segments label="Speed" options={SPEEDS.map((n) => ({ key: String(n), label: `${n}×` }))} value={String(speed)} onPick={(key) => setSpeed(Number(key))} />
      </div>
    </div>
  );
}

function OutputSection({
  view,
  api,
  onExport,
  onToggleToolpath,
  onSelectItems,
}: {
  view: EmbroideryView;
  api: EmbroideryApi;
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
        <span className="tabular-nums">{view.total.stitches}</span>
      </div>
      <div className="flex justify-between">
        <PanelSwitch label="Toolpath view" checked={view.toolpath} onCheckedChange={() => onToggleToolpath(view.page)} onReturnFocus={() => undefined} />
      </div>
      {view.toolpath && view.play.length > 0 ? <PlayBar view={view} api={api} /> : null}
      <button type="button" className={BUTTON} disabled={!view.exportAllowed} onClick={onExport}>
        <span>Export job file</span>
        <span className="text-[var(--panel-muted-fg)]">{view.fileType}</span>
      </button>
    </section>
  );
}

import { useEffect, useRef, useState } from "react";

import { ToolbarSwitch } from "@/components/ToolbarSwitch";
import { setBridgeSettings, useBridgeSettings } from "@/hooks/useBridgeSettings";
import {
  BRIDGES_RANGE,
  type BridgeMode,
  type FieldRange,
  REACH_RANGE,
  WIDTH_RANGE,
  clampField,
} from "@/lib/bridgeText";

/** How long a limit notice stays (UX notes §4.4). */
const NOTICE_MS = 3000;

const FIELD_CLASS =
  "h-7 w-16 rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-white px-1.5 text-right text-sm tabular-nums outline-none focus:ring-2 focus:ring-[var(--editor-accent)]";

const BUTTON_CLASS =
  "my-1 h-7 whitespace-nowrap rounded-md px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

/** Up to two decimals, no trailing zeros. */
function formatValue(value: number): string {
  return String(Math.round(value * 100) / 100);
}

interface NumberFieldProps {
  label: string;
  /** The accessible name, as "Width, millimetres". */
  name: string;
  /** "mm" for a length; "" for a count. */
  unit: string;
  range: FieldRange;
  step: number;
  value: number;
  onCommit: (value: number) => void;
  onNotice: (text: string) => void;
  onReturnFocus: () => void;
}

/** A number field whose typed value commits on Enter or blur, clamped with a limit notice; a
 * stepper click or an arrow key commits at once (criterion 5). */
function NumberField({ label, name, unit, range, step, value, onCommit, onNotice, onReturnFocus }: NumberFieldProps) {
  const shown = formatValue(value);
  const [text, setText] = useState(shown);
  const [invalid, setInvalid] = useState(false);
  const editing = useRef(false);
  useEffect(() => {
    if (!editing.current) {
      setText(shown);
    }
  }, [shown]);

  const commit = (raw: string) => {
    const clamped = clampField(range, label, raw.trim() === "" ? Number.NaN : Number(raw), unit === "" ? "" : ` ${unit}`);
    if (clamped === null) {
      setInvalid(true);
      onNotice("Enter a number");
      return;
    }
    setInvalid(false);
    const next = step >= 1 ? Math.round(clamped.value) : clamped.value;
    if (clamped.notice) {
      onNotice(clamped.notice);
    }
    setText(formatValue(next));
    onCommit(next);
  };

  return (
    <label className="flex items-center gap-1.5 text-[var(--toolbar-icon)]">
      {label}
      <input
        type="number"
        min={range.min}
        max={range.max}
        step={step}
        aria-label={name}
        aria-invalid={invalid || undefined}
        value={text}
        onFocus={() => {
          editing.current = true;
        }}
        onBlur={(event) => {
          editing.current = false;
          commit(event.currentTarget.value);
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit(event.currentTarget.value);
            event.currentTarget.blur();
            onReturnFocus();
          } else if (event.key === "Escape") {
            event.preventDefault();
            setText(shown);
            setInvalid(false);
            event.currentTarget.blur();
            onReturnFocus();
          }
        }}
        onChange={(event) => {
          setText(event.target.value);
          // A stepper click or an arrow key fires an input event without a text-editing input
          // type: one commit each. Typing waits for Enter or blur.
          const inputType = (event.nativeEvent as InputEvent).inputType ?? "";
          if (!/^(insert(?!Replacement)|delete)/.test(inputType)) {
            commit(event.target.value);
          }
        }}
        className={`${FIELD_CLASS} ${invalid ? "border-2 border-[var(--field-invalid)]" : ""}`}
        style={{ color: "var(--toolbar-icon)" }}
      />
      {unit === "" ? null : <span>{unit}</span>}
    </label>
  );
}

const MODES: readonly { mode: BridgeMode; label: string }[] = [
  { mode: "lettering", label: "Lettering" },
  { mode: "stencil", label: "Stencil" },
];

/** The Mode pill: Lettering | Stencil (criterion 21, S7). One Tab stop, arrows choose. */
function ModePill({ mode, onChange }: { mode: BridgeMode; onChange: (mode: BridgeMode) => void }) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  return (
    <div
      role="radiogroup"
      aria-label="Mode"
      className="my-1 flex h-7 overflow-hidden rounded-md ring-[1.5px] ring-[var(--toolbar-icon)]"
    >
      {MODES.map((entry, index) => {
        const active = entry.mode === mode;
        return (
          <button
            key={entry.mode}
            ref={(element) => {
              buttons.current[index] = element;
            }}
            type="button"
            role="radio"
            aria-checked={active}
            tabIndex={active ? 0 : -1}
            onClick={() => onChange(entry.mode)}
            onKeyDown={(event) => {
              if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) {
                event.preventDefault();
                const next = event.key === "Home" ? 0 : event.key === "End" ? MODES.length - 1 : (index + 1) % MODES.length;
                onChange(MODES[next].mode);
                buttons.current[next]?.focus();
              }
            }}
            className="px-2 text-xs outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            style={
              active
                ? { background: "var(--toolbar-icon-active-bg)", color: "var(--toolbar-icon-active-fg)" }
                : { color: "var(--toolbar-icon)" }
            }
          >
            {entry.label}
          </button>
        );
      })}
    </div>
  );
}

export interface BridgeGroupProps {
  /** Enabled: the Select tool with one or more objects on a laser page. */
  ready: boolean;
  /** Runs Bridge with the current values (the rail's busy rules and notice). */
  onBridge: () => void;
  onReturnFocus: () => void;
}

/**
 * The Bridge group of the Select bar (`specs/0156-fused-lettering/`; customer answer of
 * 2026-11-26 to Frage 395: no chip, no preview, the fields in the Select bar): Mode, Width, Reach
 * and Join all in Lettering, Bridges per island in Stencil, and the Bridge button. The values are
 * app preferences shared with the Boolean card's Bridge button. Only labels that name a control
 * or a state; no hint text.
 */
export function BridgeGroup({ ready, onBridge, onReturnFocus }: BridgeGroupProps) {
  const settings = useBridgeSettings();
  const [notice, setNotice] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);
  const showNotice = (text: string) => {
    setNotice(text);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setNotice(null), NOTICE_MS);
  };
  const stencil = settings.mode === "stencil";
  return (
    <div role="group" aria-label="Bridge" className="relative flex flex-wrap content-start items-center gap-x-2 py-1">
      <ModePill mode={settings.mode} onChange={(mode) => setBridgeSettings({ mode })} />
      <NumberField
        label="Width"
        name="Width, millimetres"
        unit="mm"
        range={WIDTH_RANGE}
        step={0.1}
        value={settings.widthMm}
        onCommit={(widthMm) => setBridgeSettings({ widthMm })}
        onNotice={showNotice}
        onReturnFocus={onReturnFocus}
      />
      {stencil ? (
        <NumberField
          label="Bridges"
          name="Bridges per island"
          unit=""
          range={BRIDGES_RANGE}
          step={1}
          value={settings.bridgesPerIsland}
          onCommit={(bridgesPerIsland) => setBridgeSettings({ bridgesPerIsland })}
          onNotice={showNotice}
          onReturnFocus={onReturnFocus}
        />
      ) : (
        <>
          <NumberField
            label="Reach"
            name="Reach, millimetres"
            unit="mm"
            range={REACH_RANGE}
            step={0.1}
            value={settings.reachMm}
            onCommit={(reachMm) => setBridgeSettings({ reachMm })}
            onNotice={showNotice}
            onReturnFocus={onReturnFocus}
          />
          <ToolbarSwitch
            label="Join all"
            tooltip="Join all"
            checked={settings.joinAll}
            onCheckedChange={(joinAll) => setBridgeSettings({ joinAll })}
          />
        </>
      )}
      <button
        type="button"
        aria-disabled={!ready}
        onClick={() => {
          if (ready) {
            onBridge();
          }
        }}
        className={`${BUTTON_CLASS} ${ready ? "" : "opacity-40"}`}
      >
        Bridge
      </button>
      <div
        role="status"
        className="absolute top-full left-0 mt-1 w-max max-w-[320px] rounded-md px-2 py-1 text-xs"
        style={notice ? { background: "var(--popover)", color: "var(--toolbar-icon)" } : { display: "none" }}
      >
        {notice}
      </div>
    </div>
  );
}

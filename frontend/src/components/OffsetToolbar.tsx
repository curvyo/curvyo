import { useRef, useState } from "react";
import type { ReactNode } from "react";

import { CapIcon, JoinIcon } from "@/components/panel/StyleIcons";
import type { OffsetApi } from "@/hooks/useOffset";
import {
  type OffsetCapName,
  type OffsetDirectionName,
  type OffsetJoinName,
  type OffsetResultName,
  formatDistance,
  offsetStatus,
} from "@/lib/offsetText";

interface OffsetToolbarProps {
  offset: OffsetApi;
  /** Returns the keyboard focus to the canvas (Escape in the bar). */
  onReturnFocus: () => void;
}

interface Choice<T extends string> {
  value: T;
  label: string;
  icon?: ReactNode;
}

const DIRECTIONS: Choice<OffsetDirectionName>[] = [
  { value: "out", label: "Out" },
  { value: "in", label: "In" },
  { value: "both", label: "Both" },
];

const CORNERS: Choice<OffsetJoinName>[] = [
  { value: "round", label: "Round", icon: <JoinIcon join="round" /> },
  { value: "miter", label: "Miter", icon: <JoinIcon join="miter" /> },
  { value: "bevel", label: "Bevel", icon: <JoinIcon join="bevel" /> },
];

const ENDS: Choice<OffsetCapName>[] = [
  { value: "round", label: "Round", icon: <CapIcon cap="round" /> },
  { value: "flat", label: "Flat", icon: <CapIcon cap="butt" /> },
  { value: "square", label: "Square", icon: <CapIcon cap="square" /> },
];

const RESULTS: Choice<OffsetResultName>[] = [
  { value: "new", label: "New object" },
  { value: "change", label: "Change original" },
];

const ITEM_CLASS =
  "flex h-7 items-center justify-center px-2 text-xs text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-ring aria-disabled:opacity-40";

/** One radio group of the bar: one Tab stop, arrows move and choose (criterion 25). */
function RadioGroup<T extends string>({
  name,
  choices,
  value,
  disabled,
  onChange,
}: {
  name: string;
  choices: Choice<T>[];
  value: T;
  disabled: boolean;
  onChange: (value: T) => void;
}) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const current = Math.max(
    0,
    choices.findIndex((choice) => choice.value === value),
  );
  return (
    <div
      role="radiogroup"
      aria-label={name}
      aria-disabled={disabled || undefined}
      className="flex overflow-hidden rounded-md ring-1 ring-border"
    >
      {choices.map((choice, index) => (
        <button
          key={choice.value}
          ref={(element) => {
            buttons.current[index] = element;
          }}
          type="button"
          role="radio"
          aria-checked={choice.value === value}
          aria-label={choice.label}
          aria-disabled={disabled || undefined}
          title={choice.label}
          tabIndex={index === current ? 0 : -1}
          onClick={() => {
            if (!disabled) {
              onChange(choice.value);
            }
          }}
          onKeyDown={(event) => {
            const step =
              event.key === "ArrowRight" || event.key === "ArrowDown"
                ? 1
                : event.key === "ArrowLeft" || event.key === "ArrowUp"
                  ? -1
                  : 0;
            if (step === 0) {
              return;
            }
            event.preventDefault();
            const next = (index + step + choices.length) % choices.length;
            buttons.current[next]?.focus();
            if (!disabled) {
              onChange(choices[next].value);
            }
          }}
          className={ITEM_CLASS}
          style={
            choice.value === value
              ? {
                  background: "var(--toolbar-icon-active-bg)",
                  color: "var(--toolbar-icon-active-fg)",
                }
              : undefined
          }
        >
          {choice.icon ?? choice.label}
        </button>
      ))}
    </div>
  );
}

function Divider() {
  return <div className="mx-1 h-5 w-px bg-border" aria-hidden />;
}

/** The Distance field (criteria 6, 25): typed, with arrow steps; Enter commits and previews,
 * Enter again applies; Escape restores the typed text, then returns to the canvas. */
function DistanceField({
  distanceMm,
  inputRef,
  setDistance,
  apply,
  disabled,
  onReturnFocus,
}: {
  distanceMm: number;
  inputRef: OffsetApi["distanceRef"];
  setDistance: OffsetApi["setDistance"];
  apply: OffsetApi["apply"];
  disabled: boolean;
  onReturnFocus: () => void;
}) {
  const shown = formatDistance(distanceMm);
  const [typed, setTyped] = useState<string | null>(null);
  const [invalid, setInvalid] = useState(false);
  const commit = (text: string): boolean => {
    const value = Number(text.replace(",", ".").replace("−", "-"));
    if (text.trim() === "" || !Number.isFinite(value)) {
      setInvalid(true);
      return false;
    }
    setInvalid(!setDistance(value));
    setTyped(null);
    return true;
  };
  return (
    <label className="flex items-center gap-1 text-xs text-[var(--toolbar-icon)]">
      Distance
      <span className="relative flex items-center">
        <input
          ref={inputRef}
          type="text"
          inputMode="decimal"
          aria-label="Distance, millimetres"
          aria-disabled={disabled || undefined}
          aria-invalid={invalid || undefined}
          value={typed ?? shown}
          onChange={(event) => setTyped(event.target.value)}
          onBlur={() => {
            if (typed !== null) {
              commit(typed);
            }
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              if (typed !== null) {
                commit(typed);
              } else if (!disabled) {
                apply();
              }
            } else if (event.key === "Escape") {
              event.preventDefault();
              if (typed !== null) {
                setTyped(null);
                setInvalid(false);
              } else {
                onReturnFocus();
              }
            } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
              event.preventDefault();
              const step = event.shiftKey ? 1 : event.ctrlKey || event.metaKey ? 0.01 : 0.1;
              const sign = event.key === "ArrowUp" ? 1 : -1;
              const base = typed !== null ? Number(typed) : distanceMm;
              const next = Math.round((base + sign * step) * 1000) / 1000;
              setTyped(null);
              setInvalid(!setDistance(next));
            }
          }}
          className="h-7 w-20 rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-white pr-7 pl-1.5 text-right text-xs outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-invalid:border-[var(--field-invalid)]"
        />
        <span className="pointer-events-none absolute right-1.5 text-[var(--panel-muted-fg)]">mm</span>
      </span>
    </label>
  );
}

/**
 * The Offset bar (`docs/design-system.md`, row "Offset bar"; `specs/0038-path-offset` criteria 4
 * to 8, 25): in the overlay row while the Offset tool is active, every control always present and
 * greyed (`aria-disabled`) when it does not apply, with the reason in the status text. It wraps by
 * whole groups. The values are the session's; the bar decides nothing.
 */
export function OffsetToolbar({ offset, onReturnFocus }: OffsetToolbarProps) {
  const { view } = offset;
  const none = view.operands === 0;
  return (
    <div
      data-context-bar=""
      className="pointer-events-auto flex min-h-9 max-w-full min-w-0 flex-wrap items-center gap-x-3 gap-y-1 rounded-lg px-2 py-1"
      style={{
        background: "var(--toolbar-bg)",
        boxShadow: "var(--panel-elevation-shadow)",
      }}
      onFocus={() => offset.setBarFocus(true)}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
          offset.setBarFocus(false);
        }
      }}
    >
      <DistanceField
        distanceMm={view.barDistanceMm}
        inputRef={offset.distanceRef}
        setDistance={offset.setDistance}
        apply={offset.apply}
        disabled={none}
        onReturnFocus={onReturnFocus}
      />
      <Divider />
      <RadioGroup
        name="Direction"
        choices={DIRECTIONS}
        value={view.direction}
        disabled={none}
        onChange={offset.setDirection}
      />
      <Divider />
      <div className="flex items-center gap-1 text-xs text-[var(--toolbar-icon)]">
        Corners
        <RadioGroup
          name="Corners"
          choices={CORNERS}
          value={view.join}
          disabled={none}
          onChange={offset.setJoin}
        />
      </div>
      <Divider />
      <div className="flex items-center gap-1 text-xs text-[var(--toolbar-icon)]">
        Ends
        <RadioGroup
          name="Ends"
          choices={ENDS}
          value={view.cap}
          disabled={none || !view.hasOpen}
          onChange={offset.setCap}
        />
      </div>
      <Divider />
      <RadioGroup
        name="Result"
        choices={RESULTS}
        value={view.result}
        disabled={none}
        onChange={offset.setResult}
      />
      <Divider />
      <button
        type="button"
        aria-label="Apply offset"
        aria-disabled={none || undefined}
        onClick={() => {
          if (!none) {
            offset.apply();
          }
        }}
        className="h-7 rounded-md px-3 text-xs font-medium outline-none focus-visible:ring-2 focus-visible:ring-ring aria-disabled:opacity-40"
        style={{
          background: "var(--toolbar-icon-active-bg)",
          color: "var(--toolbar-icon-active-fg)",
        }}
      >
        Apply
      </button>
      <div
        role="status"
        aria-live="polite"
        className="w-[200px] truncate text-xs text-[var(--panel-muted-fg)]"
      >
        {offsetStatus(view)}
      </div>
    </div>
  );
}

import { useContext, useId, useRef, useState } from "react";

import { FieldNoticeContext } from "@/components/workbench/fieldNotice";
import { cleanName, noticeUnder } from "@/lib/fixtureSets";

/** A row's name in place: Enter or leaving commits, Escape cancels (`0113` UX notes 4.1, 5.2). */
export function RenameField({
  name,
  label,
  onCommit,
  onCancel,
}: {
  name: string;
  label: string;
  onCommit: (text: string, byKey: boolean) => void;
  onCancel: () => void;
}) {
  const [text, setText] = useState(name);
  const done = useRef(false);
  const finish = (commit: boolean, byKey = false) => {
    if (done.current) return;
    done.current = true;
    if (commit) onCommit(text, byKey);
    else onCancel();
  };
  return (
    <input
      autoFocus
      onFocus={(event) => event.currentTarget.select()}
      aria-label={label}
      maxLength={40}
      value={text}
      onChange={(event) => setText(event.target.value)}
      onBlur={() => finish(true)}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          finish(true, true);
        } else if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          finish(false);
        }
      }}
      className="h-6 min-w-0 flex-1 rounded-[4px] border border-[var(--editor-accent)] bg-white px-1 text-[13px] outline-none"
    />
  );
}

/**
 * A bar number field: the label inside the left edge, the unit inside the right; Enter or blur
 * commits, Escape reverts, Up and Down step by 1 (Shift 10) and commit on key-up. While nothing is
 * typed the field shows the stored value, so a value the session clamped reads as what was kept.
 */
export function NumberField({
  label,
  unit,
  value,
  onCommit,
  width = 244,
  whole = false,
  describedBy,
  fill = false,
  disabled = false,
  readOnly = false,
}: {
  label: string;
  unit: string;
  value: number;
  onCommit: (value: number) => void;
  /** The field's width in px. */
  width?: number;
  /** A whole number: typed decimals are rounded. */
  whole?: boolean;
  describedBy?: string;
  /** Fills its container's width instead of keeping `width`, so it never leaves a narrow panel. */
  fill?: boolean;
  /** Greyed out and not editable, still showing its value. */
  disabled?: boolean;
  /** A locked fixture's value (`0113` criterion 32): label, value and unit as text, no field. */
  readOnly?: boolean;
}) {
  const shown = String(Number(value.toFixed(3)));
  const [draft, setDraft] = useState<string | null>(null);
  const text = draft ?? shown;
  const id = useId();
  const notices = useContext(FieldNoticeContext);
  const notice = noticeUnder(notices?.notice ?? null, id);
  const commit = () => {
    if (draft === null) return;
    const number = Number(draft.replace(",", "."));
    setDraft(null);
    if (draft.trim() === "" || !Number.isFinite(number)) return;
    const next = whole ? Math.round(number) : number;
    if (next === value) return;
    notices?.claim(id);
    onCommit(next);
    // A clamp that lands on the stored value changes nothing in the view; the field reads it again.
  };
  if (readOnly) return <ValueLine label={label} unit={unit} value={shown} fill={fill} width={width} />;
  return (
    // The column keeps the field's own width; its clamp notice wraps under it.
    <div className={fill ? "flex min-w-0 flex-1 flex-col items-stretch gap-1" : "flex shrink-0 flex-col items-start gap-1 self-start"}>
      {/* `width` is the least width: a value the label leaves no room for widens the field (its
          unit wraps whole), it is never cut to its last digits. */}
      <label
        className={`flex h-7 shrink-0 items-center gap-1 rounded-[5px] border border-[var(--field-border)] bg-white px-2 text-sm focus-within:ring-2 focus-within:ring-[var(--editor-accent)] ${fill ? "min-w-0" : ""} ${disabled ? "opacity-50" : ""}`}
        style={fill ? undefined : { minWidth: width }}
        aria-disabled={disabled || undefined}
      >
        {/* Filling a narrow row, the label gives way before the value is ever cut. */}
        <span className={`text-xs text-[var(--panel-muted-fg)] ${fill ? "min-w-0 truncate" : "shrink-0"}`}>{label}</span>
        <input
          aria-label={unit === "" ? label : `${label} (${unit})`}
          aria-description={describedBy}
          inputMode="decimal"
          disabled={disabled}
          value={text}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              commit();
            } else if (event.key === "Escape") {
              event.preventDefault();
              setDraft(null);
            } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
              event.preventDefault();
              const current = Number((draft ?? shown).replace(",", "."));
              if (!Number.isFinite(current)) return;
              const step = (event.shiftKey ? 10 : 1) * (event.key === "ArrowUp" ? 1 : -1);
              setDraft(String(Number((current + step).toFixed(3))));
            }
          }}
          onKeyUp={(event) => {
            if (event.key === "ArrowUp" || event.key === "ArrowDown") commit();
          }}
          className={`${fill ? "shrink-0 grow" : "min-w-0 flex-1"} bg-transparent text-right tabular-nums outline-none disabled:cursor-not-allowed`}
          style={{ width: `${Math.max(text.length, 1) + 0.5}ch` }}
        />
        {unit === "" ? null : (
          <span aria-hidden className="shrink-0 text-xs text-[var(--panel-muted-fg)]">
            {unit}
          </span>
        )}
      </label>
      {notice ? (
        <p role="status" className="w-0 min-w-full text-xs text-[var(--toolbar-icon)]">
          {notice}
        </p>
      ) : null}
    </div>
  );
}

/** A value shown as text where its field would stand: the label left, value and unit right. */
function ValueLine({ label, unit, value, fill, width }: { label: string; unit: string; value: string; fill: boolean; width: number }) {
  return (
    <div
      className={`flex h-7 items-center gap-1 px-2 text-sm ${fill ? "min-w-0 flex-1" : "shrink-0 self-start"}`}
      style={fill ? undefined : { minWidth: width }}
    >
      <span className={`text-xs text-[var(--panel-muted-fg)] ${fill ? "min-w-0 truncate" : "shrink-0"}`}>{label}</span>
      <span className="ml-auto shrink-0 tabular-nums text-[var(--toolbar-icon)]">{value}</span>
      {unit === "" ? null : (
        <span aria-hidden className="shrink-0 text-xs text-[var(--panel-muted-fg)]">
          {unit}
        </span>
      )}
    </div>
  );
}

/** A text field (a slot's name); Enter or blur commits, Escape reverts. */
export function TextField({ label, value, onCommit }: { label: string; value: string; onCommit: (text: string) => void }) {
  const [text, setText] = useState(value);
  const commit = () => {
    const clean = cleanName(text);
    if (clean !== null && clean !== value) onCommit(clean);
  };
  return (
    <label className="flex h-7 w-[244px] items-center gap-1 rounded-[5px] border border-[var(--field-border)] bg-white px-2 text-sm focus-within:ring-2 focus-within:ring-[var(--editor-accent)]">
      <span className="shrink-0 text-xs text-[var(--panel-muted-fg)]">{label}</span>
      <input
        aria-label={label}
        maxLength={40}
        value={text}
        onChange={(event) => setText(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit();
          } else if (event.key === "Escape") {
            event.preventDefault();
            setText(value);
          }
        }}
        className="min-w-0 flex-1 bg-transparent outline-none"
      />
    </label>
  );
}

import { useRef } from "react";

import { OverrideRow } from "@/components/panel/CloneBlock";
import { EntryField } from "@/components/panel/EntryField";
import { LeadGlyph } from "@/components/panel/OperationGlyphs";
import { Tooltip } from "@/components/ui/tooltip";
import type { OperationToolsApi } from "@/hooks/useOperationTools";
import { PATH_FIELDS, optionStep, optionText, type OptionField } from "@/lib/operationOptionRows";
import type { LeadKind, PathOptionValues } from "@/lib/operationSettingsView";
import type { Direction, OperationRow } from "@/lib/operationsView";
import { CHIP_IS_MESSAGE, outcomeOf, typedNumber } from "@/lib/settingsFieldText";

const LEADS: readonly LeadKind[] = ["none", "line", "arc"];
const LEAD_NAMES: Record<LeadKind, string> = { none: "None", line: "Line", arc: "Arc" };
const DIRECTIONS: readonly Direction[] = ["automatic", "as_drawn", "reversed"];
const DIRECTION_NAMES: Record<Direction, string> = { automatic: "Automatic", as_drawn: "As drawn", reversed: "Reversed" };

/**
 * A radiogroup of segments with one Tab stop (roving tabindex); the arrows move and set, Space and
 * Enter set, a pointer press returns the focus to the row. `value` `null` presses none (mixed).
 */
function SegmentPill<T extends string>({
  name,
  values,
  value,
  segment,
  width,
  onSet,
  onReturnFocus,
}: {
  name: string;
  values: readonly T[];
  value: T | null;
  segment: (value: T) => { label: React.ReactNode; name: string };
  width: string;
  onSet: (value: T) => void;
  onReturnFocus: () => void;
}) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const current = value === null ? 0 : values.indexOf(value);
  const set = (index: number, keyboard: boolean) => {
    const next = values[index];
    if (next === undefined) return;
    if (next !== value) onSet(next);
    if (keyboard) refs.current[index]?.focus();
    else onReturnFocus();
  };
  return (
    <div
      role="radiogroup"
      aria-label={name}
      className={`flex h-7 shrink-0 rounded-[5px] border-[1.5px] border-[var(--toolbar-icon)] ${width}`}
      onKeyDown={(event) => {
        const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
        if (step === 0) return;
        event.preventDefault();
        set((current + step + values.length) % values.length, true);
      }}
    >
      {values.map((v, index) => {
        const on = v === value;
        const look = segment(v);
        return (
          <Tooltip key={v} side="bottom" content={look.name}>
            <button
              ref={(node) => {
                refs.current[index] = node;
              }}
              type="button"
              role="radio"
              aria-checked={on}
              aria-label={look.name}
              tabIndex={index === current ? 0 : -1}
              onClick={(event) => set(index, event.detail === 0)}
              className="flex flex-1 items-center justify-center rounded-[3px] text-xs text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-checked:bg-[var(--toolbar-icon-active-bg)] aria-checked:text-[var(--toolbar-icon-active-fg)]"
            >
              {look.label}
            </button>
          </Tooltip>
        );
      })}
    </div>
  );
}

function LengthField({
  field,
  shown,
  onSubmit,
  onReturnFocus,
}: {
  field: OptionField;
  shown: number;
  onSubmit: (value: number) => string;
  onReturnFocus: () => void;
}) {
  return (
    <EntryField
      label={field.name}
      prefix={field.label}
      shown={optionText(shown)}
      mixed={false}
      suffix={field.unit}
      width={field.width}
      gutter={32}
      commitOnBlur
      inputMode="decimal"
      onSubmit={(text) => outcomeOf(onSubmit(typedNumber(text)))}
      messages={CHIP_IS_MESSAGE}
      stepText={optionStep(field)}
      onReturnFocus={onReturnFocus}
    />
  );
}

/**
 * The three rows of path options (`ux-notes.md` 2.3): Lead-in and Lead-out (label, None | Line |
 * Arc pill, length field absent on None) and Overcut. `edit` writes one field: the operation's in
 * the lower part, the items' own in the item part.
 */
function PathRows({
  values,
  edit,
  onReturnFocus,
}: {
  values: PathOptionValues;
  edit: (option: string, value: number) => string;
  onReturnFocus: () => void;
}) {
  const lead = (which: "in" | "out") => {
    const kind = which === "in" ? values.leadIn : values.leadOut;
    const label = which === "in" ? "Lead-in" : "Lead-out";
    return (
      <div className="flex items-center gap-1">
        <span className="w-14 shrink-0 text-xs text-[var(--toolbar-icon)]">{label}</span>
        <SegmentPill
          name={label}
          values={LEADS}
          value={kind}
          width="w-[84px]"
          segment={(v) => ({ label: <LeadGlyph kind={v} mirrored={which === "out"} />, name: LEAD_NAMES[v] })}
          onSet={(v) => edit(which === "in" ? "lead_in" : "lead_out", LEADS.indexOf(v))}
          onReturnFocus={onReturnFocus}
        />
        {kind !== "none" ? (
          <LengthField
            field={which === "in" ? PATH_FIELDS.leadIn : PATH_FIELDS.leadOut}
            shown={which === "in" ? values.leadInMm : values.leadOutMm}
            onSubmit={(mm) => edit(which === "in" ? "lead_in_mm" : "lead_out_mm", mm)}
            onReturnFocus={onReturnFocus}
          />
        ) : null}
      </div>
    );
  };
  return (
    <>
      {lead("in")}
      {lead("out")}
      <LengthField field={PATH_FIELDS.overcut} shown={values.overcutMm} onSubmit={(mm) => edit("overcut", mm)} onReturnFocus={onReturnFocus} />
    </>
  );
}

const HAIRLINE = { background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" };

/**
 * The Path options group of an unfolded Cut or Cut-out row (`specs/0139` criterion 13,
 * `ux-notes.md` 2): heading with the Lead glyph, Lead-in, Lead-out and Overcut. Absent on every
 * other kind (hidden, not disabled).
 */
export function OperationPathGroup({ row, tools, onReturnFocus }: { row: OperationRow; tools: OperationToolsApi; onReturnFocus: () => void }) {
  const path = row.settings?.options?.path ?? null;
  if (path === null) return null;
  return (
    <div className="flex flex-col gap-1">
      <div aria-hidden className="my-1 h-px" style={HAIRLINE} />
      <div className="flex h-6 items-center gap-1 text-xs font-semibold text-[var(--toolbar-icon)]">
        <LeadGlyph kind="line" size={14} />
        Path options
      </div>
      <PathRows values={path} edit={(option, value) => tools.edit(row.id, option, value)} onReturnFocus={onReturnFocus} />
    </div>
  );
}

/**
 * The path rows of the item part (`specs/0139` criteria 19 to 21, `ux-notes.md` 3.2 rows 2 to 4):
 * the Path options pill with the operation's summary or the own rows, the Start readout with the
 * Automatic button while any is set, and the Direction pick of open paths.
 */
export function OperationItemPathRows({ row, tools, onReturnFocus }: { row: OperationRow; tools: OperationToolsApi; onReturnFocus: () => void }) {
  const item = row.itemPath;
  if (item === null) return null;
  const { options, start, direction } = item;
  return (
    <>
      {options ? (
        <>
          <OverrideRow
            label="Path options"
            segments={[
              { own: false, label: "Operation's", name: "Operation's", title: "Operation's" },
              { own: true, label: "Own", name: "Own", title: "Own" },
            ]}
            state={options.own === null ? "mixed" : options.own ? "own" : "original"}
            onSet={(own) => tools.setPathOwn(row.id, own)}
            onReturnFocus={onReturnFocus}
            onPressedAt={() => undefined}
          />
          {options.summary !== null ? <p className="text-xs text-[var(--panel-muted-fg)]">{options.summary}</p> : null}
          {options.values !== null ? (
            <PathRows values={options.values} edit={(option, value) => tools.editOwnPath(row.id, option, value)} onReturnFocus={onReturnFocus} />
          ) : null}
        </>
      ) : null}
      {start ? (
        <div className="flex h-7 items-center gap-1">
          <span className="w-14 shrink-0 text-xs text-[var(--toolbar-icon)]">Start</span>
          <span className={`flex-1 text-xs ${start.set ? "text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}`}>{start.readout}</span>
          {start.set ? (
            <button
              type="button"
              aria-label="Automatic start"
              onClick={() => {
                tools.setStartAutomatic(row.id);
                onReturnFocus();
              }}
              className="h-7 rounded-md border border-[var(--toolbar-icon)] px-2 text-xs text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            >
              Automatic
            </button>
          ) : null}
        </div>
      ) : null}
      {direction ? (
        <div className="flex flex-col gap-1">
          <span className="text-xs text-[var(--panel-muted-fg)]">Direction</span>
          <SegmentPill
            name="Direction"
            values={DIRECTIONS}
            value={direction.value}
            width="w-full"
            segment={(v) => ({ label: DIRECTION_NAMES[v], name: DIRECTION_NAMES[v] })}
            onSet={(v) => tools.setDirection(row.id, v)}
            onReturnFocus={onReturnFocus}
          />
        </div>
      ) : null}
    </>
  );
}

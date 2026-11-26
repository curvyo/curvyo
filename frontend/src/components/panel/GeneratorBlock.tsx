import { ChevronDown, ChevronRight } from "lucide-react";
import { useRef, useState } from "react";

import { ValueField } from "@/components/panel/ValueField";
import { PanelSwitch } from "@/components/ui/switch";
import { ToggleGroup, type ToggleOption } from "@/components/ui/toggle-group";
import type { GeneratorBlockApi } from "@/hooks/useGenerators";
import {
  type GeneratedRow,
  automaticWords,
  dragSpan,
  parseNumber,
  positionOf,
  rowText,
  sliderEnd,
  shownNumber,
  stepValue,
  unitOfRow,
  unitWordsOf,
  valueAt,
} from "@/lib/generatedView";

const MESSAGES = { number: "Enter a number" } as const;
const WIDTH = 244;
/** The group of the joint layout, collapsed until opened. */
const ADVANCED = "Advanced";

/** One number row: the panel's Value field, the filled-bar slider of Opacity and Width, drawn on
 * the track the schema gives it (`specs/0141` revision 2, criterion 9, UX notes §15). */
function NumberRow({
  row,
  api,
  onReturnFocus,
}: {
  row: GeneratedRow;
  api: GeneratorBlockApi;
  onReturnFocus: () => void;
}) {
  // The value of the last drag or key tick, committed once when the gesture ends.
  const pending = useRef<number | null>(null);
  // An automatic row stored as 0 shows the value the generator used, muted.
  const current = row.used ?? (typeof row.value === "number" ? row.value : 0);
  const stored = typeof row.value === "number" ? row.value : 0;
  const automatic = row.used !== null;
  const unit = unitOfRow(row);
  const { min, limit } = dragSpan(row);
  const bar = positionOf(row, current);
  const commit = () => {
    const value = pending.current;
    pending.current = null;
    if (value !== null) {
      api.setValue(row.key, value);
    }
  };
  const defaultNumber = typeof row.defaultValue === "number" ? row.defaultValue : stored;
  // A path box's automatic spacing is the smallest gap laid (`0149` UX notes §5).
  const words =
    api.view?.label === "Path box" && row.key === "slit_spacing" && row.used !== null
      ? ", at the tightest curve, automatic"
      : automaticWords(row);
  return (
    <div onKeyUp={commit}>
      <ValueField
        label={row.label}
        name={row.key === "corner_angle" ? row.label : `${api.view?.label ?? ""} ${row.label.toLowerCase()}`.trim()}
        text={rowText(row)}
        bar={Math.min(1, Math.max(0, bar))}
        dragBase={bar}
        dragLimit={limit}
        dragMin={min}
        mixed={false}
        // A derived field (Pitch diameter) reserves the slot but never offers a reset.
        resettable={row.key !== "pitch_diameter" && stored !== defaultNumber}
        muted={automatic}
        valueText={`${rowText(row)} ${unitWordsOf(row)}${words}`}
        // The tooltip's first line says it too: "Rounded box lid thickness: 3 mm, follows wall
        // thickness" (`specs/0147-rounded-box` UX notes §3).
        tipValue={`${rowText(row)}${unit === "°" ? unit : unit ? ` ${unit}` : ""}${words}`}
        unit={unit}
        valueNow={current}
        unitWords={unitWordsOf(row)}
        valueMin={row.track?.from}
        valueMax={sliderEnd(row)}
        defaultText={
          "automatic" in row.kind && row.kind.automatic
            ? "automatic"
            : `${shownNumber(defaultNumber, 4)}${unit === "°" ? unit : unit ? ` ${unit}` : ""}`
        }
        messages={MESSAGES}
        onPreview={(p, grid) => {
          const value = valueAt(row, p, grid);
          pending.current = value;
          api.preview(row.key, value);
        }}
        onSettle={commit}
        onStep={(steps, grid) => {
          // The canvas follows the key; the key-up is the one commit (UX notes §15.2).
          const next = stepValue(row, pending.current ?? current, steps, grid);
          pending.current = next;
          api.preview(row.key, next);
        }}
        onReset={() => api.setValue(row.key, defaultNumber)}
        onCancel={() => {
          pending.current = null;
          api.cancelPreview();
        }}
        onSubmit={(text) => {
          const number = parseNumber(text);
          return number === null ? "invalid:number" : api.setValue(row.key, number);
        }}
        onReturnFocus={() => {
          commit();
          onReturnFocus();
        }}
      />
    </div>
  );
}

function ChoiceRow({
  row,
  api,
  onReturnFocus,
}: {
  row: GeneratedRow;
  api: GeneratorBlockApi;
  onReturnFocus: () => void;
}) {
  if (row.kind.kind !== "choice") {
    return null;
  }
  const options: ToggleOption<string>[] = row.kind.options.map((option) => ({
    value: option.id,
    label: option.label,
    tooltip: "",
    icon: <span className="text-sm">{option.label}</span>,
  }));
  return (
    <ToggleGroup
      label={row.label}
      options={options}
      value={typeof row.value === "string" ? row.value : null}
      onChange={(next) => api.setValue(row.key, next)}
      itemWidth={Math.floor(WIDTH / options.length)}
      onReturnFocus={onReturnFocus}
    />
  );
}

/** A clamp notice under the edited row (UX notes §6): the "Action notice" status look, an
 * overlay that moves no row and takes no focus. */
function ClampNotice({ text }: { text: string }) {
  return (
    <p
      role="status"
      className="pointer-events-none absolute top-full right-0 z-10 mt-1 w-max max-w-[244px] break-words rounded-md p-2 text-right text-xs"
      style={{
        background: "var(--toolbar-bg)",
        color: "var(--toolbar-icon)",
        boxShadow: "var(--panel-elevation-shadow)",
      }}
    >
      {text}
    </p>
  );
}

/**
 * The Generator block of the Style tab (`specs/0141-parametric-generators` criteria 1 to 4, UX
 * notes §3): drawn only from the generator's schema, a heading per group and a field per row,
 * then Convert to paths. A row that does not apply is not in the tree. The rules (clamping, the
 * derived Pitch diameter, the notice texts) are Rust's.
 */
export function GeneratorBlock({
  api,
  onReturnFocus,
}: {
  api: GeneratorBlockApi;
  onReturnFocus: () => void;
}) {
  const { view } = api;
  // The Advanced group starts collapsed; its state is not stored (`specs/0155` criterion 18).
  const [advancedOpen, setAdvancedOpen] = useState(false);
  if (!view) {
    return null;
  }
  const groups: { label: string; rows: GeneratedRow[] }[] = [];
  for (const row of view.rows) {
    const last = groups[groups.length - 1];
    if (last && last.label === row.group) {
      last.rows.push(row);
    } else {
      groups.push({ label: row.group, rows: [row] });
    }
  }
  return (
    <div role="group" aria-label={view.label} className="flex flex-col gap-4" data-generator-block>
      {groups.map((group) => (
        <section key={group.label} className="flex flex-col gap-2">
          {group.label === ADVANCED ? (
            <h3 className="text-sm text-[var(--toolbar-icon)]">
              <button
                type="button"
                aria-expanded={advancedOpen}
                data-generator-advanced
                onClick={() => setAdvancedOpen(!advancedOpen)}
                className="flex items-center gap-1 outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              >
                {advancedOpen ? <ChevronDown size={12} aria-hidden="true" /> : <ChevronRight size={12} aria-hidden="true" />}
                {group.label}
              </button>
            </h3>
          ) : (
            <h3 className="text-sm text-[var(--toolbar-icon)]">{group.label}</h3>
          )}
          {group.label === "Size" && view.label === "Path box" ? (
            <button
              type="button"
              data-generator-replace-outline
              onClick={() => {
                api.replaceOutline();
                onReturnFocus();
              }}
              className="h-7 w-[244px] rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            >
              Replace outline
            </button>
          ) : null}
          {(group.label === ADVANCED && !advancedOpen ? [] : group.rows).map((row) => (
            // Each row is the anchor of its clamp notice, 4 px under it (UX notes §6).
            <div key={row.key} className="relative w-fit">
              {row.kind.kind === "switch" ? (
                <div className="flex h-7 items-center">
                  <PanelSwitch
                    label={row.label}
                    checked={row.value === true}
                    onCheckedChange={(on) => api.setValue(row.key, on)}
                    onReturnFocus={onReturnFocus}
                  />
                </div>
              ) : row.kind.kind === "choice" ? (
                <ChoiceRow row={row} api={api} onReturnFocus={onReturnFocus} />
              ) : (
                <NumberRow row={row} api={api} onReturnFocus={onReturnFocus} />
              )}
              {api.notice?.key === row.key ? <ClampNotice text={api.notice.text} /> : null}
            </div>
          ))}
        </section>
      ))}
      <button
        type="button"
        data-generator-convert
        onClick={() => {
          api.convert();
          // The button leaves the tree with the block, by pointer or by key: focus goes to the
          // canvas (UX notes §11).
          onReturnFocus();
        }}
        className="h-7 w-[244px] rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        Convert to paths
      </button>
    </div>
  );
}

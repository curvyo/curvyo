import { ChevronDown, ChevronRight, TriangleAlert } from "lucide-react";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";

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
        resettable={row.key !== "pitch_diameter" && row.key !== "width" && stored !== defaultNumber}
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

/** The rows whose choice strip carries its own visible name: two strips with the same cells
 * (Finder Outer and Inner, `specs/0144` UX notes §3). */
const LABELLED_GROUPS = new Set(["Finder"]);

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
  const count = row.kind.options.length;
  // Four long names wrap into two rows of two (Module shape); four short ones stay in a row
  // (Error correction).
  const wrap = count === 4 && row.kind.options.some((option) => option.label.length > 8);
  const labelled = LABELLED_GROUPS.has(row.group);
  const itemWidth = wrap ? Math.floor(WIDTH / 2) : Math.floor((labelled ? 176 : WIDTH) / count);
  const options: ToggleOption<string>[] = row.kind.options.map((option) => ({
    value: option.id,
    label: option.label,
    tooltip: "",
    icon: <span className="text-sm">{option.label}</span>,
  }));
  const strip = (
    <ToggleGroup
      label={row.label}
      options={options}
      value={typeof row.value === "string" ? row.value : null}
      onChange={(next) => api.setValue(row.key, next)}
      itemWidth={itemWidth}
      columns={wrap ? 2 : undefined}
      onReturnFocus={onReturnFocus}
    />
  );
  return labelled ? (
    <div className="flex w-[244px] items-center gap-2">
      <span className="w-[60px] shrink-0 text-sm text-[var(--toolbar-icon)]">{row.label}</span>
      {strip}
    </div>
  ) : (
    strip
  );
}

/**
 * The multi-line text field (`specs/0144` UX notes §4): the panel's Text field look, one line
 * high growing to five, Enter inserts a line break, Ctrl+Enter commits and returns to the canvas,
 * Tab and a press elsewhere commit, Escape restores. Every keystroke previews, so the code
 * follows the text; the reason content cannot be drawn is a lasting line under it.
 */
function TextRow({
  row,
  api,
  onReturnFocus,
}: {
  row: GeneratedRow;
  api: GeneratorBlockApi;
  onReturnFocus: () => void;
}) {
  const stored = typeof row.value === "string" ? row.value : "";
  const [draft, setDraft] = useState(stored);
  const editing = useRef(false);
  // The blur an Escape or a Ctrl+Enter causes must not commit a second time (or the typed text).
  const skipBlur = useRef(false);
  const field = useRef<HTMLTextAreaElement>(null);
  const reason = useId();
  // The stored text follows the document while the field is not being typed in (undo, selection).
  useEffect(() => {
    if (!editing.current) {
      setDraft(stored);
    }
  }, [stored]);
  // One line high, growing with the text to five lines (108 px), then it scrolls.
  useLayoutEffect(() => {
    const element = field.current;
    if (element) {
      element.style.height = "0px";
      element.style.height = `${Math.min(108, Math.max(28, element.scrollHeight))}px`;
    }
  }, [draft]);
  const commit = () => {
    editing.current = false;
    if (skipBlur.current) {
      skipBlur.current = false;
      return;
    }
    if (draft !== stored) {
      api.setValue(row.key, draft);
    } else {
      api.cancelPreview();
    }
  };
  const invalid = row.problem !== null;
  return (
    <div className="flex w-[244px] flex-col gap-1">
      <textarea
        ref={field}
        data-generator-text={row.key}
        aria-label="QR code content"
        aria-invalid={invalid}
        aria-describedby={invalid ? reason : undefined}
        value={draft}
        rows={1}
        spellCheck={false}
        onFocus={(event) => {
          // Tab in selects all; a click places the caret.
          if (event.currentTarget.matches(":focus-visible")) {
            event.currentTarget.select();
          }
        }}
        onChange={(event) => {
          editing.current = true;
          setDraft(event.target.value);
          api.preview(row.key, event.target.value);
        }}
        onBlur={commit}
        onKeyDown={(event) => {
          // Every key typed here stays here: tool letters, Delete and undo never reach the canvas.
          event.stopPropagation();
          if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
            event.preventDefault();
            commit();
            skipBlur.current = true;
            onReturnFocus();
          } else if (event.key === "Escape") {
            event.preventDefault();
            editing.current = false;
            skipBlur.current = true;
            setDraft(stored);
            api.cancelPreview();
            event.currentTarget.blur();
          }
        }}
        onKeyUp={(event) => event.stopPropagation()}
        className="box-border w-[244px] resize-none break-all rounded-[5px] bg-white px-2 py-[5px] text-sm leading-[18px] text-black outline-none focus:border-[var(--editor-accent)]"
        style={{
          border: invalid
            ? "2px solid var(--field-invalid)"
            : "1px solid color-mix(in srgb, var(--toolbar-icon) 60%, transparent)",
          minHeight: 28,
          maxHeight: 108,
        }}
      />
      {invalid ? (
        <p id={reason} role="status" className="flex items-start gap-1 text-xs" style={{ color: "var(--field-invalid)" }}>
          <TriangleAlert size={12} className="mt-[2px] shrink-0" aria-hidden="true" />
          <span>{row.problem}</span>
        </p>
      ) : null}
    </div>
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

/** The Parts group (`specs/0147-rounded-box` criterion 15, UX notes §9): a row per part with the
 * name of the operation it runs with, muted, and a text button Detach, or Attach once detached. */
function PartsGroup({ api, onReturnFocus }: { api: GeneratorBlockApi; onReturnFocus: () => void }) {
  const parts = api.view?.parts ?? [];
  if (parts.length === 0) {
    return null;
  }
  return (
    <section className="flex flex-col gap-2" data-generator-parts>
      <h3 className="text-sm text-[var(--toolbar-icon)]">Parts</h3>
      {parts.map((part) => (
        <div key={part.key} className="flex h-7 w-[244px] items-center gap-2" data-generator-part={part.key}>
          <span className="min-w-0 flex-1 truncate text-sm text-[var(--toolbar-icon)]">{part.name}</span>
          <span
            className="min-w-0 max-w-[96px] truncate text-xs text-[var(--toolbar-icon)] opacity-60"
            title={part.operation}
          >
            {part.operation}
          </span>
          <button
            type="button"
            disabled={!part.canDetach}
            aria-label={`${part.detached ? "Attach" : "Detach"} ${part.name.toLowerCase()}`}
            onClick={() => {
              if (part.detached) {
                api.attachPart(part.key);
              } else {
                api.detachPart(part.key);
              }
              onReturnFocus();
            }}
            className="h-6 rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] px-2 text-xs text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] disabled:opacity-40"
          >
            {part.detached ? "Attach" : "Detach"}
          </button>
        </div>
      ))}
    </section>
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
              ) : row.kind.kind === "text" ? (
                <TextRow row={row} api={api} onReturnFocus={onReturnFocus} />
              ) : (
                <NumberRow row={row} api={api} onReturnFocus={onReturnFocus} />
              )}
              {api.notice?.key === row.key ? <ClampNotice text={api.notice.text} /> : null}
            </div>
          ))}
          {group.label === "Colours" && view.contrastLow ? (
            <p role="status" className="flex w-[244px] items-start gap-1 text-xs text-[var(--toolbar-icon)]">
              <TriangleAlert size={12} className="mt-[2px] shrink-0" aria-hidden="true" />
              <span>Low contrast, may not scan</span>
            </p>
          ) : null}
        </section>
      ))}
      <PartsGroup api={api} onReturnFocus={onReturnFocus} />
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

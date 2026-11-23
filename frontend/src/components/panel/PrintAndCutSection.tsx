import {
  ChevronDown,
  ChevronRight,
  LocateFixed,
  TriangleAlert,
  X,
} from "lucide-react";
import { useId, useRef, useState } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { Tooltip } from "@/components/ui/tooltip";
import { useLinkEntry } from "@/hooks/useLinkEntry";
import type { PrintAndCutApi } from "@/hooks/usePrintAndCut";
import {
  type MarkRowView,
  headPositionState,
  nextField,
  nextUnmeasured,
  parseMeasure,
  stateIsWarning,
} from "@/lib/printAndCutView";
import { controllerWord } from "@/lib/connectorText";
import { isTauri } from "@/platform/host";

const MEASURE_MESSAGES = {
  range: "Enter a number from -10000 to 10000",
} as const;
const TOLERANCE_MESSAGES = { range: "Between 0.05 and 5 mm" } as const;

const OUTLINE_BUTTON =
  "h-7 rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-disabled:opacity-40 aria-disabled:hover:bg-transparent";
const SYMBOL_BUTTON =
  "flex h-7 w-6 shrink-0 items-center justify-center rounded text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-disabled:opacity-40 aria-disabled:hover:bg-transparent";

interface Props {
  api: PrintAndCutApi;
  /** The laser page's connector, when its profile has one (`specs/0064`). */
  profileId: string | null;
  address: string;
  onReturnFocus: () => void;
}

type Pending = Record<number, { x?: string; y?: string }>;

/** Focuses the element `selector` inside the group once the re-read view is drawn. */
function focusLater(root: HTMLElement | null, selector: string) {
  window.requestAnimationFrame(() =>
    root?.querySelector<HTMLElement>(selector)?.focus(),
  );
}

/** `pending` without the row `mark`. */
function without(pending: Pending, mark: number): Pending {
  const rest = { ...pending };
  delete rest[mark];
  return rest;
}

/**
 * The Print and cut group of a laser page's Operations tab (`specs/0121-print-and-cut` UX notes
 * §4): the sheet, a row per mark with X and Y in machine mm, Take head position and Clear, the
 * fit, the tolerance, the warnings and Apply / Reset placement. Absent while no placed sheet has
 * marks. The session holds every rule and word; a row with only one value is kept here until the
 * other is typed (it counts as empty, criterion 9).
 */
export function PrintAndCutSection({
  api,
  profileId,
  address,
  onReturnFocus,
}: Props) {
  const heading = useId();
  const root = useRef<HTMLElement | null>(null);
  const [open, setOpen] = useState(true);
  const [pending, setPending] = useState<Pending>({});
  const [refusal, setRefusal] = useState<string | null>(null);
  const connected = isTauri && profileId !== null && address !== "";
  const entry = useLinkEntry(
    connected ? profileId : null,
    address,
    connected,
    false,
  );
  const view = api.view;
  if (!view) return null;
  const position = entry.status?.position ?? null;
  // The Enter order of the group's fields (UX notes P14): X and Y of each row, Tolerance, Apply.
  const order = [
    ...view.rows
      .filter((r) => r.kind === "mark")
      .flatMap((r) => [`${r.number}x`, `${r.number}y`]),
    ...(view.show_tolerance ? ["tolerance"] : []),
    "apply",
  ];
  const enterFrom = (key: string) => () => {
    const next = nextField(order, key);
    if (next === null) return onReturnFocus();
    focusLater(
      root.current,
      next === "apply"
        ? "[data-pnc-apply]"
        : `[data-pnc-field="${next}"] input`,
    );
  };
  const applyLabel = view.apply?.label ?? "Apply";
  const applyReason = view.apply ? view.apply.reason : (view.not_usable ?? "");
  const head = headPositionState(
    entry.status?.state.name ?? null,
    position !== null,
    controllerWord(entry.status ?? null),
  );

  const valueOf = (row: MarkRowView, axis: "x" | "y") =>
    pending[row.number]?.[axis] ?? row[axis] ?? "";
  const submit = (row: MarkRowView, axis: "x" | "y", text: string): string => {
    const parsed = parseMeasure(text);
    if (parsed === undefined) return "invalid:range";
    const other = axis === "x" ? "y" : "x";
    const otherValue = parseMeasure(valueOf(row, other));
    const both = parsed !== null && typeof otherValue === "number";
    if (both) {
      const [x, y] = axis === "x" ? [parsed, otherValue] : [otherValue, parsed];
      api.measure(row.number, x, y);
      setPending((all) => without(all, row.number));
    } else {
      setPending((all) => ({
        ...all,
        [row.number]: { ...all[row.number], [axis]: text },
      }));
      if (row.x !== null) api.clear(row.number);
    }
    return "committed";
  };

  return (
    <section
      ref={root}
      aria-labelledby={heading}
      data-print-and-cut
      className="flex flex-col gap-1"
    >
      <button
        id={heading}
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        className="flex h-7 w-full items-center gap-1 rounded text-sm text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        {open ? (
          <ChevronDown size={16} aria-hidden />
        ) : (
          <ChevronRight size={16} aria-hidden />
        )}
        <span>Print and cut</span>
        <span className="ml-auto flex items-center gap-1 text-xs text-[var(--panel-muted-fg)] tabular-nums">
          {view.warn_summary && <TriangleAlert size={14} aria-hidden />}
          {view.summary}
        </span>
      </button>
      {open && (
        <div className="flex flex-col gap-1">
          {view.sheets.length > 1 ? (
            <select
              aria-label="Sheet"
              className={`${OUTLINE_BUTTON} w-full px-2`}
              value={JSON.stringify(view.sheet)}
              onChange={(event) =>
                api.chooseSheet(JSON.parse(event.target.value))
              }
            >
              {view.sheets.map((sheet) => (
                <option
                  key={JSON.stringify(sheet.id)}
                  value={JSON.stringify(sheet.id)}
                >
                  {sheet.name} ({sheet.marks} marks)
                </option>
              ))}
            </select>
          ) : (
            <div className="flex flex-col">
              <span className="text-xs text-[var(--panel-muted-fg)]">
                Sheet
              </span>
              <span
                className="truncate text-[13px] text-[var(--toolbar-icon)]"
                title={view.sheet_name}
              >
                {view.sheet_name}
              </span>
            </div>
          )}
          <span className="text-xs text-[var(--panel-muted-fg)]">
            {view.marks_line}
          </span>
          {view.second_line && (
            <span className="text-xs text-[var(--panel-muted-fg)]">
              {view.second_line}
            </span>
          )}
          {view.unreadable && (
            <p role="status" className="text-xs text-[var(--toolbar-icon)]">
              {view.unreadable}
            </p>
          )}
          <div className="flex gap-1 pl-[18px] text-xs text-[var(--panel-muted-fg)]">
            <span className="w-[86px]">X</span>
            <span className="w-[86px]">Y</span>
          </div>
          {view.rows.map((row) => (
            <div key={row.number} className="flex flex-col">
              <div
                role="group"
                aria-label={`Mark ${row.number}`}
                className="flex items-center gap-1"
              >
                <span
                  aria-hidden
                  className="w-[14px] text-center text-[13px] font-semibold text-[var(--toolbar-icon)]"
                >
                  {row.number}
                </span>
                {(["x", "y"] as const).map((axis) =>
                  row.kind === "mark" ? (
                    <div key={axis} data-pnc-field={`${row.number}${axis}`}>
                      <EntryField
                        label={`Mark ${row.number} ${axis.toUpperCase()}`}
                        shown={valueOf(row, axis)}
                        mixed={false}
                        suffix="mm"
                        width={connected ? 86 : 98}
                        commitOnBlur
                        inputMode="decimal"
                        align="right"
                        onSubmit={(text) => submit(row, axis, text)}
                        messages={MEASURE_MESSAGES}
                        onReturnFocus={onReturnFocus}
                        onEnter={enterFrom(`${row.number}${axis}`)}
                      />
                    </div>
                  ) : (
                    <span
                      key={axis}
                      className="text-right text-[13px] text-[var(--panel-muted-fg)] tabular-nums"
                      style={{ width: connected ? 86 : 98 }}
                    >
                      {row[axis] ?? ""}
                    </span>
                  ),
                )}
                {/* 2 px after Y and none between the two buttons: the row is 240 px, so it fits
                    the 244 px column a classic scrollbar leaves (UX notes §4.3). */}
                <div className="-ml-0.5 flex">
                  {connected && row.kind === "mark" && (
                    <Tooltip
                      side="top"
                      content={
                        head.enabled
                          ? "Take head position"
                          : `Take head position\n${head.cause}`
                      }
                    >
                      <button
                        type="button"
                        aria-label={`Take head position for mark ${row.number}`}
                        data-pnc-take={row.number}
                        aria-disabled={!head.enabled}
                        aria-description={head.enabled ? undefined : head.cause}
                        className={SYMBOL_BUTTON}
                        onClick={() => {
                          if (!head.enabled || !position) return;
                          api.takeHead(row.number, position[0], position[1]);
                          // The loop at the machine: focus goes on to the next mark not measured (P18).
                          const next = nextUnmeasured(view.rows, row.number);
                          if (next !== null)
                            focusLater(
                              root.current,
                              `[data-pnc-take="${next}"]`,
                            );
                        }}
                      >
                        <LocateFixed size={16} aria-hidden />
                      </button>
                    </Tooltip>
                  )}
                  {(row.x !== null || pending[row.number]) && (
                    <Tooltip side="top" content="Clear">
                      <button
                        type="button"
                        aria-label={`Clear mark ${row.number}`}
                        className={SYMBOL_BUTTON}
                        onClick={() => {
                          setPending((all) => without(all, row.number));
                          api.clear(row.number);
                        }}
                      >
                        <X size={16} aria-hidden />
                      </button>
                    </Tooltip>
                  )}
                </div>
              </div>
              {row.note && (
                <span className="pl-[18px] text-xs text-[var(--panel-muted-fg)]">
                  {row.note}
                </span>
              )}
            </div>
          ))}
          <div
            aria-hidden
            className="my-2 h-px"
            style={{
              background:
                "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
            }}
          />
          <div
            role="status"
            aria-atomic="true"
            className="flex flex-col gap-1 text-xs"
          >
            {view.state &&
              (stateIsWarning(view) ? (
                <span className="flex items-start gap-1 text-[var(--toolbar-icon)]">
                  <TriangleAlert size={16} aria-hidden className="shrink-0" />
                  {view.state}
                </span>
              ) : (
                <span className="text-[var(--panel-muted-fg)]">
                  {view.state}
                </span>
              ))}
            {view.fit && (
              <dl className="grid grid-cols-[64px_1fr] gap-y-0.5">
                <dt className="text-[var(--panel-muted-fg)]">Moved</dt>
                <dd className="text-[var(--toolbar-icon)] tabular-nums">
                  {view.fit.moved}
                </dd>
                <dt className="text-[var(--panel-muted-fg)]">Turned</dt>
                <dd className="text-[var(--toolbar-icon)] tabular-nums">
                  {view.fit.turned}
                </dd>
                <dt className="text-[var(--panel-muted-fg)]">Scale</dt>
                <dd className="text-[var(--toolbar-icon)] tabular-nums">
                  {view.fit.scale}
                </dd>
                <dt className="text-[var(--panel-muted-fg)]">Error</dt>
                <dd className="flex items-start gap-1 text-[var(--toolbar-icon)] tabular-nums">
                  {view.fit.warn && (
                    <TriangleAlert size={16} aria-hidden className="shrink-0" />
                  )}
                  <span>
                    {view.fit.error}
                    {view.fit.error_detail && <br />}
                    {view.fit.error_detail}
                  </span>
                </dd>
              </dl>
            )}
            {view.warnings.map((warning) => (
              <span
                key={warning}
                className="flex items-start gap-1 text-[var(--toolbar-icon)]"
              >
                <TriangleAlert size={16} aria-hidden className="shrink-0" />
                {warning}
              </span>
            ))}
            {view.not_usable && (
              <span className="text-[var(--toolbar-icon)]">
                {view.not_usable}
              </span>
            )}
            {view.placement_line && (
              <span className="text-[var(--panel-muted-fg)]">
                {view.placement_line}
              </span>
            )}
          </div>
          {view.show_tolerance && (
            <div data-pnc-field="tolerance">
              <EntryField
                label="Tolerance"
                prefix="Tolerance"
                shown={view.tolerance.toFixed(2)}
                mixed={false}
                suffix="mm"
                width={148}
                commitOnBlur
                inputMode="decimal"
                onSubmit={(text) => {
                  const mm = parseMeasure(text);
                  if (typeof mm !== "number" || mm < 0.05 || mm > 5)
                    return "invalid:range";
                  return api.setTolerance(mm) ? "committed" : "unchanged";
                }}
                messages={TOLERANCE_MESSAGES}
                onReturnFocus={onReturnFocus}
                onEnter={enterFrom("tolerance")}
              />
            </div>
          )}
          {refusal && (
            <p role="alert" className="text-xs text-[var(--toolbar-icon)]">
              {refusal}
            </p>
          )}
          <div className="flex gap-1">
            {/* Apply is greyed, never absent, so Reset placement never moves (UX notes §4.8);
                a fit that is not usable gives its line as the reason. */}
            <Tooltip side="top" content={applyReason || applyLabel}>
              <button
                type="button"
                data-pnc-apply
                aria-disabled={applyReason !== null}
                aria-description={applyReason || undefined}
                className={`${OUTLINE_BUTTON} w-[120px]`}
                onClick={() => {
                  if (applyReason !== null) return;
                  const text = api.apply();
                  setRefusal(text === "" ? null : text);
                }}
              >
                {applyLabel}
              </button>
            </Tooltip>
            <Tooltip
              side="top"
              content={view.reset.reason ?? "Reset placement"}
            >
              <button
                type="button"
                aria-disabled={!view.reset.enabled}
                aria-description={view.reset.reason ?? undefined}
                className={`${OUTLINE_BUTTON} w-[120px]`}
                onClick={() => {
                  if (view.reset.enabled) api.reset();
                }}
              >
                Reset placement
              </button>
            </Tooltip>
          </div>
        </div>
      )}
    </section>
  );
}

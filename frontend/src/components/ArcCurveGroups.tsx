import { CircleAlert } from "lucide-react";
import { Tooltip } from "radix-ui";
import { useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import type { ArcBarApi } from "@/hooks/useArcBar";
import { ARC_GREY_LINE, ARC_KIND_TEXT, ARC_TEXT, type ArcKindName } from "@/lib/arcText";

const ICON_BUTTON_CLASS =
  "flex size-7 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

/** Start and End are 93 px and Curve 99 px: the three fit the 308 px inside the 324 px bar
 * of an 800 px window on one row with the group's divider (13 + 93 + 93 + 99 + 2 × 4). */
const FIELD_PX = 93;
/** Curve is wider: its label is the longest and "-100.0" the widest value. */
const CURVE_FIELD_PX = 99;
/** Curve with its "limited" tag. */
const LIMITED_FIELD_PX = 148;
/** The left padding that clears each label inside the field (6 px inset, the 12 px label,
 * 4 px gap), so "-100.0" and "359.9" fit beside it. */
const START_LABEL_PX = 36;
const END_LABEL_PX = 30;
const CURVE_LABEL_PX = 40;

const TOOLTIP_STYLE = {
  background: "var(--toolbar-bg)",
  color: "var(--toolbar-icon)",
  boxShadow: "var(--panel-elevation-shadow)",
} as const;

/** One decimal, a real minus never needed: the session parses both. */
function formatOne(value: number): string {
  return (Math.round(value * 10) / 10).toFixed(1);
}

/** The tooltip of a bar control: the name in semibold, one muted line, and the reason on a
 * second muted line while the control is greyed (`docs/design-system.md`, the Flip rule). */
function BarTooltip({
  name,
  line,
  grey,
  extra,
  closed = false,
  children,
}: {
  name: string;
  line?: string;
  grey: string;
  extra?: string;
  /** Kept closed (an invalid field shows its message where the tooltip would be). */
  closed?: boolean;
  children: React.ReactNode;
}) {
  return (
    <Tooltip.Root open={closed ? false : undefined}>
      <Tooltip.Trigger asChild>{children}</Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content
          side="bottom"
          sideOffset={6}
          className="z-50 max-w-[300px] rounded-md px-2 py-1 text-xs"
          style={TOOLTIP_STYLE}
        >
          <div className="font-semibold">{name}</div>
          {line && <div style={{ color: "var(--panel-muted-fg)" }}>{line}</div>}
          {extra && <div style={{ color: "var(--panel-muted-fg)" }}>{extra}</div>}
          {grey !== "" && (
            <div style={{ color: "var(--panel-muted-fg)" }}>{ARC_GREY_LINE[grey] ?? ""}</div>
          )}
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}

/** The 16 px glyph of an arc kind or of "Whole ellipse": 1.5 px stroke, `currentColor`. */
function ArcGlyph({ kind }: { kind: ArcKindName | "whole" }) {
  const paths: Record<ArcKindName | "whole", string> = {
    // A quarter-open pie: the arc and two radii.
    pie: "M8 8 L14 8 A6 6 0 1 1 8 2 Z",
    // The same arc, open.
    arc: "M14 8 A6 6 0 1 1 8 2",
    // The same arc closed by its chord.
    chord: "M14 8 A6 6 0 1 1 8 2 Z",
    whole: "M14 8 A6 6 0 1 1 2 8 A6 6 0 1 1 14 8 Z",
  };
  return (
    <svg aria-hidden viewBox="0 0 16 16" className="size-4" fill="none">
      <path
        d={paths[kind]}
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinejoin="round"
        strokeLinecap="round"
      />
    </svg>
  );
}

interface BarNumberFieldProps {
  /** The short label inside the left edge. */
  label: string;
  /** The left padding that clears the label, px. */
  labelPx: number;
  accessibleName: string;
  suffix: string;
  widthPx: number;
  value: number;
  mixed: boolean;
  /** The reason the field is greyed, `""` when it applies. */
  grey: string;
  tooltipName: string;
  tooltipLine?: string;
  /** The "limited" tag and its tooltip line, or `null`. */
  limited: string | null;
  /** Enter: `"committed"`, `"unchanged"` or `"invalid:number"`. */
  onEnter: (text: string) => string;
  /** An arrow-key step: previewed, committed once on key-up; absent when the field has none. */
  onStepPreview?: (value: number) => void;
  onStepCommit?: () => void;
  onStepCancel?: () => void;
  onReturnFocus: () => void;
}

/**
 * A number field of the ellipse group or the Curve group (`docs/design-system.md`, "Bar
 * number field"): Enter commits once; Escape or a press elsewhere restores and writes
 * nothing; invalid input gets the 2 px invalid border and "Enter a number" under the field.
 * Greyed: `aria-disabled`, read-only, still a Tab stop with its tooltip. The rules are the
 * session's; this holds the text, the caret and the invalid mark.
 */
function BarNumberField({
  label,
  labelPx,
  accessibleName,
  suffix,
  widthPx,
  value,
  mixed,
  grey,
  tooltipName,
  tooltipLine,
  limited,
  onEnter,
  onStepPreview,
  onStepCommit,
  onStepCancel,
  onReturnFocus,
}: BarNumberFieldProps) {
  const id = useId();
  const disabled = grey !== "";
  const shown = mixed || disabled ? "" : formatOne(value);
  const [text, setText] = useState(shown);
  const [invalid, setInvalid] = useState(false);
  const editing = useRef(false);
  const stepping = useRef<number | null>(null);

  useEffect(() => {
    if (!editing.current) {
      setText(shown);
      setInvalid(false);
    }
  }, [shown]);

  const leave = (input: HTMLInputElement) => {
    editing.current = false;
    input.blur();
    onReturnFocus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (disabled) {
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (text === shown) {
        leave(event.currentTarget);
        return;
      }
      if (onEnter(text).startsWith("invalid:")) {
        setInvalid(true);
        event.currentTarget.select();
        return;
      }
      leave(event.currentTarget);
    } else if (event.key === "Escape") {
      event.preventDefault();
      if (stepping.current !== null) {
        stepping.current = null;
        onStepCancel?.();
      }
      setText(shown);
      setInvalid(false);
      leave(event.currentTarget);
    } else if ((event.key === "ArrowUp" || event.key === "ArrowDown") && onStepPreview && !mixed) {
      event.preventDefault();
      const step = (event.shiftKey ? 10 : 1) * (event.key === "ArrowUp" ? 1 : -1);
      const next = (stepping.current ?? value) + step;
      stepping.current = next;
      setText(formatOne(next));
      onStepPreview(next);
    }
  };

  const onKeyUp = (event: KeyboardEvent<HTMLInputElement>) => {
    if ((event.key === "ArrowUp" || event.key === "ArrowDown") && stepping.current !== null) {
      stepping.current = null;
      onStepCommit?.();
    }
  };

  return (
    <div className="relative flex items-center text-[var(--toolbar-icon)]">
      <BarTooltip
        name={tooltipName}
        line={tooltipLine}
        grey={grey}
        extra={limited ?? undefined}
        closed={invalid}
      >
        <div className="relative" style={{ width: widthPx }}>
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 left-1.5 -translate-y-1/2 text-[12px]"
            style={{ opacity: disabled ? 0.4 : 0.7 }}
          >
            {label}
          </span>
          <input
            id={id}
            type="text"
            inputMode="decimal"
            autoComplete="off"
            spellCheck={false}
            aria-label={accessibleName}
            aria-disabled={disabled || undefined}
            readOnly={disabled}
            aria-invalid={invalid || undefined}
            aria-describedby={invalid ? `${id}-message` : undefined}
            placeholder={mixed && !disabled ? "Mixed" : undefined}
            value={text}
            onFocus={(event) => {
              editing.current = !disabled;
              event.currentTarget.select();
            }}
            onChange={(event) => {
              if (!disabled) {
                setText(event.target.value);
                setInvalid(false);
              }
            }}
            onBlur={() => {
              editing.current = false;
              if (stepping.current !== null) {
                stepping.current = null;
                onStepCommit?.();
              }
              setText(shown);
              setInvalid(false);
            }}
            onKeyDown={onKeyDown}
            onKeyUp={onKeyUp}
            className={`h-7 w-full rounded-[5px] border bg-white text-right text-sm tabular-nums outline-none ${
              limited ? "pr-[62px]" : "pr-4"
            } ${disabled ? "opacity-40" : ""} ${
              invalid
                ? "border-[var(--field-invalid)] shadow-[inset_0_0_0_2px_var(--field-invalid)]"
                : "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] focus:border-[var(--editor-accent)] focus:shadow-[inset_0_0_0_1px_var(--editor-accent)]"
            }`}
            style={{ color: "var(--toolbar-icon)", paddingLeft: labelPx }}
          />
          {limited && (
            <span
              aria-hidden
              className="pointer-events-none absolute top-1/2 right-[16px] -translate-y-1/2 text-[12px] opacity-60"
            >
              limited
            </span>
          )}
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 right-1.5 -translate-y-1/2 text-xs"
            style={{ opacity: disabled ? 0.4 : 0.7 }}
          >
            {suffix}
          </span>
        </div>
      </BarTooltip>
      <div
        id={`${id}-message`}
        role="status"
        aria-live="polite"
        className={
          invalid
            ? "absolute top-full left-0 z-30 mt-1 flex w-max items-center gap-1 rounded-[8px] px-1.5 py-1 text-xs whitespace-nowrap"
            : "absolute h-0 overflow-hidden text-xs"
        }
        style={invalid ? TOOLTIP_STYLE : undefined}
      >
        {invalid && (
          <>
            <CircleAlert aria-hidden className="size-3 shrink-0" />
            <span>Enter a number</span>
          </>
        )}
      </div>
    </div>
  );
}

/**
 * The ellipse group of the Select bar (`specs/0021-ellipse-arcs-and-shaping/` criteria 3,
 * 31, 65, UX notes "Select bar"): two 36 px rows, the kind buttons and "Whole ellipse" on
 * the first, "Start" and "End" on the second. Every control is always present while an
 * ellipse is selected and greyed where it would do nothing.
 */
function ArcKindRow({
  arc,
  onReturnFocus,
}: {
  arc: ArcBarApi;
  onReturnFocus: () => void;
}) {
  const { state } = arc;
  const kindDisabled = state.kindGrey !== "";
  const press = (action: () => void) => (event: React.MouseEvent) => {
    if (kindDisabled) {
      return;
    }
    action();
    if (event.detail > 0) {
      onReturnFocus();
    }
  };
  return (
    <div className="flex h-9 items-center gap-3">
      <div role="group" aria-label="Arc kind" className="flex items-center gap-1">
        {(["pie", "arc", "chord"] as const).map((kind) => {
          const active = state.kind === kind;
          return (
            <BarTooltip
              key={kind}
              name={ARC_KIND_TEXT[kind].name}
              line={ARC_KIND_TEXT[kind].line}
              grey={state.kindGrey}
            >
              <button
                type="button"
                aria-label={ARC_KIND_TEXT[kind].name}
                aria-pressed={active}
                aria-disabled={kindDisabled || undefined}
                onClick={press(() => arc.setKind(kind))}
                className={`${ICON_BUTTON_CLASS} ${
                  kindDisabled
                    ? "opacity-40"
                    : active
                      ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
                      : "hover:bg-[var(--editor-accent-hover)]"
                }`}
              >
                <ArcGlyph kind={kind} />
              </button>
            </BarTooltip>
          );
        })}
      </div>
      <BarTooltip name={ARC_TEXT.whole.name} line={ARC_TEXT.whole.line} grey={state.kindGrey}>
        <button
          type="button"
          aria-label={ARC_TEXT.whole.name}
          aria-disabled={kindDisabled || undefined}
          onClick={press(arc.makeWhole)}
          className={`${ICON_BUTTON_CLASS} ${
            kindDisabled ? "opacity-40" : "hover:bg-[var(--editor-accent-hover)]"
          }`}
        >
          <ArcGlyph kind="whole" />
        </button>
      </BarTooltip>
    </div>
  );
}

/** "Start", "End" and, while it is shown, "Curve" on one row. */
function ArcFieldRow({ arc, onReturnFocus }: { arc: ArcBarApi; onReturnFocus: () => void }) {
  const { state } = arc;
  return (
    <div className="flex h-9 items-center gap-1">
      <BarNumberField
        label="Start"
        labelPx={START_LABEL_PX}
        accessibleName="Arc start"
        suffix="°"
        widthPx={FIELD_PX}
        value={state.start}
        mixed={state.startMixed}
        grey={state.anglesGrey}
        tooltipName="Start"
        tooltipLine={ARC_TEXT.angle.line}
        limited={null}
        onEnter={(text) => arc.setAngle(false, text)}
        onStepPreview={(value) => arc.previewAngle(false, value)}
        onStepCommit={arc.commitCurve}
        onStepCancel={arc.cancelCurve}
        onReturnFocus={onReturnFocus}
      />
      <BarNumberField
        label="End"
        labelPx={END_LABEL_PX}
        accessibleName="Arc end"
        suffix="°"
        widthPx={FIELD_PX}
        value={state.end}
        mixed={state.endMixed}
        grey={state.anglesGrey}
        tooltipName="End"
        tooltipLine={ARC_TEXT.angle.line}
        limited={null}
        onEnter={(text) => arc.setAngle(true, text)}
        onStepPreview={(value) => arc.previewAngle(true, value)}
        onStepCommit={arc.commitCurve}
        onStepCancel={arc.cancelCurve}
        onReturnFocus={onReturnFocus}
      />
      {/* Curve sits beside Start and End, so the group stays two rows (UX review 3). */}
      {state.curveShown && <CurveField arc={arc} onReturnFocus={onReturnFocus} />}
    </div>
  );
}

/**
 * The ellipse group of the Select bar (`specs/0021-ellipse-arcs-and-shaping/` criteria 3,
 * 31, 65, UX notes "Select bar"): the kind buttons and "Whole ellipse", then "Start", "End"
 * and "Curve". Every control is always present while an ellipse is selected and greyed
 * where it would do nothing. The bar places the two rows as two groups
 * ([`EllipseKindGroup`], [`ArcFieldGroup`]) so that they wrap on their own.
 */
export function EllipseGroup({ arc, onReturnFocus }: { arc: ArcBarApi; onReturnFocus: () => void }) {
  return (
    <Tooltip.Provider delayDuration={400}>
      <div className="flex flex-col">
        <ArcKindRow arc={arc} onReturnFocus={onReturnFocus} />
        <ArcFieldRow arc={arc} onReturnFocus={onReturnFocus} />
      </div>
    </Tooltip.Provider>
  );
}

/** The kind buttons and "Whole ellipse" as a bar group of their own. */
export function EllipseKindGroup({ arc, onReturnFocus }: { arc: ArcBarApi; onReturnFocus: () => void }) {
  return (
    <Tooltip.Provider delayDuration={400}>
      <ArcKindRow arc={arc} onReturnFocus={onReturnFocus} />
    </Tooltip.Provider>
  );
}

/** "Start", "End" and "Curve" as a bar group of their own. */
export function ArcFieldGroup({ arc, onReturnFocus }: { arc: ArcBarApi; onReturnFocus: () => void }) {
  return (
    <Tooltip.Provider delayDuration={400}>
      <ArcFieldRow arc={arc} onReturnFocus={onReturnFocus} />
    </Tooltip.Provider>
  );
}

/** The "Curve" field (criteria 26, 44, 65): the effective value in percent, "Mixed", the
 * "limited" tag, greyed with its reason for an ellipse with an arc. The arrow keys step it
 * live and commit once on key-up. */
function CurveField({ arc, onReturnFocus }: { arc: ArcBarApi; onReturnFocus: () => void }) {
  const { state } = arc;
  const limited = state.curveLimited
    ? `Stored ${formatOne(state.curveStored)} %, limited to ${formatOne(state.curve)} % by Points and Ratio; changing them brings it back.`
    : null;
  return (
    <BarNumberField
      label="Curve"
      labelPx={CURVE_LABEL_PX}
      accessibleName="Curve"
      suffix="%"
      widthPx={limited ? LIMITED_FIELD_PX : CURVE_FIELD_PX}
      value={state.curve}
      mixed={state.curveMixed}
      grey={state.curveGrey}
      tooltipName="Curve"
      limited={limited}
      onEnter={arc.setCurve}
      onStepPreview={arc.previewCurve}
      onStepCommit={arc.commitCurve}
      onStepCancel={arc.cancelCurve}
      onReturnFocus={onReturnFocus}
    />
  );
}

/** The Curve group for a selection of polygons and stars without an ellipse; with an
 * ellipse the field sits in the ellipse group's second row. */
export function CurveGroup({ arc, onReturnFocus }: { arc: ArcBarApi; onReturnFocus: () => void }) {
  return (
    <Tooltip.Provider delayDuration={400}>
      <div className="flex h-9 items-center">
        <CurveField arc={arc} onReturnFocus={onReturnFocus} />
      </div>
    </Tooltip.Provider>
  );
}

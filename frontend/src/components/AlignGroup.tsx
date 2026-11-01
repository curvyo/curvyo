import { Tooltip } from "radix-ui";
import { type ReactNode, useEffect, useId, useRef } from "react";

import type { AlignApi } from "@/hooks/useAlign";
import {
  ALIGN_ORDER,
  ALIGN_TEXT,
  alignRule,
  DISTRIBUTE_ORDER,
  DISTRIBUTE_TEXT,
  distributeRule,
  pageCueFor,
  type AlignCommand,
  type AlignReference,
  type DistributeCode,
} from "@/lib/alignText";

export const BUTTON_CLASS =
  "flex size-7 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] hover:bg-[var(--editor-accent-hover)]";

/** One align glyph: two bars of different widths with their left edges on a
 * dashed reference line (`specs/0061-align-and-distribute/` UX notes). The
 * other five are the drawing turned about the glyph's centre. */
function AlignGlyph({ command }: { command: AlignCommand }) {
  const turn = { left: 0, "centre-h": 0, right: 0, top: 90, "centre-v": 90, bottom: -90 }[
    command
  ];
  const centres = command === "centre-h" || command === "centre-v";
  return (
    <svg aria-hidden viewBox="0 0 16 16" className="size-4" fill="none">
      <g transform={turn === 0 ? undefined : `rotate(${turn} 8 8)`}>
        <line
          x1={centres ? 8 : 2.75}
          y1="2"
          x2={centres ? 8 : 2.75}
          y2="14"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeDasharray="2.5 2"
        />
        {/* The reference's own bar first (left edge on the line), the moved
         * one second, so the two widths read. */}
        <rect
          x="4.75"
          y="4"
          width={centres ? 6.5 : 7}
          height="3"
          stroke="currentColor"
          strokeWidth="1.5"
        />
        <rect
          x={centres ? 5.75 : 4.75}
          y="9"
          width="4.5"
          height="3"
          stroke="currentColor"
          strokeWidth="1.5"
        />
      </g>
    </svg>
  );
}

/** One distribute glyph: three outlined bars with equal gaps, the align
 * glyphs' 1.5 px outline; the edges variant puts dashed guides on the outer
 * bars' far edges, the centres variant gives the bars different heights and
 * a dashed guide through their centres, so the two read apart. */
function DistributeGlyph({ code }: { code: DistributeCode }) {
  const vertical = code.startsWith("v-");
  const centres = code.endsWith("centres");
  const bars = centres
    ? [
        { x: 2.75, y: 5, width: 2.5, height: 6 },
        { x: 6.75, y: 3.5, width: 2.5, height: 9 },
        { x: 10.75, y: 5, width: 2.5, height: 6 },
      ]
    : [
        { x: 2.75, y: 4, width: 2.5, height: 8 },
        { x: 6.75, y: 4, width: 2.5, height: 8 },
        { x: 10.75, y: 4, width: 2.5, height: 8 },
      ];
  const guides = centres
    ? [{ x1: 1, y1: 8, x2: 15, y2: 8 }]
    : [
        { x1: 0.75, y1: 2, x2: 0.75, y2: 14 },
        { x1: 15.25, y1: 2, x2: 15.25, y2: 14 },
      ];
  return (
    <svg aria-hidden viewBox="0 0 16 16" className="size-4" fill="none">
      <g transform={vertical ? "rotate(90 8 8)" : undefined}>
        {guides.map((guide, index) => (
          <line
            key={index}
            {...guide}
            stroke="currentColor"
            strokeWidth="1.5"
            strokeDasharray="2.5 2"
          />
        ))}
        {bars.map((bar, index) => (
          <rect key={index} {...bar} stroke="currentColor" strokeWidth="1.5" />
        ))}
      </g>
    </svg>
  );
}

/** The tooltip of an arrange button: the name semibold, the rule muted. */
export function ArrangeTooltip({
  name,
  rule,
  children,
}: {
  name: string;
  rule: string;
  children: ReactNode;
}) {
  return (
    <Tooltip.Root>
      <Tooltip.Trigger asChild>{children}</Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content
          side="bottom"
          sideOffset={6}
          className="z-50 max-w-[260px] rounded-md px-2 py-1 text-xs"
          style={{
            background: "var(--toolbar-bg)",
            color: "var(--toolbar-icon)",
            boxShadow: "var(--panel-elevation-shadow)",
          }}
        >
          <div className="font-semibold">{name}</div>
          <div style={{ color: "var(--panel-muted-fg)" }}>{rule}</div>
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}

interface CommandButtonProps {
  name: string;
  rule: string;
  reference: AlignReference;
  align: AlignApi;
  onPress: () => void;
  onReturnFocus: () => void;
  children: ReactNode;
}

/** One align or distribute button: a plain command, the Flip buttons' look.
 * While hovered or keyboard-focused with the page as the reference, the
 * active page's frame is marked on the canvas (UX notes, "Canvas cue"). */
function CommandButton({
  name,
  rule,
  reference,
  align,
  onPress,
  onReturnFocus,
  children,
}: CommandButtonProps) {
  const cue = pageCueFor(reference);
  return (
    <ArrangeTooltip name={name} rule={rule}>
      <button
        type="button"
        aria-label={name}
        onClick={(event) => {
          onPress();
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
        onPointerEnter={() => align.setPageCue(cue)}
        onPointerLeave={() => align.setPageCue(false)}
        onFocus={(event) => {
          if (event.currentTarget.matches(":focus-visible")) {
            align.setPageCue(cue);
          }
        }}
        onBlur={() => align.setPageCue(false)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            align.setPageCue(false);
          }
        }}
        className={BUTTON_CLASS}
      >
        {children}
      </button>
    </ArrangeTooltip>
  );
}

interface BarFieldProps {
  label: string;
  accessibleName: string;
  widthClass: string;
  suffix?: string;
  value: string;
  placeholder?: string;
  /** The placeholder is a count derived from the other field, drawn in
   * `--field-derived-fg` so it does not read as a typed value. */
  derivedPlaceholder?: boolean;
  error: string | null | undefined;
  onChange: (text: string) => void;
  onEnter: () => void;
  /** Escape: restores what the field group held on focus; by default the
   * field's own value. */
  onEscape?: () => void;
  /** Focus: lets the group remember what Escape restores. */
  onFocusField?: () => void;
}

/** A number field of the arrange groups, the "Bar number field" look: the
 * label outside on the left, the value right-aligned, an optional fixed
 * suffix inside the right edge, the refusal ring and chip. Enter applies;
 * Escape restores the value the field had on focus and blurs. */
export function BarField({
  label,
  accessibleName,
  widthClass,
  suffix,
  value,
  placeholder,
  derivedPlaceholder = false,
  error,
  onChange,
  onEnter,
  onEscape,
  onFocusField,
}: BarFieldProps) {
  const id = useId();
  const previous = useRef("");
  const invalid = error !== null && error !== undefined;
  return (
    <div className="relative flex items-center gap-1.5 text-[var(--toolbar-icon)]">
      <label htmlFor={`${id}-field`} className="whitespace-nowrap">
        {label}
      </label>
      <div className={`relative ${widthClass}`}>
        <input
          id={`${id}-field`}
          type="text"
          inputMode="decimal"
          autoComplete="off"
          spellCheck={false}
          aria-label={accessibleName}
          aria-invalid={invalid}
          value={value}
          placeholder={placeholder}
          onFocus={(event) => {
            previous.current = value;
            onFocusField?.();
            event.currentTarget.select();
          }}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              onEnter();
            } else if (event.key === "Escape") {
              event.preventDefault();
              if (onEscape) {
                onEscape();
              } else {
                onChange(previous.current);
              }
              event.currentTarget.blur();
            }
          }}
          className={`h-7 w-full rounded-[5px] border bg-white text-right text-sm tabular-nums outline-none ${
            derivedPlaceholder
              ? "placeholder:text-[var(--field-derived-fg)]"
              : "placeholder:text-[var(--panel-muted-fg)]"
          } ${
            suffix ? "pr-8" : "pr-1.5"
          } ${
            invalid
              ? "border-[var(--field-invalid)] shadow-[inset_0_0_0_2px_var(--field-invalid)]"
              : "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] focus:border-[var(--editor-accent)] focus:shadow-[inset_0_0_0_1px_var(--editor-accent)]"
          }`}
          style={{ color: "var(--toolbar-icon)" }}
        />
        {suffix ? (
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 right-1.5 -translate-y-1/2 text-xs"
            style={{ opacity: 0.7 }}
          >
            {suffix}
          </span>
        ) : null}
        {error ? (
          <div
            role="status"
            className="absolute top-full left-0 z-50 mt-1 w-max rounded-md px-2 py-1 text-xs"
            style={{
              background: "var(--toolbar-bg)",
              color: "var(--toolbar-icon)",
              boxShadow: "var(--panel-elevation-shadow)",
            }}
          >
            {error}
          </div>
        ) : null}
      </div>
    </div>
  );
}

/** Clears the page cue when the groups leave the bar, so no mark outlives
 * the button that drew it. */
function usePageCueCleanup(align: AlignApi) {
  const { setPageCue } = align;
  useEffect(() => () => setPageCue(false), [setPageCue]);
}

/**
 * The Align group of the Select bar (`specs/0061-align-and-distribute/`
 * criteria 1, 2, 14, 15): six symbol-only buttons, in the bar in every
 * selection state of the Select tool.
 */
export function AlignBarGroup({ align, onReturnFocus }: { align: AlignApi; onReturnFocus: () => void }) {
  usePageCueCleanup(align);
  return (
    <Tooltip.Provider delayDuration={400}>
      <div role="group" aria-label="Align" className="flex items-center gap-1 py-1">
        {ALIGN_ORDER.map((command) => (
          <CommandButton
            key={command}
            name={ALIGN_TEXT[command].name}
            rule={alignRule(command, align.alignReference)}
            reference={align.alignReference}
            align={align}
            onPress={() => align.align(command)}
            onReturnFocus={onReturnFocus}
          >
            <AlignGlyph command={command} />
          </CommandButton>
        ))}
      </div>
    </Tooltip.Provider>
  );
}

/**
 * The Distribute group of the Select bar (criteria 6 to 8, 15, 16): four
 * symbol-only buttons and the Gap field, in the bar in every selection
 * state of the Select tool.
 */
export function DistributeBarGroup({
  align,
  onReturnFocus,
}: {
  align: AlignApi;
  onReturnFocus: () => void;
}) {
  return (
    <Tooltip.Provider delayDuration={400}>
      <div role="group" aria-label="Distribute" className="flex flex-wrap items-center gap-x-1 gap-y-2 py-1">
        {DISTRIBUTE_ORDER.map((code) => (
          <CommandButton
            key={code}
            name={DISTRIBUTE_TEXT[code].name}
            rule={distributeRule(code, align.distributeReference, align.selectedCount)}
            reference={align.distributeReference}
            align={align}
            onPress={() => align.distribute(code)}
            onReturnFocus={onReturnFocus}
          >
            <DistributeGlyph code={code} />
          </CommandButton>
        ))}
        <BarField
          label="Gap"
          accessibleName="Gap, millimetres"
          widthClass="w-[84px]"
          suffix="mm"
          value={align.gap}
          error={align.gapError}
          onChange={align.setGap}
          onEnter={() => align.distribute("h-gaps")}
        />
      </div>
    </Tooltip.Provider>
  );
}

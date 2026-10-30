import { Tooltip } from "radix-ui";
import { useId, useRef } from "react";

import type { AlignApi } from "@/hooks/useAlign";
import {
  ALIGN_ORDER,
  ALIGN_TEXT,
  DISTRIBUTE_ORDER,
  DISTRIBUTE_TEXT,
  type AlignCommand,
  type DistributeCode,
} from "@/lib/alignText";

const BUTTON_CLASS =
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

/** One distribute glyph: three bars with equal gaps (the middle one outlined,
 * it is the one that moves); the centres variant gives the bars different
 * heights and a dashed line through their centres, so the two read apart. */
function DistributeGlyph({ code }: { code: DistributeCode }) {
  const vertical = code.startsWith("v-");
  const centres = code.endsWith("centres");
  const bars = centres
    ? [
        { x: 2, y: 4, width: 3, height: 8 },
        { x: 6.25, y: 3, width: 3.5, height: 10 },
        { x: 11, y: 4, width: 3, height: 8 },
      ]
    : [
        { x: 2, y: 4, width: 3, height: 8 },
        { x: 6.5, y: 4, width: 3, height: 8 },
        { x: 11, y: 4, width: 3, height: 8 },
      ];
  return (
    <svg aria-hidden viewBox="0 0 16 16" className="size-4" fill="none">
      <g transform={vertical ? "rotate(90 8 8)" : undefined}>
        {centres ? (
          <line
            x1="2"
            y1="8"
            x2="14"
            y2="8"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeDasharray="2.5 2"
          />
        ) : null}
        {bars.map((bar, index) => (
          <rect
            key={index}
            {...bar}
            stroke="currentColor"
            strokeWidth="1.5"
            fill={index === 1 ? "none" : "currentColor"}
            fillOpacity={index === 1 ? 0 : 1}
          />
        ))}
      </g>
    </svg>
  );
}

interface AlignButtonProps {
  command: AlignCommand;
  align: AlignApi;
  onReturnFocus: () => void;
}

function AlignButton({ command, align, onReturnFocus }: AlignButtonProps) {
  const text = ALIGN_TEXT[command];
  return (
    <Tooltip.Root>
      <Tooltip.Trigger asChild>
        <button
          type="button"
          aria-label={text.name}
          onClick={(event) => {
            align.align(command);
            if (event.detail > 0) {
              onReturnFocus();
            }
          }}
          className={BUTTON_CLASS}
        >
          <AlignGlyph command={command} />
        </button>
      </Tooltip.Trigger>
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
          <div className="font-semibold">{text.name}</div>
          <div style={{ color: "var(--panel-muted-fg)" }}>{text.rule}</div>
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}

interface DistributeButtonProps {
  code: DistributeCode;
  align: AlignApi;
  onReturnFocus: () => void;
}

function DistributeButton({ code, align, onReturnFocus }: DistributeButtonProps) {
  const text = DISTRIBUTE_TEXT[code];
  return (
    <Tooltip.Root>
      <Tooltip.Trigger asChild>
        <button
          type="button"
          aria-label={text.name}
          onClick={(event) => {
            align.distribute(code);
            if (event.detail > 0) {
              onReturnFocus();
            }
          }}
          className={BUTTON_CLASS}
        >
          <DistributeGlyph code={code} />
        </button>
      </Tooltip.Trigger>
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
          <div className="font-semibold">{text.name}</div>
          <div style={{ color: "var(--panel-muted-fg)" }}>{text.rule}</div>
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}

/** The Gap field next to the distribute buttons: a typed gap in millimetres
 * replaces the ends; empty keeps them (`specs/0061-align-and-distribute/`
 * criterion 8, UX notes). The "Bar number field" look: the label outside,
 * the value right-aligned with room for the `mm` suffix inside the right
 * edge, so value and unit never overlap. Enter distributes, Escape restores
 * the value the field had and writes nothing. */
function GapField({ align }: { align: AlignApi }) {
  const id = useId();
  const previous = useRef("");
  return (
    <div className="relative flex items-center gap-1.5 text-[var(--toolbar-icon)]">
      <label htmlFor={`${id}-field`}>Gap</label>
      <div className="relative w-[84px]">
        <input
          id={`${id}-field`}
          type="text"
          inputMode="decimal"
          autoComplete="off"
          spellCheck={false}
          aria-label="Gap, millimetres"
          aria-invalid={align.gapError !== null}
          value={align.gap}
          onFocus={(event) => {
            previous.current = align.gap;
            event.currentTarget.select();
          }}
          onChange={(event) => align.setGap(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              align.distribute("h-gaps");
            } else if (event.key === "Escape") {
              event.preventDefault();
              align.setGap(previous.current);
              event.currentTarget.blur();
            }
          }}
          className={`h-7 w-full rounded-[5px] border bg-white pr-8 text-right text-sm tabular-nums outline-none ${
            align.gapError !== null
              ? "border-[var(--field-invalid)] shadow-[inset_0_0_0_2px_var(--field-invalid)]"
              : "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] focus:border-[var(--editor-accent)] focus:shadow-[inset_0_0_0_1px_var(--editor-accent)]"
          }`}
          style={{ color: "var(--toolbar-icon)" }}
        />
        <span
          aria-hidden
          className="pointer-events-none absolute top-1/2 right-1.5 -translate-y-1/2 text-xs"
          style={{ opacity: 0.7 }}
        >
          mm
        </span>
        {align.gapError !== null ? (
          <div
            role="status"
            className="absolute top-full left-0 z-50 mt-1 w-max rounded-md px-2 py-1 text-xs"
            style={{
              background: "var(--toolbar-bg)",
              color: "var(--toolbar-icon)",
              boxShadow: "var(--panel-elevation-shadow)",
            }}
          >
            {align.gapError}
          </div>
        ) : null}
      </div>
    </div>
  );
}

/**
 * The Align and Distribute groups of the Select bar
 * (`specs/0061-align-and-distribute/` criterion 1): symbol-only buttons
 * after the Guide button, in the tree while something is selected, the
 * distribute group from two selected objects.
 */
export function AlignDistributeGroups({
  align,
  onReturnFocus,
}: {
  align: AlignApi;
  onReturnFocus: () => void;
}) {
  if (!align.shown) {
    return null;
  }
  return (
    <Tooltip.Provider delayDuration={400}>
      <div className="flex min-h-9 flex-wrap items-start">
        <div role="group" aria-label="Align" className="flex items-center gap-1 py-1">
          {ALIGN_ORDER.map((command) => (
            <AlignButton
              key={command}
              command={command}
              align={align}
              onReturnFocus={onReturnFocus}
            />
          ))}
        </div>
        {align.distributeShown ? (
          <>
            <div className="flex items-center gap-1 py-1">
              <Divider />
            </div>
            <div role="group" aria-label="Distribute" className="flex items-center gap-1 py-1">
              {DISTRIBUTE_ORDER.map((code) => (
                <DistributeButton
                  key={code}
                  code={code}
                  align={align}
                  onReturnFocus={onReturnFocus}
                />
              ))}
              <GapField align={align} />
            </div>
          </>
        ) : null}
      </div>
    </Tooltip.Provider>
  );
}

/** The group divider of the Select bar, drawn in the gap before its group. */
function Divider() {
  return (
    <span
      aria-hidden
      className="pointer-events-none -left-[13px] top-2 h-5 w-px"
      style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
    />
  );
}

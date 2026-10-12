import { CircleAlert } from "lucide-react";
import { useEffect, useRef } from "react";

import { CapIcon, JoinIcon } from "@/components/panel/StyleIcons";
import { StyleRow } from "@/components/panel/StyleRow";
import { ValueField } from "@/components/panel/ValueField";
import { ToggleGroup, type ToggleOption } from "@/components/ui/toggle-group";
import type { GridName } from "@/hooks/useStylePanel";
import type { OffsetApi } from "@/hooks/useOffset";
import {
  type OffsetCapName,
  type OffsetJoinName,
  formatDistance,
  offsetCaption,
} from "@/lib/offsetText";

const JOIN_OPTIONS: readonly ToggleOption<OffsetJoinName>[] = [
  { value: "round", label: "Round join", tooltip: "Round join", icon: <JoinIcon join="round" /> },
  {
    value: "miter",
    label: "Miter join",
    tooltip: "Miter join: a sharp point, clipped at 4 times the distance",
    icon: <JoinIcon join="miter" />,
  },
  { value: "bevel", label: "Bevel join", tooltip: "Bevel join: a flat cut", icon: <JoinIcon join="bevel" /> },
];

const CAP_OPTIONS: readonly ToggleOption<OffsetCapName>[] = [
  { value: "round", label: "Round cap", tooltip: "Round cap", icon: <CapIcon cap="round" /> },
  { value: "flat", label: "Flat cap", tooltip: "Flat cap: ends at the endpoint", icon: <CapIcon cap="butt" /> },
  {
    value: "square",
    label: "Square cap",
    tooltip: "Square cap: extends the distance past the endpoint",
    icon: <CapIcon cap="square" />,
  },
];

/** The step of an arrow key or a drag tick on each grid: 0.1 mm, Shift 1 mm, Ctrl 0.01 mm. */
const STEP_MM: Record<GridName, number> = { normal: 0.1, coarse: 1, fine: 0.01 };

function roundTo(value: number, step: number): number {
  return Math.round(value / step) * step;
}

interface OffsetSectionProps {
  offset: OffsetApi;
  /** After Apply or Cancel: focus to the canvas (by mouse) or the Offset button (by key). */
  onDone: (byKey: boolean) => void;
}

/**
 * The Offset section at the top of the Style tab (`specs/0038-path-offset` criteria 1 to 5,
 * `docs/design-system.md`, row "Offset section"): Distance with its caption, Join, Cap (only with
 * an open path selected), Apply and Cancel. Enter on the Distance field applies once the typed
 * value is committed (the first Enter commits it); Escape cancels.
 */
export function OffsetSection({ offset, onDone }: OffsetSectionProps) {
  const { state } = offset;
  const section = useRef<HTMLDivElement>(null);
  const caption = offsetCaption(state);
  const limit = state.dragLimitMm;
  const typed = state.typedLimitMm;
  const messages = { range: `Enter a number from -${typed} to ${typed}` } as const;

  // Focus lands in Distance when the section opens.
  useEffect(() => {
    section.current?.querySelector<HTMLElement>("[role=spinbutton]")?.focus();
  }, []);

  const focusDistance = () => {
    requestAnimationFrame(() => {
      section.current?.querySelector<HTMLElement>("[role=spinbutton]")?.focus();
    });
  };

  const setClamped = (mm: number) => {
    offset.setDistance(Math.max(-typed, Math.min(typed, Number(mm.toFixed(3)))));
  };

  const apply = (byKey: boolean) => {
    const result = offset.apply();
    if (result.kind === "applied" || result.kind === "closed_zero") {
      onDone(byKey);
    }
  };

  const cancel = (byKey: boolean) => {
    offset.close();
    onDone(byKey);
  };

  return (
    <div
      ref={section}
      role="group"
      aria-label="Offset"
      className="mb-3 flex flex-col gap-2 border-b border-[color-mix(in_srgb,var(--toolbar-icon)_20%,transparent)] pb-3"
      onKeyDownCapture={(event) => {
        // Enter on the Distance field (not while typing): the value is committed, so apply. A
        // typed value is committed by the first Enter, in the typed field.
        if (
          event.key === "Enter" &&
          (event.target as HTMLElement).getAttribute("role") === "spinbutton"
        ) {
          event.preventDefault();
          event.stopPropagation();
          apply(true);
        }
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          cancel(true);
        }
      }}
    >
      <h3 className="text-xs font-semibold text-[var(--toolbar-icon)]">Offset</h3>
      <div className="flex flex-col">
        <ValueField
          label="Distance"
          name="Distance, millimetres"
          text={`${state.distanceMm < 0 ? "-" : ""}${formatDistance(state.distanceMm, false)}`}
          bar={Math.max(0, Math.min(1, (state.distanceMm + limit) / (2 * limit)))}
          mixed={false}
          resettable={state.distanceMm !== 1}
          unit="mm"
          valueNow={state.distanceMm}
          unitWords="millimetres"
          unitWordOne="millimetre"
          valueMin={-limit}
          valueMax={limit}
          defaultText="1 mm"
          messages={messages}
          onPreview={(p, grid) => setClamped(roundTo(-limit + 2 * limit * p, STEP_MM[grid]))}
          onSettle={() => {}}
          onStep={(steps, grid) => setClamped(roundTo(state.distanceMm + steps * STEP_MM[grid], STEP_MM[grid]))}
          onReset={() => setClamped(1)}
          onSubmit={(text) => {
            const value = Number(text.trim().replace(",", ".").replace("−", "-"));
            if (text.trim() === "" || !Number.isFinite(value)) {
              return "invalid:range";
            }
            if (Math.abs(value) > typed) {
              return "invalid:range";
            }
            if (value === state.distanceMm) {
              return "unchanged";
            }
            return offset.setDistance(value) ? "committed" : "invalid:range";
          }}
          onReturnFocus={focusDistance}
        />
        <p
          aria-live="polite"
          className={`flex h-4 items-center gap-1 pt-0.5 text-xs ${
            caption.refusal ? "text-[var(--field-invalid)]" : "text-[var(--panel-muted-fg)]"
          }`}
        >
          {caption.refusal && <CircleAlert size={12} aria-hidden="true" className="shrink-0" />}
          {caption.text}
        </p>
      </div>
      <StyleRow label="Join">
        <ToggleGroup
          label="Join"
          options={JOIN_OPTIONS}
          value={state.join}
          onChange={offset.setJoin}
          itemWidth={40}
          onReturnFocus={focusDistance}
        />
      </StyleRow>
      {state.hasOpen && (
        <StyleRow label="Cap">
          <ToggleGroup
            label="Cap"
            options={CAP_OPTIONS}
            value={state.cap}
            onChange={offset.setCap}
            itemWidth={40}
            onReturnFocus={focusDistance}
          />
        </StyleRow>
      )}
      <div className="relative flex h-7 items-center gap-2">
        <button
          type="button"
          onClick={(event) => apply(event.detail === 0)}
          className="h-7 w-[72px] rounded-[5px] text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1"
          style={{
            background: "var(--toolbar-icon-active-bg)",
            color: "var(--toolbar-icon-active-fg)",
          }}
        >
          Apply
        </button>
        <button
          type="button"
          onClick={(event) => cancel(event.detail === 0)}
          className="h-7 w-[72px] rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-sm text-[var(--toolbar-icon)] outline-none hover:border-[var(--toolbar-icon)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1"
        >
          Cancel
        </button>
        <div role="alert" className="absolute top-8 left-0 z-30 w-[244px] text-xs">
          {offset.notice?.kind === "refusal" ? (
            <div
              className="flex items-start gap-1.5 rounded-lg bg-popover p-2"
              style={{
                color: "var(--field-invalid)",
                boxShadow: "0 0 0 1px var(--field-invalid), var(--panel-elevation-shadow)",
              }}
            >
              <CircleAlert size={12} aria-hidden="true" className="mt-0.5 shrink-0" />
              <span>{offset.notice.text}</span>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

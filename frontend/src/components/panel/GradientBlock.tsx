import { ArrowLeftRight, Pipette, Plus, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { ColourPicker } from "@/components/panel/ColourPicker";
import { EntryField } from "@/components/panel/EntryField";
import { LinearGradientIcon, RadialGradientIcon } from "@/components/panel/GradientIcons";
import { Swatch } from "@/components/panel/Swatch";
import { ToggleGroup, type ToggleOption } from "@/components/ui/toggle-group";
import { Tooltip } from "@/components/ui/tooltip";
import type { GradientApi } from "@/hooks/useGradient";
import type { GradientStopRow } from "@/lib/gradientView";
import { stepAngle, stepRadius, stopColour } from "@/lib/gradientText";
import { STOP_ROW_PX, STOP_WINDOW_FROM, STOP_WINDOW_ROWS, scrollToRow, stopWindow } from "@/lib/stopWindow";

const KIND_OPTIONS: readonly ToggleOption<"linear" | "radial">[] = [
  { value: "linear", label: "Linear", tooltip: "Linear", icon: <LinearGradientIcon /> },
  { value: "radial", label: "Radial", tooltip: "Radial", icon: <RadialGradientIcon /> },
];

const ANGLE_MESSAGES = { number: "Enter a number." } as const;
const RADIUS_MESSAGES = { number: "Enter a number.", range: "Enter a number above 0." } as const;
const HEX_MESSAGES = { hex: "Enter 3, 4, 6 or 8 hex digits" } as const;
const POSITION_MESSAGES = { number: "Enter a number.", range: "Enter a number from 0 to 100." } as const;

const BUTTON =
  "flex h-7 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";
const OUTLINED = `${BUTTON} w-7 border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]`;

interface GradientBlockProps {
  gradient: GradientApi;
  /** The paint the eyedropper is picking for (`stop-<n>` for a stop), or empty. */
  pickTarget: string;
  /** A stop row's eyedropper button: starts or ends picking for stop `index`. */
  onPick: (index: number) => void;
  onReturnFocus: () => void;
}

interface StopRowProps {
  gradient: GradientApi;
  row: GradientStopRow;
  index: number;
  count: number;
  active: boolean;
  picking: boolean;
  onPick: (index: number) => void;
  onReturnFocus: () => void;
}

/**
 * One stop: swatch, eyedropper, hex, Position and Remove (UX notes §4, criterion 3); the end
 * rows have no Remove. The eyedropper is the Colour row's (`0017`), for this stop.
 */
function StopRow({ gradient, row, index, count, active, picking, onPick, onReturnFocus }: StopRowProps) {
  const colour = stopColour(row.hex);
  const name = `Stop ${index + 1}`;
  const activate = () => {
    if (!active) {
      gradient.activate(index);
    }
  };
  return (
    <div
      role="group"
      aria-label={`${name} of ${count}`}
      aria-current={active ? "true" : undefined}
      data-gradient-stop={index}
      data-active={active ? "" : undefined}
      className="relative flex h-7 items-center gap-1"
      onFocusCapture={activate}
      onPointerDown={activate}
    >
      {active && (
        <span aria-hidden className="absolute top-0 -left-2 h-7 w-[3px] rounded-[1px] bg-[var(--editor-accent)]" />
      )}
      <Swatch rgb={colour?.rgb ?? 0} opacity={colour?.opacity ?? 100} mixed={colour === null} />
      <Tooltip side="left" content="Pick a color from the drawing (Esc cancels)">
        <button
          type="button"
          data-eyedropper=""
          aria-label={`Pick colour of stop ${index + 1} from the drawing`}
          aria-pressed={picking}
          onClick={(event) => {
            onPick(index);
            // A click goes on to the canvas, where the pick happens; a key press stays here.
            if (event.detail > 0) {
              onReturnFocus();
            }
          }}
          className={`${OUTLINED} shrink-0 aria-pressed:border-transparent aria-pressed:!bg-[var(--toolbar-icon-active-bg)] aria-pressed:!text-[var(--toolbar-icon-active-fg)]`}
        >
          <Pipette size={16} strokeWidth={1.5} aria-hidden />
        </button>
      </Tooltip>
      <EntryField
        label={`${name} colour`}
        shown={row.hex}
        mixed={row.hex === ""}
        width={96}
        align="left"
        compact
        onSubmit={(text) => gradient.setStopHex(index, text)}
        messages={HEX_MESSAGES}
        onReturnFocus={onReturnFocus}
      />
      {row.fixed ? (
        <span aria-label={`${name} position, percent`} className="w-16 text-right text-sm tabular-nums text-[var(--panel-muted-fg)]">
          {row.position} %
        </span>
      ) : (
        <EntryField
          label={`${name} position, percent`}
          shown={row.position}
          mixed={row.position === ""}
          suffix="%"
          width={64}
          onSubmit={(text) => gradient.setStopPosition(index, text)}
          messages={POSITION_MESSAGES}
          onReturnFocus={onReturnFocus}
        />
      )}
      {row.fixed ? (
        <span className="w-6 shrink-0" />
      ) : (
        <Tooltip side="left" content="Remove stop">
          <button
            type="button"
            aria-label={`Remove stop ${index + 1}`}
            data-gradient-remove=""
            onClick={(event) => {
              gradient.removeStop(index);
              if (event.detail > 0) {
                onReturnFocus();
              }
            }}
            className={`${BUTTON} w-6 shrink-0`}
          >
            <Trash2 size={16} strokeWidth={1.5} aria-hidden />
          </button>
        </Tooltip>
      )}
    </div>
  );
}

/**
 * The Gradient block of the Fill section (`specs/0124-gradient-fill` UX notes §3 to §5): the Kind
 * row with Angle or Radius, the stop rows, the colour picker under the active stop, Add stop and
 * Reverse; with different gradients in the selection only "Mixed gradients" and Reverse. There
 * is no Opacity field: alpha is per stop, in the 8-digit hex.
 */
export function GradientBlock({ gradient, pickTarget, onPick, onReturnFocus }: GradientBlockProps) {
  const { view } = gradient;
  const section = useRef<HTMLDivElement>(null);
  const active = Math.min(view.activeStop, Math.max(0, view.stops.length - 1));
  const activeRow = view.stops[active];
  const activeColour = activeRow ? stopColour(activeRow.hex) : null;
  const pressButton = (run: () => void) => (event: React.MouseEvent) => {
    run();
    // A pointer press returns the keyboard to the canvas; a key press keeps it here.
    if (event.detail > 0) {
      onReturnFocus();
    }
  };
  // Above `STOP_WINDOW_FROM` stops only the rows in view are rendered (UX review U4); the picker
  // then sits under the list.
  const windowed = !view.mixedGradients && view.stops.length > STOP_WINDOW_FROM;
  const list = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  useEffect(() => {
    const box = list.current;
    if (windowed && box) {
      box.scrollTop = scrollToRow(active, box.scrollTop);
    }
  }, [windowed, active]);
  const shown = stopWindow(view.stops.length, scrollTop);
  const stopRow = (row: GradientStopRow, index: number) => (
    <StopRow
      gradient={gradient}
      row={row}
      index={index}
      count={view.stops.length}
      active={index === active}
      picking={pickTarget === `stop-${index}`}
      onPick={onPick}
      onReturnFocus={onReturnFocus}
    />
  );
  const picker = (
    <div className="my-1" data-gradient-picker>
      <ColourPicker
        name={`Stop ${active + 1}`}
        rgb={activeColour?.rgb ?? 0}
        mixed={activeColour === null}
        hsvOf={gradient.hsvOf}
        rgbOf={gradient.rgbOf}
        onPreview={(h, s, v) => gradient.panelPicker.preview(active, h, s, v)}
        onSettle={gradient.panelPicker.settle}
        onOpenEntry={() =>
          section.current
            ?.querySelector<HTMLInputElement>(`input[aria-label="Stop ${active + 1} colour"]`)
            ?.focus()
        }
      />
    </div>
  );
  const actions = (
    <div className="flex items-center gap-1">
      {!view.mixedGradients && (
        <Tooltip side="left" content="Add stop">
          <button type="button" aria-label="Add stop" data-gradient-add="" onClick={pressButton(gradient.addStop)} className={OUTLINED}>
            <Plus size={16} strokeWidth={1.5} aria-hidden />
          </button>
        </Tooltip>
      )}
      <Tooltip side="left" content="Reverse">
        <button type="button" aria-label="Reverse" data-gradient-reverse="" onClick={pressButton(gradient.reverse)} className={OUTLINED}>
          <ArrowLeftRight size={16} strokeWidth={1.5} aria-hidden />
        </button>
      </Tooltip>
    </div>
  );
  return (
    <div ref={section} data-gradient-block className="flex flex-col gap-2">
      <div className="flex h-7 items-center gap-2">
        <ToggleGroup
          label="Gradient kind"
          options={KIND_OPTIONS}
          value={view.kind === "linear" || view.kind === "radial" ? view.kind : null}
          onChange={gradient.setKind}
          itemWidth={44}
          onReturnFocus={onReturnFocus}
        />
        {view.angleShown && (
          <EntryField
            label="Gradient angle, degrees"
            prefix="Angle"
            shown={view.angle}
            mixed={view.angle === ""}
            suffix="°"
            width={148}
            onSubmit={gradient.setAngleText}
            messages={ANGLE_MESSAGES}
            stepText={stepAngle}
            onReturnFocus={onReturnFocus}
          />
        )}
        {view.radiusShown && (
          <EntryField
            label="Gradient radius, millimetres"
            prefix="Radius"
            shown={view.radius}
            mixed={view.radius === ""}
            suffix="mm"
            width={148}
            onSubmit={gradient.setRadiusText}
            messages={RADIUS_MESSAGES}
            stepText={stepRadius}
            onReturnFocus={onReturnFocus}
          />
        )}
      </div>
      {view.mixedGradients ? (
        <p className="h-4 text-xs text-[var(--panel-muted-fg)]">Mixed gradients</p>
      ) : windowed ? (
        <>
          <div
            ref={list}
            data-gradient-stop-window=""
            onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
            className="relative -ml-2 overflow-y-auto pl-2"
            style={{ height: STOP_WINDOW_ROWS * STOP_ROW_PX - 4 }}
          >
            <div className="relative" style={{ height: view.stops.length * STOP_ROW_PX - 4 }}>
              {view.stops.slice(shown.first, shown.end).map((row, offset) => {
                const index = shown.first + offset;
                return (
                  <div key={index} className="absolute right-0 left-0" style={{ top: index * STOP_ROW_PX }}>
                    {stopRow(row, index)}
                  </div>
                );
              })}
            </div>
          </div>
          {activeRow && picker}
        </>
      ) : (
        // The picker is drawn under the active row (CSS `order`) but comes after every row in the
        // document, so Tab runs through the rows, then the picker (criterion 7).
        <div className="flex flex-col gap-1">
          {view.stops.map((row, index) => (
            <div key={index} style={{ order: 2 * index }}>
              {stopRow(row, index)}
            </div>
          ))}
          {activeRow && <div style={{ order: 2 * active + 1 }}>{picker}</div>}
        </div>
      )}
      {actions}
    </div>
  );
}

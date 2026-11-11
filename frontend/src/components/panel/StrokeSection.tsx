import { OPACITY_CLICK_MIN_P, VALUE_FIELD_NAMES, VALUE_FIELD_SCALE_END } from "@/lib/sliderNames";
import { useHexEntry } from "@/hooks/useHexEntry";
import { useRef } from "react";

import { ColourBlock } from "@/components/panel/ColourBlock";
import { ColourPicker } from "@/components/panel/ColourPicker";
import { DashRows } from "@/components/panel/DashRows";
import { EntryField } from "@/components/panel/EntryField";
import { MarkerRows } from "@/components/panel/MarkerRows";
import { PaintRow } from "@/components/panel/PaintRow";
import { StyleRow } from "@/components/panel/StyleRow";
import { CAP_OPTIONS, JOIN_OPTIONS, PAINT_OPTIONS } from "@/components/panel/styleOptions";
import { ValueField } from "@/components/panel/ValueField";
import { useRowsFocus } from "@/components/panel/useRowsFocus";
import { ToggleGroup } from "@/components/ui/toggle-group";
import { Tooltip } from "@/components/ui/tooltip";
import { WIDTH_PRESETS, WIDTH_PRESET_GLYPHS } from "@/lib/widthPresets";
import type { StylePanelApi } from "@/hooks/useStylePanel";

/** The segmented-control look of `ToggleGroup` for the width rows' command buttons. */
const SEGMENTS =
  "inline-flex w-fit shrink-0 self-start overflow-hidden rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]";
const SEGMENT =
  "flex h-7 items-center justify-center bg-transparent text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] aria-pressed:!bg-[var(--toolbar-icon-active-bg)] aria-pressed:!text-[var(--toolbar-icon-active-fg)]";
const SEGMENT_DIVIDER = "border-l border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]";

const WIDTH_MESSAGES = { width: "Enter a number from 0 to 1000" } as const;
const POINT_WIDTH_MESSAGES = { width: "Enter a width of 0 mm or more" } as const;

/** A point width as the field shows it: up to three decimals, no trailing zeros. */
function formatPointWidth(mm: number): string {
  return String(Number(mm.toFixed(3)));
}
const PERCENT_MESSAGES = { percent: "Enter a number from 0 to 100" } as const;

interface StrokeSectionProps {
  panel: StylePanelApi;
  onReturnFocus: () => void;
}

/**
 * The stroke section (`specs/0017-style-panel-rework` criteria 5 to 10): the
 * Stroke title with its Paint switch on one row, and under it Color, the
 * picker, Opacity, Width, Dash, Dashes, Join and Cap. While every edited
 * stroke is off the rows are not in the tree at all; with some on and some off
 * they show their stored values.
 */
export function StrokeSection({ panel, onReturnFocus }: StrokeSectionProps) {
  const { view } = panel;
  const section = useRef<HTMLDivElement>(null);
  const hex = useRef<HTMLInputElement>(null);
  const hexEntry = useHexEntry(hex, onReturnFocus);
  const rowsFocus = useRowsFocus(view.strokeRows, section);
  const markersFocus = useRowsFocus(view.markersShown, section);
  return (
    <div ref={section} data-style-section className="flex flex-col gap-2">
      <PaintRow
        title="Stroke"
        groupLabel="Stroke paint"
        options={PAINT_OPTIONS}
        paint={view.strokePaint}
        onChange={panel.setStrokePaint}
        onReturnFocus={onReturnFocus}
        firstFocus
      />
      {view.strokeRows && (
        <div className="contents" {...rowsFocus}>
          <ColourBlock
            name="Stroke"
            rgb={view.strokeColor}
            hex={view.strokeHex}
            hexMixed={view.strokeHexMixed}
            opacity={view.strokeOpacity}
            picking={view.pickTarget === "stroke"}
            onPick={() => panel.beginPick("stroke")}
            onSubmitHex={(text) => panel.setText("stroke-color", text)}
            onReturnFocus={hexEntry.returnFocus}
            hexRef={hex}
          />
          <ColourPicker
            name="Stroke"
            rgb={view.strokeColor}
            mixed={view.strokeColorMixed}
            hsvOf={panel.hsvOf}
            rgbOf={panel.rgbOf}
            onPreview={(hue, saturation, value) =>
              panel.previewHsv("stroke-color", hue, saturation, value)
            }
            onSettle={panel.settle}
            onOpenEntry={hexEntry.open}
          />
          <ValueField
            label="Opacity"
            name={VALUE_FIELD_NAMES.strokeOpacity}
            text={view.strokeOpacityText}
            bar={view.strokeOpacityBar}
            mixed={view.strokeOpacityMixed}
            resettable={view.strokeOpacityResettable}
            unit="%"
            valueNow={view.strokeOpacity}
            unitWords="percent"
            valueMax={VALUE_FIELD_SCALE_END.opacity}
            clickMinP={OPACITY_CLICK_MIN_P}
            defaultText={view.opacityDefaultText}
            messages={PERCENT_MESSAGES}
            onPreview={(p, grid) => panel.previewValue("stroke-opacity", p, grid)}
            onSettle={panel.settle}
            onStep={(steps, grid) => panel.stepValue("stroke-opacity", steps, grid)}
            onReset={() => panel.resetValue("stroke-opacity")}
            onSubmit={(text) => panel.setText("stroke-opacity", text)}
            onReturnFocus={onReturnFocus}
          />
          <ValueField
            label="Width"
            name={VALUE_FIELD_NAMES.strokeWidth}
            text={view.strokeWidthText}
            bar={view.strokeWidthBar}
            dragBase={view.strokeWidthDragBase}
            dragLimit={view.widthDragLimit}
            mixed={view.strokeWidthMixed}
            resettable={view.strokeWidthResettable}
            unit="mm"
            valueNow={view.strokeWidth}
            unitWords="millimetres"
            valueMax={VALUE_FIELD_SCALE_END.strokeWidth}
            defaultText={view.widthDefaultText}
            messages={WIDTH_MESSAGES}
            onPreview={(p, grid) => panel.previewValue("stroke-width", p, grid)}
            onSettle={panel.settle}
            onStep={(steps, grid) => panel.stepValue("stroke-width", steps, grid)}
            onReset={() => panel.resetValue("stroke-width")}
            onSubmit={(text) => panel.setText("stroke-width", text)}
            onReturnFocus={onReturnFocus}
          />
          <StyleRow label="Profile">
            {/* The segmented look; the preset the path has is marked (UX review U9). */}
            <div role="group" aria-label="Width profile" className={SEGMENTS}>
              {WIDTH_PRESETS.map((preset, index) => (
                <Tooltip key={preset.code} content={preset.tooltip} side="bottom">
                  <button
                    type="button"
                    aria-label={preset.label}
                    aria-pressed={panel.widthPreset === preset.code}
                    className={`${SEGMENT} w-9 ${index > 0 ? SEGMENT_DIVIDER : ""}`}
                    onClick={() => {
                      panel.applyWidthPreset(preset.code);
                      onReturnFocus();
                    }}
                  >
                    <svg width={18} height={18} viewBox="0 0 24 24" aria-hidden="true">
                      <path d={WIDTH_PRESET_GLYPHS[preset.code]} fill="currentColor" />
                    </svg>
                  </button>
                </Tooltip>
              ))}
            </div>
          </StyleRow>
          <StyleRow label="Edit">
            {/* One Done only, in the canvas bar: here the button stays "Edit width", pressed
             * while the mode is on (UX review U4, U9). */}
            <div className={SEGMENTS}>
              <button
                type="button"
                aria-pressed={panel.widthEditActive}
                className={`${SEGMENT} px-3 text-xs whitespace-nowrap`}
                onClick={() => {
                  if (panel.widthEditActive) {
                    panel.endWidthEdit();
                  } else {
                    panel.beginWidthEdit();
                  }
                  onReturnFocus();
                }}
              >
                Edit width
              </button>
            </div>
          </StyleRow>
          {panel.widthEditActive && panel.widthPointMm !== null ? (
            <StyleRow label="Point">
              <EntryField
                label="Width at the selected point, millimetres"
                shown={formatPointWidth(panel.widthPointMm)}
                mixed={false}
                suffix="mm"
                width={96}
                inputMode="decimal"
                onSubmit={panel.setWidthPointMm}
                messages={POINT_WIDTH_MESSAGES}
                onReturnFocus={onReturnFocus}
              />
            </StyleRow>
          ) : null}
          <DashRows panel={panel} onReturnFocus={onReturnFocus} />
          <StyleRow label="Join">
            <ToggleGroup
              label="Stroke join"
              options={JOIN_OPTIONS}
              value={view.strokeJoin === "mixed" ? null : view.strokeJoin}
              onChange={panel.setStrokeJoin}
              itemWidth={40}
              onReturnFocus={onReturnFocus}
            />
          </StyleRow>
          <StyleRow label="Cap">
            <ToggleGroup
              label="Stroke cap"
              options={CAP_OPTIONS}
              value={view.strokeCap === "mixed" ? null : view.strokeCap}
              onChange={panel.setStrokeCap}
              itemWidth={40}
              onReturnFocus={onReturnFocus}
            />
          </StyleRow>
          {view.markersShown && (
            <div className="contents" {...markersFocus}>
              <MarkerRows panel={panel} onReturnFocus={onReturnFocus} />
            </div>
          )}
        </div>
      )}
    </div>
  );
}

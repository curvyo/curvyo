import { Ban, Eye, Merge, Paintbrush, X, type LucideIcon } from "lucide-react";
import { useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { ColourPicker } from "@/components/panel/ColourPicker";
import { EntryField } from "@/components/panel/EntryField";
import { Swatch } from "@/components/panel/Swatch";
import { ICON_BUTTON_CLASS } from "@/components/TextBar";
import { Tooltip as AppTooltip } from "@/components/ui/tooltip";
import type { TracePaletteApi } from "@/hooks/useTracePalette";
import type { HsvTriple } from "@/hooks/useStylePanel";
import {
  EXCLUDE,
  EXCLUDED,
  ISOLATE,
  MERGE_INTO,
  PALETTE,
  RECOLOUR,
  RECOLOUR_HEX_MESSAGE,
  REMOVE,
  RESET,
  colourName,
  mergedIntoText,
  recolourHex,
  recolouredText,
  swatchName,
} from "@/lib/tracePaletteText";

const TEXT_BUTTON_CLASS =
  "h-7 whitespace-nowrap rounded-md px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** `#RRGGBB` as the packed colour the swatch takes. */
function packed(hex: string): number {
  return Number.parseInt(hex.slice(1, 7), 16);
}

function hexOf(rgb: number): string {
  return `#${rgb.toString(16).padStart(6, "0").toUpperCase()}`;
}

function pressedClass(on: boolean): string {
  return on ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]" : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]";
}

/** One icon action of the detail row: its name in the tooltip and as its accessible name. */
function Action({
  label,
  icon: Icon,
  pressed,
  disabled = false,
  onPress,
}: {
  label: string;
  icon: LucideIcon;
  pressed?: boolean;
  disabled?: boolean;
  onPress: () => void;
}) {
  return (
    <AppTooltip side="bottom" content={label}>
      <button
        type="button"
        aria-label={label}
        aria-pressed={pressed}
        aria-disabled={disabled || undefined}
        data-trace-palette-action={label}
        onClick={() => {
          if (!disabled) {
            onPress();
          }
        }}
        className={`${ICON_BUTTON_CLASS} ${disabled ? "opacity-40" : pressedClass(Boolean(pressed))}`}
      >
        <Icon size={16} strokeWidth={1.5} aria-hidden />
      </button>
    </AppTooltip>
  );
}

interface TracePaletteProps {
  palette: TracePaletteApi;
  hsvOf: (rgb: number) => HsvTriple;
  rgbOf: (hue: number, saturation: number, value: number) => number;
}

/**
 * The palette of a multicolour trace (`specs/0089` criteria 26 and 29, UX notes §7): a strip of
 * swatches (one Tab stop, the arrows move and select; picked colours carry a square corner mark,
 * an excluded colour the Ban glyph at 40 %, a merged one its target's dot at 50 %) and, for the
 * selected swatch, the detail row with its hex and name and the five actions (Pick colours with Single colour: Remove only); Recolour opens the
 * Style section's inline picker under it. Nothing needs colour perception: every state is a
 * shape and words in the accessible name.
 */
export function TracePalette({ palette, hsvOf, rgbOf }: TracePaletteProps) {
  const [focus, setFocus] = useState(0);
  const [picking, setPicking] = useState(false);
  const strip = useRef<HTMLDivElement>(null);
  const { swatches, selected, merging } = palette;
  if (swatches.length === 0) {
    return null;
  }
  const at = Math.min(focus, swatches.length - 1);
  const choose = (hex: string) => {
    if (merging) {
      palette.mergeInto(hex);
      return;
    }
    setPicking(false);
    palette.select(selected === hex ? null : hex);
  };
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (step !== 0) {
      event.preventDefault();
      const next = (at + step + swatches.length) % swatches.length;
      setFocus(next);
      if (!merging) {
        palette.select(swatches[next].hex);
      }
      (event.currentTarget.querySelectorAll("[data-trace-palette-swatch]")[next] as HTMLElement | undefined)?.focus();
    } else if ((event.key === "Delete" || event.key === "Backspace") && swatches[at]?.fixed && !merging) {
      // A picked colour goes (`ux-notes.md` §22.4); the strip keeps its one Tab stop.
      event.preventDefault();
      palette.remove(swatches[at].hex);
      const root = event.currentTarget;
      requestAnimationFrame(() => root.querySelector<HTMLElement>('[data-trace-palette-swatch][tabindex="0"]')?.focus());
    } else if (event.key === "Escape" && (merging || selected)) {
      event.preventDefault();
      event.stopPropagation();
      if (merging) {
        palette.cancelMerge();
      } else {
        palette.select(null);
      }
    }
  };
  const detail = selected ? swatches.find((s) => s.hex === selected) : undefined;
  const detailState = detail ? palette.state(detail.hex) : null;
  const words = (hex: string) => {
    const state = palette.state(hex);
    const parts = [swatchName(hex, state.fill)];
    if (swatches.find((s) => s.hex === hex)?.fixed) parts.push("picked");
    if (state.excluded) parts.push("excluded");
    if (state.mergedInto) parts.push(`merged into ${state.mergedInto}`);
    if (palette.isolated === hex) parts.push("isolated");
    return parts.join(", ");
  };
  return (
    <div data-trace-palette="" className="flex flex-col py-1">
      <div
        ref={strip}
        role="listbox"
        aria-label={PALETTE}
        aria-orientation="horizontal"
        className="flex flex-wrap items-center gap-1"
        onKeyDown={keyDown}
      >
        {swatches.map((swatch, index) => {
          const state = palette.state(swatch.hex);
          const isSelected = selected === swatch.hex;
          const target = merging !== null && merging !== swatch.hex;
          return (
            <button
              key={swatch.hex}
              type="button"
              role="option"
              aria-selected={isSelected}
              aria-label={words(swatch.hex)}
              data-trace-palette-swatch={swatch.hex}
              tabIndex={index === at ? 0 : -1}
              onClick={() => {
                setFocus(index);
                choose(swatch.hex);
              }}
              className={`relative flex size-7 items-center justify-center rounded-md outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
                isSelected ? "ring-2 ring-[var(--editor-accent)]" : target ? "outline-dashed outline-1 outline-[var(--editor-accent)]" : ""
              }`}
            >
              <span className={state.excluded ? "opacity-40" : state.mergedInto ? "opacity-50" : ""}>
                <Swatch rgb={packed(state.fill)} opacity={100} mixed={false} size={20} />
              </span>
              {swatch.fixed ? (
                <span aria-hidden data-trace-mark="picked" className="absolute left-0.5 top-0.5 size-1.5 bg-[var(--toolbar-icon)]" />
              ) : null}
              {state.excluded ? (
                <Ban aria-hidden data-trace-mark="excluded" size={14} strokeWidth={2} className="absolute text-[var(--toolbar-icon)]" />
              ) : null}
              {state.mergedInto ? (
                <span
                  aria-hidden
                  data-trace-mark="merged"
                  className="absolute bottom-0.5 right-0.5 size-1.5 rounded-full"
                  style={{ background: state.mergedInto, boxShadow: "0 0 0 1px var(--swatch-border)" }}
                />
              ) : null}
            </button>
          );
        })}
      </div>
      {detail && detailState ? (
        <div data-trace-palette-detail="" className="flex h-9 items-center gap-1">
          <span className="mr-1 text-xs tabular-nums text-[var(--toolbar-icon)]">
            {detailState.fill !== detail.hex ? recolouredText(detailState.fill, detail.hex) : `${detail.hex} · ${colourName(detail.hex)}`}
            {detailState.excluded ? ` · ${EXCLUDED}` : ""}
            {detailState.mergedInto ? ` · ${mergedIntoText(detailState.mergedInto)}` : ""}
          </span>
          {palette.pickedOnly ? null : (
            <>
              <Action label={ISOLATE} icon={Eye} pressed={palette.isolated === detail.hex} onPress={() => palette.isolate(detail.hex)} />
              <Action label={RECOLOUR} icon={Paintbrush} pressed={picking} onPress={() => setPicking((p) => !p)} />
              <Action
                label={MERGE_INTO}
                icon={Merge}
                pressed={merging === detail.hex || detailState.mergedInto !== null}
                disabled={swatches.length < 2}
                onPress={() => palette.startMerge(detail.hex)}
              />
              <Action label={EXCLUDE} icon={Ban} pressed={detailState.excluded} onPress={() => palette.toggleExclude(detail.hex)} />
            </>
          )}
          {detail.fixed ? <Action label={REMOVE} icon={X} onPress={() => palette.remove(detail.hex)} /> : null}
        </div>
      ) : null}
      {detail && detailState && picking ? (
        <div data-trace-palette-picker="" className="flex flex-col gap-1 py-1">
          <div className="flex h-7 items-center gap-2">
            <EntryField
              label={`${RECOLOUR} hex`}
              shown={detailState.fill}
              mixed={false}
              width={96}
              align="left"
              onSubmit={(text) => {
                const fill = recolourHex(text);
                if (fill === null) {
                  return "invalid:hex";
                }
                if (fill === detailState.fill) {
                  return "unchanged";
                }
                palette.recolour(detail.hex, fill);
                return "committed";
              }}
              messages={{ hex: RECOLOUR_HEX_MESSAGE }}
              onReturnFocus={() => strip.current?.querySelector<HTMLElement>('[data-trace-palette-swatch][tabindex="0"]')?.focus()}
            />
            <button type="button" className={TEXT_BUTTON_CLASS} onClick={() => palette.recolour(detail.hex, null)}>
              {RESET}
            </button>
          </div>
          <ColourPicker
            name={RECOLOUR}
            rgb={packed(detailState.fill)}
            mixed={false}
            hsvOf={hsvOf}
            rgbOf={rgbOf}
            onPreview={(hue, saturation, value) => palette.recolour(detail.hex, hexOf(rgbOf(hue, saturation, value)))}
            onSettle={() => undefined}
            onOpenEntry={() => undefined}
          />
        </div>
      ) : null}
    </div>
  );
}

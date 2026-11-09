import { useEffect, useRef, useState, type KeyboardEvent } from "react";

import { SwatchCell } from "@/components/TraceColourGroup";
import type { TracePaletteApi } from "@/hooks/useTracePalette";
import { focusAfterRemoval } from "@/lib/tracePickedColours";
import { PALETTE, PALETTE_HINT, paletteKey, paletteSwatchLabel, paletteTooltip } from "@/lib/tracePaletteText";

interface TracePaletteProps {
  palette: TracePaletteApi;
  /** The row belongs to an older result (a running or stopped trace): drawn at 50 %. */
  stale: boolean;
  onReturnFocus: () => void;
}

/**
 * The palette of a multicolour trace beside Colours (`specs/0089` criteria 26 and 29, UX notes
 * §26): the swatches of the Pick colours row, one Tab stop with roving arrows; a click toggles
 * Exclude, Shift while pointing isolates, Shift+click deletes. Nothing needs colour perception:
 * the colour's code, name and state are the accessible name.
 */
export function TracePalette({ palette, stale, onReturnFocus }: TracePaletteProps) {
  const [focus, setFocus] = useState(0);
  const [hovered, setHovered] = useState<string | null>(null);
  const [shift, setShift] = useState(false);
  const row = useRef<HTMLDivElement>(null);
  const { swatches, setIsolated } = palette;

  // Shift+hover isolates (a view only); it ends with the Shift key or the hover.
  useEffect(() => {
    const keys = (event: globalThis.KeyboardEvent) => setShift(event.shiftKey);
    const blur = () => setShift(false);
    window.addEventListener("keydown", keys);
    window.addEventListener("keyup", keys);
    window.addEventListener("blur", blur);
    return () => {
      window.removeEventListener("keydown", keys);
      window.removeEventListener("keyup", keys);
      window.removeEventListener("blur", blur);
    };
  }, []);
  const target = hovered !== null && shift && swatches.some((s) => s.hex === hovered) ? hovered : null;
  useEffect(() => {
    setIsolated(target);
  }, [target, setIsolated]);
  useEffect(() => () => setIsolated(null), [setIsolated]);

  if (swatches.length === 0) {
    return null;
  }
  const at = Math.min(focus, swatches.length - 1);
  const focusSwatch = (index: number) => {
    requestAnimationFrame(() => row.current?.querySelectorAll<HTMLElement>("[data-trace-palette-swatch]")[index]?.focus());
  };
  const remove = (index: number, byKey: boolean) => {
    const hex = swatches[index]?.hex;
    if (hex === undefined) {
      return;
    }
    setHovered(null);
    const next = focusAfterRemoval(index, swatches.length) ?? 0;
    setFocus(next);
    palette.remove(hex);
    if (byKey) {
      focusSwatch(next);
    } else {
      onReturnFocus();
    }
  };
  const keyDown = (index: number) => (event: KeyboardEvent<HTMLButtonElement>) => {
    const action = paletteKey(event.key, event.shiftKey, index, swatches.length);
    if (action === null) {
      return;
    }
    // Enter and Space are handled here, so the button's own click does not toggle again; Delete
    // and Backspace belong to the swatch, not to the canvas's delete.
    event.preventDefault();
    event.stopPropagation();
    if ("move" in action) {
      setFocus(action.move);
      row.current?.querySelectorAll<HTMLElement>("[data-trace-palette-swatch]")[action.move]?.focus();
    } else if ("toggle" in action) {
      setFocus(action.toggle);
      palette.toggleExclude(swatches[action.toggle].hex);
    } else {
      remove(action.remove, true);
    }
  };
  return (
    <div ref={row} role="group" aria-label={PALETTE} data-trace-palette="" className={`my-1 mr-[-2px] ml-[2px] flex flex-wrap ${stale ? "opacity-50" : ""}`}>
      {swatches.map((swatch, index) => {
        const excluded = palette.excluded(swatch.hex);
        return (
          <SwatchCell
            key={swatch.hex}
            hex={swatch.hex}
            kind="palette"
            stop={index === at}
            label={paletteSwatchLabel(swatch.hex, excluded)}
            tooltip={
              <>
                <span className="block tabular-nums">{paletteTooltip(swatch.hex)}</span>
                <span className="block text-[10px] opacity-70">{PALETTE_HINT}</span>
              </>
            }
            excluded={excluded}
            onClick={(event) => {
              // A key press is handled in `keyDown`; its click is not a second press.
              if (event.detail === 0) {
                return;
              }
              setFocus(index);
              if (event.shiftKey) {
                remove(index, false);
              } else {
                palette.toggleExclude(swatch.hex);
                onReturnFocus();
              }
            }}
            onKeyDown={keyDown(index)}
            onPointerMove={(event) => {
              setShift(event.shiftKey);
              setHovered(swatch.hex);
            }}
            onPointerLeave={() => setHovered((now) => (now === swatch.hex ? null : now))}
          />
        );
      })}
    </div>
  );
}

import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";

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
 * §26, §27): the swatches of the Pick colours row, one Tab stop with roving arrows; a press on a
 * swatch's x deletes its colour, Ctrl+click (Cmd on a Mac) toggles Exclude, a plain click elsewhere does
 * nothing. Nothing needs colour perception: the colour's code, name and state are the accessible
 * name.
 */
export function TracePalette({ palette, stale, onReturnFocus }: TracePaletteProps) {
  const [focus, setFocus] = useState(0);
  const row = useRef<HTMLDivElement>(null);
  const { swatches } = palette;
  // After a key removal: the swatch (the next one, else the previous) to focus once the new swatch list (not the old one) is drawn.
  const pendingFocus = useRef<{ target: string; hex: string } | null>(null);
  useEffect(() => {
    const pending = pendingFocus.current;
    if (pending === null || swatches.some((s) => s.hex === pending.hex)) {
      return;
    }
    pendingFocus.current = null;
    if (swatches.length === 0) {
      // The last colour went: the keyboard goes back to the canvas.
      onReturnFocus();
      return;
    }
    const index = swatches.findIndex((s) => s.hex === pending.target);
    row.current?.querySelectorAll<HTMLElement>("[data-trace-palette-swatch]")[index < 0 ? swatches.length - 1 : index]?.focus();
  }, [swatches, onReturnFocus]);

  if (swatches.length === 0) {
    return null;
  }
  const at = Math.min(focus, swatches.length - 1);
  const remove = (index: number, byKey: boolean) => {
    const hex = swatches[index]?.hex;
    if (hex === undefined) {
      return;
    }
    const next = focusAfterRemoval(index, swatches.length) ?? 0;
    setFocus(next);
    palette.remove(hex);
    if (byKey) {
      pendingFocus.current = { target: swatches[index + 1]?.hex ?? swatches[index - 1]?.hex ?? "", hex };
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
              if (event.ctrlKey || event.metaKey) {
                palette.toggleExclude(swatch.hex);
                onReturnFocus();
              } else if (onRemoveGlyph(event)) {
                remove(index, false);
              }
            }}
            onKeyDown={keyDown(index)}
          />
        );
      })}
    </div>
  );
}

/** Half the x glyph with its casing, px: 12 px plus 1.5 px either side. */
const X_REACH = 7.5;

/** Whether a press landed on the swatch's x (`ux-notes.md` §27: only the x deletes), by where it
 * landed, so it also works where the x was not drawn yet (no hover on a touch screen). */
function onRemoveGlyph(event: MouseEvent<HTMLButtonElement>): boolean {
  const cell = event.currentTarget.getBoundingClientRect();
  return Math.abs(event.clientX - (cell.left + cell.width / 2)) <= X_REACH && Math.abs(event.clientY - (cell.top + cell.height / 2)) <= X_REACH;
}

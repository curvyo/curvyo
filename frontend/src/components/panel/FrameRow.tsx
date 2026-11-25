import { Scan } from "lucide-react";
import { type KeyboardEvent, useEffect, useRef } from "react";

import { GLYPH } from "@/components/workbench/machineStyles";
import { SymbolButton } from "@/components/workbench/SymbolButton";
import { useFocusMode } from "@/hooks/useFocusPoint";
import { type MachineControlsModel } from "@/hooks/useMachineControls";
import { type FramePage } from "@/hooks/useFramePage";
import { FRAME_SHAPES, frameSlot, headPlace, nextShape } from "@/lib/frameControls";

/**
 * The Frame row of the Machine controls footer (`specs/0118` slice 2b, UX notes W28 to W38): the
 * Frame button, which traces the page's outline with the laser off, and its pick Rectangle |
 * Rubber band, directly above the toolbar. While this app's frame runs it also gives the head
 * ring on the canvas the controller's position (W33).
 */
export function FrameRow({ model, frame }: { model: MachineControlsModel; frame: FramePage }) {
  const { entry } = model;
  const status = entry.status;
  const slot = frameSlot(entry, frame.view);
  const shape = frame.shape;
  const radios = useRef<(HTMLButtonElement | null)[]>([]);
  // The ring follows the status while the frame this app sent runs; the first position of a
  // Head position frame is where it began (W33).
  const first = useRef<[number, number] | null>(null);
  const framing = status?.state.name === "Framing";
  const sent = model.sentFrame;
  const { head, clearHead, relative } = frame;
  // While the Focus point mode is on, the ring shows the head at its work position (W59).
  const focusing = useFocusMode();
  useEffect(() => {
    if (!framing) first.current = null;
    else if (first.current === null && status?.position) first.current = [status.position[0], status.position[1]];
    const at = status?.position ?? null;
    const place = framing && sent ? headPlace(status, relative, first.current) : focusing && at !== null ? { x: at[0], y: at[1] } : null;
    if (place === null) clearHead();
    else head(place.x, place.y);
  }, [status, framing, sent, relative, head, clearHead, focusing]);
  useEffect(() => clearHead, [clearHead]);
  const move = (event: KeyboardEvent, at: number) => {
    const to = FRAME_SHAPES.findIndex((s) => s.value === nextShape(shape, event.key));
    if (to === at) return;
    event.preventDefault();
    frame.setShape(FRAME_SHAPES[to].value);
    radios.current[to]?.focus();
  };
  const press = () => {
    if (slot.name === "Confirm frame") model.confirmFrame();
    else model.frame(frame.bytes);
  };
  return (
    <div role="group" aria-label="Frame" className="flex items-center gap-2">
      <SymbolButton
        name={slot.name}
        enabled={slot.enabled}
        busy={slot.busy}
        cause={slot.cause}
        describedBy={slot.name === "Confirm frame" ? "machine-controls-state" : undefined}
        side="top"
        onPress={press}
      >
        <Scan {...GLYPH} />
      </SymbolButton>
      <div role="radiogroup" aria-label="Frame shape" className="flex overflow-hidden rounded-md border border-[var(--field-border)]">
        {FRAME_SHAPES.map(({ value, label }, at) => (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={shape === value}
            tabIndex={shape === value ? 0 : -1}
            ref={(node) => {
              radios.current[at] = node;
            }}
            className={`h-7 px-2 text-xs outline-none pointer-coarse:h-9 pointer-coarse:px-2.5 focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
              shape === value
                ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
                : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
            }`}
            onClick={() => frame.setShape(value)}
            onKeyDown={(event) => move(event, at)}
          >
            {label}
          </button>
        ))}
      </div>
    </div>
  );
}

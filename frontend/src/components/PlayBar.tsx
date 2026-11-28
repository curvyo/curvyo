import { Pause, Play } from "lucide-react";
import { useLayoutEffect, useRef, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { usePlayBar } from "@/hooks/usePlayBar";
import { usePublishPlayBarClearance } from "@/hooks/usePlayBarClearance";
import { useSliderPointer } from "@/hooks/useSliderPointer";
import type { WasmSession } from "@/lib/editorSession";
import { barPlace, readoutParts, sliderMinWidth } from "@/lib/outputText";

const THUMB = 14;
/** The embroidery Speed segments (`0160` E13: 1× is the machine's speed). */
const SPEEDS = [1, 4, 16];
/** The embroidery bar's minimum width, which holds the Speed segments (`0160` E14). */
const SPEED_BAR_MIN = 440;
const INSET = THUMB / 2 + 1;
const TARGET_HEIGHT = 24;

/** Where the page frame is in the canvas, px. */
export interface BarFrame {
  left: number;
  bottom: number;
  width: number;
}

/** The slider's width, tracked: the pointer rules work in the slider's own pixels. */
function useWidth(ref: React.RefObject<HTMLElement | null>): number {
  const [width, setWidth] = useState(200);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return undefined;
    const read = () => setWidth(element.getBoundingClientRect().width);
    read();
    const observer = new ResizeObserver(read);
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref]);
  return width;
}

/**
 * The play bar of the Toolpath view (`specs/0029-laser-job-preview-and-output` criterion 37; UX
 * notes 5): Play and Pause as one symbol button, the one slider of the job's time (the look and
 * pointer rules of the Ratio sliders, `useSliderPointer`), and the time-only readout on one line. Shown only while the
 * page's Toolpath view is pressed; the controls are disabled, not hidden, while nothing runs.
 */
export function PlayBar({ getSession, frame }: { getSession: () => WasmSession | null; frame: BarFrame }) {
  const play = usePlayBar(getSession);
  const bar = useRef<HTMLDivElement>(null);
  const track = useRef<HTMLDivElement>(null);
  const trackWidth = useWidth(track);
  const [barHeight, setBarHeight] = useState(40);
  const [canvas, setCanvas] = useState({ width: 10000, height: 10000 });
  const [leftInset, setLeftInset] = useState(8);
  // The bar's own height (it wraps) and the canvas it sticks to.
  useLayoutEffect(() => {
    const element = bar.current;
    const parent = element?.parentElement;
    if (!element || !parent) return undefined;
    const read = () => {
      setBarHeight(element.getBoundingClientRect().height);
      setCanvas({ width: parent.clientWidth, height: parent.clientHeight });
      // The tool columns end at `--rail-right`; the bar keeps 12 px from them.
      const rail = parseFloat(getComputedStyle(parent).getPropertyValue("--rail-right"));
      setLeftInset(Number.isFinite(rail) ? rail + 12 : 8);
    };
    read();
    const observer = new ResizeObserver(read);
    observer.observe(element);
    observer.observe(parent);
    return () => observer.disconnect();
  }, []);
  const view = play.view;
  const place = barPlace(frame, canvas, barHeight, leftInset);
  usePublishPlayBarClearance(canvas.height - place.top);
  const fraction = view?.fraction ?? 1;
  const enabled = view?.enabled ?? false;
  const inner = Math.max(1, trackWidth - 2 * INSET);
  const thumbLeft = INSET + fraction * inner;
  const at = (x: number) => Math.min(1, Math.max(0, (x - INSET) / inner));
  const pointer = useSliderPointer({
    thumb: () => ({ at: { x: thumbLeft, y: TARGET_HEIGHT / 2 }, radius: THUMB / 2 }),
    onClick: (point) => play.seek(at(point.x)),
    onDrag: (point) => play.seek(at(point.x)),
    onDragEnd: () => {},
    onDoubleClick: () => {},
  });
  const readout = view?.readout ?? "";
  const playing = view?.playing ?? false;
  const Glyph = playing ? Pause : Play;
  const name = playing ? "Pause" : "Play";
  return (
    <div
      ref={bar}
      role="group"
      aria-label="Toolpath playback"
      data-play-bar
      className="pointer-events-auto absolute z-10 flex h-10 items-center gap-x-2.5 rounded-lg bg-[var(--toolbar-bg)] py-1 pr-3 pl-1.5"
      style={{
        ...place,
        ...(view?.speed ? { width: Math.max(place.width, SPEED_BAR_MIN) } : {}),
        boxShadow: "var(--panel-elevation-shadow)",
      }}
    >
      <Tooltip side="top" content={name}>
        <button
          type="button"
          aria-label={name}
          disabled={!enabled}
          onClick={play.toggle}
          className="inline-flex size-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] disabled:opacity-40 disabled:hover:bg-transparent"
        >
          <Glyph size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        </button>
      </Tooltip>
      <div
        ref={track}
        role="slider"
        tabIndex={enabled ? 0 : -1}
        aria-label="Run position"
        aria-disabled={!enabled}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(fraction * 100)}
        aria-valuetext={readout}
        className={`group relative flex-1 rounded-[5px] outline-none ${enabled ? "cursor-pointer" : "opacity-40"}`}
        style={{ height: TARGET_HEIGHT, minWidth: sliderMinWidth(place.width), touchAction: "manipulation" }}
        onPointerDown={enabled ? pointer.onPointerDown : undefined}
        onPointerMove={enabled ? pointer.onPointerMove : undefined}
        onPointerUp={enabled ? pointer.onPointerUp : undefined}
        onPointerCancel={enabled ? pointer.onPointerCancel : undefined}
        onKeyDown={(event) => {
          if (enabled && play.key(event.key)) event.preventDefault();
        }}
      >
        <span
          aria-hidden
          className="pointer-events-none absolute top-1/2 h-1 -translate-y-1/2 rounded-full"
          style={{ left: INSET, right: INSET, background: "color-mix(in srgb, var(--toolbar-icon) 35%, transparent)" }}
        />
        <span
          aria-hidden
          className="pointer-events-none absolute top-1/2 h-1 -translate-y-1/2 rounded-full bg-[var(--editor-accent)]"
          style={{ left: INSET, width: thumbLeft - INSET }}
        />
        <span
          aria-hidden
          className="pointer-events-none absolute top-1/2 rounded-full border-2 border-[var(--editor-accent)] bg-white group-focus-visible:shadow-[0_0_0_2px_var(--toolbar-bg),0_0_0_4px_var(--editor-accent)]"
          style={{ width: THUMB, height: THUMB, left: thumbLeft - THUMB / 2, marginTop: -THUMB / 2 }}
        />
      </div>
      <span
        data-play-readout
        className="flex shrink-0 items-center gap-1 whitespace-nowrap text-xs tabular-nums text-[var(--toolbar-icon)]"
      >
        {view?.colour ? (
          // The thread of the colour block at the head (embroidery, `0160` E11).
          <span aria-hidden className="size-2.5 shrink-0 rounded-sm" style={{ background: view.colour }} />
        ) : null}
        {readoutParts(readout).map((part, index) => (
          <span key={index} className={part.muted ? "text-[var(--panel-muted-fg)]" : undefined}>
            {part.text}
          </span>
        ))}
      </span>
      {view?.speed ? (
        // The bar's own segments (the Choice-strip rule for bars, UX unify 7), session state.
        <div role="radiogroup" aria-label="Speed" className="flex shrink-0 gap-px">
          {SPEEDS.map((speed) => (
            <button
              key={speed}
              type="button"
              role="radio"
              aria-checked={view.speed === speed}
              tabIndex={view.speed === speed ? 0 : -1}
              onClick={() => play.setSpeed(speed)}
              onKeyDown={(event) => {
                const at = SPEEDS.indexOf(view.speed ?? 16);
                const next = event.key === "ArrowRight" ? at + 1 : event.key === "ArrowLeft" ? at - 1 : null;
                if (next === null) return;
                event.preventDefault();
                play.setSpeed(SPEEDS[Math.max(0, Math.min(SPEEDS.length - 1, next))]);
              }}
              className="h-7 w-8 rounded text-xs tabular-nums text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-checked:bg-[var(--value-fill)]"
            >
              {speed}×
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

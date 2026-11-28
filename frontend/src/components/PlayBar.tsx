import { Pause, Play } from "lucide-react";
import { useLayoutEffect, useRef, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { usePlayBar } from "@/hooks/usePlayBar";
import { usePublishPlayBarClearance } from "@/hooks/usePlayBarClearance";
import { useSliderPointer } from "@/hooks/useSliderPointer";
import type { WasmSession } from "@/lib/editorSession";
import { barPlace, readoutParts, sliderMinWidth } from "@/lib/outputText";
import { SLIDER_INSET_PX, SLIDER_THUMB_PX, SliderMarks } from "@/components/ui/slider";

const THUMB = SLIDER_THUMB_PX;
const INSET = SLIDER_INSET_PX;
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
      className="pointer-events-auto absolute z-10 flex h-9 items-center gap-x-2.5 rounded-[8px] bg-[var(--toolbar-bg)] px-2"
      style={{ ...place, boxShadow: "var(--panel-elevation-shadow)" }}
    >
      <Tooltip side="top" content={name}>
        <button
          type="button"
          aria-label={name}
          disabled={!enabled}
          onClick={play.toggle}
          className="inline-flex size-7 shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] disabled:opacity-40 disabled:hover:bg-transparent"
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
        <SliderMarks thumbLeft={thumbLeft} />
      </div>
      <span
        data-play-readout
        className="shrink-0 whitespace-nowrap text-xs tabular-nums text-[var(--toolbar-icon)]"
      >
        {readoutParts(readout).map((part, index) => (
          <span key={index} className={part.muted ? "text-[var(--panel-muted-fg)]" : undefined}>
            {part.text}
          </span>
        ))}
      </span>
    </div>
  );
}

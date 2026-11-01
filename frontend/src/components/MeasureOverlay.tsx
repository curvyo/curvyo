import { useEffect, useRef, useState, type CSSProperties } from "react";
import { Unlink } from "lucide-react";
import type { WasmSession } from "@/lib/editorSession";
import { chipPlace, EMPTY_MEASURE_VIEW, readMeasureView, type MeasureChip, type MeasureLabel } from "@/lib/measureView";

/** The 2px `--dim-halo` around dimension text, as the Sketch dimensions draw it. */
const HALO = "0 0 2px var(--dim-halo), 0 0 2px var(--dim-halo), 0 0 2px var(--dim-halo)";
const AT = (x: number, y: number): CSSProperties => ({ left: 0, top: 0, transform: `translate(${x}px, ${y}px) translate(-50%, -50%)` });

/**
 * The texts of the Vector dimensions, the Measure tool's mode label and its readout chip
 * (`specs/0070-measure-and-dimension` criteria 1, 2, 7 and 12), all placed by Rust in the last
 * frame. Pointer input goes to the canvas; the texts are `aria-hidden` (the Layers rows name them).
 */
export function MeasureOverlay({ getSession, measuring }: { getSession: () => WasmSession | null; measuring: boolean }) {
  const [view, setView] = useState(EMPTY_MEASURE_VIEW);
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const next = getSession()?.measure_view_json() ?? "";
      if (next !== previous) {
        previous = next;
        setView(readMeasureView(next));
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  return <div className="pointer-events-none absolute inset-0 z-10 overflow-hidden" data-measure-overlay="">
    {view.dimensions.map((label, index) => <DimensionText key={`${label.id}:${index}`} label={label} />)}
    {measuring && view.readout && <Chip chip={view.readout} kind="readout" />}
    {measuring && view.mode && <Chip chip={view.mode} kind="mode" />}
    {/* Polite live regions: the mode label on each change, the value on a pin or a placement. */}
    <div aria-live="polite" role="status" className="sr-only">{measuring ? view.mode?.lines[0] ?? "" : ""}</div>
    <div aria-live="polite" role="status" className="sr-only" data-measure-speech="">{measuring ? view.speech : ""}</div>
  </div>;
}

function DimensionText({ label }: { label: MeasureLabel }) {
  const color = label.selected ? "var(--editor-accent)" : "var(--dim-guide)";
  return <span aria-hidden data-measure-dimension={label.linked ? "linked" : "unlinked"}
    className="absolute flex items-center gap-0.5 rounded-full px-1 text-[12px] leading-4 whitespace-nowrap tabular-nums"
    style={{ ...AT(label.x, label.y), color, textShadow: HALO,
      border: label.selected ? "1px solid var(--editor-accent)" : "1px solid transparent" }}>
    {!label.linked && <Unlink size={10} strokeWidth={1.5} aria-hidden />}
    {label.text}
  </span>;
}

function Chip({ chip, kind }: { chip: MeasureChip; kind: "readout" | "mode" }) {
  const ref = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState<{ left: number; top: number } | null>(null);
  const key = `${chip.x}|${chip.y}|${chip.lines.join("|")}`;
  useEffect(() => {
    const box = ref.current;
    const parent = box?.parentElement;
    if (!box || !parent) return;
    const size = { width: box.offsetWidth, height: box.offsetHeight };
    const canvas = { width: parent.clientWidth, height: parent.clientHeight };
    // The mode label sits below and right of the pointer, the readout above and right.
    const at = kind === "mode" ? { x: chip.x, y: chip.y + 28 + size.height } : chip;
    setPlace(chipPlace(at, size, canvas));
  }, [key, kind, chip]);
  return <div ref={ref} aria-hidden data-measure-chip={kind}
    className="absolute rounded px-1.5 py-0.5 text-xs whitespace-nowrap tabular-nums"
    style={{ left: place?.left ?? -9999, top: place?.top ?? -9999,
      background: "var(--toolbar-bg)", color: "var(--toolbar-icon)" }}>
    {chip.lines.map((line, index) => <div key={index}>{line}</div>)}
  </div>;
}

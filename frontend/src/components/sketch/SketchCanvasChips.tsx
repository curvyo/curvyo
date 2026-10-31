import { useEffect, useId, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react";

import { EntryField } from "@/components/EntryField";
import type { SketchApi } from "@/hooks/useSketch";
import { placeSketchChip, type Size } from "@/lib/readoutPlacement";
import type { SketchEntry } from "@/lib/sketchView";

/** The tool rail's clearance from the canvas's left edge, as the Pen entry chip keeps it. */
const TOOL_RAIL_CLEAR_PX = 128;
/** The mode label's corner sits 10px from the segment start (UX notes revision 2, section 2). */
const LABEL_OFFSET_PX = 10;
/** The entry box keeps 14px off its object (UX notes revision 2, section 3). */
const ENTRY_OFFSET_PX = 14;
/** The Pen entry chip's field widths and label paddings (`docs/design-system.md`). */
const FIELD_PX = [154, 150];
const LABEL_PX = [48, 44];
const MODIFIER_KEYS = new Set(["Shift", "Control", "Meta", "Alt"]);

type ContainerRef = React.RefObject<HTMLDivElement | null>;

/** The rendered chip's and the canvas's size, measured after each layout. */
function useSizes(chip: React.RefObject<HTMLElement | null>, container: ContainerRef, key: unknown) {
  const [sizes, setSizes] = useState<{ chip: Size; canvas: Size }>({
    chip: { width: 0, height: 0 },
    canvas: { width: Number.POSITIVE_INFINITY, height: Number.POSITIVE_INFINITY },
  });
  useLayoutEffect(() => {
    if (!chip.current || !container.current) return;
    const next = {
      chip: { width: chip.current.offsetWidth, height: chip.current.offsetHeight },
      canvas: { width: container.current.clientWidth, height: container.current.clientHeight },
    };
    setSizes(previous => (JSON.stringify(previous) === JSON.stringify(next) ? previous : next));
  }, [chip, container, key]);
  return sizes;
}

function useCanvas(sketch: SketchApi) {
  const { presentation } = sketch;
  const canvas = useSyncExternalStore(presentation.subscribeAnnotations, presentation.canvas);
  const open = useSyncExternalStore(presentation.subscribeAnnotations, presentation.entryOpen);
  return { canvas, open };
}

/**
 * The mode label (`specs/0066-constraint-sketcher` AC34, AC35): Rust names the mode and the
 * segment start; this places the chip in the quadrant Rust gives and announces each change.
 */
export function SketchModeLabel({ sketch, containerRef }: { sketch: SketchApi; containerRef: ContainerRef }) {
  const { canvas } = useCanvas(sketch);
  const label = sketch.view.workbench === "sketch" ? canvas.mode_label : null;
  const ref = useRef<HTMLDivElement>(null);
  const sizes = useSizes(ref, containerRef, `${label?.text}|${label?.hint}|${label?.error}`);
  return <>
    {/* A polite live region says the new mode on each change (UX notes revision 2, section 2). */}
    <div aria-live="polite" role="status" className="sr-only">{label?.text ?? ""}</div>
    {label && <div ref={ref} aria-hidden data-sketch-mode-label
      className="pointer-events-none absolute z-20 rounded px-1.5 py-0.5 text-xs whitespace-nowrap"
      style={{
        ...placeSketchChip({ x: label.x, y: label.y }, { x: label.away_x, y: label.away_y }, sizes.chip, sizes.canvas, LABEL_OFFSET_PX, TOOL_RAIL_CLEAR_PX),
        background: "var(--toolbar-bg)",
        color: "var(--toolbar-icon)",
      }}>
      {label.text}
      {label.hint && <div className="text-[11px] opacity-80">{label.hint}</div>}
      {/* A refused pick is said here, at the object (UX notes revision 2 review, change 7). */}
      {label.error && <div role="alert" className="text-[11px] text-[var(--field-invalid)]">{label.error}</div>}
    </div>}
  </>;
}

/**
 * The on-canvas entry box (AC33): a Sketch sibling of the Pen entry chip sharing `EntryField`.
 * Tab on the canvas opens it, Tab cycles its fields, Enter commits, Escape closes it.
 */
export function SketchEntryBox({ sketch, containerRef }: { sketch: SketchApi; containerRef: ContainerRef }) {
  const { canvas, open } = useCanvas(sketch);
  const { presentation } = sketch;
  // A placed dimension label opens its entry at once, as the options bar's field did.
  const placed = sketch.view.dimension_position;
  useEffect(() => {
    if (placed) presentation.setEntryOpen(true);
  }, [placed?.x, placed?.y, presentation, placed]);
  // A double-click or Enter on a dimension text opens its entry at the text (AC31, AC33).
  const editing = canvas.entry?.kind === "edit" ? canvas.entry.constraint : undefined;
  useEffect(() => {
    if (editing) presentation.setEntryOpen(true);
  }, [editing, presentation]);
  if (!open || !canvas.entry || sketch.view.workbench !== "sketch") return null;
  return <EntryBox key={`${canvas.entry.kind}:${canvas.entry.group}:${canvas.entry.fields.length}:${canvas.entry.constraint ?? ""}`} sketch={sketch} entry={canvas.entry} containerRef={containerRef} />;
}

function EntryBox({ sketch, entry, containerRef }: { sketch: SketchApi; entry: SketchEntry; containerRef: ContainerRef }) {
  const ref = useRef<HTMLDivElement>(null);
  const inputs = useRef<(HTMLInputElement | null)[]>([]);
  const messageId = useId();
  const [drafts, setDrafts] = useState<string[] | null>(null);
  // The fields the maker typed into; only these become driving dimensions (AC39).
  const [typed, setTyped] = useState<boolean[]>([]);
  const [reference, setReference] = useState(entry.reference_on ?? false);
  const editing = entry.kind === "edit";
  const [frozen, setFrozen] = useState<{ left: number; top: number } | null>(null);
  const [dismissed, setDismissed] = useState(false);
  const sizes = useSizes(ref, containerRef, entry.group);
  // Focus the first field after the press that opened the box has finished its own default
  // focus on the canvas (a placed dimension opens the box during that press).
  useEffect(() => {
    const focus = () => { inputs.current[0]?.focus(); inputs.current[0]?.select(); };
    focus();
    const frame = requestAnimationFrame(focus);
    return () => cancelAnimationFrame(frame);
  }, []);
  const texts = drafts ?? entry.fields.map(field => field.value);
  const live = placeSketchChip({ x: entry.x, y: entry.y }, { x: entry.away_x, y: entry.away_y }, sizes.chip, sizes.canvas,
    ENTRY_OFFSET_PX, TOOL_RAIL_CLEAR_PX, { x: entry.pointer_x, y: entry.pointer_y });
  const at = frozen ?? live;
  const close = () => {
    if (editing) sketch.run(s => s.sketch_close_dimension_edit());
    sketch.presentation.setEntryOpen(false); sketch.clearError(); sketch.focusCanvas();
  };
  const submit = () => {
    const [first = ""] = reference && !editing ? entry.fields.map(field => field.value) : texts;
    const done = editing
      ? sketch.runInBox(s => s.sketch_dimension_text(entry.constraint ?? "", first, reference))
      : entry.kind === "dimension"
        ? sketch.runInBox(s => s.sketch_apply_action(entry.action ?? "", first, reference))
        : sketch.runInBox(s => s.sketch_complete_fields(JSON.stringify(
          texts.map((text, index) => ({ text, typed: typed[index] ?? false })))));
    setDismissed(false);
    // A value that leaves no valid shape keeps the box open on its dimension, with the reason.
    const refused = sketch.presentation.canvas().entry?.error;
    if (done && !refused) { close(); return; }
    const field = refused?.field ?? 0;
    requestAnimationFrame(() => { inputs.current[field]?.focus(); inputs.current[field]?.select(); });
  };
  const onKeyDown = (index: number) => (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (MODIFIER_KEYS.has(event.key)) return;
    if (event.key === "Tab") {
      event.preventDefault();
      const count = entry.fields.length;
      const next = inputs.current[(index + (event.shiftKey ? count - 1 : 1)) % count];
      next?.focus(); next?.select();
    } else if (event.key === "Enter") {
      event.preventDefault();
      submit();
    } else if (event.key === "Escape") {
      event.preventDefault();
      close();
    }
    // Every other key stays in the field: Alt and letters are text input here (AC34).
    event.stopPropagation();
  };
  // Rust names the refused field; typing again hides the message until the next Enter.
  const refusal = dismissed ? null : entry.error ?? null;
  return <div ref={ref} role="group" aria-label={entry.group} data-sketch-entry
    className="absolute z-30 rounded-[8px] p-1.5"
    style={{ left: at.left, top: at.top, background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)", color: "var(--toolbar-icon)" }}
    onPointerDown={event => event.stopPropagation()}
    // The pointer on the box leaves the canvas: the box stays where it is (UX notes, section 3).
    onPointerEnter={() => { if (!frozen) setFrozen(live); }}>
    <div className="flex items-center gap-1">
      {entry.fields.map((field, index) => <EntryField key={field.label} label={field.label} width={FIELD_PX[index] ?? FIELD_PX[0]}
        labelWidth={LABEL_PX[index] ?? LABEL_PX[0]} suffix={field.suffix} ariaLabel={field.name} invalid={refusal?.field === index} messageId={messageId}
        value={reference && !editing ? field.value : texts[index] ?? ""} readOnly={reference && !editing}
        inputRef={node => { inputs.current[index] = node; }}
        onChange={text => {
          if (reference && !editing) return;
          if (!frozen) setFrozen(live);
          setDrafts(texts.map((value, i) => (i === index ? text : value)));
          setTyped(entry.fields.map((_, i) => i === index || (typed[i] ?? false)));
          setDismissed(true);
        }}
        onKeyDown={onKeyDown(index)} onFocus={event => event.target.select()} />)}
    </div>
    {entry.reference && <label className="mt-1 flex items-center gap-1 text-xs">
      <input type="checkbox" tabIndex={-1} className="size-3.5" checked={reference}
        onChange={event => { setReference(event.target.checked); inputs.current[0]?.focus(); }} />Reference
    </label>}
    {refusal && <p id={messageId} role="alert" className="mt-1 text-xs text-[var(--field-invalid)]">{refusal.message}</p>}
  </div>;
}

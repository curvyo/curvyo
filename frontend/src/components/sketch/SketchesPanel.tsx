import { quantityText } from "@/lib/sketchView";
import { memo, useRef, useState } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import type { PageRow } from "@/lib/pages";
import { KindChip } from "@/components/KindChip";
import { selectClonesLabel, usageLine } from "@/lib/sketchPages";
import type { SketchApi, SketchConstraint } from "@/hooks/useSketch";
import { SketchField, sketchButton } from "./SketchField";
const title = (value: string) => value.replace(/(^|[-_])\w/g, part => part.replace(/[-_]/, " ").toUpperCase());
function ConstraintRow({ row, sketch }: { row: SketchConstraint; sketch: SketchApi }) {
  const selected = row.selected;
  const remove = (button: HTMLButtonElement) => {
    const section = button.closest("[data-constraints]");
    const buttons = Array.from(section?.querySelectorAll<HTMLButtonElement>("[data-constraint-row]") ?? []);
    const index = buttons.findIndex(b => b.dataset.constraintRow === row.id);
    if (sketch.run(s => s.sketch_remove_constraint(row.id))) requestAnimationFrame(() => {
      const remaining = section?.querySelectorAll<HTMLButtonElement>("[data-constraint-row]");
      (remaining?.[Math.min(index, remaining.length - 1)] ?? section?.querySelector<HTMLElement>("button"))?.focus();
    });
  };
  return <div data-sketch-dimension={row.id} className={`rounded border-l-2 pl-1 ${row.conflicting ? "border-red-500" : selected ? "border-[var(--editor-accent)]" : "border-transparent"}`}>
    <div className="flex items-center gap-1">
      <button data-constraint-row={row.id} className={`${sketchButton} min-w-0 flex-1 text-left`} aria-pressed={selected}
        onClick={() => sketch.run(s => s.sketch_inspect_constraint(row.id, false))}
        onDoubleClick={() => sketch.inspectConstraint(row.id, true)}
        onKeyDown={e => { if (e.key === "Delete") { e.preventDefault(); e.stopPropagation(); remove(e.currentTarget); } if (e.key === "Enter" && row.expression !== null) { e.preventDefault(); sketch.inspectConstraint(row.id, true); } }}>
        {title(row.kind)}{row.reference ? " · Reference" : ""}{row.conflicting ? " · Conflict" : row.redundant ? " · Redundant" : ""}
      </button>
      {row.expression !== null && <button className={sketchButton} aria-label={`Edit ${row.kind}`} onClick={() => sketch.inspectConstraint(row.id, true)}>Edit</button>}
      <button className={sketchButton} aria-label={`Remove ${row.kind}`} onClick={e => remove(e.currentTarget)}>×</button>
    </div>
    {row.expression !== null && <>
      {!row.reference && sketch.view.edit_constraint === row.id ? <SketchField autoFocus label={`${title(row.kind)} expression`} value={row.expression} commit={text => sketch.run(s => s.sketch_dimension_text(row.id, text, row.reference))} focusCanvas={sketch.focusCanvas} /> : <output className="block px-2 text-xs">{quantityText(row.value)}</output>}
      {selected && <label className="flex gap-2 px-2 text-xs"><input type="checkbox" checked={row.reference} onChange={e => sketch.run(s => s.sketch_dimension_text(row.id, row.expression ?? "", e.target.checked))} />Reference</label>}
    </>}
  </div>;
}
/** The full-width outline look of "Fit to content" (UX notes §3). */
const wideButton = `${sketchButton} flex w-full items-center justify-between border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] pointer-coarse:min-h-9`;

/** The Use group (`specs/0099` criteria 11, 12): how the sketch is used, Select clones and
 * Place clone on. Every command that activates another page leaves focus on the canvas. */
function UseGroup({ sketch, pages }: { sketch: SketchApi; pages: PageRow[] }) {
  const { view } = sketch;
  const usage = view.usage ?? { clones: 0, pages: 0 };
  const [open, setOpen] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const button = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const targets = pages.filter((p) => p.kind !== "sketch" && p.kind !== "unknown");
  const canPlace = view.drawable === true && targets.length > 0;
  const rows = () => Array.from(listRef.current?.querySelectorAll<HTMLElement>("[role=option]") ?? []);
  const place = (id: string) => {
    setOpen(false);
    if (sketch.run((s) => s.place_sketch_clone_on(id))) sketch.focusCanvas();
  };
  return (
    <div role="group" aria-label="Use" className="space-y-2">
      <p className="text-xs text-[var(--panel-muted-fg)]">{usageLine(usage.clones, usage.pages)}</p>
      {usage.clones > 0 && (
        <button
          type="button"
          className={wideButton}
          onClick={() => {
            let notice: string | null = null;
            sketch.run((s) => {
              const result = s.select_sketch_clones();
              if (result.kind === "refused") notice = result.notice;
            });
            setRefusal(notice);
            if (!notice) sketch.focusCanvas();
          }}
        >
          {selectClonesLabel(usage.clones)}
        </button>
      )}
      {refusal && (
        <p role="status" className="text-xs">
          {refusal}
        </p>
      )}
      {canPlace && (
        <>
          <button
            ref={button}
            type="button"
            className={wideButton}
            aria-expanded={open}
            aria-controls="place-clone-on"
            onClick={() => {
              setOpen(!open);
              if (!open) requestAnimationFrame(() => rows()[0]?.focus());
            }}
          >
            Place clone on
            {open ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
          </button>
          {open && (
            <div
              id="place-clone-on"
              ref={listRef}
              role="listbox"
              aria-label="Place clone on"
              className="max-h-[216px] overflow-y-auto rounded p-1"
              style={{ background: "var(--toolbar-bg)" }}
              onKeyDown={(e) => {
                const all = rows();
                const at = all.indexOf(document.activeElement as HTMLElement);
                if (e.key === "ArrowDown" || e.key === "ArrowUp") {
                  e.preventDefault();
                  all[Math.max(0, Math.min(all.length - 1, at + (e.key === "ArrowDown" ? 1 : -1)))]?.focus();
                } else if (e.key === "Home" || e.key === "End") {
                  e.preventDefault();
                  all[e.key === "Home" ? 0 : all.length - 1]?.focus();
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  e.stopPropagation();
                  setOpen(false);
                  button.current?.focus();
                } else if (e.key === "Tab") {
                  setOpen(false);
                } else return;
              }}
            >
              {targets.map((page) => (
                <div
                  key={page.id}
                  role="option"
                  aria-selected={false}
                  tabIndex={-1}
                  title={page.name}
                  className="flex min-h-9 cursor-default items-center gap-2 rounded px-2 text-xs hover:bg-[var(--editor-accent-hover)] focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)] pointer-coarse:min-h-11"
                  onClick={() => place(page.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      place(page.id);
                    }
                  }}
                >
                  <span className="min-w-0 flex-1 truncate">{page.name}</span>
                  <KindChip kind={{ icon: page.kindIcon, word: page.kindWord, off: page.kindOff }} />
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}

function SketchesPanelBody({ sketch, pages }: { sketch: SketchApi; pages: PageRow[] }) {
  const { view } = sketch;
  const rows = useRef<HTMLDivElement>(null);
  const status = typeof view.status === "object" && view.status !== null ? `Under-constrained · ${view.status.UnderConstrained} DOF` : view.status === "FullyConstrained" ? "Fully constrained · 0 DOF" : view.status === "Conflicting" ? "Conflicting" : view.status === "SolverFailure" ? "Solver failure" : view.status ?? "";
  return <div className="space-y-4 text-[var(--toolbar-icon)]">
    {sketch.error && <p id="sketch-error" role="alert" className="text-xs text-red-600">{sketch.error}</p>}
    {view.active && <>
      <section aria-label="Sketch status" className="space-y-1 text-xs"><p>{status}</p>{typeof view.status === "object" && view.status !== null && <button className={sketchButton} onClick={() => sketch.run(s => s.sketch_show_free())}>Show free geometry</button>}{view.stale && <p>Last valid geometry</p>}{view.diagnosis && <p>{view.diagnosis}</p>}</section>
      <UseGroup sketch={sketch} pages={pages} />
      <label className="flex gap-2 text-xs"><input type="checkbox" checked={view.origin_fixed ?? false} onChange={e => sketch.run(s => s.sketch_fix_origin(e.target.checked))} />Fixed origin</label>
      <details aria-label="Sketch geometry"><summary className="cursor-pointer text-xs font-medium focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)]">Geometry ({view.entities.length})</summary><div className="mt-1" aria-label="Sketch entities">
        {view.entities.map((entity,index) => <button key={entity.id} className={`${sketchButton} flex w-full justify-between text-left`} aria-pressed={entity.selected}
          onClick={e => sketch.run(s => s.sketch_select_entity(entity.id, e.shiftKey))}>
          <span>{entity.origin ? "Origin" : `${entity.kind} ${index + 1}`}</span><span>{entity.construction ? "Construction" : entity.fully_constrained === true ? "Fully constrained" : ""}</span>
        </button>)}
      </div><div className="flex gap-1">
        <button className={sketchButton} aria-pressed={view.entities.some(e => e.selected) && view.entities.filter(e => e.selected).every(e => e.construction)} onClick={() => sketch.run(s => s.sketch_selection_construction(!view.entities.filter(e => e.selected).every(e => e.construction)))}>Construction</button>
        <button className={sketchButton} onClick={() => sketch.run(s => s.sketch_delete_selection())}>Delete</button>
        <button className={sketchButton} onClick={() => sketch.run(s => s.sketch_inspect_selection())}>Inspect</button>
      </div>
      {/* An explicit start from a selected open endpoint (UX notes revision 2, section 2). */}
      {view.can_resume && <div className="flex gap-1">{[["polyline","Resume polyline"],["arc","Resume arc"]].map(([tool,label]) =>
        <button key={tool} className={sketchButton} onClick={() => { if (sketch.run(s => s.set_sketch_tool(tool))) sketch.focusCanvas(); }}>{label}</button>)}</div>}
      </details>
      <div ref={rows} data-constraints className="space-y-1" aria-label="Sketch relationships">{view.constraints.map(row => <ConstraintRow key={row.id} row={row} sketch={sketch} />)}</div>
      {/* The page becomes a drawing page (`0099` criterion 15); hidden with nothing to draw. */}
      {view.drawable && <button className={wideButton} onClick={() => { if (sketch.run(s => s.sketch_convert_to_path())) sketch.focusCanvas(); }}>Convert to paths</button>}
    </>}
  </div>;
}

// Pointer motion changes annotation positions every frame. Properties only needs a
// repaint when its displayed domain state changes; draft state still updates locally.
function panelPresentation(sketch: SketchApi) {
  const { annotations: _annotations, ...view } = sketch.view;
  return JSON.stringify({ ...view, error: sketch.error,
    constraints: view.constraints.map(row => ({ ...row, value: quantityText(row.value) })),
  });
}
/** What of the page list the Use group shows: the drawing and laser pages it can place on. */
const pagesPresentation = (pages: PageRow[]) =>
  JSON.stringify(pages.map((p) => [p.id, p.name, p.kind, p.kindWord, p.kindIcon, p.kindOff]));
export const SketchesPanel = memo(SketchesPanelBody, (before, after) =>
  panelPresentation(before.sketch) === panelPresentation(after.sketch) &&
  pagesPresentation(before.pages) === pagesPresentation(after.pages));

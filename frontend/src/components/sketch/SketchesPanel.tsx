import { quantityText } from "@/lib/sketchView";
import { memo, useRef, useState } from "react";
import type { SketchApi, SketchConstraint } from "@/hooks/useSketch";
import { SketchField, sketchButton, sketchInput } from "./SketchField";
import { SketchParameters } from "./SketchParameters";
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
function SketchesPanelBody({ sketch }: { sketch: SketchApi }) {
  const { view } = sketch;
  const [dimension, setDimension] = useState<string | null>(null);
  const [text, setText] = useState(""); const [reference, setReference] = useState(false);
  const rows = useRef<HTMLDivElement>(null);
  const status = typeof view.status === "object" && view.status !== null ? `Under-constrained · ${view.status.UnderConstrained} DOF` : view.status === "FullyConstrained" ? "Fully constrained · 0 DOF" : view.status === "Conflicting" ? "Conflicting" : view.status === "SolverFailure" ? "Solver failure" : view.status ?? "";
  return <div className="space-y-4 text-[var(--toolbar-icon)]">
    <div className="flex gap-1"><select aria-label="Active sketch" className={sketchInput} value={view.active ?? ""} onChange={e => sketch.run(s => s.enter_sketch(e.target.value))}>
      <option value="" disabled>Select sketch</option>{view.sketches.map(s => <option key={s.id} value={s.id}>{s.name}</option>)}
    </select><button className={sketchButton} onClick={() => { if (sketch.run(s => s.new_sketch())) sketch.focusCanvas(); }}>New</button></div>
    {sketch.error && <p id="sketch-error" role="alert" className="text-xs text-red-600">{sketch.error}</p>}
    {view.active && <>
      <section aria-label="Sketch status" className="space-y-1 text-xs"><p>{status}</p>{typeof view.status === "object" && view.status !== null && <button className={sketchButton} onClick={() => sketch.run(s => s.sketch_show_free())}>Show free geometry</button>}{view.stale && <p>Last valid geometry</p>}{view.diagnosis && <p>{view.diagnosis}</p>}</section>
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
      </div></details>
      <section aria-label="Constraint actions"><h3 className="text-xs font-medium">Constraints and dimensions</h3><div className="grid grid-cols-2 gap-1">
        {view.actions.map(action => <button key={action.name} className={sketchButton} aria-disabled={!action.enabled} title={action.reason ?? title(action.name)} onClick={() => {
          if (!action.enabled) return;
          if (["length","distance","radius","diameter","angle"].includes(action.name)) { setDimension(action.name); setText(""); }
          else sketch.run(s => s.sketch_apply_action(action.name,"",false));
        }}>{title(action.name)}</button>)}
      </div>{dimension && <form className="mt-2 space-y-1" onSubmit={e => { e.preventDefault(); if (sketch.run(s => s.sketch_apply_action(dimension,text,reference))) { setDimension(null); sketch.focusCanvas(); } }} onKeyDown={e => { if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); setDimension(null); sketch.focusCanvas(); } }}>
        <label className="block text-xs">{title(dimension)}<input autoFocus className={sketchInput} aria-label={`New ${dimension}`} aria-describedby={sketch.error ? "sketch-error" : undefined} aria-invalid={!!sketch.error} value={text} onChange={e => setText(e.target.value)} /></label>
        <label className="flex gap-2 text-xs"><input type="checkbox" checked={reference} onChange={e => setReference(e.target.checked)} />Reference</label><button className={sketchButton} type="submit">Apply</button>
      </form>}</section>
      <div ref={rows} data-constraints className="space-y-1" aria-label="Sketch relationships">{view.constraints.map(row => <ConstraintRow key={row.id} row={row} sketch={sketch} />)}</div>
      <SketchParameters sketch={sketch} />
      <button className={sketchButton} onClick={() => { if (sketch.run(s => s.sketch_convert_to_path())) sketch.focusCanvas(); }}>Convert to paths</button>
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
export const SketchesPanel = memo(SketchesPanelBody, (before, after) =>
  panelPresentation(before.sketch) === panelPresentation(after.sketch));

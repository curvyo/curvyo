import { useRef, useState } from "react";
import type { SketchApi, SketchParameter } from "@/hooks/useSketch";
import { SketchField, sketchButton, sketchInput } from "./SketchField";
import { quantityText } from "@/lib/sketchView";

function ParameterRow({ parameter: p, sketch }: { parameter: SketchParameter; sketch: SketchApi }) {
  const save = (name: string, expression: string, dimension: string) => sketch.run(s => s.sketch_parameter_text(p.id, name, expression, dimension));
  return <div data-parameter className="grid grid-cols-[1fr_70px_28px] gap-1 border-b border-[var(--field-border)] py-2">
    <SketchField label={`Parameter ${p.name} name`} value={p.name} commit={name => save(name, p.expression ?? "", p.dimension)} focusCanvas={sketch.focusCanvas} />
    <select className={sketchInput} aria-label={`${p.name} unit`} value={p.dimension} onChange={e => save(p.name, p.expression ?? "", e.target.value)}>
      <option value="Length">mm</option><option value="Angle">rad</option><option value="Scalar">Scalar</option>
    </select>
    <button className={sketchButton} aria-label={`Remove parameter ${p.name}`} onClick={e => {
      const section = e.currentTarget.closest("section");
      const next = e.currentTarget.closest("[data-parameter]")?.nextElementSibling?.querySelector<HTMLInputElement>("input");
      if (sketch.run(s => s.sketch_remove_parameter(p.id))) requestAnimationFrame(() => (next ?? section?.querySelector<HTMLButtonElement>("[data-add-parameter]"))?.focus());
    }}>×</button>
    <div className="col-span-3"><SketchField label={`${p.name} expression`} value={p.expression ?? ""} commit={text => save(p.name, text, p.dimension)} focusCanvas={sketch.focusCanvas} /></div>
    <div className="col-span-3 flex items-center justify-between"><output className="text-xs text-[var(--toolbar-icon)]">{quantityText(p.value)}</output><button className={sketchButton} onClick={() => sketch.run(s => s.sketch_inspect_parameter(p.id))}>Inspect</button></div>
  </div>;
}
export function SketchParameters({ sketch }: { sketch: SketchApi }) {
  const addButton = useRef<HTMLButtonElement>(null);
  const cancel = () => { setName(""); setExpression(""); setAdding(false); requestAnimationFrame(() => addButton.current?.focus()); };
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState(""); const [expression, setExpression] = useState(""); const [unit, setUnit] = useState("Length");
  const add = () => { if (sketch.run(s => s.sketch_add_parameter(name, expression, unit))) { setAdding(false); setName(""); setExpression(""); sketch.focusCanvas(); } };
  return <section aria-label="Parameters" className="space-y-1"><h3 className="text-xs font-medium">Parameters</h3>
    {sketch.view.parameters.map(p => <ParameterRow key={p.id} parameter={p} sketch={sketch} />)}
    {adding ? <form className="grid grid-cols-[1fr_70px] gap-1" onSubmit={e => { e.preventDefault(); add(); }} onKeyDown={e => { if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); cancel(); } }}>
      <input autoFocus className={sketchInput} aria-label="New parameter name" aria-describedby={sketch.error ? "sketch-error" : undefined} aria-invalid={!!sketch.error} value={name} onChange={e => setName(e.target.value)} />
      <select className={sketchInput} aria-label="New parameter unit" value={unit} onChange={e => setUnit(e.target.value)}><option value="Length">mm</option><option value="Angle">rad</option><option value="Scalar">Scalar</option></select>
      <input className={`${sketchInput} col-span-2`} aria-label="New parameter expression" aria-describedby={sketch.error ? "sketch-error" : undefined} aria-invalid={!!sketch.error} value={expression} onChange={e => setExpression(e.target.value)} />
      <button className={sketchButton} type="submit">Add</button><button className={sketchButton} type="button" onClick={cancel}>Cancel</button>
    </form> : <button data-add-parameter ref={addButton} className={sketchButton} onClick={() => setAdding(true)}>Add parameter</button>}
  </section>;
}

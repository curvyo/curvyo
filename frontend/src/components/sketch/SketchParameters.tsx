import { Eye, Plus, Trash2, X } from "lucide-react";
import { useRef, useState } from "react";
import type { SketchApi, SketchParameter } from "@/hooks/useSketch";
import { SketchField, sketchButton } from "./SketchField";
import { FIELD_WIDE as sketchInput, GLYPH } from "@/components/ui/looks";
import { ToggleGroup } from "@/components/ui/toggle-group";
import { quantityText } from "@/lib/sketchView";
import { ExposeControls } from "./ExposeControls";
import { Button } from "@/components/ui/button";

const units = [
  {value:"Length",label:"Millimetres",tooltip:"Length in millimetres",icon:"mm"},
  {value:"Angle",label:"Radians",tooltip:"Angle in radians",icon:"rad"},
  {value:"Scalar",label:"Scalar",tooltip:"Unitless value",icon:"Scalar",width:48},
] as const;

function ParameterRow({ parameter: p, sketch }: { parameter: SketchParameter; sketch: SketchApi }) {
  // Inline delete confirm (customer rule): while clones hold an own value of this exposed parameter
  // the first press arms the trash and turns the row soft red; the second removes (`0099` UX 12.1).
  const holders = p.expose?.on ? p.expose.holders : 0;
  const [armed, setArmed] = useState(false);
  const save = (name: string, expression: string, dimension: string) => sketch.run(s => s.sketch_parameter_text(p.id, name, expression, dimension));
  return <div data-parameter className={`grid grid-cols-[minmax(0,1fr)_auto_28px] gap-1 border-b border-[var(--field-border)] py-2 ${armed ? "bg-[var(--destructive-soft)]" : ""}`}>
    <SketchField label={`Parameter ${p.name} name`} value={p.name} commit={name => save(name, p.expression ?? "", p.dimension)} focusCanvas={sketch.focusCanvas} />
    <ToggleGroup label={`${p.name} unit`} options={units} value={p.dimension} onChange={unit => save(p.name, p.expression ?? "", unit)} itemWidth={32} onReturnFocus={sketch.focusCanvas} />
    <button className={armed ? `${sketchButton} bg-[var(--destructive)] text-white` : sketchButton}
      aria-label={armed ? `Press again to remove ${p.name}. ${holders} ${holders === 1 ? "clone loses its" : "clones lose their"} own value.` : `Remove parameter ${p.name}`}
      onBlur={() => setArmed(false)}
      onKeyDown={e => { if (e.key === "Escape" && armed) { e.preventDefault(); e.stopPropagation(); setArmed(false); } }}
      onClick={e => {
        if (holders > 0 && !armed) { setArmed(true); return; }
        const section = e.currentTarget.closest("section");
        const next = e.currentTarget.closest("[data-parameter]")?.nextElementSibling?.querySelector<HTMLInputElement>("input");
        if (sketch.run(s => s.sketch_remove_parameter(p.id))) requestAnimationFrame(() => (next ?? section?.querySelector<HTMLButtonElement>("[data-add-parameter]"))?.focus());
      }}><Trash2 {...GLYPH} aria-hidden /></button>
    <div className="col-span-3"><SketchField label={`${p.name} expression`} value={p.expression ?? ""} commit={text => save(p.name, text, p.dimension)} focusCanvas={sketch.focusCanvas} /></div>
    <div className="col-span-3 flex items-center justify-between"><output className="text-xs text-[var(--toolbar-icon)]">{quantityText(p.value)}</output><Button icon={Eye} onClick={() => sketch.run(s => s.sketch_inspect_parameter(p.id))}>Inspect</Button></div>
    {p.expose && <ExposeControls expose={p.expose} subject={p.name} sketch={sketch} />}
  </div>;
}
export function SketchParameters({ sketch }: { sketch: SketchApi }) {
  const addButton = useRef<HTMLButtonElement>(null);
  const cancel = () => { setName(""); setExpression(""); setAdding(false); requestAnimationFrame(() => addButton.current?.focus()); };
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState(""); const [expression, setExpression] = useState(""); const [unit, setUnit] = useState("Length");
  const add = () => { if (sketch.run(s => s.sketch_add_parameter(name, expression, unit))) { setAdding(false); setName(""); setExpression(""); sketch.focusCanvas(); } };
  return <section aria-label="Parameters" className="space-y-1">
    {sketch.error && <p id="sketch-error" role="alert" className="text-xs text-red-600">{sketch.error}</p>}
    {sketch.view.parameters.map(p => <ParameterRow key={p.id} parameter={p} sketch={sketch} />)}
    {adding ? <form className="grid grid-cols-[minmax(0,1fr)_auto] gap-1" onSubmit={e => { e.preventDefault(); add(); }} onKeyDown={e => { if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); cancel(); } }}>
      <input autoFocus className={sketchInput} aria-label="New parameter name" aria-describedby={sketch.error ? "sketch-error" : undefined} aria-invalid={!!sketch.error} value={name} onChange={e => setName(e.target.value)} />
      <ToggleGroup label="New parameter unit" options={units} value={unit} onChange={setUnit} itemWidth={32} onReturnFocus={() => {}} />
      <input className={`${sketchInput} col-span-2`} aria-label="New parameter expression" aria-describedby={sketch.error ? "sketch-error" : undefined} aria-invalid={!!sketch.error} value={expression} onChange={e => setExpression(e.target.value)} />
      <Button icon={X} onClick={cancel}>Cancel</Button><Button variant="primary" icon={Plus} type="submit">Add</Button>
    </form> : <Button icon={Plus} data-add-parameter ref={addButton} onClick={() => setAdding(true)}>Add parameter</Button>}
  </section>;
}

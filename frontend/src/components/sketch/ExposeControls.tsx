import { PanelSwitch } from "@/components/ui/switch";
import type { SketchApi } from "@/hooks/useSketch";
import type { SketchExpose } from "@/lib/sketchView";
import { stopTitle } from "@/lib/sketchInstanceView";
import { SketchField } from "./SketchField";

/** A limit as the field shows it: a number, or "None" (unlimited) as a muted state word. */
const limitText = (value: number | null) => (value === null ? "" : String(Number(value.toFixed(6))));

/**
 * The Expose switch of a parameter or a driving dimension, with its limits and (for a dimension)
 * its name (`specs/0099-sketch-pages` criterion 23, UX notes 12.1): one step "Expose parameter",
 * "Stop exposing parameter" and "Change exposed parameter". Off by default; the fields are in the
 * tree only while the switch is on. Rust applies every rule (a name in use, the limits).
 */
export function ExposeControls({ expose, subject, sketch }: { expose: SketchExpose; subject: string; sketch: SketchApi }) {
  const command = (run: (session: Parameters<Parameters<SketchApi["run"]>[0]>[0]) => string) =>
    sketch.run(session => { const refusal = run(session); if (refusal !== "") throw refusal; });
  const unit = expose.angle ? "°" : "mm";
  const limits = (min: string, max: string, name: string) =>
    command(s => s.sketch_exposed_limits(expose.key, min, max, name));
  const nameValue = expose.name ?? expose.suggest ?? "";
  return <div data-expose={expose.key} className="col-span-3 flex flex-col gap-1">
    <div className="flex h-7 items-center" title={expose.on ? stopTitle(expose.holders) : "Clones may set their own value"}>
      <PanelSwitch label="Expose" name={`Expose ${subject}`} checked={expose.on}
        onCheckedChange={on => command(s => on ? s.sketch_expose(expose.key, expose.suggest ?? "") : s.sketch_unexpose(expose.key))}
        onReturnFocus={sketch.focusCanvas} />
    </div>
    {expose.on && <>
      {expose.suggest !== undefined && <SketchField label={`${subject} exposed name`} value={nameValue}
        commit={text => command(s => s.sketch_exposed_limits(expose.key, limitText(expose.min), limitText(expose.max), text))}
        focusCanvas={sketch.focusCanvas} />}
      <div className="flex gap-2">
        <div className="w-[118px]"><SketchField label={`${subject} minimum, ${unit}`} value={limitText(expose.min)}
          commit={text => limits(text, limitText(expose.max), expose.suggest !== undefined ? nameValue : "")} focusCanvas={sketch.focusCanvas} />
          {expose.min === null && <span aria-hidden className="block px-1 text-[10px] text-[var(--panel-muted-fg)]">Min: None</span>}</div>
        <div className="w-[118px]"><SketchField label={`${subject} maximum, ${unit}`} value={limitText(expose.max)}
          commit={text => limits(limitText(expose.min), text, expose.suggest !== undefined ? nameValue : "")} focusCanvas={sketch.focusCanvas} />
          {expose.max === null && <span aria-hidden className="block px-1 text-[10px] text-[var(--panel-muted-fg)]">Max: None</span>}</div>
      </div>
    </>}
  </div>;
}

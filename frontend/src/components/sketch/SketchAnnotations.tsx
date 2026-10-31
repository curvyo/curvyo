import { useSyncExternalStore } from "react";
import type { SketchApi } from "@/hooks/useSketch";
/** Labels are positioned by Rust and never forward presses into creation gestures. */
export function SketchAnnotations({ sketch }: { sketch: SketchApi }) {
  const annotations = useSyncExternalStore(sketch.presentation.subscribeAnnotations, sketch.presentation.annotations);
  if(sketch.view.workbench!=="sketch") return null;
  return <div className="pointer-events-none absolute inset-0 z-10 overflow-hidden" aria-label="Sketch annotations">
    {annotations.map(label=><button key={label.id} type="button" aria-label={`${label.text}${label.dimension?", edit dimension":""}`} aria-pressed={label.selected}
      className="pointer-events-auto absolute -translate-x-1/2 -translate-y-1/2 rounded bg-[var(--toolbar-bg)] px-1 text-[11px] text-[var(--toolbar-icon)] shadow-sm focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)] aria-pressed:text-[var(--editor-accent)]"
      tabIndex={label.preview ? -1 : undefined}
      style={{pointerEvents:label.preview ? "none" : undefined,left:0,top:0,transform:`translate(${label.x}px, ${label.y}px)`, border:label.conflicting?"1px solid var(--destructive)":undefined}}
      onPointerDown={e=>e.stopPropagation()} onPointerUp={e=>e.stopPropagation()}
      onClick={e=>{e.stopPropagation();sketch.run(s=>s.sketch_inspect_constraint(label.id,false));if(e.detail>0)sketch.focusCanvas();}}
      onDoubleClick={e=>{e.stopPropagation();sketch.inspectConstraint(label.id,true);}}
      onKeyDown={e=>{if(e.key==="Enter"){e.preventDefault();e.stopPropagation();sketch.inspectConstraint(label.id,true);}}}>{label.text}</button>)}
  </div>;
}

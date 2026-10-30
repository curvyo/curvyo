import { memo, useState } from "react";
import { Circle, Dot, Ellipse, MousePointer2, PenLine, RectangleHorizontal, Spline, Square, CornerDownRight } from "lucide-react";
import type { SketchApi } from "@/hooks/useSketch";
import { RAIL_CARD_CLASS, RAIL_CARD_STYLE } from "@/components/railCard";
import { Tooltip } from "@/components/ui/tooltip";
import { sketchButton, sketchInput } from "./SketchField";
const tools = [ ["select","Select",MousePointer2], ["point","Point",Dot], ["polyline","Polyline",PenLine], ["arc","Arc",Spline], ["circle","Circle",Circle], ["ellipse","Ellipse",Ellipse], ["rectangle","Rectangle",Square], ["slot","Slot",RectangleHorizontal], ["fillet","Fillet",CornerDownRight] ] as const;
const family = (tool: string) => tool === "constraints" ? "select" : tool.split("-")[0];
function SketchToolRailBody({ sketch, paused }: { sketch: SketchApi; paused: boolean }) {
  return <nav aria-label="Sketch tools" inert={paused} className="pointer-events-none absolute top-3 bottom-1 left-3 z-30 flex w-12 flex-col gap-2">
    <div className={`${RAIL_CARD_CLASS} overflow-y-auto`} style={RAIL_CARD_STYLE} onKeyDown={e => {
      if (!["ArrowUp","ArrowDown","Home","End"].includes(e.key)) return;
      const buttons=Array.from(e.currentTarget.querySelectorAll<HTMLButtonElement>("button"));
      const index=buttons.indexOf(document.activeElement as HTMLButtonElement);
      e.preventDefault(); buttons[e.key === "Home" ? 0 : e.key === "End" ? buttons.length-1 : (index+(e.key === "ArrowDown" ? 1 : -1)+buttons.length)%buttons.length]?.focus();
    }}>{tools.map(([tool,label,Icon]) => <Tooltip key={tool} content={label} side="right"><button type="button" aria-label={label} aria-pressed={family(sketch.view.tool) === tool} className="pointer-events-auto flex size-10 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)]"
      onClick={e => { sketch.run(s => s.set_sketch_tool(tool)); if(e.detail>0) sketch.focusCanvas(); }}><Icon size={20} strokeWidth={1.5} /></button></Tooltip>)}</div>
  </nav>;
}
function SketchToolbarBody({ sketch }: { sketch: SketchApi }) {
  const [first,setFirst]=useState(""); const [second,setSecond]=useState("");
  const tool=family(sketch.view.tool);
  const variants = tool === "select" ? [["select","Entities"],["constraints","Constraints"]] : tool === "circle" ? [["circle","Centre and radius"],["circle-three-points","Three points"]] : tool === "ellipse" ? [["ellipse","Centre and axes"],["ellipse-three-points","Axis endpoints and point"]] : [];
  const numeric = ["point","circle","ellipse","rectangle","slot","fillet"].includes(sketch.view.tool);
  const labels = tool === "point" ? ["X","Y"] : tool === "rectangle" ? ["Width","Height"] : tool === "ellipse" ? ["First axis","Second axis"] : tool === "slot" ? ["Width"] : ["Radius"];
  return <form aria-label="Sketch tool options" className="pointer-events-auto flex max-w-full flex-wrap items-center gap-2 rounded-lg bg-[var(--toolbar-bg)] p-2 text-[var(--toolbar-icon)] shadow-md ring-1 ring-[var(--field-border)]" onSubmit={e => { e.preventDefault(); if(sketch.run(s=>s.sketch_complete_numeric(first,second))) {setFirst("");setSecond("");sketch.focusCanvas();} }}>
    {variants.length>0 ? <select className={`${sketchInput} !w-auto`} aria-label="Sketch tool variant" value={sketch.view.tool} onChange={e=>sketch.run(s=>s.set_sketch_tool(e.target.value))}>{variants.map(([id,label])=><option key={id} value={id}>{label}</option>)}</select> : <span className="px-1 text-xs">{tools.find(([id])=>id===tool)?.[1]}</span>}
    {numeric && <>{labels.map((label,index)=><label key={label} className="flex items-center gap-1 text-xs">{label}<input className={`${sketchInput} !w-24`} aria-label={`Sketch ${label}`} aria-describedby={sketch.error ? "sketch-error" : undefined} aria-invalid={!!sketch.error} value={index===0?first:second} onChange={e=>index===0?setFirst(e.target.value):setSecond(e.target.value)} onKeyDown={e=>{if(e.key==="Escape"){e.preventDefault();e.stopPropagation();setFirst("");setSecond("");sketch.run(s=>s.key_down("Escape",false,false,false,false,false,false,0));sketch.focusCanvas();}}}/><span>mm</span></label>)}
      {tool==="fillet" && <button type="button" className={sketchButton} onClick={()=>sketch.run(s=>s.sketch_preview_fillet_text(first))}>Preview</button>}
      <button className={sketchButton} type="submit">Apply</button></>}
  </form>;
}

// Pointer previews change annotations, not tool controls. Commands still read the live session.
export const SketchToolRail = memo(SketchToolRailBody, (a, b) =>
  a.paused === b.paused && a.sketch.view.active === b.sketch.view.active && a.sketch.view.tool === b.sketch.view.tool);
export const SketchToolbar = memo(SketchToolbarBody, (a, b) =>
  a.sketch.view.active === b.sketch.view.active && a.sketch.view.tool === b.sketch.view.tool && a.sketch.error === b.sketch.error);

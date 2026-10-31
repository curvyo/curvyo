import { memo, useState, useEffect, useRef } from "react";
import { Circle, Dot, Ellipse, MousePointer2, PenLine, RectangleHorizontal, Spline, Square, CornerDownRight, Link2, Ruler } from "lucide-react";
import type { SketchApi } from "@/hooks/useSketch";
import { RAIL_CARD_CLASS, RAIL_CARD_STYLE } from "@/components/railCard";
import { Tooltip } from "@/components/ui/tooltip";
import { TracePresetMenu } from "@/components/TracePresetMenu";
import { sketchButton, sketchInput } from "./SketchField";
const tools = [["select","Select",MousePointer2],["point","Point",Dot],["polyline","Polyline",PenLine],["arc","Arc",Spline],["circle","Circle",Circle],["ellipse","Ellipse",Ellipse],["rectangle","Rectangle",Square],["slot","Slot",RectangleHorizontal],["fillet","Fillet",CornerDownRight],["constraints","Constraints",Link2],["dimension","Dimension",Ruler]] as const;
const family = (tool: string) => tool.split("-")[0];
const title = (value: string) => value.replace(/(^|-)\w/g, part => part.replace("-", " ").toUpperCase());
function SketchToolRailBody({ sketch, paused }: { sketch: SketchApi; paused: boolean }) {
  return <nav aria-label="Sketch tools" inert={paused} className="pointer-events-none absolute top-3 bottom-1 left-3 z-30 flex w-12 flex-col gap-2">
    <div className={`${RAIL_CARD_CLASS} overflow-y-auto`} style={RAIL_CARD_STYLE} onKeyDown={e => {
      if (!["ArrowUp","ArrowDown","Home","End"].includes(e.key)) return;
      const buttons=Array.from(e.currentTarget.querySelectorAll<HTMLButtonElement>("button"));
      const index=buttons.indexOf(document.activeElement as HTMLButtonElement);
      e.preventDefault(); buttons[e.key === "Home" ? 0 : e.key === "End" ? buttons.length-1 : (index+(e.key === "ArrowDown" ? 1 : -1)+buttons.length)%buttons.length]?.focus();
    }}>{tools.map(([tool,label,Icon]) => <Tooltip key={tool} content={label} side="right"><button type="button" aria-label={label}
      aria-pressed={tool === "dimension" ? sketch.view.constraint_tool === "dimension" : family(sketch.view.tool) === tool && sketch.view.constraint_tool !== "dimension"}
      className={`pointer-events-auto flex size-10 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] ${tool === "constraints" ? "mt-2 border-t border-[var(--field-border)]" : ""}`}
      onClick={e => { sketch.run(s => tool === "constraints" || tool === "dimension" ? s.set_sketch_constraint_tool(tool === "dimension" ? "dimension" : "coincident") : s.set_sketch_tool(tool)); if(e.detail>0) sketch.focusCanvas(); }}><Icon size={20} strokeWidth={1.5} /></button></Tooltip>)}</div>
  </nav>;
}
const selectedEntities = (sketch: SketchApi) => sketch.view.entities.filter(entity => entity.selected).map(entity => entity.id).join("\0");
function SketchToolbarBody({ sketch }: { sketch: SketchApi }) {
  const selectionKey = selectedEntities(sketch);
  const numericInitialized = useRef(false);
  const [first,setFirst]=useState(""); const [second,setSecond]=useState("");
  const [reference,setReference]=useState(false);
  useEffect(() => {setFirst("");setSecond("");numericInitialized.current=false;}, [selectionKey, sketch.view.draft_epoch, sketch.view.mode, sketch.view.constraint_tool, sketch.view.dimension_action]);
  useEffect(() => {
    if (!sketch.view.dimension_position) return;
    const field = document.querySelector<HTMLInputElement>("[data-sketch-numeric]");
    field?.focus(); field?.select();
  }, [sketch.view.dimension_position?.x, sketch.view.dimension_position?.y]);
  const tool=family(sketch.view.tool);
  const relation=tool === "constraints";
  const inferred=sketch.view.dimension_action;
  const action=sketch.view.constraint_tool === "dimension" ? inferred : sketch.view.constraint_tool;
  const eligibility=sketch.view.actions.find(item=>item.name===action);
  const dimensional=!!action && ["length","distance","radius","diameter","angle"].includes(action);
  const variants = tool === "arc" ? [["arc","Three points"],["arc-center","Centre, start, end"]] : tool === "circle" ? [["circle","Centre and radius"],["circle-three-points","Three points"]] : tool === "ellipse" ? [["ellipse","Centre and axes"],["ellipse-three-points","Axis endpoints and point"]] : [];
  const numeric = relation ? dimensional : ["point","polyline","circle","ellipse","rectangle","slot","fillet","arc-center"].includes(sketch.view.tool);
  const chainArc = tool === "polyline" && (sketch.view.mode ?? "").includes("arc");
  const labels = relation ? [title(action ?? "Value")] : tool === "point" ? ["X","Y"] : tool === "polyline" ? (chainArc ? ["Radius"] : sketch.view.mode && sketch.view.mode !== "free-line" ? ["Length"] : ["Length","Angle"]) : tool === "rectangle" ? ["Width","Height"] : tool === "ellipse" ? ["First axis","Second axis"] : tool === "slot" ? ["Width"] : ["Radius"];
  const submit=()=>{
    if (relation && (!action || !eligibility?.enabled)) return;
    if(sketch.run(s=>relation ? s.sketch_apply_action(action!,first,dimensional && reference) : s.sketch_complete_numeric(first,second))) {
      setFirst("");setSecond("");numericInitialized.current=false;sketch.focusCanvas();
    }
  };
  return <form aria-label="Sketch tool options" className="pointer-events-auto flex max-w-full flex-wrap items-center gap-2 rounded-lg bg-[var(--toolbar-bg)] p-2 text-[var(--toolbar-icon)] shadow-md ring-1 ring-[var(--field-border)]"
    onSubmit={e=>{e.preventDefault();submit();}}
    onContextMenu={e=>{e.preventDefault();sketch.run(s=>s.sketch_secondary_press());setFirst("");setSecond("");numericInitialized.current=false;sketch.focusCanvas();}}>
    {variants.length>0 ? <TracePresetMenu name="Sketch tool variant" widthClass="w-[208px]" current={sketch.view.tool} edited={false} label={variants.find(([id])=>id===sketch.view.tool)?.[1] ?? tool}
      presets={variants.map(([key,label])=>({key,label,detail:""}))} onPick={item=>sketch.run(s=>s.set_sketch_tool(item.key))}/> : <span className="px-1 text-xs">{sketch.view.constraint_tool === "dimension" ? "Dimension" : tools.find(([id])=>id===tool)?.[1]}</span>}
    {tool === "select" && <>{[["polyline","Resume polyline"],["arc","Resume arc"]].map(([tool,label])=><button key={tool} className={sketchButton} type="button" aria-disabled={!sketch.view.can_resume} title={sketch.view.can_resume ? undefined : "Select one open line or arc endpoint"} onClick={()=>{if(sketch.view.can_resume){sketch.run(s=>s.set_sketch_tool(tool));sketch.focusCanvas();}}}>{label}</button>)}</>}
    {tool === "arc" && <span className="text-xs">{sketch.view.pick_role}</span>}
    {tool === "polyline" && <TracePresetMenu name="Polyline mode" widthClass="w-[224px]" current={sketch.view.mode ?? "free-line"} edited={false} label={title(sketch.view.mode ?? "free-line")}
      presets={(sketch.view.modes ?? []).map(mode=>({key:mode.id,label:title(mode.id),detail:mode.reason ?? "",disabled:!mode.enabled}))} onPick={item=>sketch.run(s=>s.set_sketch_mode(item.key))}/>}
    {relation && sketch.view.constraint_tool !== "dimension" && <TracePresetMenu name="Constraint tool" widthClass="w-[176px]" current={action ?? "coincident"} edited={false} label={title(action ?? "coincident")}
      presets={sketch.view.actions.filter(item=>!["length","distance","radius","diameter","angle"].includes(item.name)).map(item=>({key:item.name,label:title(item.name),detail:""}))} onPick={item=>sketch.run(s=>s.set_sketch_constraint_tool(item.key))}/>}
    {relation && (inferred === "diameter" || inferred === "radius") && <TracePresetMenu name="Circle dimension" current={inferred} edited={false} label={title(inferred)} presets={[{key:"diameter",label:"Diameter",detail:""},{key:"radius",label:"Radius",detail:""}]} onPick={item=>sketch.run(s=>s.set_sketch_circle_dimension(item.key === "radius"))}/>}
    {numeric && labels.map((label,index)=><label key={label} className="flex items-center gap-1 text-xs">{label}<input data-sketch-numeric className={`${sketchInput} !w-24`} aria-label={`Sketch ${label}`} aria-describedby={sketch.error ? "sketch-toolbar-error" : undefined} aria-invalid={!!sketch.error}
      onFocus={e=>{if(!numericInitialized.current){const values=sketch.numericValues(action ?? "");setFirst(values[0]);setSecond(values[1]);numericInitialized.current=true;const input=e.currentTarget;requestAnimationFrame(()=>input.select());}}}
      readOnly={relation && dimensional && reference || sketch.view.tool === "arc-center" && sketch.view.pending_picks >= 2}
      value={sketch.view.tool === "arc-center" && sketch.view.pending_picks >= 2 ? String(sketch.view.arc_radius ?? "") : index===0?first:second} onChange={e=>index===0?setFirst(e.target.value):setSecond(e.target.value)} onKeyDown={e=>{
        if(e.key==="Escape"){e.preventDefault();e.stopPropagation();setFirst("");setSecond("");numericInitialized.current=false;sketch.focusCanvas();}
        if(e.key==="Tab" && !e.ctrlKey && !e.metaKey && !e.altKey){const fields=e.currentTarget.form?.querySelectorAll<HTMLInputElement>("[data-sketch-numeric]");if(fields?.length){e.preventDefault();const next=fields[(index+(e.shiftKey?-1:1)+fields.length)%fields.length];next.focus();next.select();}}
      }}/><span>{label === "Angle" ? "°" : "mm"}</span></label>)}
    {relation && dimensional && <label className="flex gap-1 text-xs"><input type="checkbox" checked={reference} onChange={e=>{setReference(e.target.checked);if(e.target.checked){const values=sketch.numericValues(action ?? "");setFirst(values[0]);setSecond(values[1]);numericInitialized.current=true;}}}/>Reference</label>}
    {tool==="fillet" && <button type="button" className={sketchButton} onClick={()=>sketch.run(s=>s.sketch_preview_fillet_text(first))}>Preview</button>}
    {(numeric || relation) && !(sketch.view.tool === "arc-center" && sketch.view.pending_picks >= 2) && <button className={sketchButton} type="submit" aria-disabled={relation && !eligibility?.enabled} title={relation ? eligibility?.reason ?? "Select compatible geometry" : undefined}>Apply</button>}
    {sketch.error && <span id="sketch-toolbar-error" role="alert" className="w-full text-xs text-[var(--destructive)]">{sketch.error}</span>}
  </form>;
}
export const SketchToolRail = memo(SketchToolRailBody, (a,b)=>a.paused===b.paused && a.sketch.view.active===b.sketch.view.active && a.sketch.view.tool===b.sketch.view.tool && a.sketch.view.constraint_tool===b.sketch.view.constraint_tool);
export const SketchToolbar = memo(SketchToolbarBody, (a,b)=>selectedEntities(a.sketch)===selectedEntities(b.sketch) && a.sketch.view.active===b.sketch.view.active && a.sketch.view.tool===b.sketch.view.tool && a.sketch.view.mode===b.sketch.view.mode && a.sketch.view.draft_epoch===b.sketch.view.draft_epoch && a.sketch.view.can_resume===b.sketch.view.can_resume && a.sketch.view.pending_picks===b.sketch.view.pending_picks && a.sketch.view.arc_radius===b.sketch.view.arc_radius && a.sketch.view.constraint_tool===b.sketch.view.constraint_tool && a.sketch.view.dimension_action===b.sketch.view.dimension_action && JSON.stringify(a.sketch.view.dimension_position)===JSON.stringify(b.sketch.view.dimension_position) && JSON.stringify(a.sketch.view.actions)===JSON.stringify(b.sketch.view.actions) && JSON.stringify(a.sketch.view.modes)===JSON.stringify(b.sketch.view.modes) && a.sketch.error===b.sketch.error);

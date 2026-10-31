import { memo } from "react";
import { Circle, Dot, Ellipse, MousePointer2, PenLine, RectangleHorizontal, Spline, Square, CornerDownRight, Link2, Ruler } from "lucide-react";
import type { SketchApi } from "@/hooks/useSketch";
import { RAIL_CARD_CLASS, RAIL_CARD_STYLE } from "@/components/railCard";
import { Tooltip } from "@/components/ui/tooltip";
const tools = [["select","Select",MousePointer2],["point","Point",Dot],["polyline","Polyline",PenLine],["arc","Arc",Spline],["circle","Circle",Circle],["ellipse","Ellipse",Ellipse],["rectangle","Rectangle",Square],["slot","Slot",RectangleHorizontal],["fillet","Fillet",CornerDownRight],["constraints","Constraints",Link2],["dimension","Dimension",Ruler]] as const;
const family = (tool: string) => tool.split("-")[0];
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
export const SketchToolRail = memo(SketchToolRailBody, (a,b)=>a.paused===b.paused && a.sketch.view.active===b.sketch.view.active && a.sketch.view.tool===b.sketch.view.tool && a.sketch.view.constraint_tool===b.sketch.view.constraint_tool);

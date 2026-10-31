import { memo, useRef, useState } from "react";
import { Circle, Dot, Ellipse, MousePointer2, PenLine, RectangleHorizontal, Spline, Square, CornerDownRight } from "lucide-react";
import type { SketchApi } from "@/hooks/useSketch";
import { RAIL_CARD_CLASS, RAIL_CARD_STYLE } from "@/components/railCard";
import { Tooltip } from "@/components/ui/tooltip";
import { ConstraintIcon } from "@/components/sketch/icons/constraintIcons";
import type { SketchRailTool } from "@/lib/sketchView";

const tools = [["select","Select",MousePointer2],["point","Point",Dot],["polyline","Polyline",PenLine],["arc","Arc",Spline],["circle","Circle",Circle],["ellipse","Ellipse",Ellipse],["rectangle","Rectangle",Square],["slot","Slot",RectangleHorizontal],["fillet","Fillet",CornerDownRight]] as const;
const family = (tool: string) => tool.split("-")[0];
const BUTTON_CLASS = "pointer-events-auto flex size-10 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)]";
/** A 1px, 25% divider follows Fixed and Midpoint (UX notes revision 2, section 1). */
const DIVIDER_AFTER = new Set(["fixed", "midpoint"]);
const ROVING_KEYS = ["ArrowUp", "ArrowDown", "Home", "End"];

function roving(key: string, index: number, count: number) {
  return key === "Home" ? 0 : key === "End" ? count - 1 : (index + (key === "ArrowDown" ? 1 : -1) + count) % count;
}

function SketchToolRailBody({ sketch, paused }: { sketch: SketchApi; paused: boolean }) {
  const constraintArmed = sketch.view.tool === "constraints";
  return <nav aria-label="Sketch tools" inert={paused} className="pointer-events-none absolute top-3 bottom-1 left-3 z-30 flex gap-2">
    <div className={`${RAIL_CARD_CLASS} self-start overflow-y-auto`} style={RAIL_CARD_STYLE} onKeyDown={e => {
      if (!ROVING_KEYS.includes(e.key)) return;
      const buttons=Array.from(e.currentTarget.querySelectorAll<HTMLButtonElement>("button"));
      const index=buttons.indexOf(document.activeElement as HTMLButtonElement);
      e.preventDefault(); buttons[roving(e.key, index, buttons.length)]?.focus();
    }}>{tools.map(([tool,label,Icon]) => <Tooltip key={tool} content={label} side="right"><button type="button" aria-label={label}
      aria-pressed={!constraintArmed && family(sketch.view.tool) === tool}
      className={BUTTON_CLASS}
      onClick={e => { sketch.run(s => s.set_sketch_tool(tool)); if(e.detail>0) sketch.focusCanvas(); }}><Icon size={20} strokeWidth={1.5} /></button></Tooltip>)}</div>
    <ConstraintCard sketch={sketch} />
  </nav>;
}

/**
 * The Constraints card (AC24) in column B: Rust says per tool whether a press applies, arms or is
 * unavailable, with the reason as the tooltip. One Tab stop, Up/Down/Home/End inside.
 */
function ConstraintCard({ sketch }: { sketch: SketchApi }) {
  const rail = sketch.view.rail ?? [];
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const [stop, setStop] = useState(0);
  const press = (tool: SketchRailTool, byMouse: boolean) => {
    if (tool.state === "unavailable") return;
    sketch.run(s => s.sketch_rail_press(tool.name));
    if (byMouse) sketch.focusCanvas();
  };
  return <div role="toolbar" aria-label="Constraints" aria-orientation="vertical" className={`${RAIL_CARD_CLASS} self-start`} style={RAIL_CARD_STYLE}>
    {rail.map((tool, index) => <div key={tool.name} className="flex flex-col items-center">
      <Tooltip content={tool.tooltip} side="right"><button type="button" ref={node => { buttons.current[index] = node; }}
        aria-label={tool.label} aria-pressed={tool.pressed} aria-disabled={tool.state === "unavailable" || undefined}
        data-sketch-rail={tool.name} tabIndex={index === stop ? 0 : -1}
        className={`${BUTTON_CLASS} ${tool.state === "unavailable" ? "opacity-40 hover:bg-transparent" : ""}`}
        onFocus={() => setStop(index)}
        onKeyDown={e => {
          if (!ROVING_KEYS.includes(e.key)) return;
          e.preventDefault(); buttons.current[roving(e.key, index, rail.length)]?.focus();
        }}
        onClick={e => press(tool, e.detail > 0)}><ConstraintIcon name={tool.name} size={20} /></button></Tooltip>
      {DIVIDER_AFTER.has(tool.name) && <div aria-hidden className="mt-1 h-px w-8" style={{ background: "var(--toolbar-icon)", opacity: 0.25 }} />}
    </div>)}
  </div>;
}

const railKey = (view: SketchApi["view"]) => JSON.stringify(view.rail ?? []);
export const SketchToolRail = memo(SketchToolRailBody, (a,b)=>a.paused===b.paused && a.sketch.view.active===b.sketch.view.active && a.sketch.view.tool===b.sketch.view.tool && railKey(a.sketch.view)===railKey(b.sketch.view));

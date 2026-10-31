import { useSyncExternalStore, type CSSProperties } from "react";
import type { SketchApi } from "@/hooks/useSketch";
import type { SketchAnnotation, SketchSymbol } from "@/lib/sketchView";
import { ConstraintIcon } from "@/components/sketch/icons/constraintIcons";

/** The 2px `--dim-halo` around dimension text, so it reads over geometry (UX notes revision 2, section 7). */
const HALO = "0 0 2px var(--dim-halo), 0 0 2px var(--dim-halo), 0 0 2px var(--dim-halo)";
const AT = (x: number, y: number): CSSProperties => ({ left: 0, top: 0, transform: `translate(${x}px, ${y}px) translate(-50%, -50%)` });

/**
 * Dimension texts, constraint symbols and transient labels, all placed by Rust (AC31, AC32).
 * Pointer input goes to the canvas, where Rust hit-tests them; keyboard and screen readers reach
 * the dimension texts here (Enter edits) and the symbols by name.
 */
export function SketchAnnotations({ sketch }: { sketch: SketchApi }) {
  const { presentation } = sketch;
  const annotations = useSyncExternalStore(presentation.subscribeAnnotations, presentation.annotations);
  const symbols = useSyncExternalStore(presentation.subscribeAnnotations, presentation.symbols);
  if(sketch.view.workbench!=="sketch") return null;
  const edit = (id: string) => { if (sketch.run(s => s.sketch_edit_dimension(id))) presentation.setEntryOpen(true); };
  return <div className="pointer-events-none absolute inset-0 z-10 overflow-hidden" aria-label="Sketch annotations">
    {symbols.badges.map((symbol, index) => <SymbolBadge key={`${symbol.id}:${index}`} symbol={symbol} />)}
    {symbols.more.map((more, index) => <span key={`more:${index}`} aria-label={`${more.count} more constraints`} role="img"
      className="absolute flex size-4 items-center justify-center rounded-[3px] text-[9px] text-[var(--toolbar-icon)]"
      style={{ ...AT(more.x, more.y), background: "var(--sketch-symbol-bg)", border: "1px solid color-mix(in srgb, var(--toolbar-icon) 40%, transparent)" }}>+{more.count}</span>)}
    {annotations.map(label => label.dimension && !label.preview
      ? <DimensionText key={label.id} label={label} onSelect={() => sketch.run(s => s.sketch_inspect_constraint(label.id, false))} onEdit={() => edit(label.id)} />
      : <span key={label.id} className="absolute rounded bg-[var(--toolbar-bg)] px-1 text-[11px] text-[var(--toolbar-icon)] shadow-sm"
        style={{ ...AT(label.x, label.y), border: label.conflicting ? "1px solid var(--destructive)" : undefined }}>{label.text}</span>)}
  </div>;
}

function DimensionText({ label, onSelect, onEdit }: { label: SketchAnnotation; onSelect: () => void; onEdit: () => void }) {
  const color = label.selected ? "var(--editor-accent)" : label.reference ? "var(--dim-reference)" : "var(--dim-driving)";
  const border = label.conflicting ? "1px solid var(--destructive)" : label.selected ? "1px solid var(--editor-accent)" : "1px solid transparent";
  return <button type="button" aria-label={label.name || label.text} aria-pressed={label.selected} data-sketch-dimension-text={label.id}
    className="absolute rounded-full px-1 text-[12px] leading-4 whitespace-nowrap tabular-nums focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)]"
    style={{ ...AT(label.x, label.y), color, border, textShadow: HALO, visibility: label.editing ? "hidden" : undefined,
      background: label.hovered ? "var(--editor-accent-hover)" : undefined }}
    onClick={e => { if (e.detail === 0) onSelect(); }}
    onKeyDown={e => { if (e.key === "Enter") { e.preventDefault(); e.stopPropagation(); onEdit(); } }}>
    {label.fx && <span className="mr-0.5 text-[10px] text-[var(--panel-muted-fg)]">fx</span>}{label.text}
  </button>;
}

function SymbolBadge({ symbol }: { symbol: SketchSymbol }) {
  if (symbol.dot) {
    return <span role="img" aria-label={symbol.name} className="absolute size-1.5 rounded-full"
      style={{ ...AT(symbol.x, symbol.y), background: "var(--toolbar-icon)" }} />;
  }
  const ground = symbol.selected ? "var(--editor-accent)" : symbol.hovered ? "var(--editor-accent-hover)" : "var(--sketch-symbol-bg)";
  const glyph = symbol.conflicting ? "var(--destructive)" : symbol.selected ? "#FFFFFF" : "var(--toolbar-icon)";
  const border = symbol.conflicting ? "var(--destructive)" : "color-mix(in srgb, var(--toolbar-icon) 40%, transparent)";
  return <span role="img" aria-label={symbol.name} data-sketch-symbol={symbol.glyph}
    className="absolute flex size-4 items-center justify-center rounded-[3px]"
    style={{ ...AT(symbol.x, symbol.y), background: ground, color: glyph, border: `1px solid ${border}` }}>
    <ConstraintIcon name={symbol.glyph} size={12} />
    {symbol.index !== null && <span aria-hidden className="absolute -right-1 -bottom-1 text-[8px] leading-none font-semibold">{symbol.index}</span>}
  </span>;
}

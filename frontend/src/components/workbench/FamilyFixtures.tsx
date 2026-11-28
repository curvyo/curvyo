import { useState } from "react";
import { Plus, Trash2 } from "lucide-react";

import { EntryField } from "@/components/panel/EntryField";
import { PresetStrip } from "@/components/panel/PresetStrip";
import { PanelSwitch } from "@/components/ui/switch";
import type { ManufacturingView, WorkbenchesApi } from "@/hooks/useWorkbenches";
import type { CncTool } from "@/lib/cncView";

/** The heading look of a Fixtures or Tools section (`0113` 4.1). */
const HEADING = "text-[11px] font-semibold text-[var(--panel-muted-fg)]";
/** The `0113` 4.2 outline text button. */
const TEXT_BUTTON =
  "flex h-7 items-center gap-1 rounded border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] px-2 text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:min-h-9";

interface FamilyTabProps {
  view: ManufacturingView;
  workbenches: WorkbenchesApi;
  onReturnFocus: () => void;
}

/** A committed text field: `"committed"`, or the session's message as `invalid:`. */
function field(
  key: string,
  label: string,
  unit: string,
  shown: string,
  width: number,
  commit: (text: string) => string,
  onReturnFocus: () => void,
) {
  const messages: Record<string, string> = {};
  return (
    <EntryField
      key={key}
      label={unit ? `${label}, ${unit}` : label}
      prefix={label}
      shown={shown}
      mixed={false}
      suffix={unit || undefined}
      width={width}
      align={unit ? "right" : "left"}
      onSubmit={(text) => {
        const message = commit(text);
        if (message === "") return "committed";
        messages[key] = message;
        return `invalid:${key}`;
      }}
      messages={messages}
      onReturnFocus={onReturnFocus}
      commitOnBlur
    />
  );
}

/**
 * The Fixtures tab of a plotter, embroidery or CNC page (UX unify 9): the mat (size, grip, Mat
 * grid), the hoop, or the stock, with the laser's Choice strips, switches and white fields.
 */
export function FamilyFixtures({ view, workbenches, onReturnFocus }: FamilyTabProps) {
  if (view.plotter) {
    const mat = view.plotter.mat;
    if (!mat) return null;
    return (
      <section aria-label="Mat" className="flex flex-col gap-2">
        <h3 className={HEADING}>Mat</h3>
        <PresetStrip
          label="Mat size"
          cells={mat.sizes.map((s) => ({ id: s.key, label: s.label, tooltip: s.label, content: s.label }))}
          value={mat.size}
          onPress={(id) => workbenches.plotter.setMat(id)}
          onReturnFocus={onReturnFocus}
          width="w-[228px]"
        />
        <PresetStrip
          label="Grip"
          cells={mat.grips.map((g) => ({
            id: g.key,
            label: g.word,
            tooltip: g.word,
            content: (
              <>
                <span aria-hidden className="size-3 shrink-0 rounded-sm" style={{ background: g.colour }} />
                {g.word}
              </>
            ),
          }))}
          value={mat.grip}
          onPress={(id) => workbenches.plotter.setGrip(id)}
          onReturnFocus={onReturnFocus}
          width="w-[228px]"
        />
        <PanelSwitch
          label="Mat grid"
          checked={mat.grid}
          onCheckedChange={(on) => workbenches.plotter.setGrid(on)}
          onReturnFocus={onReturnFocus}
        />
      </section>
    );
  }
  if (view.embroidery) {
    const hoop = view.embroidery.hoop;
    return (
      <section aria-label="Hoop" className="flex flex-col gap-2">
        <h3 className={HEADING}>Hoop</h3>
        <PresetStrip
          label="Hoop"
          cells={hoop.sizes.map((s) => ({ id: s.key, label: s.label, tooltip: s.label, content: s.label }))}
          value={hoop.key}
          onPress={(id) => workbenches.embroidery.setHoop(id)}
          onReturnFocus={onReturnFocus}
          width="w-[228px]"
        />
      </section>
    );
  }
  if (view.cnc) {
    const stock = view.cnc.stock;
    const set = (key: string) => (text: string) => workbenches.cnc.setStock(key, text);
    return (
      <section aria-label="Stock" className="flex flex-col gap-2">
        <h3 className={HEADING}>Stock</h3>
        <div className="flex gap-1">
          {field("width_mm", "Width", "mm", String(stock.width), 112, set("width_mm"), onReturnFocus)}
          {field("height_mm", "Height", "mm", String(stock.height), 112, set("height_mm"), onReturnFocus)}
        </div>
        {field("thickness_mm", "Thickness", "mm", String(stock.thickness), 112, set("thickness_mm"), onReturnFocus)}
        {field("material", "Material", "", stock.material, 228, set("material"), onReturnFocus)}
      </section>
    );
  }
  return null;
}

const TOOL_FIELDS: { label: string; unit: string; key: keyof CncTool["numbers"] }[] = [
  { label: "Diameter", unit: "mm", key: "diameter_mm" },
  { label: "Flutes", unit: "", key: "flutes" },
  { label: "Feed", unit: "mm/min", key: "feed_mm_min" },
  { label: "Plunge", unit: "mm/min", key: "plunge_mm_min" },
  { label: "Spindle", unit: "rpm", key: "rpm" },
  { label: "Step-down", unit: "mm", key: "step_down_mm" },
];

/**
 * The Tools tab of a CNC page (UX unify 9, `0161` 5): the page's end mills as rows of the
 * Materials look, "+ Tool", one unfolded at a time with its white fields, and the armed trash
 * (the last tool stays).
 */
export function CncToolsTab({ view, workbenches, onReturnFocus }: FamilyTabProps) {
  const [unfolded, setUnfolded] = useState<string | null>(null);
  const [armed, setArmed] = useState<string | null>(null);
  const cnc = view.cnc;
  if (!cnc) return null;
  const last = cnc.tools.length === 1;
  return (
    <section aria-label="Tools" className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <h3 className={HEADING}>Tools</h3>
        <button type="button" className={TEXT_BUTTON} onClick={() => setUnfolded(workbenches.cnc.addTool() || unfolded)}>
          <Plus size={14} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          Tool
        </button>
      </div>
      <ul role="list" className="flex flex-col">
        {cnc.tools.map((tool) => (
          <li
            key={tool.id}
            className="flex flex-col gap-1 rounded-md px-1 py-1"
            style={armed === tool.id ? { background: "color-mix(in srgb, var(--destructive) 12%, transparent)" } : undefined}
          >
            <div className="flex min-h-10 items-center gap-2 pointer-coarse:min-h-12">
              <button
                type="button"
                aria-label={`${tool.name}, ${tool.line2}`}
                aria-expanded={unfolded === tool.id}
                className="flex min-w-0 flex-1 flex-col items-start text-left outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
                onClick={() => setUnfolded(unfolded === tool.id ? null : tool.id)}
              >
                <span className="truncate text-sm text-[var(--toolbar-icon)]">{tool.name}</span>
                <span className="text-[11px] text-[var(--panel-muted-fg)]">{tool.line2}</span>
              </button>
              {last ? null : (
                <button
                  type="button"
                  aria-label={armed === tool.id ? `Press again to delete ${tool.name}` : `Delete ${tool.name}`}
                  className="flex size-7 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
                  onBlur={() => setArmed(null)}
                  onClick={() => {
                    if (armed === tool.id) {
                      workbenches.cnc.removeTool(tool.id);
                      setArmed(null);
                    } else setArmed(tool.id);
                  }}
                >
                  <Trash2 size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                </button>
              )}
            </div>
            {unfolded === tool.id ? (
              <div className="flex flex-wrap gap-1 border-l-2 border-[var(--editor-accent)] pl-3">
                {TOOL_FIELDS.map((f) =>
                  field(
                    f.key,
                    f.label,
                    f.unit,
                    String(tool.numbers[f.key]),
                    112,
                    (text) => workbenches.cnc.setTool(tool.id, f.key, text),
                    onReturnFocus,
                  ),
                )}
              </div>
            ) : null}
          </li>
        ))}
      </ul>
    </section>
  );
}

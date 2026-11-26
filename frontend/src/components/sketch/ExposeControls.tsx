import { ChevronDown, ChevronUp, Link } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";

import { PanelSwitch } from "@/components/ui/switch";
import type { SketchApi } from "@/hooks/useSketch";
import type { SketchExpose, SketchLink } from "@/lib/sketchView";
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
  const root = useRef<HTMLDivElement>(null);
  /** Where focus goes once the row has redrawn after a Link or Unlink (UX notes 12.9.1). */
  const focusAfter = useRef<string[] | null>(null);
  useEffect(() => {
    const wanted = focusAfter.current;
    if (wanted === null) return;
    focusAfter.current = null;
    for (const selector of wanted) {
      const target = root.current?.querySelector<HTMLElement>(selector);
      if (target) { target.focus(); return; }
    }
  });
  const command = (run: (session: Parameters<Parameters<SketchApi["run"]>[0]>[0]) => string) =>
    sketch.run(session => { const refusal = run(session); if (refusal !== "") throw refusal; });
  const unit = expose.angle ? "°" : "mm";
  const limits = (min: string, max: string, name: string) =>
    command(s => s.sketch_exposed_limits(expose.key, min, max, name));
  const nameValue = expose.name ?? expose.suggest ?? "";
  return <div ref={root} data-expose={expose.key} className="col-span-3 flex flex-col gap-1">
    <div className="flex h-7 items-center" title={expose.on ? stopTitle(expose.holders, expose.link?.partners) : "Clones may set their own value"}>
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
      {expose.link && <LinkLine link={expose.link} subject={subject}
        onLink={partner => { focusAfter.current = ["[data-unlink]"]; command(s => s.sketch_link(expose.key, partner)); }}
        onUnlink={() => { focusAfter.current = ["[data-link-open]", "[role=switch]"]; command(s => s.sketch_unlink(expose.key)); }} />}
    </>}
  </div>;
}

/**
 * The Link line of an exposed value (`specs/0099-sketch-pages` criteria 32 and 35, UX notes
 * 12.9.1): a Link button that opens an inline list of the groups and values of its kind (no
 * popup), or, while it is in a group, "Linked with ..." and an Unlink button. One step each.
 */
function LinkLine({ link, subject, onLink, onUnlink }: { link: SketchLink; subject: string; onLink: (partner: string) => void; onUnlink: () => void }) {
  const [open, setOpen] = useState(false);
  const listId = useId();
  const opener = useRef<HTMLButtonElement>(null);
  const close = () => { setOpen(false); opener.current?.focus(); };
  if (link.group !== null) {
    const text = `Linked with ${(link.partners ?? []).join(", ")}`;
    return <div className="mt-2 flex h-7 items-center gap-1 text-xs text-[var(--toolbar-icon)]" data-link-line>
      <span aria-hidden className="inline-flex items-center gap-0.5 text-[11px] font-medium text-[var(--panel-muted-fg)]"><Link size={12} strokeWidth={1.5} />{link.group}</span>
      <span className="min-w-0 flex-1 truncate" title={text}>{text}</span>
      <button type="button" data-unlink aria-label={`Unlink ${subject}`} onClick={onUnlink}
        className="h-7 rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] px-2 text-sm hover:border-[var(--toolbar-icon)]">Unlink</button>
    </div>;
  }
  const options = link.options ?? [];
  return <div className="mt-2 flex flex-col gap-1" data-link-line>
    <button ref={opener} type="button" data-link-open aria-expanded={open} aria-controls={listId} aria-label={`Link ${subject}`}
      onClick={() => setOpen(!open)}
      onKeyDown={e => { if (e.key === "Escape" && open) { e.preventDefault(); e.stopPropagation(); close(); } }}
      className="flex h-7 w-[244px] items-center justify-between rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] px-2 text-sm text-[var(--toolbar-icon)] hover:border-[var(--toolbar-icon)]">
      Link{open ? <ChevronUp size={14} aria-hidden /> : <ChevronDown size={14} aria-hidden />}
    </button>
    {open && <div id={listId} role="listbox" aria-label={`Link ${subject} with`} className="flex w-[244px] flex-col"
      onKeyDown={e => { if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(); } }}>
      {options.map(option => <div key={option.with} role="option" aria-selected={false} tabIndex={0}
        aria-label={option.group ? `Group ${option.group}: ${option.label}` : option.label}
        onClick={() => { setOpen(false); onLink(option.with); }}
        onKeyDown={e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setOpen(false); onLink(option.with); } }}
        className="flex h-7 cursor-pointer items-center gap-1 rounded-[4px] px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]">
        {option.group && <span aria-hidden className="inline-flex items-center gap-0.5 text-[11px] font-medium text-[var(--panel-muted-fg)]"><Link size={12} strokeWidth={1.5} />{option.group}</span>}
        <span className="truncate">{option.label}</span>
      </div>)}
    </div>}
  </div>;
}

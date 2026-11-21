import { Check, Crop } from "lucide-react";
import { useEffect } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { Tooltip } from "@/components/ui/tooltip";
import type { CropBarApi } from "@/hooks/useCropBar";
import { fieldStep, fieldText, typedField } from "@/lib/adjustText";

/** The bar widths of Left, Top, Width, Height (`ux-notes.md` §3.9). */
const WIDTHS: Record<string, number> = { left_pct: 96, top_pct: 96, width_pct: 104, height_pct: 112 };

/** The elements whose own Enter press wins over leaving the bar. */
const CONTROLS =
  "button, a[href], input, textarea, select, [contenteditable='true'], [role=button], [role=radio], [role=tab], [role=slider], [role=option], [role=menuitem], [role=checkbox], [role=switch], [role=group][tabindex]";

/**
 * The Crop bar (`specs/0135` criterion 5, `ux-notes.md` §3.9, P10, P11): it replaces the Select
 * bar while a Crop row is edited on the canvas. The Crop glyph and the word "Crop" (a name, not a
 * control), the four fields of the row in %, then Done. Every field commit is one step; Done,
 * Enter on the canvas and Escape leave (no Apply and no Cancel: undo covers it).
 */
export function CropBar({ api, onReturnFocus }: { api: CropBarApi; onReturnFocus: () => void }) {
  const view = api.view;
  const close = api.close;
  useEffect(() => {
    if (!view) return undefined;
    const key = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      // Escape leaves unless a field takes it first (the cascade of §3.9); Enter leaves only on
      // the canvas: on a panel control it keeps its own press (a fold, a strip cell, a button).
      const leaves =
        event.key === "Escape"
          ? !target?.closest("input, textarea, [contenteditable='true']")
          : event.key === "Enter" && !target?.closest(CONTROLS);
      if (!leaves) return;
      // One key is one cascade step: leaving the bar is all it does (the canvas would otherwise
      // also take the Escape and clear the selection), so the key is taken before anyone else.
      event.preventDefault();
      event.stopPropagation();
      close();
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [view, close]);
  if (!view) return null;
  return (
    <div
      data-context-bar=""
      role="toolbar"
      aria-label="Crop"
      className="pointer-events-auto flex h-11 items-center gap-1 rounded-lg px-2 text-sm"
      style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
    >
      <Crop aria-hidden size={16} strokeWidth={1.5} className="text-[var(--toolbar-icon)]" />
      <span className="mr-1 text-sm text-[var(--toolbar-icon)]">Crop</span>
      {view.fields.map((field) => (
        <EntryField
          key={field.key}
          label={`Crop ${field.label}, %`}
          prefix={field.label}
          shown={fieldText(field.value, field.decimals)}
          mixed={false}
          suffix="%"
          width={WIDTHS[field.key] ?? 104}
          commitOnBlur
          inputMode="decimal"
          messages={{ range: field.range }}
          stepText={fieldStep(field.decimals)}
          onSubmit={(text) => {
            const typed = typedField(text);
            if (!Number.isFinite(typed)) return "invalid:range";
            if (typed === field.value) return "unchanged";
            return api.editField(field.key, typed) === "" ? "committed" : "invalid:range";
          }}
          onReturnFocus={onReturnFocus}
        />
      ))}
      <Tooltip side="bottom" content="Done (Enter)">
        <button
          type="button"
          aria-label="Done"
          onClick={() => {
            close();
            onReturnFocus();
          }}
          className="flex size-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          <Check aria-hidden size={16} strokeWidth={1.5} />
        </button>
      </Tooltip>
    </div>
  );
}

import { useEffect, useRef, useState } from "react";
export const sketchInput = "h-7 min-w-0 w-full rounded border border-[var(--field-border)] bg-[var(--toolbar-bg)] px-2 text-xs text-[var(--toolbar-icon)] outline-none focus:border-[var(--editor-accent)]";
export const sketchButton = "min-h-7 rounded px-2 text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] aria-disabled:opacity-40";
/** Commit whole drafts once; modifier keys and native text undo keep their usual routing. */
export function SketchField({ label, value, commit, focusCanvas, autoFocus = false }: { label: string; value: string; commit: (text: string) => boolean; focusCanvas: () => void; autoFocus?: boolean }) {
  const [draft, setDraft] = useState<string | null>(null);
  const [invalid, setInvalid] = useState(false);
  const dirty = useRef(false);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => { if (autoFocus) { input.current?.focus(); input.current?.select(); } }, [autoFocus]);
  const submit = () => {
    if (!dirty.current) return true;
    if (!commit(draft ?? value)) { setInvalid(true); return false; }
    dirty.current = false; setDraft(null); setInvalid(false); return true;
  };
  return <input ref={input} className={sketchInput} aria-label={label} aria-invalid={invalid} aria-describedby={invalid ? "sketch-error" : undefined} value={draft ?? value}
    onChange={event => { dirty.current = true; setDraft(event.target.value); setInvalid(false); }}
    onKeyDown={event => {
      if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); dirty.current = false; setDraft(null); setInvalid(false); focusCanvas(); }
      if (event.key === "Enter") { event.preventDefault(); event.stopPropagation(); if (submit()) focusCanvas(); }
      if (event.key === "Tab" && !submit()) event.preventDefault();
    }} onBlur={() => { if (dirty.current) submit(); }} />;
}

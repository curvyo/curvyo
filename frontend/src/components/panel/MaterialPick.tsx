import { ChevronDown, ChevronRight } from "lucide-react";
import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";

import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import { useScrollWindow } from "@/hooks/useScrollWindow";
import { MATERIAL_WINDOW_ABOVE } from "@/lib/materialRows";
import { PICK_FILTER_ABOVE, filterPick, kindsText, thicknessText } from "@/lib/operationSettingsRows";
import type { PickMaterial } from "@/lib/operationSettingsView";
import { visibleRange } from "@/lib/operationWindow";

const PICK_ROW = 28;
const OVERSCAN = 112;

interface MaterialPickProps {
  /** The operation's name, or "3 operations": the pick's accessible name. */
  subject: string;
  /** The source row's value: the material's name, "Own numbers" or "Mixed". */
  value: string;
  mixed: boolean;
  /** The material of the (single) selection, `null` for own numbers or a mixed selection. */
  currentId: string | null;
  entries: readonly PickMaterial[];
  /** The words that replace the symbols of a material the operation cannot take ("No Score set"). */
  disabledFor: (id: string) => string | null;
  /** The pick: a material id, or `""` for Own numbers. */
  onPick: (id: string) => void;
}

/** The kind symbols of a material, Cut red and the others ink (`OperationGlyph`). */
function KindSymbols({ kinds }: { kinds: PickMaterial["kinds"] }) {
  return (
    <span className="flex gap-1" aria-hidden>
      {kinds.map((kind) => (
        <OperationGlyph key={kind} kind={kind} size={12} />
      ))}
    </span>
  );
}

/**
 * The Material pick of the Settings block (`specs/0028-material-test-library` `ux-notes.md` 8.2):
 * a disclosure button and, open, an inline radiogroup (no popup) with "Own numbers" first, then
 * the machine's materials; a material without the needed set stays, muted and `aria-disabled`,
 * with "No Score set" in place of its symbols. Above 8 materials a Filter field heads the list.
 */
export function MaterialPick(props: MaterialPickProps) {
  const { subject, value, mixed, currentId, entries, disabledFor, onPick } = props;
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [stop, setStop] = useState(0);
  const listId = useId();
  const sourceRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const filtered = useMemo(() => filterPick(entries, query), [entries, query]);
  // The radio rows: Own numbers first (id ""), then the filtered materials.
  const choices = useMemo(() => [{ id: "" }, ...filtered.map((entry) => ({ id: entry.id }))], [filtered]);
  const windowed = open && choices.length > MATERIAL_WINDOW_ABOVE;
  const scroll = useScrollWindow(listRef, windowed, choices.length);
  const range = windowed
    ? visibleRange(
        choices.map(() => PICK_ROW),
        scroll?.top ?? 0,
        scroll?.height ?? 480,
        OVERSCAN,
        stop,
        null,
      )
    : { first: 0, last: choices.length, before: 0, after: 0 };

  // Opening puts the focus on the current choice; closing hands it back to the source row.
  const focusRow = useRef<number | null>(null);
  useEffect(() => {
    if (!open) return;
    const at = Math.max(0, choices.findIndex((choice) => choice.id === (currentId ?? "")));
    setStop(at);
    focusRow.current = entries.length > PICK_FILTER_ABOVE ? null : at;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);
  useEffect(() => {
    const at = focusRow.current;
    if (at === null || !open) return;
    focusRow.current = null;
    listRef.current?.querySelector<HTMLElement>(`[data-choice="${at}"]`)?.focus();
  });

  const close = (refocus: boolean) => {
    setOpen(false);
    setQuery("");
    if (refocus) sourceRef.current?.focus();
  };
  const pick = (id: string) => {
    if (disabledFor(id) !== null) return;
    onPick(id);
    close(true);
  };
  const onListKey = (event: ReactKeyboardEvent<HTMLElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close(true);
      return;
    }
    const step = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const next = Math.min(choices.length - 1, Math.max(0, stop + step));
    setStop(next);
    focusRow.current = next;
  };

  const Chevron = open ? ChevronDown : ChevronRight;
  return (
    <div className="flex flex-col">
      <button
        ref={sourceRef}
        type="button"
        aria-label={`Material of ${subject}`}
        aria-expanded={open}
        aria-controls={open ? listId : undefined}
        onClick={() => (open ? close(false) : setOpen(true))}
        className="flex h-7 w-[244px] items-center gap-1 rounded text-left hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        <Chevron size={12} aria-hidden className="shrink-0" />
        <span className="text-xs text-[var(--panel-muted-fg)]">Material</span>
        <span
          className={`min-w-0 flex-1 truncate text-right text-[13px] ${mixed ? "text-[var(--field-placeholder)]" : "text-[var(--toolbar-icon)]"}`}
        >
          {value}
        </span>
      </button>
      {open ? (
        <div id={listId} className="flex flex-col" onKeyDown={onListKey}>
          {entries.length > PICK_FILTER_ABOVE ? (
            <input
              type="text"
              aria-label="Filter"
              placeholder="Filter"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "ArrowDown") {
                  event.preventDefault();
                  event.stopPropagation();
                  setStop(0);
                  listRef.current?.querySelector<HTMLElement>('[data-choice="0"]')?.focus();
                }
              }}
              className="mb-1 h-7 rounded border border-[var(--lane-line)] bg-white px-2 text-[13px] text-[var(--toolbar-icon)]"
            />
          ) : null}
          <div ref={listRef} role="radiogroup" aria-label={`Material of ${subject}`} className="flex flex-col">
            <div aria-hidden style={{ height: range.before }} />
            {choices.slice(range.first, range.last).map((choice, offset) => {
              const at = range.first + offset;
              const entry = choice.id === "" ? null : filtered[at - 1];
              const checked = choice.id === (currentId ?? "") && !mixed;
              const reason = disabledFor(choice.id);
              return (
                <button
                  key={choice.id || "own"}
                  type="button"
                  role="radio"
                  data-choice={at}
                  aria-checked={checked}
                  aria-disabled={reason !== null ? true : undefined}
                  aria-label={
                    entry ? `${entry.name}, ${thicknessText(entry.thicknessMm)}, ${reason ?? kindsText(entry.kinds)}` : "Own numbers"
                  }
                  title={reason ?? undefined}
                  tabIndex={at === stop ? 0 : -1}
                  onFocus={() => setStop(at)}
                  onClick={() => pick(choice.id)}
                  style={{ height: PICK_ROW }}
                  className={`relative flex shrink-0 items-center gap-2 rounded pl-2 pr-1 text-left text-[13px] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${checked ? "bg-[var(--value-fill)]" : ""} ${reason !== null ? "text-[var(--panel-muted-fg)]" : "text-[var(--toolbar-icon)]"}`}
                >
                  {checked ? (
                    <span aria-hidden className="absolute left-0 top-0 h-full w-[3px] rounded bg-[var(--accent)]" />
                  ) : null}
                  <span className="min-w-0 flex-1 truncate">{entry ? entry.name : "Own numbers"}</span>
                  {entry ? (
                    <>
                      <span className="text-xs tabular-nums text-[var(--panel-muted-fg)]">{thicknessText(entry.thicknessMm)}</span>
                      {reason !== null ? (
                        <span className="text-[11px] text-[var(--panel-muted-fg)]">{reason}</span>
                      ) : (
                        <KindSymbols kinds={entry.kinds} />
                      )}
                    </>
                  ) : null}
                </button>
              );
            })}
            <div aria-hidden style={{ height: range.after }} />
          </div>
        </div>
      ) : null}
    </div>
  );
}

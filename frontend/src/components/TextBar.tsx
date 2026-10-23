// The Text bar (`specs/0074-text-objects` criterion 11; UX notes §3): the Font button and the
// Size field, then the Style group (`specs/0078-font-style-buttons`). In the Text tool it is the
// overlay row's bar; in the Select tool the same controls are the Select bar's Text and Style
// groups.
import { CaseSensitive, TriangleAlert } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Divider, useRowStarts } from "@/components/SelectToolbar";
import { StyleGroup } from "@/components/StyleGroup";
import { Tooltip } from "@/components/ui/tooltip";
import type { TextBarState } from "@/hooks/useText";
import type { WeightCode } from "@/lib/styleText";
import { parseSize, sizeStep, sizeText } from "@/lib/textText";

/** How long the clamp notice stays under the field (Action notice, status). */
const NOTICE_MS = 3000;

interface TextControlsProps {
  bar: TextBarState;
  /** The Font button: shows the Fonts tab with the current face row focused. */
  onShowFonts: () => void;
  /** A typed or stepped size; returns the notice under the field, if any. */
  onSetSize: (mm: number) => string | null;
  onPreviewSize: (mm: number | null) => void;
  onReturnFocus: () => void;
}

/** The two controls, without the pill (the Select bar holds them as a group). */
export function TextControls({ bar, onShowFonts, onSetSize, onPreviewSize, onReturnFocus }: TextControlsProps) {
  const shown = sizeText(bar.sizeMm);
  const [draft, setDraft] = useState<string | null>(null);
  const [invalid, setInvalid] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const stepping = useRef<number | null>(null);

  useEffect(() => {
    if (!notice) {
      return undefined;
    }
    const timer = window.setTimeout(() => setNotice(null), NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notice]);

  const commit = (mm: number) => {
    const text = onSetSize(mm);
    setNotice(text);
    setDraft(null);
    setInvalid(false);
  };

  const fontLabel = bar.font || "Mixed";
  const tooltip = bar.desktop ? "Font (Shift+Ctrl+T)" : "Font";
  return (
    <div className="flex items-center gap-3">
      <Tooltip
        side="bottom"
        content={
          <span>
            {tooltip}
            {bar.font ? <span className="block">{bar.font}</span> : null}
            {bar.missing ? <span className="block">{`Missing on this computer. Shown in ${bar.substitute}.`}</span> : null}
          </span>
        }
      >
        <button
          type="button"
          data-text-font-button
          onClick={onShowFonts}
          className="flex h-7 w-[168px] items-center gap-1.5 rounded-[5px] px-2 text-sm text-[var(--toolbar-icon)] outline-none ring-1 ring-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          {bar.missing ? (
            <TriangleAlert size={16} aria-hidden className="shrink-0 text-[var(--field-invalid)]" />
          ) : (
            <CaseSensitive size={16} aria-hidden className="shrink-0" />
          )}
          <span className={`min-w-0 flex-1 truncate text-left ${bar.font ? "" : "text-[var(--field-placeholder)]"}`}>
            {fontLabel}
          </span>
          {bar.missing ? <span className="shrink-0 text-xs text-[var(--field-invalid)]">missing</span> : null}
        </button>
      </Tooltip>
      <div className="relative">
        <label className="flex h-7 w-[120px] items-center rounded-[5px] bg-white px-2 text-sm ring-1 ring-border focus-within:ring-2 focus-within:ring-[var(--editor-accent)]">
          <span className="text-[var(--toolbar-icon)]">Size</span>
          <input
            type="text"
            inputMode="decimal"
            aria-label="Size"
            aria-invalid={invalid}
            value={draft ?? shown}
            placeholder={bar.sizeMm === null ? "Mixed" : undefined}
            onChange={(event) => {
              setDraft(event.target.value);
              setInvalid(false);
            }}
            onKeyDown={(event) => {
              if (event.key === "ArrowUp" || event.key === "ArrowDown") {
                event.preventDefault();
                const base = stepping.current ?? bar.sizeMm ?? 10;
                const step = sizeStep(event.shiftKey, event.ctrlKey || event.metaKey);
                const next = Math.round((base + (event.key === "ArrowUp" ? step : -step)) * 100) / 100;
                stepping.current = Math.min(2000, Math.max(0.1, next));
                onPreviewSize(stepping.current);
                setDraft(sizeText(stepping.current));
              } else if (event.key === "Enter") {
                event.preventDefault();
                const mm = parseSize(draft ?? shown);
                if (mm === null) {
                  setInvalid(true);
                  setNotice("Enter a number");
                  return;
                }
                commit(mm);
                onReturnFocus();
              } else if (event.key === "Escape") {
                event.preventDefault();
                event.stopPropagation();
                stepping.current = null;
                onPreviewSize(null);
                setDraft(null);
                setInvalid(false);
                onReturnFocus();
              }
            }}
            onKeyUp={(event) => {
              if ((event.key === "ArrowUp" || event.key === "ArrowDown") && stepping.current !== null) {
                const mm = stepping.current;
                stepping.current = null;
                commit(mm);
              }
            }}
            onBlur={() => {
              if (stepping.current !== null) {
                const mm = stepping.current;
                stepping.current = null;
                commit(mm);
              }
              setDraft(null);
            }}
            className="min-w-0 flex-1 bg-transparent text-right tabular-nums outline-none placeholder:text-[var(--field-placeholder)]"
          />
          <span className="pl-1 text-[var(--toolbar-icon)]">mm</span>
        </label>
        {notice ? (
          <div
            role="status"
            className="absolute top-full right-0 mt-1 rounded-lg bg-[var(--toolbar-bg)] p-2 text-xs whitespace-nowrap text-[var(--toolbar-icon)] shadow-[var(--panel-elevation-shadow)]"
          >
            {notice}
          </div>
        ) : null}
      </div>
    </div>
  );
}

interface TextBarProps extends TextControlsProps {
  /** Edit mode: Bold and Italic name their keys. */
  editing: boolean;
  onPressWeight: (code: WeightCode) => string | null;
  onPressItalic: () => string | null;
}

/** The Text tool's bar in the overlay row: one pill with the Font button and the Size field,
 * then the Style group (`specs/0078-font-style-buttons`, UX notes §1); it wraps by whole groups
 * and draws no divider at the start of a row. */
export function TextBar({ editing, onPressWeight, onPressItalic, ...props }: TextBarProps) {
  const barRef = useRef<HTMLDivElement>(null);
  useRowStarts(barRef);
  return (
    <div
      ref={barRef}
      role="toolbar"
      aria-label="Text"
      className="pointer-events-auto flex min-w-0 flex-wrap items-start gap-x-3 rounded-lg px-2"
      style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
    >
      <div className="flex h-9 items-center">
        <TextControls {...props} />
      </div>
      <div className="group/row flex h-9 items-center gap-3">
        <Divider />
        <StyleGroup
          style={props.bar.style}
          editing={editing}
          onPressWeight={onPressWeight}
          onPressItalic={onPressItalic}
          onReturnFocus={props.onReturnFocus}
        />
      </div>
    </div>
  );
}

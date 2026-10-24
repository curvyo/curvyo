// The Text bar (`specs/0074-text-objects` criterion 11; `specs/0084-text-bar-second-row`): row 1
// holds the controls used on every text (the Font jump button and the Size field, the Alignment
// group, the Style group, the expand button), row 2 the rare ones (Writing mode, Glyph
// orientation, Glyphs), shown below row 1 while the maker has opened it. In the Select tool the
// same controls are the Select bar's Text groups.
import { ChevronsDown, ChevronsUp, Omega, TriangleAlert, Type } from "lucide-react";
import { type ReactNode, useEffect, useId, useRef, useState } from "react";

import { AlignmentGroup } from "@/components/AlignmentGroup";
import { OrientationGroup, WritingModeGroup } from "@/components/WritingGroups";
import type { OrientationCode, WritingModeCode } from "@/lib/textWritingText";
import { Divider, useRowStarts } from "@/components/SelectToolbar";
import { BAR_ROW_CLASS } from "@/lib/barLayout";
import { StyleGroup } from "@/components/StyleGroup";
import { Tooltip } from "@/components/ui/tooltip";
import { shiftStyle, useNoticeShift } from "@/hooks/useNoticeShift";
import type { TextBarState } from "@/hooks/useText";
import type { WeightCode } from "@/lib/styleText";
import type { AlignCode } from "@/lib/textLayoutText";
import { changedRow2Controls, changedText, expandLabel, fontButtonName, fontTooltipLines } from "@/lib/textBarRows";
import { parseSize, sizeStep, sizeText } from "@/lib/textText";

/** A 28 x 28 icon button of the bar (the Glyphs button's look, `0082` UX notes §1). */
const ICON_BUTTON_CLASS =
  "flex size-7 shrink-0 items-center justify-center rounded-md outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

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
  const noticeRef = useRef<HTMLDivElement>(null);
  const shift = useNoticeShift(noticeRef, notice);

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

  const fontName = fontButtonName(bar);
  return (
    <div className="flex items-center gap-1">
      <Tooltip
        side="bottom"
        content={
          <span>
            {fontTooltipLines(bar).map((line) => (
              <span key={line} className="block">
                {line}
              </span>
            ))}
          </span>
        }
      >
        <button
          type="button"
          data-text-font-button
          aria-label={fontName}
          onClick={onShowFonts}
          className={`${ICON_BUTTON_CLASS} text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]`}
        >
          {bar.missing ? (
            <TriangleAlert size={16} strokeWidth={1.5} aria-hidden className="text-[var(--field-invalid)]" />
          ) : (
            <Type size={16} strokeWidth={1.5} aria-hidden />
          )}
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
            ref={noticeRef}
            role="status"
            style={shiftStyle(shift)}
            className="absolute top-full right-0 z-10 mt-1 rounded-lg bg-[var(--toolbar-bg)] p-2 text-xs whitespace-nowrap text-[var(--toolbar-icon)] shadow-[var(--panel-elevation-shadow)]"
          >
            {notice}
          </div>
        ) : null}
      </div>
    </div>
  );
}

/** The Glyphs button's state and press (`specs/0082-glyph-table` criterion 2). */
export interface GlyphsButtonState {
  /** In the tree: a text in edit mode or exactly one text selected. */
  shown: boolean;
  /** The pane is open in Glyphs mode. */
  pressed: boolean;
  onPress: (byKeyboard: boolean) => void;
}

/** The Glyphs icon button (UX notes §1): opens the preview pane in Glyphs mode, closes it when
 * pressed again. */
export function GlyphsButton({ glyphs }: { glyphs: GlyphsButtonState }) {
  return (
    <Tooltip side="bottom" content="Glyphs">
      <button
        type="button"
        aria-label="Glyphs"
        aria-pressed={glyphs.pressed}
        onClick={(event) => glyphs.onPress(event.detail === 0)}
        className={`${ICON_BUTTON_CLASS} ${
          glyphs.pressed
            ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
            : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
        }`}
      >
        <Omega size={16} strokeWidth={1.5} aria-hidden />
      </button>
    </Tooltip>
  );
}

/** Row 2 open or closed: one per-user preference shared by the Text bar and the Select bar
 * (`0084` criterion 14). */
export interface Row2State {
  open: boolean;
  /** A press of the expand button; `byKeyboard` keeps the focus on it (criteria 11, 15). */
  onToggle: (byKeyboard: boolean) => void;
}

/** The expand button at the end of row 1 (`0084` criteria 10, 11, 13): a double chevron, no
 * popup; while row 2 is closed and holds a changed value, a 6px accent dot at its top right. */
export function ExpandButton({
  row2,
  controls,
  bar,
  onReturnFocus,
}: {
  row2: Row2State;
  /** The id of row 2's container. */
  controls: string;
  bar: TextBarState;
  onReturnFocus: () => void;
}) {
  const descriptionId = useId();
  const label = expandLabel(row2.open);
  const changed = row2.open ? null : changedText(changedRow2Controls(bar));
  return (
    <>
      <Tooltip
        side="bottom"
        content={
          <span>
            <span className="block">{label}</span>
            {changed ? <span className="block">{changed}</span> : null}
          </span>
        }
      >
        <button
          type="button"
          aria-label={label}
          aria-expanded={row2.open}
          aria-controls={controls}
          aria-describedby={changed ? descriptionId : undefined}
          data-text-row2-toggle
          onClick={(event) => {
            const byKeyboard = event.detail === 0;
            row2.onToggle(byKeyboard);
            if (!byKeyboard) {
              onReturnFocus();
            }
          }}
          className={`${ICON_BUTTON_CLASS} relative text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]`}
        >
          {row2.open ? (
            <ChevronsUp size={16} strokeWidth={1.5} aria-hidden />
          ) : (
            <ChevronsDown size={16} strokeWidth={1.5} aria-hidden />
          )}
          {changed ? (
            <span
              aria-hidden
              data-text-row2-dot
              className="absolute top-0.5 right-0.5 size-1.5 rounded-full bg-[var(--editor-accent)] ring-1 ring-[var(--toolbar-bg)]"
            />
          ) : null}
        </button>
      </Tooltip>
      {changed ? (
        <span id={descriptionId} hidden>
          {changed}
        </span>
      ) : null}
    </>
  );
}

/** Row 2's container (`0084` criteria 7, 12, 16; UX notes §2): the full inner width of the pill
 * and never wider (`w-0 min-w-full`), so opening it re-wraps no row 1 line; a 1px separator at its
 * top inside the first line's padding; it wraps by whole groups. */
export function TextRow2({ id, role, children }: { id: string; role: "toolbar" | "group"; children: ReactNode }) {
  const rowRef = useRef<HTMLDivElement>(null);
  useRowStarts(rowRef);
  return (
    <div
      id={id}
      ref={rowRef}
      role={role}
      aria-label="More text options"
      className={`${BAR_ROW_CLASS} relative w-0 min-w-full before:pointer-events-none before:absolute before:inset-x-0 before:top-0 before:h-px before:bg-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)]`}
    >
      {children}
    </div>
  );
}

/** One bar group: a divider 13px to its left unless it starts a line (`Divider`). */
export function BarGroup({ divider = true, className = "flex h-9 items-center", children }: {
  divider?: boolean;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={`group/row relative ${className}`}>
      {divider ? <Divider /> : null}
      {children}
    </div>
  );
}

interface TextBarProps extends TextControlsProps {
  /** Edit mode: Bold and Italic name their keys. */
  editing: boolean;
  onPressWeight: (code: WeightCode) => string | null;
  onPressItalic: () => string | null;
  onSetAlign: (align: AlignCode) => void;
  onSetWritingMode: (mode: WritingModeCode) => void;
  onSetOrientation: (orientation: OrientationCode) => void;
  glyphs: GlyphsButtonState;
  row2: Row2State;
}

/** The Text tool's bar in the overlay row: one pill, row 1 (Font and Size, Alignment, Style,
 * expand) and, while open, row 2 (Writing mode, Glyph orientation, Glyphs) under it
 * (`0084` criteria 6, 7). Each row wraps by whole groups; a wrapped line starts at the pill's
 * padding. */
export function TextBar({
  editing,
  onPressWeight,
  onPressItalic,
  onSetAlign,
  onSetWritingMode,
  onSetOrientation,
  glyphs,
  row2,
  ...props
}: TextBarProps) {
  const rowRef = useRef<HTMLDivElement>(null);
  useRowStarts(rowRef);
  const row2Id = useId();
  return (
    <div
      className="pointer-events-auto flex min-w-0 flex-col rounded-lg px-2"
      style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
    >
      <div ref={rowRef} role="toolbar" aria-label="Text" className={BAR_ROW_CLASS}>
        <BarGroup divider={false}>
          <TextControls {...props} />
        </BarGroup>
        <BarGroup>
          <AlignmentGroup align={props.bar.align} onPress={onSetAlign} onReturnFocus={props.onReturnFocus} />
        </BarGroup>
        <BarGroup>
          <StyleGroup
            style={props.bar.style}
            editing={editing}
            onPressWeight={onPressWeight}
            onPressItalic={onPressItalic}
            onReturnFocus={props.onReturnFocus}
          />
        </BarGroup>
        <BarGroup>
          <ExpandButton row2={row2} controls={row2Id} bar={props.bar} onReturnFocus={props.onReturnFocus} />
        </BarGroup>
      </div>
      {row2.open ? (
        <TextRow2 id={row2Id} role="toolbar">
          <TextRow2Groups
            bar={props.bar}
            glyphs={glyphs}
            onSetWritingMode={onSetWritingMode}
            onSetOrientation={onSetOrientation}
            onReturnFocus={props.onReturnFocus}
          />
        </TextRow2>
      ) : null}
    </div>
  );
}

/** Row 2's groups in their order (`0084` criterion 7): Writing mode, Glyph orientation, Glyphs
 * (UX notes §0.1, Proposal); Taper and the On path options follow once they are built. */
export function TextRow2Groups({
  bar,
  glyphs,
  onSetWritingMode,
  onSetOrientation,
  onReturnFocus,
}: {
  bar: TextBarState;
  glyphs?: GlyphsButtonState;
  onSetWritingMode: (mode: WritingModeCode) => void;
  onSetOrientation: (orientation: OrientationCode) => void;
  onReturnFocus: () => void;
}) {
  return (
    <>
      <BarGroup divider={false}>
        <WritingModeGroup mode={bar.writingMode} onPress={onSetWritingMode} onReturnFocus={onReturnFocus} />
      </BarGroup>
      <BarGroup>
        <OrientationGroup
          mode={bar.writingMode}
          orientation={bar.orientation}
          onPress={onSetOrientation}
          onReturnFocus={onReturnFocus}
        />
      </BarGroup>
      {glyphs?.shown ? (
        <BarGroup>
          <GlyphsButton glyphs={glyphs} />
        </BarGroup>
      ) : null}
    </>
  );
}

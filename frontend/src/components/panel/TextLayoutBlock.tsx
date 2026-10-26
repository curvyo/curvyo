// The text layout block at the top of the Fonts tab (`specs/0079-text-layout-controls` criteria 6,
// 6a, 7, 10, 12 to 14, 16, 26, 27b, 27c; UX notes §3 to §6, §10.1, §10.3): a "Text layout" header
// that folds the block (closed by default, remembered per user, a dot while folded with a changed
// value; customer 2026-10-25), then Letter, Word and Line spacing, 8 px, Character rotation and
// Taper, 8 px, Horizontal and Vertical kerning and Per-character rotation, each with its "All
// characters" button, 12 px below. It is in the tree only while a text is selected or edited (the
// customer's exception to "Hidden, not disabled", design system).
import { ChevronDown, ChevronRight, WholeWord } from "lucide-react";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";

import { ValueField } from "@/components/panel/ValueField";
import { Tooltip } from "@/components/ui/tooltip";
import type { TextLayoutApi } from "@/hooks/useTextLayout";
import {
  APPLY_TO_ALL_NAME,
  LAYOUT_FIELDS,
  LAYOUT_FIELD_LABELS,
  type LayoutFieldCode,
  type LayoutFieldState,
  appliesToAll,
  foldChangedText,
  layoutDefaultText,
  layoutResetLabel,
  layoutTipValue,
  layoutUnit,
  layoutUnitWords,
  layoutValueText,
  readLayoutOpen,
  writeLayoutOpen,
} from "@/lib/textLayoutText";

const MESSAGES = { number: "Enter a number" } as const;

/** The 28 x 28 icon button beside a per-character field (the Text bar's icon button look). */
const ALL_BUTTON_CLASS =
  "flex size-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] aria-disabled:opacity-40 aria-disabled:hover:bg-transparent";

interface TextLayoutBlockProps {
  layout: TextLayoutApi;
  fields: LayoutFieldState[];
  /** Focus back to the canvas, or to the edit sink in edit mode. */
  onReturnFocus: () => void;
}

export function TextLayoutBlock({ layout, fields, onReturnFocus }: TextLayoutBlockProps) {
  const block = useRef<HTMLDivElement>(null);
  const bodyId = useId();
  const descriptionId = useId();
  const [open, setOpen] = useState(readLayoutOpen);
  const changed = foldChangedText(open, fields);
  const returnFocus = useRef(onReturnFocus);
  useEffect(() => {
    returnFocus.current = onReturnFocus;
  }, [onReturnFocus]);

  // Focus in the block when it goes (Ctrl+Z removed the text, a canvas click deselected) goes
  // to the canvas, never to the body (UX notes §5).
  useLayoutEffect(() => {
    const element = block.current;
    return () => {
      if (element?.contains(document.activeElement)) {
        window.requestAnimationFrame(() => returnFocus.current());
      }
    };
  }, []);

  const field = (code: LayoutFieldCode, index: number) => {
    const state = fields[index];
    return (
      <div key={code} className="relative flex items-center gap-1">
        <ValueField
          label={LAYOUT_FIELD_LABELS[code]}
          name={LAYOUT_FIELD_LABELS[code]}
          text={state.text}
          bar={state.barTo}
          barFrom={state.barFrom}
          zero={state.zero}
          dragBase={state.dragBase}
          dragMin={state.dragMin}
          dragLimit={state.dragLimit}
          mixed={state.mixed}
          muted={state.auto}
          resettable={state.resettable}
          unit={layoutUnit(code)}
          valueNow={state.value}
          unitWords={layoutUnitWords(code)}
          valueMin={state.trackMin}
          valueMax={state.trackMax}
          valueText={layoutValueText(code, state)}
          tipValue={layoutTipValue(code, state)}
          defaultText={layoutDefaultText(code)}
          resetLabel={layoutResetLabel(code, layout.editing)}
          narrow={appliesToAll(code)}
          messages={MESSAGES}
          onPreview={(p, grid) => layout.preview(code, p, grid)}
          onSettle={layout.settle}
          onStep={(steps, grid) => layout.step(code, steps, grid)}
          onReset={() => layout.reset(code)}
          onSubmit={(text) => layout.submit(code, text)}
          onReturnFocus={onReturnFocus}
        />
        {appliesToAll(code) ? (
          <Tooltip side="left" content={APPLY_TO_ALL_NAME}>
            <button
              type="button"
              aria-label={APPLY_TO_ALL_NAME}
              aria-disabled={state.mixed || undefined}
              data-apply-to-all={code}
              onClick={(event) => {
                if (!state.mixed) {
                  layout.applyToAll(code);
                }
                // `detail` is 0 for keyboard activation, 1 or more for a click.
                if (event.detail > 0) {
                  onReturnFocus();
                }
              }}
              className={ALL_BUTTON_CLASS}
            >
              <WholeWord size={16} strokeWidth={1.5} aria-hidden />
            </button>
          </Tooltip>
        ) : null}
        {layout.notice?.field === code ? (
          <div
            role="status"
            className="absolute top-full left-0 z-30 mt-1 w-max max-w-[480px] rounded-lg bg-[var(--toolbar-bg)] p-2 text-xs text-[var(--toolbar-icon)] shadow-[var(--panel-elevation-shadow)]"
          >
            {layout.notice.text}
          </div>
        ) : null}
      </div>
    );
  };

  return (
    <div ref={block} data-text-layout-block="" className="mb-3 flex shrink-0 flex-col gap-2">
      <button
        type="button"
        aria-expanded={open}
        aria-controls={bodyId}
        aria-describedby={changed ? descriptionId : undefined}
        data-text-layout-toggle=""
        onClick={(event) => {
          const next = !open;
          setOpen(next);
          writeLayoutOpen(next);
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
        className="flex h-7 w-full items-center gap-1 rounded-[5px] px-1 text-left text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        {open ? <ChevronDown size={12} aria-hidden /> : <ChevronRight size={12} aria-hidden />}
        <span>Text layout</span>
        {changed ? (
          <span
            aria-hidden
            data-text-layout-dot=""
            className="ml-1 size-1.5 rounded-full bg-[var(--editor-accent)]"
          />
        ) : null}
      </button>
      {changed ? (
        <span id={descriptionId} hidden>
          {changed}
        </span>
      ) : null}
      <div id={bodyId} hidden={!open} className="flex flex-col gap-2">
        {open ? (
          <>
            <div className="flex flex-col gap-1">
              {LAYOUT_FIELDS.slice(0, 3).map((code, i) => field(code, i))}
            </div>
            <div className="flex flex-col gap-1">
              {LAYOUT_FIELDS.slice(3, 5).map((code, i) => field(code, i + 3))}
            </div>
            <div className="flex flex-col gap-1">
              {LAYOUT_FIELDS.slice(5, 8).map((code, i) => field(code, i + 5))}
            </div>
          </>
        ) : null}
      </div>
    </div>
  );
}

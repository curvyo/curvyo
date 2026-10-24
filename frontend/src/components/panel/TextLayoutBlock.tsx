// The text layout block at the top of the Fonts tab (`specs/0079-text-layout-controls` criteria 6,
// 6a, 7, 10, 12 to 14; UX notes §3 to §6): Letter, Word and Line spacing, 8 px, Character
// rotation, 12 px below. It is in the tree only while a text is selected or edited (the
// customer's exception to "Hidden, not disabled", design system); no title, no fold.
import { useEffect, useLayoutEffect, useRef } from "react";

import { ValueField } from "@/components/panel/ValueField";
import type { TextLayoutApi } from "@/hooks/useTextLayout";
import {
  LAYOUT_FIELDS,
  LAYOUT_FIELD_LABELS,
  type LayoutFieldCode,
  type LayoutFieldState,
  layoutDefaultText,
  layoutTipValue,
  layoutUnit,
  layoutUnitWords,
  layoutValueText,
} from "@/lib/textLayoutText";

const MESSAGES = { number: "Enter a number" } as const;

interface TextLayoutBlockProps {
  layout: TextLayoutApi;
  fields: LayoutFieldState[];
  /** Focus back to the canvas, or to the edit sink in edit mode. */
  onReturnFocus: () => void;
}

export function TextLayoutBlock({ layout, fields, onReturnFocus }: TextLayoutBlockProps) {
  const block = useRef<HTMLDivElement>(null);
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
      <div key={code} className="relative">
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
          messages={MESSAGES}
          onPreview={(p, grid) => layout.preview(code, p, grid)}
          onSettle={layout.settle}
          onStep={(steps, grid) => layout.step(code, steps, grid)}
          onReset={() => layout.reset(code)}
          onSubmit={(text) => layout.submit(code, text)}
          onReturnFocus={onReturnFocus}
        />
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
      <div className="flex flex-col gap-1">{LAYOUT_FIELDS.slice(0, 3).map((code, i) => field(code, i))}</div>
      {field(LAYOUT_FIELDS[3], 3)}
    </div>
  );
}

// The Style group of the Text bar and the Select bar (`specs/0078-font-style-buttons` criteria
// 1 to 7, 16 and 21; UX notes §1 to §7): five weight buttons as one radio group with roving
// focus (arrows move focus, Enter or Space presses), and the Italic toggle 12 px after it.
// Greyed buttons keep their Tab stop and tooltip; the session decides every state.
import { Italic } from "lucide-react";
import { Tooltip } from "radix-ui";
import { useEffect, useRef, useState } from "react";

import { WeightGlyph } from "@/components/StyleGlyphs";
import {
  type StyleButtonState,
  type StyleState,
  WEIGHT_CODES,
  WEIGHT_NAMES,
  type WeightCode,
  styleTooltip,
  weightFocusMove,
  weightTabStop,
} from "@/lib/styleText";

/** How long a notice stays under the pressed button (Action notice, status). */
const NOTICE_MS = 3000;

const MAC = /mac/i.test(navigator.platform);

const TOOLTIP_STYLE = {
  background: "var(--toolbar-bg)",
  color: "var(--toolbar-icon)",
  boxShadow: "var(--panel-elevation-shadow)",
} as const;

const BUTTON_CLASS =
  "flex size-7 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

/** The look of a button's state (UX notes §3); no transition. */
function stateClass(state: StyleButtonState): string {
  if (state.pressed === "on") {
    return `bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] ${state.greyed ? "opacity-40" : ""}`;
  }
  if (state.greyed) {
    return "cursor-default opacity-40";
  }
  if (state.pressed === "mixed") {
    return "bg-[color-mix(in_srgb,var(--toolbar-icon-active-bg)_35%,transparent)] hover:bg-[var(--editor-accent-hover)]";
  }
  return "hover:bg-[var(--editor-accent-hover)]";
}

function StyleTooltip({ name, line, children }: { name: string; line: string | null; children: React.ReactNode }) {
  return (
    <Tooltip.Root>
      <Tooltip.Trigger asChild>{children}</Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content
          side="bottom"
          sideOffset={6}
          className="z-50 max-w-[300px] rounded-md px-2 py-1 text-xs"
          style={TOOLTIP_STYLE}
        >
          <div className="font-semibold">{name}</div>
          {line ? <div style={{ color: "var(--panel-muted-fg)" }}>{line}</div> : null}
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}

interface StyleGroupProps {
  style: StyleState;
  /** Edit mode: Bold and Italic name their keys. */
  editing: boolean;
  /** A press; returns the notice to show under the button, if any. */
  onPressWeight: (code: WeightCode) => string | null;
  onPressItalic: () => string | null;
  /** Focus back to the canvas, or to the edit sink in edit mode. */
  onReturnFocus: () => void;
}

export function StyleGroup({ style, editing, onPressWeight, onPressItalic, onReturnFocus }: StyleGroupProps) {
  const [focused, setFocused] = useState<number | null>(null);
  const [notice, setNotice] = useState<{ text: string; at: WeightCode | "italic" } | null>(null);
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const tabStop = focused ?? weightTabStop(style);

  useEffect(() => {
    if (!notice) {
      return undefined;
    }
    const timer = window.setTimeout(() => setNotice(null), NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notice]);

  /** A press acts on pointer up (`click`); a greyed one does nothing and keeps its tooltip. A
   * mouse press hands focus back so typing goes on; Enter or Space keeps it here. */
  const press = (state: StyleButtonState, at: WeightCode | "italic", act: () => string | null) =>
    (event: React.MouseEvent) => {
      if (state.greyed) {
        event.preventDefault();
        return;
      }
      const text = act();
      setNotice(text ? { text, at } : null);
      if (event.detail > 0) {
        onReturnFocus();
      }
    };

  const keepTooltip = (state: StyleButtonState) => (event: React.PointerEvent) => {
    if (state.greyed) {
      event.preventDefault();
    }
  };

  const noticeUnder = (at: WeightCode | "italic") =>
    notice?.at === at ? (
      <div
        role="status"
        className="absolute top-full left-0 z-30 mt-1 w-max max-w-[480px] rounded-lg bg-[var(--toolbar-bg)] p-2 text-xs text-[var(--toolbar-icon)] shadow-[var(--panel-elevation-shadow)]"
      >
        {notice.text}
      </div>
    ) : null;

  const italicTip = styleTooltip("italic", style.italic, editing, MAC);
  return (
    <div role="group" aria-label="Style" className="flex items-center gap-3">
      <div
        role="radiogroup"
        aria-label="Weight"
        className="flex items-center gap-1"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            onReturnFocus();
            return;
          }
          const next = weightFocusMove(event.key, focused ?? tabStop);
          if (next !== null) {
            event.preventDefault();
            setFocused(next);
            buttons.current[next]?.focus();
          }
        }}
        onBlur={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
            setFocused(null);
          }
        }}
      >
        {WEIGHT_CODES.map((code, index) => {
          const state = style.weights[index];
          const tip = styleTooltip(code, state, editing, MAC);
          return (
            <span key={code} className="relative">
              <StyleTooltip name={tip.name} line={tip.line}>
                <button
                  ref={(element) => {
                    buttons.current[index] = element;
                  }}
                  type="button"
                  role="radio"
                  aria-label={WEIGHT_NAMES[code]}
                  aria-checked={state.pressed === "on"}
                  aria-disabled={state.greyed ? true : undefined}
                  tabIndex={index === tabStop ? 0 : -1}
                  onFocus={() => setFocused(index)}
                  onPointerDown={keepTooltip(state)}
                  onClick={press(state, code, () => onPressWeight(code))}
                  className={`${BUTTON_CLASS} ${stateClass(state)}`}
                >
                  <WeightGlyph weight={code} />
                </button>
              </StyleTooltip>
              {noticeUnder(code)}
            </span>
          );
        })}
      </div>
      <span className="relative">
        <StyleTooltip name={italicTip.name} line={italicTip.line}>
          <button
            type="button"
            aria-label="Italic"
            aria-pressed={style.italic.pressed === "mixed" ? "mixed" : style.italic.pressed === "on"}
            aria-disabled={style.italic.greyed ? true : undefined}
            onPointerDown={keepTooltip(style.italic)}
            onClick={press(style.italic, "italic", onPressItalic)}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                event.stopPropagation();
                onReturnFocus();
              }
            }}
            className={`${BUTTON_CLASS} ${stateClass(style.italic)}`}
          >
            <Italic size={16} strokeWidth={1.5} aria-hidden />
          </button>
        </StyleTooltip>
        {noticeUnder("italic")}
      </span>
    </div>
  );
}

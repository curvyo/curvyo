// The Writing mode and Glyph orientation groups of the Text bar and the Select bar
// (`specs/0079-text-layout-controls` criteria 20, 21; UX notes §10.2): two radio groups of three
// buttons with the Alignment group's look and keys; the orientation buttons are greyed (and act
// on nothing) while the text is horizontal. Custom glyphs on Lucide's 24 grid. The session
// decides the pressed state; a press acts on pointer up.
import { Tooltip } from "radix-ui";
import { type ReactNode, useRef, useState } from "react";

import {
  ORIENTATION_CODES,
  ORIENTATION_NAMES,
  type OrientationCode,
  WRITING_MODE_CODES,
  WRITING_MODE_NAMES,
  type WritingModeCode,
  choiceFocusMove,
  orientationGreyed,
  writingTooltipLine,
} from "@/lib/textWritingText";

const TOOLTIP_STYLE = {
  background: "var(--toolbar-bg)",
  color: "var(--toolbar-icon)",
  boxShadow: "var(--panel-elevation-shadow)",
} as const;

const BUTTON_CLASS =
  "flex size-7 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

/** A 16px glyph on a 24 grid, 1.5 stroke, round caps (the Lucide look). */
function Glyph({ d }: { d: string }) {
  return (
    <svg
      aria-hidden
      width={16}
      height={16}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d={d} />
    </svg>
  );
}

// A cap "A" upright, and turned a quarter clockwise (apex to the right).
const A_UP_TOP = "M6 10.5 10 2.5l4 8M7.5 7.5h5";
const A_UP_LOW = "M6 21.5l4-8 4 8M7.5 18.5h5";
const A_SIDE_TOP = "M3.5 3 11.5 7l-8 4M6.5 4.5v5";
const A_SIDE_LOW = "M3.5 13l8 4-8 4M6.5 14.5v5";
const DOWN_ARROW = "M19 3v18M16 18l3 3 3-3";

const WRITING_GLYPHS: Record<WritingModeCode, string> = {
  horizontal: "M4 6h16M17 3l3 3-3 3M4 12h16M4 18h10",
  "vertical-rl": "M18 4v16M15 17l3 3 3-3M12 4v16M6 4v10",
  "vertical-lr": "M6 4v16M3 17l3 3 3-3M12 4v16M18 4v10",
};

const ORIENTATION_GLYPHS: Record<OrientationCode, string> = {
  auto: `${A_SIDE_TOP}M4 13.5h8v8H4zM4 17.5h8M8 13.5v8${DOWN_ARROW}`,
  upright: `${A_UP_TOP}${A_UP_LOW}${DOWN_ARROW}`,
  sideways: `${A_SIDE_TOP}${A_SIDE_LOW}${DOWN_ARROW}`,
};

interface ChoiceGroupProps<T extends string> {
  label: string;
  codes: readonly T[];
  names: Record<T, string>;
  glyphs: Record<T, string>;
  pressed: T | null;
  greyed: boolean;
  onPress: (code: T) => void;
  onReturnFocus: () => void;
}

function ChoiceGroup<T extends string>(props: ChoiceGroupProps<T>): ReactNode {
  const { codes, pressed, greyed, onPress, onReturnFocus } = props;
  const [focused, setFocused] = useState<number | null>(null);
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const tabStop = focused ?? (pressed ? codes.indexOf(pressed) : 0);
  const line = writingTooltipLine(pressed, greyed);
  return (
    <div
      role="radiogroup"
      aria-label={props.label}
      className="flex items-center gap-1"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          onReturnFocus();
          return;
        }
        const next = choiceFocusMove(event.key, focused ?? tabStop, codes.length);
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
      {codes.map((code, index) => {
        const on = pressed === code;
        return (
          <Tooltip.Root key={code}>
            <Tooltip.Trigger asChild>
              <button
                ref={(element) => {
                  buttons.current[index] = element;
                }}
                type="button"
                role="radio"
                aria-label={props.names[code]}
                aria-checked={on}
                aria-disabled={greyed ? true : undefined}
                tabIndex={index === tabStop ? 0 : -1}
                onFocus={() => setFocused(index)}
                onClick={(event) => {
                  if (!on && !greyed) {
                    onPress(code);
                  }
                  // A mouse press hands focus back; Enter or Space keeps it here.
                  if (event.detail > 0) {
                    onReturnFocus();
                  }
                }}
                className={`${BUTTON_CLASS} ${greyed ? "cursor-default opacity-40" : ""} ${
                  on
                    ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
                    : greyed
                      ? ""
                      : "hover:bg-[var(--editor-accent-hover)]"
                }`}
              >
                <Glyph d={props.glyphs[code]} />
              </button>
            </Tooltip.Trigger>
            <Tooltip.Portal>
              <Tooltip.Content
                side="bottom"
                sideOffset={6}
                className="z-50 max-w-[300px] rounded-md px-2 py-1 text-xs"
                style={TOOLTIP_STYLE}
              >
                <div className="font-semibold">{props.names[code]}</div>
                {line ? <div style={{ color: "var(--panel-muted-fg)" }}>{line}</div> : null}
              </Tooltip.Content>
            </Tooltip.Portal>
          </Tooltip.Root>
        );
      })}
    </div>
  );
}

/** The Writing mode group. */
export function WritingModeGroup(props: {
  mode: WritingModeCode | null;
  onPress: (mode: WritingModeCode) => void;
  onReturnFocus: () => void;
}) {
  return (
    <ChoiceGroup
      label="Writing mode"
      codes={WRITING_MODE_CODES}
      names={WRITING_MODE_NAMES}
      glyphs={WRITING_GLYPHS}
      pressed={props.mode}
      greyed={false}
      onPress={props.onPress}
      onReturnFocus={props.onReturnFocus}
    />
  );
}

/** The Glyph orientation group, greyed while the text is horizontal (criterion 21). */
export function OrientationGroup(props: {
  mode: WritingModeCode | null;
  orientation: OrientationCode | null;
  onPress: (orientation: OrientationCode) => void;
  onReturnFocus: () => void;
}) {
  return (
    <ChoiceGroup
      label="Glyph orientation"
      codes={ORIENTATION_CODES}
      names={ORIENTATION_NAMES}
      glyphs={ORIENTATION_GLYPHS}
      pressed={props.orientation}
      greyed={orientationGreyed(props.mode)}
      onPress={props.onPress}
      onReturnFocus={props.onReturnFocus}
    />
  );
}

// The preview pane's mode switch (`specs/0082-glyph-table` criterion 1, UX notes §1): a radio
// strip Fonts | Glyphs with one Tab stop on the pressed cell; Left and Right move and choose
// (no wrap), Home and End; a mouse press hands the focus back, a key keeps it on the strip.
import { useRef } from "react";

import type { PreviewMode } from "@/lib/fontPreviewState";

const MODES: { mode: PreviewMode; label: string }[] = [
  { mode: "fonts", label: "Fonts" },
  { mode: "glyphs", label: "Glyphs" },
];

interface ModeSwitchProps {
  mode: PreviewMode;
  onMode: (mode: PreviewMode) => void;
  onReturnFocus: () => void;
}

export function ModeSwitch({ mode, onMode, onReturnFocus }: ModeSwitchProps) {
  const cells = useRef<(HTMLButtonElement | null)[]>([]);
  const choose = (index: number) => {
    const next = MODES[Math.min(MODES.length - 1, Math.max(0, index))];
    onMode(next.mode);
    cells.current[MODES.indexOf(next)]?.focus();
  };
  const at = MODES.findIndex((m) => m.mode === mode);
  return (
    <div
      role="radiogroup"
      aria-label="Preview mode"
      className="flex h-7 shrink-0 gap-px overflow-hidden rounded-[5px] ring-1 ring-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]"
      onKeyDown={(event) => {
        const to: Record<string, number> = { ArrowLeft: at - 1, ArrowRight: at + 1, Home: 0, End: MODES.length - 1 };
        if (event.key in to) {
          event.preventDefault();
          choose(to[event.key]);
        }
      }}
    >
      {MODES.map((entry, index) => {
        const pressed = entry.mode === mode;
        return (
          <button
            key={entry.mode}
            ref={(el) => {
              cells.current[index] = el;
            }}
            type="button"
            role="radio"
            aria-checked={pressed}
            tabIndex={pressed ? 0 : -1}
            onClick={(event) => {
              onMode(entry.mode);
              if (event.detail !== 0) {
                onReturnFocus();
              }
            }}
            className={`min-w-8 px-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
              pressed
                ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
                : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
            }`}
          >
            {entry.label}
          </button>
        );
      })}
    </div>
  );
}

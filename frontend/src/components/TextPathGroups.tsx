// The On path button of row 1 and Release from path in row 2 (`specs/0080-text-on-path`
// criteria 1, 2, 23; UX notes §1, §5), with the custom 16px glyphs (1.5 stroke) the entry box of
// the position handle shares. The session decides every state; a press acts on pointer up.
import type { ReactNode } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { type TextPathBar, onPathEnabled, onPathTooltipLine } from "@/lib/textPathText";

const ICON_BUTTON_CLASS =
  "flex size-7 shrink-0 items-center justify-center rounded-md outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

/** A 16px glyph on a 16 grid (UX notes §1, row "Command glyphs"). */
export function Glyph16({ children }: { children: ReactNode }) {
  return (
    <svg
      aria-hidden
      width={16}
      height={16}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {children}
    </svg>
  );
}

/** An arc with a "T" standing on its apex: text on a curve (UX notes §1). */
export function OnPathGlyph() {
  return (
    <Glyph16>
      <path d="M2 13.5 Q8 4.5 14 13.5" />
      <path d="M8 9V3.5M5.5 3.5h5" />
    </Glyph16>
  );
}

/** Mirror: a line crossed by a vertical arrow with two heads (UX notes §5). */
export function MirrorGlyph() {
  return (
    <Glyph16>
      <path d="M2 8h12M8 2.5v11M6 4.5l2-2 2 2M6 11.5l2 2 2-2" />
    </Glyph16>
  );
}

/** What the On path controls do; each returns nothing, the bar re-reads the session after. */
export interface TextPathActions {
  bar: TextPathBar;
  onPress: () => void;
  onRelease: () => void;
}

/** The On path button of row 1 (UX notes §1): pressed only while pick mode runs; disabled in
 * place with two or more texts, its tooltip naming the state. */
export function OnPathButton({ path, onReturnFocus }: { path: TextPathActions; onReturnFocus: () => void }) {
  const enabled = onPathEnabled(path.bar);
  const line = onPathTooltipLine(path.bar);
  return (
    <Tooltip
      side="bottom"
      content={
        <span>
          <span className="block">On path</span>
          {line ? <span className="block text-[var(--panel-muted-fg)]">{line}</span> : null}
        </span>
      }
    >
      <button
        type="button"
        aria-label="On path"
        aria-pressed={path.bar.picking}
        aria-disabled={enabled ? undefined : true}
        data-text-on-path
        onClick={(event) => {
          if (enabled) {
            path.onPress();
          }
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
        className={`${ICON_BUTTON_CLASS} ${
          path.bar.picking
            ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
            : enabled
              ? "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
              : "cursor-default text-[var(--toolbar-icon)] opacity-40"
        }`}
      >
        <OnPathGlyph />
      </button>
    </Tooltip>
  );
}

/** A 28 x 28 command button of the On path groups. */
function CommandIcon({
  label,
  onPress,
  onReturnFocus,
  children,
}: {
  label: string;
  onPress: () => void;
  onReturnFocus: () => void;
  children: ReactNode;
}) {
  return (
    <Tooltip side="bottom" content={label}>
      <button
        type="button"
        aria-label={label}
        onClick={(event) => {
          onPress();
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
        className={`${ICON_BUTTON_CLASS} text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]`}
      >
        {children}
      </button>
    </Tooltip>
  );
}

/** Release from path. */
export function ReleaseButton({ path, onReturnFocus }: { path: TextPathActions; onReturnFocus: () => void }) {
  return (
    <CommandIcon label="Release from path" onPress={path.onRelease} onReturnFocus={onReturnFocus}>
      <Glyph16>
        <path d="M2 14 Q8 9 14 14" strokeDasharray="1.5 1.5" />
        <path d="M2 7h12M8 7V2.5M5.5 2.5h5" />
      </Glyph16>
    </CommandIcon>
  );
}

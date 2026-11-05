import { createElement } from "react";

import { glyphOf } from "@/workbenches/iconNames";

/** What a page's kind chip shows (`specs/0098-page-kinds` criterion 6), from the session. */
export interface KindInfo {
  /** The Lucide name of the glyph: `PenTool`, `DraftingCompass`, `Zap`, `CircleHelp`. */
  icon: string;
  /** Drawing, Sketch, an output kind's word from its manifest (with " (off)" when its extension is
   * off), or Unknown kind. */
  word: string;
  /** The extension of an output page is switched off: the glyph is muted, the word says it. */
  off?: boolean;
}

/**
 * The kind chip (UX notes §1): a 12 px Lucide glyph at 1.5 px absolute stroke, 3 px gap, then the
 * kind word. Static text: no hover, no focus stop, no press. Shape and word, never colour alone.
 */
export function KindChip({
  kind,
  size = 11,
  hidden = false,
  className = "",
}: {
  kind: KindInfo;
  /** The word's size in px (11 in the page row, 12 elsewhere). */
  size?: 11 | 12;
  /** `aria-hidden` where the row already reads "<name>, <kind> page". */
  hidden?: boolean;
  className?: string;
}) {
  return (
    <span
      aria-hidden={hidden || undefined}
      data-kind-chip=""
      className={`inline-flex shrink-0 items-center gap-[3px] whitespace-nowrap ${className}`}
      style={{ fontSize: size }}
    >
      {createElement(glyphOf(kind.icon), {
        size: 12,
        strokeWidth: 1.5,
        absoluteStrokeWidth: true,
        "aria-hidden": true,
        style: { color: kind.off ? "var(--panel-muted-fg)" : "currentColor" },
      })}
      <span>{kind.word}</span>
    </span>
  );
}

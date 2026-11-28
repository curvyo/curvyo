import { CircleAlert, TriangleAlert } from "lucide-react";
import type { CSSProperties, ReactNode, Ref } from "react";

import { SMALL_GLYPH } from "@/components/ui/looks";

interface ActionNoticeProps {
  /** A refusal: the validation chip look and `role="alert"`; otherwise a status on the canvas
   * chip surface. */
  refusal?: boolean;
  /** Where it hangs (`absolute top-full left-0 mt-1`, z-index): the caller's. */
  className?: string;
  style?: CSSProperties;
  /** The widest it may grow, px or a CSS length (360 by default; 244 in the panel's column). */
  maxWidth?: number | string;
  id?: string;
  noticeRef?: Ref<HTMLDivElement>;
  /** `false` inside a live region the caller keeps mounted (the rail's notice): no own role. */
  announce?: boolean;
  children: ReactNode;
}

/**
 * The action notice (`docs/ui-elements.md` §16): 4 px under the control that caused it, text
 * only, no pointer events, never focused. A status sits on the canvas chip surface (12 px, 8 px
 * radius, shadow); a refusal takes the validation chip look with its `CircleAlert`.
 */
export function ActionNotice({
  refusal = false,
  className = "",
  style,
  maxWidth = 360,
  id,
  noticeRef,
  announce = true,
  children,
}: ActionNoticeProps) {
  const look = refusal
    ? "flex items-start gap-1 bg-popover text-[var(--field-invalid)] ring-1 ring-[var(--field-invalid)]"
    : "bg-[var(--toolbar-bg)] text-[var(--toolbar-icon)] shadow-[var(--panel-elevation-shadow)]";
  return (
    <div
      ref={noticeRef}
      id={id}
      role={announce ? (refusal ? "alert" : "status") : undefined}
      className={`pointer-events-none w-max break-words rounded-[8px] px-2 py-1 text-xs ${look} ${className}`}
      style={{ maxWidth, ...style }}
    >
      {refusal ? <CircleAlert {...SMALL_GLYPH} aria-hidden className="mt-0.5 shrink-0" /> : null}
      {refusal ? <span className="min-w-0">{children}</span> : children}
    </div>
  );
}

/** The place of the alert line's 12 px glyph on its first text line. */
export const ALERT_GLYPH_CLASS = "mt-0.5 shrink-0";

/**
 * The alert line (`docs/ui-elements.md` §16): under the row it belongs to, in the panel's flow,
 * 12 px `--field-invalid` with `TriangleAlert`, `role="alert"`. A standing list of findings (the
 * Output checks) uses the same line with `announce={false}`, its own glyph (`OctagonX` for a
 * block) and, for a warning, the ink `tone`.
 */
export function AlertLine({
  id,
  className = "",
  icon,
  tone = "alert",
  announce = true,
  children,
}: {
  id?: string;
  className?: string;
  /** The glyph, 12 px with `ALERT_GLYPH_CLASS`; `TriangleAlert` by default. */
  icon?: ReactNode;
  /** `"warn"`: the ink colour, for a warning that blocks nothing. */
  tone?: "alert" | "warn";
  /** `false` in a list that stays on screen: no `role="alert"`. */
  announce?: boolean;
  children: ReactNode;
}) {
  const ink = tone === "alert" ? "text-[var(--field-invalid)]" : "text-[var(--toolbar-icon)]";
  return (
    <p id={id} role={announce ? "alert" : undefined} className={`flex items-start gap-1 text-xs ${ink} ${className}`}>
      {icon ?? <TriangleAlert {...SMALL_GLYPH} aria-hidden className={ALERT_GLYPH_CLASS} />}
      <span className="min-w-0">{children}</span>
    </p>
  );
}

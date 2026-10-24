// The notices of the Fonts tab (`specs/0075-font-library` UX notes §4,
// `specs/0081-inline-delete-confirm` UX notes §7): the library file's under the strip, Add font's
// under its button, Remove's under its family row.
import { CircleAlert } from "lucide-react";

import type { FontNotice } from "@/hooks/useFontLibrary";

interface TabNoticeProps {
  notice: FontNotice | null;
  anchor: FontNotice["anchor"];
  /** The family row key, for the "row" anchor. */
  row?: string;
  /** The row's height: the notice's top is 4 px below the row's bottom. */
  rowHeight?: number;
}

/** A notice 4 px under its anchor: the strip's left-aligned, Add font's and a row's
 * right-aligned. */
export function TabNotice({ notice, anchor, row, rowHeight }: TabNoticeProps) {
  if (!notice || notice.anchor !== anchor || notice.row !== row) {
    return null;
  }
  const side = anchor === "strip" ? "left-0" : "right-0";
  const style = rowHeight === undefined ? undefined : { top: rowHeight };
  return notice.kind === "refusal" ? (
    <div
      role="alert"
      style={style}
      className={`pointer-events-none absolute top-full ${side} z-30 mt-1 flex w-max max-w-[244px] items-start gap-1 rounded-md bg-popover px-2 py-1 text-xs text-[var(--field-invalid)] ring-1 ring-[var(--field-invalid)]`}
    >
      <CircleAlert size={12} aria-hidden className="mt-0.5 shrink-0" />
      {notice.text}
    </div>
  ) : (
    <p
      role="status"
      className={`pointer-events-none absolute top-full ${side} z-30 mt-1 w-max max-w-[244px] rounded-md bg-[var(--toolbar-bg)] px-2 py-1 text-xs text-[var(--toolbar-icon)]`}
      style={{ ...style, boxShadow: "var(--panel-elevation-shadow)" }}
    >
      {notice.text}
    </p>
  );
}

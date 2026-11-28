// The notices of the Fonts tab (`specs/0075-font-library` UX notes §4,
// `specs/0081-inline-delete-confirm` UX notes §7): the library file's under the strip, Add font's
// under its button, Remove's under its family row.

import type { FontNotice } from "@/hooks/useFontLibrary";
import { ActionNotice } from "@/components/ui/notice";

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
    <ActionNotice
      refusal
      maxWidth={244}
      style={style}
      className={`absolute top-full ${side} z-30 mt-1`}
    >
      {notice.text}
    </ActionNotice>
  ) : (
    <ActionNotice maxWidth={244} style={style} className={`absolute top-full ${side} z-30 mt-1`}>
      {notice.text}
    </ActionNotice>
  );
}

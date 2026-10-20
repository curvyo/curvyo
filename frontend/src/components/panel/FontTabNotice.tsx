// The notices of the Fonts tab (`specs/0075-font-library` UX notes §4): the library file's under
// the strip, Add font's under its button.
import { CircleAlert } from "lucide-react";

import type { FontNotice } from "@/hooks/useFontLibrary";

/** A notice 4 px under its anchor: the strip's left-aligned, Add font's right-aligned. */
export function TabNotice({ notice, anchor }: { notice: FontNotice | null; anchor: FontNotice["anchor"] }) {
  if (!notice || notice.anchor !== anchor) {
    return null;
  }
  const side = anchor === "add" ? "right-0" : "left-0";
  return notice.kind === "refusal" ? (
    <div
      role="alert"
      className={`pointer-events-none absolute top-full ${side} z-30 mt-1 flex w-max max-w-[244px] items-start gap-1 rounded-md bg-popover px-2 py-1 text-xs text-[var(--field-invalid)] ring-1 ring-[var(--field-invalid)]`}
    >
      <CircleAlert size={12} aria-hidden className="mt-0.5 shrink-0" />
      {notice.text}
    </div>
  ) : (
    <p
      role="status"
      className={`pointer-events-none absolute top-full ${side} z-30 mt-1 w-max max-w-[244px] rounded-md bg-[var(--toolbar-bg)] px-2 py-1 text-xs text-[var(--toolbar-icon)]`}
      style={{ boxShadow: "var(--panel-elevation-shadow)" }}
    >
      {notice.text}
    </p>
  );
}

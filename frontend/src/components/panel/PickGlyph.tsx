import { ArrowDownToLine, Check, ChevronsRight } from "lucide-react";

import type { PickKind } from "@/lib/picks";

/**
 * The glyph of a pick (`specs/0146` `ux-notes.md` 2.2): a check for Best, a smaller check with a
 * "2" for Second best, the speed chevrons for Fastest, the down arrow to a line for Deepest. One
 * glyph per kind everywhere, so the symbol is the word in a small space.
 */
export function PickGlyph({ kind, size = 16, className }: { kind: PickKind; size?: number; className?: string }) {
  const stroke = size >= 12 ? 1.5 : 1;
  switch (kind) {
    case "best":
      return <Check size={size} strokeWidth={stroke} aria-hidden className={className} />;
    case "second_best":
      return (
        <svg
          width={size}
          height={size}
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth={stroke * (24 / size)}
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden
          className={className}
        >
          <path d="M3 15 7.5 19.5 16 10.5" />
          <path d="M14.5 6.2a2.6 2.6 0 0 1 5 .8c0 1.9-2.2 3-5 5.8h5.4" />
        </svg>
      );
    case "fastest":
      return <ChevronsRight size={size} strokeWidth={stroke} aria-hidden className={className} />;
    case "deepest":
      return <ArrowDownToLine size={size} strokeWidth={stroke} aria-hidden className={className} />;
  }
}

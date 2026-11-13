import { ChevronRight, CircleDashed, Flame, Image, MapPin } from "lucide-react";

import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import { testRowName } from "@/lib/materialRows";
import type { TestRow as Row } from "@/lib/materialsView";

/** The state symbol of line 2: Lasered, a grid on a page, or neither. */
function StateGlyph({ row }: { row: Row }) {
  if (row.lasered) return <Flame size={12} aria-hidden className="shrink-0" />;
  const onPage = !row.line.endsWith("not on a page");
  return (
    <>
      <CircleDashed size={12} aria-hidden className="shrink-0" />
      {onPage ? <MapPin size={12} aria-hidden className="shrink-0" /> : null}
    </>
  );
}

/**
 * One test of a material (`specs/0112` `ux-notes.md` 8.1): the kind glyph, the id in mono, a
 * photo mark and a chevron on line 1; the state and the laser or page on line 2. A press or Enter
 * opens the test.
 */
export function TestRowView({ row, onOpen }: { row: Row; onOpen: () => void }) {
  return (
    <li>
      <button
        type="button"
        data-test-id={row.id}
        aria-label={testRowName(row)}
        onClick={onOpen}
        className="flex h-10 w-[228px] flex-col justify-center rounded px-1 text-left text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-12"
      >
        <span className="flex h-[22px] items-center gap-1">
          <OperationGlyph kind={row.kind} />
          <span className="flex-1 font-mono text-[13px] font-semibold tracking-[0.08em]">{row.id}</span>
          {row.photo ? <Image size={14} aria-hidden className="shrink-0 text-[var(--panel-muted-fg)]" /> : null}
          <ChevronRight size={16} aria-hidden className="shrink-0" />
        </span>
        <span className="flex h-[14px] items-center gap-1 truncate text-[11px] text-[var(--panel-muted-fg)]">
          <StateGlyph row={row} />
          <span className="truncate">{row.line}</span>
        </span>
      </button>
    </li>
  );
}

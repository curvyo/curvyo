import { ChevronRight } from "lucide-react";
import { Fragment } from "react";

import { collapsedName, crumbRow } from "@/lib/groupText";

interface OpenGroupsProps {
  /** How many groups deep the Select tool is; 0 at the document root, where nothing is drawn. */
  depth: number;
  /** Makes `level` the context: 0 leaves every level, `n` keeps the first `n` groups. */
  onLeaveToLevel: (level: number) => void;
}

const CRUMB_CLASS =
  "flex h-7 items-center rounded-md px-3 outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/**
 * The indicator "Open groups" (`docs/design-system.md`, row "Entered group"): the persistent line
 * of the canvas notice slot while a group is entered. "Document", then one crumb per entered
 * group; every crumb but the last is a text button that makes that level the context, the last is
 * text. A muted "Esc: leave" closes the line. Absent at the root.
 */
export function OpenGroups({ depth, onLeaveToLevel }: OpenGroupsProps) {
  if (depth === 0) {
    return null;
  }
  const { crumbs, collapsed, collapsedAt } = crumbRow(depth);
  return (
    <div
      className="pointer-events-auto flex h-7 max-w-[480px] items-center gap-2 rounded-full pr-3 pl-1 text-xs"
      style={{
        background: "var(--toolbar-bg)",
        color: "var(--toolbar-icon)",
        boxShadow: "var(--panel-elevation-shadow)",
      }}
    >
      <nav aria-label="Open groups">
        <ol className="flex items-center">
          {crumbs.map((crumb, index) => (
            <Fragment key={crumb.level}>
              {collapsed > 0 && index === collapsedAt ? (
                <li className="flex items-center" aria-label={collapsedName(collapsed)}>
                  <span style={{ color: "var(--panel-muted-fg)" }} aria-hidden="true">
                    ...
                  </span>
                  <ChevronRight size={12} aria-hidden="true" />
                </li>
              ) : null}
              <li className="flex items-center">
                {crumb.current ? (
                  <span className="flex h-7 items-center px-3 font-semibold" aria-current="location">
                    {crumb.label}
                  </span>
                ) : (
                  <button type="button" className={CRUMB_CLASS} onClick={() => onLeaveToLevel(crumb.level)}>
                    {crumb.label}
                  </button>
                )}
                {index < crumbs.length - 1 ? <ChevronRight size={12} aria-hidden="true" /> : null}
              </li>
            </Fragment>
          ))}
        </ol>
      </nav>
      <span style={{ color: "var(--panel-muted-fg)" }}>Esc: leave</span>
    </div>
  );
}

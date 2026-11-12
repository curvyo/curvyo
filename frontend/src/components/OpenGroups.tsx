import { ChevronRight, Lock } from "lucide-react";
import { Fragment } from "react";

import { collapsedName, crumbName, crumbRow } from "@/lib/groupText";
import type { EnteredGroup } from "@/lib/operationsView";

interface OpenGroupsProps {
  /** How many groups deep the Select tool is; 0 at the document root, where nothing is drawn. */
  depth: number;
  /** Makes `level` the context: 0 leaves every level, `n` keeps the first `n` groups. */
  onLeaveToLevel: (level: number) => void;
  /** The read-only group of a laser page (`specs/0111` criterion 11): the root crumb is the page
   * name and a "Read-only" segment follows the groups. */
  laserGroup?: EnteredGroup | null;
}

const CRUMB_CLASS =
  "flex h-7 items-center rounded-md px-3 outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/**
 * The indicator "Open groups" (`docs/design-system.md`, row "Entered group"): the persistent line
 * of the canvas notice slot while a group is entered. "Document", then one crumb per entered
 * group; every crumb but the last is a text button that makes that level the context, the last is
 * text. A muted "Esc: leave" closes the line. Absent at the root.
 */
export function OpenGroups({ depth, onLeaveToLevel, laserGroup = null }: OpenGroupsProps) {
  if (depth === 0) {
    return null;
  }
  const { crumbs, collapsed, collapsedAt } = crumbRow(
    depth,
    laserGroup ? { root: laserGroup.page, groups: laserGroup.groups } : undefined,
  );
  return (
    <div
      className="pointer-events-auto flex h-7 max-w-[480px] items-center gap-2 rounded-full pr-3 pl-1 text-xs whitespace-nowrap"
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
                <li className="flex items-center">
                  <span
                    role="img"
                    aria-label={collapsedName(collapsed)}
                    style={{ color: "var(--panel-muted-fg)" }}
                  >
                    ...
                  </span>
                  <ChevronRight size={12} aria-hidden="true" />
                </li>
              ) : null}
              <li className="flex items-center">
                {crumb.current ? (
                  <span
                    className="flex h-7 items-center px-3 font-semibold"
                    aria-current="location"
                    aria-label={crumbName(crumb)}
                  >
                    {crumb.label}
                  </span>
                ) : (
                  <button
                    type="button"
                    className={CRUMB_CLASS}
                    aria-label={crumbName(crumb)}
                    onClick={() => onLeaveToLevel(crumb.level)}
                  >
                    {crumb.label}
                  </button>
                )}
                {index < crumbs.length - 1 ? <ChevronRight size={12} aria-hidden="true" /> : null}
              </li>
            </Fragment>
          ))}
        </ol>
      </nav>
      {laserGroup ? (
        <>
          <span aria-hidden style={{ color: "var(--panel-muted-fg)" }}>
            ·
          </span>
          <span className="flex items-center gap-1" style={{ color: "var(--panel-muted-fg)" }}>
            <Lock size={12} aria-hidden />
            Read-only
          </span>
          <span aria-hidden style={{ color: "var(--panel-muted-fg)" }}>
            ·
          </span>
        </>
      ) : null}
      <span style={{ color: "var(--panel-muted-fg)" }}>Esc: leave</span>
    </div>
  );
}

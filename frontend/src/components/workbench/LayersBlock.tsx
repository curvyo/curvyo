import { EyeOff, Lock, TriangleAlert } from "lucide-react";
import { type KeyboardEvent, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

import { ToolbarSwitch } from "@/components/ToolbarSwitch";
import { Tooltip } from "@/components/ui/tooltip";
import type { LayerRoleRow, LayersView, WorkbenchesApi } from "@/hooks/useWorkbenches";
import {
  ROLES,
  type RoleName,
  roleAfterPress,
  roleKey,
  roleRowHeight,
  roleRowWindow,
  roleTabStop,
} from "@/lib/roleRowKeys";

/** The gap between two rows, px (UX notes §3). */
const ROW_GAP_PX = 8;

const ROLE_LABELS: Record<RoleName, string> = { cut: "Cut", engrave: "Engrave", ignore: "Ignore" };

/** The 16 px glyph of a role: the shape of its look on the output page (UX notes §4). */
function RoleGlyph({ role }: { role: RoleName }) {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" aria-hidden className="shrink-0">
      {role === "engrave" ? (
        <rect x={2} y={2} width={12} height={12} fill="currentColor" />
      ) : (
        <rect
          x={2.75}
          y={2.75}
          width={10.5}
          height={10.5}
          fill="none"
          stroke="currentColor"
          strokeWidth={1.5}
          strokeDasharray={role === "ignore" ? "3 2" : undefined}
        />
      )}
    </svg>
  );
}

/** The legend's sample of a role look in its token colour (UX notes §2 item 3). */
function LegendSample({ role }: { role: RoleName }) {
  const colour = `var(--role-${role})`;
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" aria-hidden className="shrink-0">
      {role === "cut" ? <rect x={2.5} y={2.5} width={11} height={11} fill="none" stroke={colour} strokeWidth={1} /> : null}
      {role === "engrave" ? <rect x={2} y={2} width={12} height={12} fill={colour} /> : null}
      {role === "ignore" ? (
        <rect x={2.5} y={2.5} width={11} height={11} fill="none" stroke={colour} strokeWidth={1} strokeDasharray="4 3" />
      ) : null}
    </svg>
  );
}

/** The captions of a row, in order (UX notes §3). */
function captions(row: LayerRoleRow): string[] {
  const lines: string[] = [];
  if (row.hidden) {
    lines.push("Hidden in the drawing: not output");
  }
  if (row.unknown) {
    lines.push("Unknown role in file, treated as no role.");
  }
  return lines;
}

/** The nearest ancestor that scrolls, or null. */
function scrollParent(element: HTMLElement | null): HTMLElement | null {
  for (let node = element?.parentElement ?? null; node; node = node.parentElement) {
    const overflow = getComputedStyle(node).overflowY;
    if (overflow === "auto" || overflow === "scroll") {
      return node;
    }
  }
  return null;
}

const ROLE_BUTTON =
  "flex h-7 flex-1 items-center justify-center gap-1 rounded-md text-sm outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 [@media(pointer:coarse)]:h-9";

/**
 * The Layers block of the Laser tab (`specs/0027-manufacturing-roles` criteria 1, 2, 6, 11, 14,
 * 15, 18; UX notes §2 to §8): the Role view switch, the legend, the notice, then one row per layer
 * with its read-only hidden and locked glyphs and three role buttons. The role buttons are one Tab
 * stop with roving focus (`roleRowKeys`); more than 60 rows are windowed in the tab's scroll area.
 */
export function LayersBlock({
  label,
  layers,
  workbenches,
  onReturnFocus,
}: {
  label: string;
  layers: LayersView;
  workbenches: WorkbenchesApi;
  onReturnFocus: () => void;
}) {
  const listRef = useRef<HTMLUListElement>(null);
  const [last, setLast] = useState<{ key: string; column: number; index: number } | null>(null);
  // Whether a role button holds the focus, so a row that disappears under it hands it on.
  const hadFocus = useRef(false);
  const pendingFocus = useRef<{ key: string; column: number } | null>(null);
  const [scroll, setScroll] = useState({ top: 0, height: 800 });
  const coarse = useMemo(() => window.matchMedia?.("(pointer: coarse)").matches ?? false, []);
  const rows = layers.rows;
  const keys = rows.map((row) => row.key);
  // The last focused button; when its row is gone, the same button of the row now at its place.
  const current =
    last !== null && !keys.includes(last.key) && keys.length > 0
      ? { key: keys[Math.min(last.index, keys.length - 1)], column: last.column }
      : last;
  const tabStop = roleTabStop(
    keys,
    rows.map((row) => row.role),
    current,
  );
  const heights = rows.map((row) => roleRowHeight(captions(row).length, coarse) + ROW_GAP_PX);
  const focusedRow = current === null ? -1 : keys.indexOf(current.key);
  const range = roleRowWindow(heights, scroll.top, scroll.height, focusedRow);

  const measure = useCallback(() => {
    const list = listRef.current;
    const scroller = scrollParent(list);
    if (!list || !scroller) {
      return;
    }
    const offset = list.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop;
    setScroll((old) => {
      const next = { top: scroller.scrollTop - offset, height: scroller.clientHeight };
      return old.top === next.top && old.height === next.height ? old : next;
    });
  }, []);
  useLayoutEffect(() => {
    const scroller = scrollParent(listRef.current);
    if (!scroller || rows.length <= 60) {
      return;
    }
    measure();
    scroller.addEventListener("scroll", measure, { passive: true });
    window.addEventListener("resize", measure);
    return () => {
      scroller.removeEventListener("scroll", measure);
      window.removeEventListener("resize", measure);
    };
  }, [measure, rows.length]);

  const focusButton = (key: string, column: number) => {
    const button = listRef.current?.querySelector<HTMLElement>(`[data-role-button="${CSS.escape(`${key}/${column}`)}"]`);
    if (button) {
      button.focus();
      button.scrollIntoView?.({ block: "nearest" });
      return true;
    }
    return false;
  };
  useEffect(() => {
    const target = pendingFocus.current;
    if (target !== null && focusButton(target.key, target.column)) {
      pendingFocus.current = null;
    }
  });
  // Focus follows the layer: a focused row that is gone hands the focus to the row that took its
  // place, else the one above (UX notes §7); focus is never dropped to the document body.
  useEffect(() => {
    if (hadFocus.current && document.activeElement === document.body && current !== null) {
      focusButton(current.key, current.column);
    }
  });

  const press = (row: LayerRoleRow, role: RoleName | null) => {
    workbenches.setLayerRole(row.key, role);
  };
  const onKey = (event: KeyboardEvent<HTMLButtonElement>, index: number, column: number) => {
    const row = rows[index];
    if (!row || event.altKey || event.ctrlKey || event.metaKey) {
      return;
    }
    const action = roleKey(event.key, index, column, rows.length);
    if (action === null) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    if (action.kind === "focus") {
      const target = { key: keys[action.row], column: action.column };
      setLast({ ...target, index: action.row });
      if (!focusButton(target.key, target.column)) {
        pendingFocus.current = target;
        const top = heights.slice(0, action.row).reduce((sum, h) => sum + h, 0);
        const scroller = scrollParent(listRef.current);
        const list = listRef.current;
        if (scroller && list) {
          const offset = list.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop;
          scroller.scrollTop = offset + top - scroller.clientHeight / 2;
        }
      }
    } else if (action.kind === "press") {
      press(row, roleAfterPress(row.role, ROLES[column]));
    } else if (action.kind === "clear") {
      press(row, null);
    } else {
      onReturnFocus();
    }
  };

  if (rows.length === 0) {
    return (
      <section aria-label={label} className="flex flex-col gap-1">
        <h3 className="flex h-7 items-center text-sm text-[var(--toolbar-icon)]">{label}</h3>
        <p className="text-sm text-[var(--panel-muted-fg)]">The drawing has no layers yet.</p>
      </section>
    );
  }

  // `parked` keeps a focused row that is scrolled out of the window in the tree (UX notes §8).
  const renderRow = (index: number, parked = false) => {
    const row = rows[index];
    const lines = captions(row);
    const captionIds = lines.map((_, n) => `role-caption-${row.key}-${n}`);
    const groupName = row.outside ? "Role, objects outside layers" : `Role, layer ${row.name}`;
    return (
      <li
        key={row.key}
        role="listitem"
        aria-posinset={index + 1}
        aria-setsize={rows.length}
        className={`flex flex-col ${parked ? "h-0 overflow-hidden" : ""}`}
        style={{ paddingBottom: parked ? 0 : ROW_GAP_PX }}
      >
        {row.outside && index > 0 ? (
          <div
            aria-hidden
            className="mb-2 h-px"
            style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
          />
        ) : null}
        <div className="flex h-7 items-center gap-1">
          <Tooltip side="left" content={row.name}>
            <span
              className={`min-w-0 flex-1 truncate text-sm ${row.hidden ? "text-[var(--panel-muted-fg)]" : "text-[var(--toolbar-icon)]"}`}
            >
              {row.name}
            </span>
          </Tooltip>
          {row.locked ? (
            <Tooltip side="left" content="Locked in the drawing">
              <span role="img" aria-label="Locked in the drawing" className="text-[var(--toolbar-icon)]">
                <Lock size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
              </span>
            </Tooltip>
          ) : null}
          {row.hidden ? (
            <Tooltip side="left" content="Hidden in the drawing">
              <span role="img" aria-label="Hidden in the drawing" className="text-[var(--toolbar-icon)]">
                <EyeOff size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
              </span>
            </Tooltip>
          ) : null}
          {row.role === null ? <span className="shrink-0 text-xs text-[var(--panel-muted-fg)]">No role</span> : null}
        </div>
        <div
          role="group"
          aria-label={groupName}
          aria-describedby={captionIds.length > 0 ? captionIds.join(" ") : undefined}
          className="flex h-7 items-center gap-1 [@media(pointer:coarse)]:h-9"
        >
          {ROLES.map((role, column) => {
            const pressed = row.role === role;
            const isStop = tabStop !== null && tabStop.row === index && tabStop.column === column;
            return (
              <button
                key={role}
                type="button"
                data-role-button={`${row.key}/${column}`}
                aria-pressed={pressed}
                tabIndex={isStop ? 0 : -1}
                onFocus={() => {
                  hadFocus.current = true;
                  setLast({ key: row.key, column, index });
                }}
                onBlur={(event) => {
                  if (!listRef.current?.contains(event.relatedTarget as Node | null)) {
                    hadFocus.current = false;
                  }
                }}
                onKeyDown={(event) => onKey(event, index, column)}
                onClick={() => press(row, roleAfterPress(row.role, role))}
                className={`${ROLE_BUTTON} ${
                  pressed
                    ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] hover:bg-[var(--toolbar-icon-active-bg)]"
                    : "text-[var(--toolbar-icon)]"
                }`}
                style={pressed ? undefined : { border: "1px solid color-mix(in srgb, var(--toolbar-icon) 60%, transparent)" }}
              >
                <RoleGlyph role={role} />
                {ROLE_LABELS[role]}
              </button>
            );
          })}
        </div>
        {lines.map((line, n) => (
          <p key={line} id={captionIds[n]} className="mt-1 h-3 text-xs leading-3 text-[var(--panel-muted-fg)]">
            {line}
          </p>
        ))}
      </li>
    );
  };

  const shown: number[] = [];
  for (let index = range.first; index <= range.last; index += 1) {
    shown.push(index);
  }
  return (
    <section aria-label={label} className="flex flex-col gap-1">
      <h3 className="flex h-7 items-center text-sm text-[var(--toolbar-icon)]">{label}</h3>
      <div className="flex h-7 items-center justify-between">
        <ToolbarSwitch
          label="Role view"
          tooltip="Role view"
          checked={layers.role_view}
          onCheckedChange={(on) => workbenches.setRoleView(on)}
        />
      </div>
      {layers.role_view ? (
        <div aria-hidden className="flex h-4 items-center gap-3 text-xs text-[var(--toolbar-icon)]">
          {ROLES.map((role) => (
            <span key={role} className="flex items-center gap-1">
              <LegendSample role={role} />
              {ROLE_LABELS[role]}
            </span>
          ))}
        </div>
      ) : null}
      {layers.notices.length > 0 ? (
        <div
          role="status"
          className="mt-1 flex items-start gap-2 rounded-md p-2 text-xs text-[var(--toolbar-icon)]"
          style={{
            background: "var(--toolbar-bg)",
            border: "1px solid color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
          }}
        >
          <TriangleAlert size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="shrink-0" />
          <span className="flex flex-col">
            {layers.notices.map((line) => (
              <span key={line}>{line}</span>
            ))}
          </span>
        </div>
      ) : null}
      <ul ref={listRef} role="list" aria-label="Layer roles" className="mt-2 flex flex-col">
        {range.before > 0 ? <li role="none" aria-hidden style={{ height: range.before }} /> : null}
        {range.extra !== null && range.extra < range.first ? renderRow(range.extra, true) : null}
        {shown.map((index) => renderRow(index))}
        {range.extra !== null && range.extra > range.last ? renderRow(range.extra, true) : null}
        {range.after > 0 ? <li role="none" aria-hidden style={{ height: range.after }} /> : null}
      </ul>
    </section>
  );
}

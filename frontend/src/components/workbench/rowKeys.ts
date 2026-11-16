import type { KeyboardEvent } from "react";

/**
 * Left and Right on a focused row walk its line-1 controls (a check box, Lock, the eye, the armed
 * trash, the arrows): `selector` names them. Returns whether the key was taken. A key on a control
 * that is not one of them, or with a modifier, is not.
 */
export function walkRowControls(event: KeyboardEvent<HTMLElement>, selector: string): boolean {
  if ((event.key !== "ArrowLeft" && event.key !== "ArrowRight") || event.altKey || event.ctrlKey || event.metaKey) return false;
  const row = event.currentTarget;
  const stops = Array.from(row.querySelectorAll<HTMLElement>(selector)).filter((el) => el.closest("li") === row);
  const onRow = event.target === row;
  const at = onRow ? -1 : stops.indexOf(event.target as HTMLElement);
  if (stops.length === 0 || (!onRow && at < 0)) return false;
  event.preventDefault();
  const to = event.key === "ArrowRight" ? at + 1 : at - 1;
  if (to >= 0 && to < stops.length) stops[to]?.focus();
  else if (to < 0) row.focus();
  return true;
}

/** The classes of a 24px icon button of a row. */
export const ICON_BUTTON =
  "inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** The classes of the armed trash (`0081`). */
export const ARMED_TRASH =
  "flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[5px] bg-[var(--destructive)] text-white";

/** The classes of a 16px check box (the Active and On this page checks). */
export function checkBoxClass(on: boolean): string {
  return `flex h-4 w-4 shrink-0 items-center justify-center rounded-[3px] border-[1.5px] border-[var(--toolbar-icon)] text-white outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${on ? "bg-[var(--toolbar-icon)]" : "bg-white"}`;
}

/**
 * After the row `item` is removed, the focus goes to the row now at its place, else the one
 * before it, else `fallback` (the section's add control), never to the body. Call it before the
 * removal: it reads the row's place, and moves the focus once the list has re-rendered.
 */
export function focusAfterRemoval(item: HTMLElement | null, fallback: () => HTMLElement | null | undefined): void {
  const list = item?.parentElement ?? null;
  const index = list && item ? Array.from(list.children).indexOf(item) : -1;
  requestAnimationFrame(() => {
    const rows = list?.isConnected ? Array.from(list.children) : [];
    const next = (rows[index] ?? rows[index - 1]) as HTMLElement | undefined;
    (next ?? fallback())?.focus();
  });
}

/** The Tab stop of the add bar named `label` (a fallback of `focusAfterRemoval`). */
export function addBarStop(label: string): HTMLElement | null {
  return document.querySelector<HTMLElement>(`[role=toolbar][aria-label="${CSS.escape(label)}"] button[tabindex="0"]`);
}

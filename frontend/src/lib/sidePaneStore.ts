// The canvas area's one side pane (`specs/0152-library-browser` criterion 2, ADR 0086 decision 6):
// which kind is open. Opening one kind closes the open other by construction: each kind registers
// how it closes, and `claimPane` calls it. The kinds keep their own layout state.

/** The kinds of the one side pane. */
export type PaneKind = "font-preview" | "library-page" | "library-browser" | "before-after";

let open: PaneKind | null = null;
const closers = new Map<PaneKind, () => void>();
const listeners = new Set<() => void>();

function changed(): void {
  for (const listener of listeners) {
    listener();
  }
}

/** The open kind, or null. */
export function openPane(): PaneKind | null {
  return open;
}

/** Listens to changes of the open kind. */
export function subscribePane(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Registers how `kind` closes itself; the returned function unregisters it. */
export function registerPaneCloser(kind: PaneKind, close: () => void): () => void {
  closers.set(kind, close);
  return () => {
    if (closers.get(kind) === close) {
      closers.delete(kind);
    }
  };
}

/** `kind` opens: the other open kind closes first. */
export function claimPane(kind: PaneKind): void {
  if (open === kind) {
    return;
  }
  const before = open;
  open = kind;
  if (before !== null) {
    closers.get(before)?.();
  }
  changed();
}

/** `kind` closed; nothing happens when another kind is open. */
export function releasePane(kind: PaneKind): void {
  if (open === kind) {
    open = null;
    changed();
  }
}

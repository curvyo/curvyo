// The link of a shared workspace moves with the maker from one document session to the next
// (`specs/0094-workspace` criteria 30 and 31, ADR 0036 decision 8): one connection per workspace,
// whichever document is open. The editor's session swap moves it; the transport keeps its socket.

/** The parts of a session the move uses (`WasmSession`). */
export interface LinkHolder {
  has_workspace_link(): boolean;
  take_workspace_link(): unknown;
  attach_workspace_link(link: never): void;
}

let handOver: ((next: LinkHolder) => void) | null = null;

/** The transport's hand-over, called when a link moved; returns the unsubscribe. */
export function onWorkspaceLinkMoved(listener: (next: LinkHolder) => void): () => void {
  handOver = listener;
  return () => {
    if (handOver === listener) {
      handOver = null;
    }
  };
}

/** Moves the shared workspace's link from `from` to `to`; whether there was one to move. */
export function moveWorkspaceLink(from: LinkHolder | null, to: LinkHolder): boolean {
  if (!from || !from.has_workspace_link()) {
    return false;
  }
  const link = from.take_workspace_link();
  if (link === undefined || link === null) {
    return false;
  }
  to.attach_workspace_link(link as never);
  handOver?.(to);
  return true;
}

/** A session that can leave its room (`WasmSession`). */
export interface Leaving {
  has_workspace_link(): boolean;
  leave_sync(): void;
}

/** Ends the shared workspace's link of `session` (Close workspace, another workspace): its
 * socket closes and the link is not moved on. */
export function endWorkspaceLink(session: Leaving | null): void {
  if (session?.has_workspace_link()) {
    session.leave_sync();
  }
}

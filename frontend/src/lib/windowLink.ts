// The page side of the link between the two windows of one project (`specs/0158-pen-display`
// slice 2a, ADR 0096 A5): one frame's shuttle of bundles, pure over the session's calls, so a
// test drives it with fakes.

/** The session calls the shuttle uses. */
export interface LinkSession {
  link_frame(): Uint8Array | undefined;
  link_take_project(): Uint8Array | undefined;
  link_take_changed(): boolean;
  link_take_save(): boolean;
  link_take_notice(): string;
  link_state(): string;
}

/** The hub's fixed message that the pen window is gone (`window_bridge::PEER_GONE`). */
export function isPeerGone(bytes: Uint8Array): boolean {
  return bytes.length === 2 && bytes[0] === 1 && bytes[1] === 2;
}

/** What one frame of the link left for the page. */
export interface LinkFrame {
  /** A project that arrived (pen window): open it as a fresh session. */
  project: Uint8Array | null;
  /** Imports changed the document: refresh the panels. */
  changed: boolean;
  /** The pen window asked for a save (main window). */
  save: boolean;
  /** A notice for this window, empty for none. */
  notice: string;
  /** `none`, `waiting`, `held`, `out-of-sync` or `live`. */
  state: string;
}

/** Runs the link's frame on `session` and hands its bundle, if any, to `send`, in order. */
export function linkFrame(session: LinkSession, send: (bytes: Uint8Array) => void): LinkFrame {
  const out = session.link_frame();
  if (out) send(out);
  return {
    project: session.link_take_project() ?? null,
    changed: session.link_take_changed(),
    save: session.link_take_save(),
    notice: session.link_take_notice(),
    state: session.link_state(),
  };
}

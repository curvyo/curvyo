// The render loop's one wasm call per frame (`WasmSession::frame_state`, `specs/0126` phase 1):
// the positions of its numbers and flags, and the view text the loop compares between frames.

/** Changes with every change of the session. */
export const FRAME_GENERATION = 0;
/** 1 when the canvas lacks the session's present picture: `render()` draws it. */
export const FRAME_NEEDED = 1;
/** The view's origin x and y and its scale. */
export const FRAME_VIEW = 2;
/** The flags below. */
export const FRAME_FLAGS = 5;
/** The frames the session submitted to the GPU so far (the performance probe). */
export const FRAME_SUBMITTED = 6;

/** The session has a sync link: its frame start runs every frame. */
export const FRAME_SYNC_LINK = 1;
/** A width-editing notice or the unreadable-image notice waits to be taken. */
export const FRAME_NOTICES = 2 | 4;

/** The view as text, equal for equal views: origin x, origin y and scale. */
export function frameView(state: ArrayLike<number>): string {
  return `${state[FRAME_VIEW]},${state[FRAME_VIEW + 1]},${state[FRAME_VIEW + 2]}`;
}

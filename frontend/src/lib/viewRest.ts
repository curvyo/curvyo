/**
 * Whether the canvas view is moving or has rested (`specs/0025-raster-trace` criterion 56): the
 * render loop feeds it the view of every frame; a change makes it moving, 150 ms without one
 * makes it rest. The Trace tool draws its kept meshes while the view moves and tessellates them
 * again for the new zoom once it rests.
 */

/** The time without a change of the view after which it has rested. */
export const VIEW_REST_MS = 150;

/** The moving state of the view across frames. */
export class ViewRest {
  private view: string | null = null;
  private changedAt = 0;
  private moving = false;

  /** The frame's `view` (any text that changes with the view) at `now` ms: the new state when
   * it changed (`true` moving, `false` rested), else `null`. The first frame only remembers. */
  update(view: string, now: number): boolean | null {
    if (this.view === null) {
      this.view = view;
      return null;
    }
    if (view !== this.view) {
      this.view = view;
      this.changedAt = now;
      if (!this.moving) {
        this.moving = true;
        return true;
      }
      return null;
    }
    if (this.moving && now - this.changedAt >= VIEW_REST_MS) {
      this.moving = false;
      return false;
    }
    return null;
  }
}

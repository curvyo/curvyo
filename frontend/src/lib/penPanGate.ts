// Holds the pen sample feed back while the View card's Pan toggle is on (`specs/0158-pen-display`
// slice 2a): a pen drag then only pans, and Pan switched on mid-stroke cancels the stroke.
import type { NativeBatch } from "./penProbe.ts";
import type { CanvasRect, PenFlush, PenSampleFeed, PointerLike } from "../platform/penSamples.ts";

export class PenPanGate {
  private wasPan = false;

  readonly feed: PenSampleFeed;
  private readonly isPan: () => boolean;

  constructor(feed: PenSampleFeed, isPan: () => boolean) {
    this.feed = feed;
    this.isPan = isPan;
  }

  /** A canvas Pointer Event; dropped while Pan is on. */
  pointer(event: PointerLike, rect: CanvasRect): void {
    if (!this.isPan()) this.feed.pointer(event, rect);
  }

  /** A native batch; dropped while Pan is on. */
  native(batch: NativeBatch, rect: CanvasRect): void {
    if (!this.isPan()) this.feed.native(batch, rect);
  }

  /** The frame's batch. The frame Pan turns on forgets the feed's contact and calls `cancel` (the
   * session drops its stroke, as Escape does); while Pan is on nothing is returned. */
  take(cancel: () => void): PenFlush | null {
    if (this.isPan()) {
      if (!this.wasPan) {
        this.wasPan = true;
        this.feed.reset();
        cancel();
      }
      return null;
    }
    this.wasPan = false;
    return this.feed.take();
  }
}

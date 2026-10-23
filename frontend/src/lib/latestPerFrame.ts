// Coalesces a stream of values (the pointer moves of a drag) into at most one delivery per
// animation frame, the latest value winning (`specs/0077-font-preview` criterion 27).

/** The scheduler the coalescer uses: `requestAnimationFrame` and its cancel in the app. */
export interface FrameScheduler {
  request: (run: () => void) => number;
  cancel: (handle: number) => void;
}

const animationFrames: FrameScheduler = {
  request: (run) => window.requestAnimationFrame(run),
  cancel: (handle) => window.cancelAnimationFrame(handle),
};

/** Push values any number of times; `deliver` gets the latest once in the next frame. `flush`
 * delivers a pending value now (the drag ended), `cancel` drops it. */
export function latestPerFrame(deliver: (value: number) => void, frames: FrameScheduler = animationFrames) {
  let pending: number | undefined;
  let handle: number | null = null;
  const run = () => {
    handle = null;
    if (pending !== undefined) {
      const value = pending;
      pending = undefined;
      deliver(value);
    }
  };
  return {
    push(value: number) {
      pending = value;
      handle ??= frames.request(run);
    },
    flush() {
      if (handle !== null) {
        frames.cancel(handle);
      }
      run();
    },
    cancel() {
      if (handle !== null) {
        frames.cancel(handle);
        handle = null;
      }
      pending = undefined;
    },
  };
}

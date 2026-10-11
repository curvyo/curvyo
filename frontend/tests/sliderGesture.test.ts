// Run with `npm test`. The pointer gestures of the sliders (`src/lib/sliderGesture.ts`,
// `specs/0047-transform-polish/` criteria 17 to 19, 21, 24 and 27).

import assert from "node:assert/strict";
import { test } from "node:test";

import { DOUBLE_CLICK_MS, SliderGesture } from "../src/lib/sliderGesture.ts";

const thumb = { at: { x: 50, y: 12 }, radius: 8 };

test("a press and release inside the slop on the track is a click on the track", () => {
  const gesture = new SliderGesture();
  gesture.down({ x: 20, y: 12 }, false, thumb);
  assert.deepEqual(gesture.move({ x: 21, y: 12 }), { kind: "none" });
  assert.deepEqual(gesture.up({ x: 21, y: 12 }, 0), {
    kind: "click",
    onThumb: false,
  });
});

test("a click on the thumb is a click on the thumb: the value stays", () => {
  const gesture = new SliderGesture();
  gesture.down({ x: 56, y: 12 }, false, thumb);
  assert.deepEqual(gesture.up({ x: 56, y: 12 }, 0), {
    kind: "click",
    onThumb: true,
  });
  // Just outside the drawn extent is the track.
  gesture.down({ x: 59, y: 12 }, false, thumb);
  assert.deepEqual(gesture.up({ x: 59, y: 12 }, 1000), {
    kind: "click",
    onThumb: false,
  });
});

test("a drag starts at 3 px, or 8 px for a touch, and ends with a drag-end", () => {
  const mouse = new SliderGesture();
  mouse.down({ x: 20, y: 12 }, false, thumb);
  assert.deepEqual(mouse.move({ x: 22.9, y: 12 }), { kind: "none" });
  assert.deepEqual(mouse.move({ x: 23, y: 12 }), {
    kind: "drag-start",
    grab: null,
  });
  assert.deepEqual(mouse.move({ x: 40, y: 12 }), { kind: "drag" });
  assert.equal(mouse.dragging, true);
  assert.deepEqual(mouse.up({ x: 40, y: 12 }, 0), { kind: "drag-end" });
  assert.equal(mouse.dragging, false);

  const touch = new SliderGesture();
  touch.down({ x: 20, y: 12 }, true, thumb);
  assert.deepEqual(touch.move({ x: 27.5, y: 12 }), { kind: "none" });
  assert.deepEqual(touch.move({ x: 28, y: 12 }), {
    kind: "drag-start",
    grab: null,
  });
});

test("a drag from the thumb keeps the offset between the thumb and the pointer", () => {
  const gesture = new SliderGesture();
  gesture.down({ x: 55, y: 12 }, false, thumb);
  assert.deepEqual(gesture.move({ x: 60, y: 12 }), {
    kind: "drag-start",
    grab: { x: -5, y: 0 },
  });
});

test("a thumb that is not there (mixed) is never grabbed", () => {
  const gesture = new SliderGesture();
  gesture.down({ x: 50, y: 12 }, false, null);
  assert.deepEqual(gesture.up({ x: 50, y: 12 }, 0), {
    kind: "click",
    onThumb: false,
  });
});

test("two clicks within the interval and 4 px are a double-click, a third starts over", () => {
  const gesture = new SliderGesture();
  gesture.down({ x: 20, y: 12 }, false, thumb);
  assert.deepEqual(gesture.up({ x: 20, y: 12 }, 1000), {
    kind: "click",
    onThumb: false,
  });
  gesture.down({ x: 24, y: 12 }, false, thumb);
  assert.deepEqual(gesture.up({ x: 24, y: 12 }, 1000 + DOUBLE_CLICK_MS), {
    kind: "double-click",
  });
  gesture.down({ x: 24, y: 12 }, false, thumb);
  assert.deepEqual(gesture.up({ x: 24, y: 12 }, 1200 + DOUBLE_CLICK_MS), {
    kind: "click",
    onThumb: false,
  });
});

test("clicks too late, too far apart or separated by a drag are not a double-click", () => {
  const late = new SliderGesture();
  late.down({ x: 20, y: 12 }, false, thumb);
  late.up({ x: 20, y: 12 }, 0);
  late.down({ x: 20, y: 12 }, false, thumb);
  assert.equal(late.up({ x: 20, y: 12 }, DOUBLE_CLICK_MS + 1).kind, "click");

  const far = new SliderGesture();
  far.down({ x: 20, y: 12 }, false, thumb);
  far.up({ x: 20, y: 12 }, 0);
  far.down({ x: 25, y: 12 }, false, thumb);
  assert.equal(far.up({ x: 25, y: 12 }, 100).kind, "click");

  const dragged = new SliderGesture();
  dragged.down({ x: 20, y: 12 }, false, thumb);
  dragged.up({ x: 20, y: 12 }, 0);
  dragged.down({ x: 20, y: 12 }, false, thumb);
  dragged.move({ x: 40, y: 12 });
  assert.equal(dragged.up({ x: 40, y: 12 }, 100).kind, "drag-end");
  dragged.down({ x: 40, y: 12 }, false, thumb);
  assert.equal(dragged.up({ x: 40, y: 12 }, 150).kind, "click");
});

test("a double-tap allows 24 px between the taps", () => {
  const gesture = new SliderGesture();
  gesture.down({ x: 20, y: 12 }, true, thumb);
  gesture.up({ x: 20, y: 12 }, 0);
  gesture.down({ x: 44, y: 12 }, true, thumb);
  assert.equal(gesture.up({ x: 44, y: 12 }, 200).kind, "double-click");
});

test("a cancelled press is nothing", () => {
  const gesture = new SliderGesture();
  gesture.down({ x: 20, y: 12 }, false, thumb);
  gesture.cancel();
  assert.equal(gesture.up({ x: 20, y: 12 }, 0).kind, "none");
  assert.equal(gesture.move({ x: 90, y: 12 }).kind, "none");
});

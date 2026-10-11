// Run with `npm test`. Independent tester acceptance tests for the pure logic of Part B of
// `specs/0047-transform-polish/specification.md` (criteria 17 to 24, 27, 28, 31), written from the
// specification text and run against `src/lib/sliderGesture.ts` and `src/lib/sliderValue.ts`.
// The DOM behaviour (pointer capture, focus, the entry placement, touch-action in a real
// engine, hit targets on screen) cannot run here and is NOT VERIFIED; the last tests read the
// source files for the static facts the criteria name (strings, attributes) and say so.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import {
  CLICK_SLOP_PX,
  DOUBLE_CLICK_MS,
  DOUBLE_CLICK_SLOP_PX,
  DOUBLE_TAP_SLOP_PX,
  SliderGesture,
  TOUCH_SLOP_PX,
  type Thumb,
} from "../src/lib/sliderGesture.ts";
import {
  RATIO_MESSAGE,
  RATIO_RANGE,
  parseRatioEntry,
  positionOfValue,
  snapToStep,
  valueAtPosition,
} from "../src/lib/sliderValue.ts";

const thumb: Thumb = { at: { x: 100, y: 12 }, radius: 8 };
const track = { x: 20, y: 12 };

function click(g: SliderGesture, at: { x: number; y: number }, time: number, touch = false) {
  g.down(at, touch, thumb);
  return g.up(at, time);
}

// ---- criterion 17: click versus drag, thresholds ---------------------------------

test("17: a press and release without movement on the track is a click on the track", () => {
  const g = new SliderGesture();
  g.down(track, false, thumb);
  assert.deepEqual(g.up(track, 0), { kind: "click", onThumb: false });
});

test("17: movement below 3 px is still a click; 3 px or more is a drag (mouse)", () => {
  assert.equal(CLICK_SLOP_PX, 3);
  const a = new SliderGesture();
  a.down(track, false, thumb);
  assert.deepEqual(a.move({ x: track.x + 2.99, y: track.y }), { kind: "none" });
  assert.deepEqual(a.up({ x: track.x + 2.99, y: track.y }, 0), { kind: "click", onThumb: false });
  const b = new SliderGesture();
  b.down(track, false, thumb);
  const first = b.move({ x: track.x + 3, y: track.y });
  assert.equal(first.kind, "drag-start");
  assert.equal(b.dragging, true);
  assert.deepEqual(b.up({ x: track.x + 3, y: track.y }, 0), { kind: "drag-end" });
});

test("17: the movement is measured as a distance, not per axis", () => {
  const g = new SliderGesture();
  g.down(track, false, thumb);
  // 2.2 + 2.2 diagonal is 3.11 px: a drag.
  assert.equal(g.move({ x: track.x + 2.2, y: track.y + 2.2 }).kind, "drag-start");
});

test("17: a touch uses 8 px", () => {
  assert.equal(TOUCH_SLOP_PX, 8);
  const a = new SliderGesture();
  a.down(track, true, thumb);
  assert.deepEqual(a.move({ x: track.x + 7.9, y: track.y }), { kind: "none" });
  assert.equal(a.up({ x: track.x + 7.9, y: track.y }, 0).kind, "click");
  const b = new SliderGesture();
  b.down(track, true, thumb);
  assert.equal(b.move({ x: track.x + 8, y: track.y }).kind, "drag-start");
});

test("17/19: a drag never ends in a click, and the next click is a first click again", () => {
  const g = new SliderGesture();
  g.down(track, false, thumb);
  g.move({ x: track.x + 30, y: track.y });
  assert.equal(g.up({ x: track.x + 30, y: track.y }, 10).kind, "drag-end");
  assert.deepEqual(click(g, { x: 30, y: 12 }, 20), { kind: "click", onThumb: false });
});

test("17: a move without a press, a release without a press and a cancel do nothing", () => {
  const g = new SliderGesture();
  assert.deepEqual(g.move({ x: 1, y: 1 }), { kind: "none" });
  assert.deepEqual(g.up({ x: 1, y: 1 }, 0), { kind: "none" });
  g.down(track, false, thumb);
  g.cancel();
  assert.deepEqual(g.up(track, 0), { kind: "none" });
  assert.equal(g.dragging, false);
});

// ---- criterion 18: thumb ---------------------------------------------------------

test("18: a click within the thumb's drawn extent is a click on the thumb; just outside is not", () => {
  const g = new SliderGesture();
  assert.deepEqual(click(g, { x: 107.9, y: 12 }, 0), { kind: "click", onThumb: true });
  const h = new SliderGesture();
  assert.deepEqual(click(h, { x: 108.5, y: 12 }, 0), { kind: "click", onThumb: false });
});

test("18: a drag from the thumb keeps the grab offset; from the track there is none", () => {
  const g = new SliderGesture();
  g.down({ x: 104, y: 13 }, false, thumb);
  const ev = g.move({ x: 130, y: 13 });
  assert.deepEqual(ev, { kind: "drag-start", grab: { x: -4, y: -1 } });
  const t = new SliderGesture();
  t.down({ x: 40, y: 12 }, false, thumb);
  assert.deepEqual(t.move({ x: 60, y: 12 }), { kind: "drag-start", grab: null });
});

// ---- criterion 20: mixed (no thumb) ----------------------------------------------

test("20: with no thumb (mixed) every click is a track click", () => {
  const g = new SliderGesture();
  g.down({ x: 100, y: 12 }, false, null);
  assert.deepEqual(g.up({ x: 100, y: 12 }, 0), { kind: "click", onThumb: false });
  g.down({ x: 100, y: 12 }, false, null);
  assert.deepEqual(g.move({ x: 140, y: 12 }), { kind: "drag-start", grab: null });
});

// ---- criterion 21, 24: double-click ------------------------------------------------

test("21/24: two clicks within the interval and 4 px are click then double-click", () => {
  assert.equal(DOUBLE_CLICK_SLOP_PX, 4);
  const g = new SliderGesture();
  assert.equal(click(g, { x: 40, y: 12 }, 1000).kind, "click");
  assert.deepEqual(click(g, { x: 44, y: 12 }, 1000 + DOUBLE_CLICK_MS), { kind: "double-click" });
});

test("21: 4 px apart is a double-click, more than 4 px is two clicks", () => {
  const a = new SliderGesture();
  click(a, { x: 40, y: 12 }, 0);
  assert.equal(click(a, { x: 44, y: 12 }, 100).kind, "double-click");
  const b = new SliderGesture();
  click(b, { x: 40, y: 12 }, 0);
  assert.equal(click(b, { x: 44.5, y: 12 }, 100).kind, "click");
  // The distance is Euclidean.
  const c = new SliderGesture();
  click(c, { x: 40, y: 12 }, 0);
  assert.equal(click(c, { x: 43, y: 15 }, 100).kind, "click");
});

test("21: too late is two clicks; the interval is the platform's (a fixed half second here)", () => {
  const g = new SliderGesture();
  click(g, { x: 40, y: 12 }, 0);
  assert.equal(click(g, { x: 40, y: 12 }, DOUBLE_CLICK_MS + 1).kind, "click");
});

test("21: a third click of a triple click does not reopen the entry", () => {
  const g = new SliderGesture();
  assert.equal(click(g, { x: 40, y: 12 }, 0).kind, "click");
  assert.equal(click(g, { x: 40, y: 12 }, 100).kind, "double-click");
  assert.equal(click(g, { x: 40, y: 12 }, 200).kind, "click");
  assert.equal(click(g, { x: 40, y: 12 }, 300).kind, "double-click");
});

test("21: a double-click works on the thumb as well as on the track", () => {
  const g = new SliderGesture();
  assert.deepEqual(click(g, { x: 100, y: 12 }, 0), { kind: "click", onThumb: true });
  assert.deepEqual(click(g, { x: 100, y: 12 }, 100), { kind: "double-click" });
});

test("21: a drag between the two clicks breaks the pair", () => {
  const g = new SliderGesture();
  click(g, { x: 40, y: 12 }, 0);
  g.down({ x: 40, y: 12 }, false, thumb);
  g.move({ x: 90, y: 12 });
  g.up({ x: 90, y: 12 }, 50);
  assert.equal(click(g, { x: 40, y: 12 }, 100).kind, "click");
});

test("21: a cancelled press between the clicks does not make the next one a double-click on its own", () => {
  const g = new SliderGesture();
  click(g, { x: 40, y: 12 }, 0);
  g.down({ x: 40, y: 12 }, false, thumb);
  g.cancel();
  // The next full click still pairs with the first one only if it is within the interval:
  // this is a click that follows a click, which is a double-click by the rule of the spec.
  assert.equal(click(g, { x: 41, y: 12 }, 100).kind, "double-click");
});

test("27: a double-tap is two taps within the interval and 24 px", () => {
  assert.equal(DOUBLE_TAP_SLOP_PX, 24);
  const a = new SliderGesture();
  click(a, { x: 40, y: 12 }, 0, true);
  assert.equal(click(a, { x: 64, y: 12 }, 100, true).kind, "double-click");
  const b = new SliderGesture();
  click(b, { x: 40, y: 12 }, 0, true);
  assert.equal(click(b, { x: 65, y: 12 }, 100, true).kind, "click");
});

test("24: the second press moves nothing: only `down`/`move` can start a value change, and the second click is reported as double-click without a position change event", () => {
  const g = new SliderGesture();
  click(g, { x: 40, y: 12 }, 0);
  g.down({ x: 43, y: 12 }, false, thumb);
  // Within the slop: no drag-start, no value change at the press.
  assert.deepEqual(g.move({ x: 44, y: 12 }), { kind: "none" });
  assert.deepEqual(g.up({ x: 44, y: 12 }, 80), { kind: "double-click" });
});

test("hostile: NaN, infinity and huge coordinates and time stamps never throw", () => {
  const g = new SliderGesture();
  for (const v of [Number.NaN, Infinity, -Infinity, 1e308, -1e308]) {
    g.down({ x: v, y: v }, false, thumb);
    g.move({ x: v, y: 0 });
    g.up({ x: 0, y: v }, v);
    g.cancel();
  }
  assert.ok(true);
});

test("hostile: a time stamp that runs backwards is not a double-click", () => {
  const g = new SliderGesture();
  click(g, { x: 40, y: 12 }, 5000);
  // A second click 'before' the first one: the clock was reset or the event came from
  // another frame. It cannot be 'within the interval' of a click that has not happened yet.
  assert.equal(click(g, { x: 40, y: 12 }, 100).kind, "click");
});

test("hostile: NaN pointer movement is not a drag that moves the thumb to NaN", () => {
  const g = new SliderGesture();
  g.down(track, false, thumb);
  const ev = g.move({ x: Number.NaN, y: 0 });
  // Whatever it reports, a NaN position must not be treated as a legitimate drag start
  // (the slider would set its value from a NaN position).
  assert.notEqual(ev.kind, "drag-start");
});

// ---- criterion 17: values at positions (S1, S2) ----------------------------------

const W = 100;

test("17: S1 clicked at the middle gives 0.50, at the left end 0.01, at the right end 0.99", () => {
  assert.equal(valueAtPosition(W / 2, W, RATIO_RANGE), 0.5);
  assert.equal(valueAtPosition(0, W, RATIO_RANGE), 0.01);
  assert.equal(valueAtPosition(W, W, RATIO_RANGE), 0.99);
});

test("17: positions outside the track clamp to the ends; the value is on the 0.01 grid", () => {
  assert.equal(valueAtPosition(-50, W, RATIO_RANGE), 0.01);
  assert.equal(valueAtPosition(500, W, RATIO_RANGE), 0.99);
  for (let x = 0; x <= W; x += 0.37) {
    const v = valueAtPosition(x, W, RATIO_RANGE);
    assert.ok(v >= 0.01 && v <= 0.99);
    assert.ok(Math.abs(Math.round(v * 100) - v * 100) < 1e-9, `${v} is not on the grid`);
  }
});

test("17: the value is monotone in the position", () => {
  let last = 0;
  for (let x = 0; x <= W; x += 0.1) {
    const v = valueAtPosition(x, W, RATIO_RANGE);
    assert.ok(v >= last);
    last = v;
  }
});

test("17: positionOfValue is the inverse within a step", () => {
  for (let v = 0.01; v <= 0.99; v += 0.01) {
    const x = positionOfValue(v, W, RATIO_RANGE);
    assert.equal(valueAtPosition(x, W, RATIO_RANGE), Number(v.toFixed(2)));
  }
});

test("hostile: a zero or negative track width gives the minimum, not NaN", () => {
  assert.equal(valueAtPosition(5, 0, RATIO_RANGE), 0.01);
  assert.equal(valueAtPosition(5, -10, RATIO_RANGE), 0.01);
});

test("hostile: a NaN or infinite pointer position gives a finite value in range", () => {
  for (const x of [Number.NaN, Infinity, -Infinity]) {
    const v = valueAtPosition(x, W, RATIO_RANGE);
    assert.ok(Number.isFinite(v) && v >= 0.01 && v <= 0.99, `x=${x} gave ${v}`);
  }
  assert.ok(Number.isFinite(snapToStep(Number.NaN, RATIO_RANGE)));
});

// ---- criterion 22: the typed entry ------------------------------------------------

test("22: the message", () => {
  assert.equal(RATIO_MESSAGE, "Enter a number from 0.01 to 0.99");
});

test("22: valid entries are rounded to two decimals; the spec's examples", () => {
  assert.deepEqual(parseRatioEntry("0.456"), { ok: true, value: 0.46 });
  assert.deepEqual(parseRatioEntry("0.5"), { ok: true, value: 0.5 });
  assert.deepEqual(parseRatioEntry("0.01"), { ok: true, value: 0.01 });
  assert.deepEqual(parseRatioEntry("0.99"), { ok: true, value: 0.99 });
  assert.deepEqual(parseRatioEntry("0.985"), { ok: true, value: 0.99 });
  assert.deepEqual(parseRatioEntry("0.0149"), { ok: true, value: 0.01 });
});

test("22: a decimal point or a comma is accepted (EntryField rule)", () => {
  assert.deepEqual(parseRatioEntry("0,5"), { ok: true, value: 0.5 });
  assert.deepEqual(parseRatioEntry(" 0.5 "), { ok: true, value: 0.5 });
  assert.deepEqual(parseRatioEntry(".5"), { ok: true, value: 0.5 });
});

test("22: 0.995 is refused, checked as typed before the rounding", () => {
  assert.deepEqual(parseRatioEntry("0.995"), { ok: false });
  assert.deepEqual(parseRatioEntry("0.9900001"), { ok: false });
  assert.deepEqual(parseRatioEntry("0.0099"), { ok: false });
  assert.deepEqual(parseRatioEntry("0.009999"), { ok: false });
});

test("22: anything else is refused", () => {
  for (const text of [
    "",
    " ",
    "abc",
    "0",
    "1",
    "1.0",
    "-0.5",
    "NaN",
    "Infinity",
    "inf",
    "1e-1",
    "1e999",
    "0x1",
    "0.5.5",
    "0,5,5",
    "50%",
    "0.5 0.6",
    "++0.5",
    "٠.٥",
  ]) {
    assert.deepEqual(parseRatioEntry(text), { ok: false }, `'${text}' must be refused`);
  }
});

test("22: no accepted entry is outside the range or off the grid (sweep)", () => {
  for (let i = -50; i <= 1200; i += 1) {
    const text = (i / 1000).toString();
    const r = parseRatioEntry(text);
    if (r.ok) {
      assert.ok(r.value >= 0.01 && r.value <= 0.99, text);
      assert.ok(Math.abs(Math.round(r.value * 100) - r.value * 100) < 1e-9, text);
      assert.ok(i / 1000 >= 0.01 && i / 1000 <= 0.99, `${text} accepted outside the typed range`);
    } else {
      assert.ok(i / 1000 < 0.01 || i / 1000 > 0.99, `${text} refused inside the range`);
    }
  }
});

// ---- static facts the criteria name (the DOM itself is NOT VERIFIED) --------------

const src = (path: string) => readFileSync(new URL(`../src/${path}`, import.meta.url), "utf8");

test("26/28/58 (static): S1 and S2 take their names from the table; the tooltip's second line states both gestures", () => {
  const select = src("components/SelectToolbar.tsx");
  const shape = src("components/ShapeToolbar.tsx");
  assert.match(select, /label=\{RATIO_SLIDER_NAMES\.selectedStars\}/);
  assert.match(shape, /label=\{RATIO_SLIDER_NAMES\.newStars\}/);
  assert.match(src("lib/sliderNames.ts"), /HELP_RATIO = "Click to set, double-click to type"/);
});

test("26 (static): S3 and S4 tooltip, names, aria-valuetext, keyshortcuts", () => {
  const picker = src("components/panel/ColourPicker.tsx");
  assert.match(src("lib/sliderNames.ts"), /Click to set, double-click to type the hex value/);
  assert.match(picker, /HELP_COLOUR/);
  assert.match(picker, /colourSliderName\(name, "saturationValue"\)/);
  assert.match(picker, /aria-keyshortcuts="Enter F2"/);
  assert.match(picker, /Saturation \$\{percentOf\(hsv\.s\)\} %, value \$\{percentOf\(hsv\.v\)\} %/);
  assert.match(picker, /Hue \$\{Math\.round\(hsv\.h\)\} degrees/);
  const ratio = src("components/RatioSlider.tsx");
  assert.match(ratio, /aria-keyshortcuts="Enter F2"/);
  assert.match(ratio, /aria-valuetext=\{mixed \? "Mixed"/);
});

test("27 (static): touch-action manipulation on the ratio sliders, none on the colour picker", () => {
  assert.match(src("components/RatioSlider.tsx"), /touchAction: "manipulation"/);
  assert.match(src("components/panel/ColourPicker.tsx"), /touchAction: "none"/);
});

test("28 (static): the pointer targets are 24 px high (ratio slider and hue slider)", () => {
  assert.match(src("components/RatioSlider.tsx"), /TARGET_HEIGHT = 24/);
  assert.match(src("components/panel/ColourPicker.tsx"), /HUE_TARGET_HEIGHT = 24/);
});

test("31 (static): all four sliders use the one shared pointer hook", () => {
  assert.match(src("components/RatioSlider.tsx"), /useSliderPointer/);
  const picker = src("components/panel/ColourPicker.tsx");
  assert.equal((picker.match(/useSliderPointer\(/g) ?? []).length, 2);
});

test("31 (static): no native range input is left", () => {
  const files = [
    "components/RatioSlider.tsx",
    "components/SelectToolbar.tsx",
    "components/ShapeToolbar.tsx",
    "components/panel/ColourPicker.tsx",
  ];
  for (const f of files) {
    assert.doesNotMatch(src(f), /type="range"/, f);
  }
});

test("42 (static): the Absolute segment's text is the spec's, and the old texts are gone", () => {
  const chip = src("components/MoveEntryChip.tsx");
  assert.ok(
    chip.includes(
      "Position of the chosen point of the selection's box. Enter uses the chosen point, the centre until you choose another. Click a marked point on the canvas to use it.",
    ),
  );
  assert.ok(!chip.includes("Top-left corner of the object's bounds"));
  assert.ok(!chip.includes("Top-left corner of the selection"));
});

test("41 (static): the Reference control's role, name and radio names in DOM order", () => {
  const chip = src("components/MoveEntryChip.tsx");
  assert.match(chip, /role="radiogroup"/);
  assert.match(chip, /aria-label="Reference point"/);
  assert.match(chip, /aria-posinset=\{index \+ 1\}/);
  assert.match(chip, /aria-setsize=\{REFERENCE_COUNT\}/);
  assert.match(chip, /tabIndex=\{selected \? 0 : -1\}/);
});

test("40 (static): the mark click slop is 3 px and 8 px for touch", () => {
  const hook = src("hooks/useEditorSession.ts");
  assert.match(hook, /const MARK_SLOP_PX = 3;/);
  assert.match(hook, /const MARK_TOUCH_SLOP_PX = 8;/);
});

// ---- Part C chip placement (UX notes, "Chip" and question 13): pure function ------------

import { placeMoveEntryChip } from "../src/lib/readoutPlacement.ts";

const chip = { width: 200, height: 90 };
const canvas = { width: 1200, height: 800 };

test("C placement: left 16 px right of the centre, top 16 px below the lower edge of the box", () => {
  const p = placeMoveEntryChip({ center: { x: 400, y: 300 }, boxBottomY: 380 }, chip, canvas, 172);
  assert.deepEqual(p, { left: 416, top: 396 });
});

test("C placement: never higher than 16 px below the centre glyph", () => {
  const p = placeMoveEntryChip({ center: { x: 400, y: 300 }, boxBottomY: 250 }, chip, canvas, 172);
  assert.equal(p.top, 316);
});

test("C placement: flips left at the right edge and above the glyph at the bottom", () => {
  const right = placeMoveEntryChip({ center: { x: 1100, y: 300 }, boxBottomY: 380 }, chip, canvas, 172);
  assert.equal(right.left, 1100 - 16 - chip.width);
  const bottom = placeMoveEntryChip({ center: { x: 400, y: 600 }, boxBottomY: 700 }, chip, canvas, 172);
  assert.equal(bottom.top, 600 - 16 - chip.height);
});

test("C placement: the flip is tested with the Absolute height, so the mode never changes the side", () => {
  // The chip alone (90 px) would fit below, the Absolute chip (172 px) would not.
  const anchor = { center: { x: 400, y: 600 }, boxBottomY: 640 };
  const relative = placeMoveEntryChip(anchor, chip, canvas, 90 + 82);
  const absolute = placeMoveEntryChip(anchor, { width: 200, height: 172 }, canvas, 172);
  assert.ok(relative.top < anchor.center.y, "relative flips above the glyph");
  assert.ok(absolute.top < anchor.center.y, "absolute flips above the glyph");
});

test("C placement: always inside the canvas, whatever the inputs (sweep)", () => {
  let seed = 7;
  const rnd = () => {
    seed = (seed * 1103515245 + 12345) % 2147483648;
    return seed / 2147483648;
  };
  for (let i = 0; i < 2000; i += 1) {
    const c = { width: 250 + rnd() * 1500, height: 250 + rnd() * 1000 };
    const ch = { width: 100 + rnd() * 100, height: 60 + rnd() * 120 };
    const anchor = {
      center: { x: rnd() * c.width, y: rnd() * c.height },
      boxBottomY: rnd() * c.height * 1.5,
    };
    const p = placeMoveEntryChip(anchor, ch, c, ch.height + rnd() * 90);
    assert.ok(p.left >= 0 && p.left + ch.width <= c.width + 1e-9, `left ${p.left}`);
    assert.ok(p.top >= 0 && p.top + ch.height <= c.height + 1e-9, `top ${p.top}`);
  }
});

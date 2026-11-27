// The Pen probe's statistics (`specs/0158-pen-display` slice 0, criterion 5): median, rate window,
// pairing a webview event with its native sample, and the session summary and its report.
import assert from "node:assert/strict";
import { test } from "node:test";

import { PairMatcher, RateWindow, WindowedValues, median } from "../src/lib/penProbeStats.ts";
import { ProbeSession, buildReport, normalisedTilt, type NativeBatch } from "../src/lib/penProbe.ts";

test("median of odd, even and empty lists", () => {
  assert.equal(median([]), null);
  assert.equal(median([5]), 5);
  assert.equal(median([3, 1, 2]), 2);
  assert.equal(median([4, 1, 2, 3]), 2.5);
});

test("the rate window counts events of the last second, coalesced ones with their weight", () => {
  const rate = new RateWindow(1000);
  rate.add(0);
  rate.add(500, 3);
  assert.equal(rate.perSecond(900), 4);
  assert.equal(rate.perSecond(1200), 3);
  assert.equal(rate.perSecond(5000), 0);
});

test("windowed values drop what is older than the window", () => {
  const values = new WindowedValues(1000);
  values.add(0, 10);
  values.add(900, 20);
  assert.deepEqual(values.values(950), [10, 20]);
  assert.deepEqual(values.values(1500), [20]);
});

test("the matcher pairs the same position and gives native minus webview", () => {
  const matcher = new PairMatcher(3, 300);
  assert.equal(matcher.addWeb(100, 50, 60), null);
  const pair = matcher.addNative(112, 50.5, 60);
  assert.ok(pair);
  assert.equal(pair.delayMs, 12);
  assert.ok(Math.abs(pair.distancePx - 0.5) < 1e-9);
});

test("the matcher gives a negative delay when the native sample came first", () => {
  const matcher = new PairMatcher(3, 300);
  assert.equal(matcher.addNative(100, 10, 10), null);
  const pair = matcher.addWeb(108, 10, 10);
  assert.equal(pair?.delayMs, -8);
});

test("the matcher takes the nearest position and never pairs far or old events", () => {
  const matcher = new PairMatcher(3, 300);
  matcher.addWeb(0, 10, 10);
  matcher.addWeb(1, 12, 10);
  assert.equal(matcher.addNative(5, 12, 10)?.distancePx, 0);
  assert.equal(matcher.addNative(6, 30, 30), null);
  matcher.addWeb(1000, 70, 70);
  assert.equal(matcher.addNative(2000, 70, 70), null);
  assert.ok(matcher.unmatched() >= 1);
});

test("the tilt of a Pointer Event in degrees is a fraction of 90", () => {
  assert.equal(normalisedTilt(45), 0.5);
  assert.equal(normalisedTilt(-120), -1);
});

const batch = (over: Partial<NativeBatch["samples"][number]>, device = "Huion Pen (Pen)"): NativeBatch => ({
  samples: [{ x: 10, y: 20, pressure: 0.4, tilt: { x: 0.2, y: -0.1 }, end: "tip", phase: "contact", barrel: 0, time_ms: 1, ...over }],
  dropped: 0,
  device,
  age_ms: 2,
});

test("a mouse is shown as mouse with no pressure and no native samples", () => {
  const session = new ProbeSession();
  session.web({ t: 0, x: 1, y: 1, pointerType: "mouse", pressure: 0.5, tiltX: 0, tiltY: 0, eraser: false, barrel: false, coalesced: 1 });
  const snap = session.snapshot(10);
  assert.equal(snap.web.pointerType, "mouse");
  assert.equal(snap.web.pressureShown, false);
  assert.equal(snap.native.samples, 0);
});

test("the session counts native samples, keeps peaks and the delay of matched pairs", () => {
  const session = new ProbeSession();
  session.web({ t: 100, x: 10, y: 20, pointerType: "pen", pressure: 0.5, tiltX: 0, tiltY: 0, eraser: false, barrel: false, coalesced: 2 });
  session.native(batch({ pressure: 0.9, end: "eraser", barrel: 1 }), 110);
  const snap = session.snapshot(120);
  assert.equal(snap.native.samples, 1);
  assert.equal(snap.native.peakPressure, 0.9);
  assert.equal(snap.native.eraserSeen, true);
  assert.equal(snap.native.barrelSeen, true);
  assert.equal(snap.native.device, "Huion Pen (Pen)");
  assert.equal(snap.delay.medianMs, 10);
  assert.equal(snap.delay.maxMs, 10);
  assert.equal(snap.web.eventsPerSecond, 2);
  assert.equal(snap.web.coalescedLast, 2);
});

test("the report is plain text with both routes, the delay and the compression", () => {
  const session = new ProbeSession();
  session.web({ t: 100, x: 10, y: 20, pointerType: "pen", pressure: 0.5, tiltX: 30, tiltY: 0, eraser: false, barrel: false, coalesced: 1 });
  session.native(batch({}), 108);
  session.frame(16.7, 120);
  const text = buildReport(session.snapshot(130), { route: "native", compression: false, platform: "Linux x86_64", agent: "TestAgent/1" });
  assert.match(text, /Curvyo pen probe/);
  assert.match(text, /Webview: pointer type pen/);
  assert.match(text, /Native: Huion Pen \(Pen\)/);
  assert.match(text, /Delay native minus webview: median 8\.0 ms/);
  assert.match(text, /Motion compression: off/);
  assert.match(text, /Frame time: median 16\.7 ms/);
});

test("the report says the native route is not available in a browser", () => {
  const text = buildReport(new ProbeSession().snapshot(0), { route: "unavailable", compression: true, platform: "x", agent: "y" });
  assert.match(text, /Native: not available/);
});

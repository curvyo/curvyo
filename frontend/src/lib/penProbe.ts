// The Pen probe's session (`specs/0158-pen-display` slice 0, ADR 0094 decision 8): what the
// webview's Pointer Events and the native samples showed, the measurements between them and the
// plain-text report. Pure and in memory only; nothing is stored or sent (criterion 4).
import { PairMatcher, RateWindow, WindowedValues, median } from "./penProbeStats.ts";

/** The seconds the live delay and position figures look back over. */
const LIVE_WINDOW_MS = 5000;

/** One Pointer Event of the webview, read into plain values. */
export interface WebSample {
  /** Arrival, `performance.now()` ms. */
  t: number;
  x: number;
  y: number;
  pointerType: string;
  pressure: number;
  tiltX: number;
  tiltY: number;
  /** The eraser bit (`buttons & 32`). */
  eraser: boolean;
  /** The barrel button (`buttons & 2`). */
  barrel: boolean;
  /** How many events this one stands for (itself plus its coalesced ones). */
  coalesced: number;
}

/** A native sample as `curvyo_ui_core::PenSample` serialises it. */
export interface NativeSample {
  x: number;
  y: number;
  pressure: number | null;
  tilt: { x: number; y: number } | null;
  end: "tip" | "eraser";
  phase: "hover" | "contact" | "out";
  barrel: number;
  time_ms: number;
}

/** One frame's batch as `curvyo_ui_core::PenBatch` serialises it. */
export interface NativeBatch {
  samples: NativeSample[];
  dropped: number;
  device: string;
  age_ms: number | null;
}

/** A Pointer Event's tilt in degrees as a fraction of 90, -1 to 1 (ADR 0094 decision 3). */
export function normalisedTilt(degrees: number): number {
  return Math.max(-1, Math.min(1, degrees / 90));
}

/** The numbers the page shows. */
export interface ProbeSnapshot {
  web: {
    seen: boolean;
    pointerType: string;
    /** False for a mouse: it has no pressure to show (criterion 3). */
    pressureShown: boolean;
    pressure: number;
    tiltX: number;
    tiltY: number;
    eraser: boolean;
    barrel: boolean;
    eventsPerSecond: number;
    coalescedLast: number;
    peakPressure: number;
    distinctPressures: number;
  };
  native: {
    samples: number;
    device: string;
    end: string;
    phase: string;
    pressure: number | null;
    tilt: { x: number; y: number } | null;
    barrel: number;
    samplesPerSecond: number;
    peakPressure: number;
    eraserSeen: boolean;
    barrelSeen: boolean;
    maxTilt: number;
    dropped: number;
    ageMs: number | null;
  };
  delay: { medianMs: number | null; maxMs: number | null; sessionMedianMs: number | null; sessionMaxMs: number | null; pairs: number };
  position: { medianPx: number | null; maxPx: number | null };
  frame: { medianMs: number | null; maxMs: number | null };
}

/** The fixed facts of the session the report names. */
export interface ReportContext {
  route: "native" | "unavailable";
  /** Whether GDK motion compression is on (Linux app only). */
  compression: boolean;
  platform: string;
  agent: string;
}

const MAX_KEPT = 5000;

/** Everything the probe measured since it opened. */
export class ProbeSession {
  private lastWeb: WebSample | null = null;
  private lastNative: NativeSample | null = null;
  private device = "";
  private nativeCount = 0;
  private dropped = 0;
  private ageMs: number | null = null;
  private webPeak = 0;
  private nativePeak = 0;
  private maxTilt = 0;
  private eraserSeen = false;
  private barrelSeen = false;
  private readonly webPressures = new Set<number>();
  private readonly webRate = new RateWindow();
  private readonly nativeRate = new RateWindow();
  private readonly liveDelay = new WindowedValues(LIVE_WINDOW_MS);
  private readonly liveDistance = new WindowedValues(LIVE_WINDOW_MS);
  private readonly liveFrames = new WindowedValues(LIVE_WINDOW_MS);
  private readonly matcher = new PairMatcher();
  private readonly allDelays: number[] = [];
  private readonly allDistances: number[] = [];

  /** A Pointer Event of the webview arrived. */
  web(sample: WebSample, coalescedPoints: readonly { x: number; y: number }[] = []): void {
    this.lastWeb = sample;
    this.webRate.add(sample.t, sample.coalesced);
    if (sample.pointerType !== "mouse") {
      this.webPeak = Math.max(this.webPeak, sample.pressure);
      this.webPressures.add(Math.round(sample.pressure * 1000));
    }
    // The pen's intermediate positions arrive with the event; each can be the partner of a native sample.
    for (const point of [...coalescedPoints, sample]) {
      this.pair(this.matcher.addWeb(sample.t, point.x, point.y), sample.t);
    }
  }

  /** A native batch arrived at `now`. */
  native(batch: NativeBatch, now: number): void {
    this.dropped += batch.dropped;
    this.ageMs = batch.age_ms;
    if (batch.device) {
      this.device = batch.device;
    }
    this.nativeRate.add(now, batch.samples.length);
    for (const sample of batch.samples) {
      this.nativeCount += 1;
      this.lastNative = sample;
      this.nativePeak = Math.max(this.nativePeak, sample.pressure ?? 0);
      this.eraserSeen ||= sample.end === "eraser";
      this.barrelSeen ||= sample.barrel !== 0;
      if (sample.tilt) {
        this.maxTilt = Math.max(this.maxTilt, Math.abs(sample.tilt.x), Math.abs(sample.tilt.y));
      }
      this.pair(this.matcher.addNative(now, sample.x, sample.y), now);
    }
  }

  /** The browser drew a frame `ms` after the one before. */
  frame(ms: number, now: number): void {
    this.liveFrames.add(now, ms);
  }

  private pair(pair: { delayMs: number; distancePx: number } | null, now: number): void {
    if (!pair) {
      return;
    }
    this.liveDelay.add(now, pair.delayMs);
    this.liveDistance.add(now, pair.distancePx);
    for (const [list, value] of [[this.allDelays, pair.delayMs], [this.allDistances, pair.distancePx]] as const) {
      list.push(value);
      if (list.length > MAX_KEPT) {
        list.shift();
      }
    }
  }

  snapshot(now: number): ProbeSnapshot {
    const web = this.lastWeb;
    const native = this.lastNative;
    const delays = this.liveDelay.values(now);
    const distances = this.liveDistance.values(now);
    const frames = this.liveFrames.values(now);
    const maxOf = (values: number[]) => (values.length === 0 ? null : Math.max(...values));
    const absMax = (values: number[]) => (values.length === 0 ? null : Math.max(...values.map(Math.abs)));
    return {
      web: {
        seen: web !== null,
        pointerType: web?.pointerType ?? "",
        pressureShown: web !== null && web.pointerType !== "mouse",
        pressure: web?.pressure ?? 0,
        tiltX: web?.tiltX ?? 0,
        tiltY: web?.tiltY ?? 0,
        eraser: web?.eraser ?? false,
        barrel: web?.barrel ?? false,
        eventsPerSecond: this.webRate.perSecond(now),
        coalescedLast: web?.coalesced ?? 0,
        peakPressure: this.webPeak,
        distinctPressures: this.webPressures.size,
      },
      native: {
        samples: this.nativeCount,
        device: this.device,
        end: native?.end ?? "",
        phase: native?.phase ?? "",
        pressure: native?.pressure ?? null,
        tilt: native?.tilt ?? null,
        barrel: native?.barrel ?? 0,
        samplesPerSecond: this.nativeRate.perSecond(now),
        peakPressure: this.nativePeak,
        eraserSeen: this.eraserSeen,
        barrelSeen: this.barrelSeen,
        maxTilt: this.maxTilt,
        dropped: this.dropped,
        ageMs: this.ageMs,
      },
      delay: {
        medianMs: median(delays),
        maxMs: absMax(delays),
        sessionMedianMs: median(this.allDelays),
        sessionMaxMs: absMax(this.allDelays),
        pairs: this.allDelays.length,
      },
      position: { medianPx: median(distances), maxPx: maxOf(distances) },
      frame: { medianMs: median(frames), maxMs: maxOf(frames) },
    };
  }
}

const fixed = (value: number | null, digits: number, unit: string): string =>
  value === null ? "n/a" : `${value.toFixed(digits)}${unit}`;

/** The session as plain text for the customer to paste back; names no person and nothing stored. */
export function buildReport(snapshot: ProbeSnapshot, context: ReportContext): string {
  const { web, native, delay, position, frame } = snapshot;
  const tilt = native.tilt ? `${native.tilt.x.toFixed(2)}, ${native.tilt.y.toFixed(2)}` : "none";
  const lines = [
    "Curvyo pen probe",
    `System: ${context.platform}; ${context.agent}`,
    web.seen
      ? `Webview: pointer type ${web.pointerType}, pressure ${web.pressureShown ? web.pressure.toFixed(2) : "none"} (peak ${web.peakPressure.toFixed(2)}, ${web.distinctPressures} distinct values), tilt ${web.tiltX}, ${web.tiltY} deg, eraser ${web.eraser}, barrel ${web.barrel}, ${web.eventsPerSecond.toFixed(0)} events/s`
      : "Webview: no pointer event yet",
    context.route === "unavailable"
      ? "Native: not available in this build"
      : native.samples === 0
        ? "Native: no pen sample yet"
        : `Native: ${native.device || "unknown device"}, ${native.samples} samples, ${native.samplesPerSecond.toFixed(0)} samples/s, pressure ${native.pressure === null ? "none" : native.pressure.toFixed(2)} (peak ${native.peakPressure.toFixed(2)}), tilt ${tilt} (largest ${native.maxTilt.toFixed(2)}), eraser seen ${native.eraserSeen}, barrel seen ${native.barrelSeen}, dropped ${native.dropped}, age in the app ${fixed(native.ageMs, 1, " ms")}`,
    `Delay native minus webview: median ${fixed(delay.sessionMedianMs, 1, " ms")}, largest ${fixed(delay.sessionMaxMs, 1, " ms")}, ${delay.pairs} pairs`,
    `Position difference: median ${fixed(position.medianPx, 1, " px")}, largest ${fixed(position.maxPx, 1, " px")}`,
    `Motion compression: ${context.compression ? "on" : "off"}`,
    `Frame time: median ${fixed(frame.medianMs, 1, " ms")}, largest ${fixed(frame.maxMs, 1, " ms")}`,
  ];
  return lines.join("\n");
}

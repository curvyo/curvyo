// The Pen probe's small statistics (`specs/0158-pen-display` slice 0): kept pure and free of the
// DOM so the tests load them without a bundler.

/** The median of the values, `null` for none. */
export function median(values: readonly number[]): number | null {
  if (values.length === 0) {
    return null;
  }
  const sorted = [...values].sort((a, b) => a - b);
  const mid = sorted.length >> 1;
  return sorted.length % 2 === 1 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

/** Events per second over a sliding window; an entry may stand for several events. */
export class RateWindow {
  private entries: { t: number; n: number }[] = [];
  private readonly windowMs: number;
  constructor(windowMs = 1000) {
    this.windowMs = windowMs;
  }

  add(t: number, n = 1): void {
    this.entries.push({ t, n });
  }

  /** The events within the window before `now`, scaled to one second. */
  perSecond(now: number): number {
    this.entries = this.entries.filter((entry) => now - entry.t <= this.windowMs);
    const total = this.entries.reduce((sum, entry) => sum + entry.n, 0);
    return (total * 1000) / this.windowMs;
  }
}

/** Values of the last `windowMs`, for "the last seconds". */
export class WindowedValues {
  private entries: { t: number; v: number }[] = [];
  private readonly windowMs: number;
  constructor(windowMs: number) {
    this.windowMs = windowMs;
  }

  add(t: number, v: number): void {
    this.entries.push({ t, v });
  }

  values(now: number): number[] {
    this.entries = this.entries.filter((entry) => now - entry.t <= this.windowMs);
    return this.entries.map((entry) => entry.v);
  }
}

/** Two events of the two routes that are the same motion. */
export interface Pair {
  /** Native arrival minus webview arrival, ms; negative when the native sample came first. */
  delayMs: number;
  distancePx: number;
}

interface Pending {
  t: number;
  x: number;
  y: number;
}

/** Pairs each event of one route with the nearest-position unpaired event of the other that
 * arrived at most `maxAgeMs` before; an event with no partner counts as unmatched. */
export class PairMatcher {
  private web: Pending[] = [];
  private native: Pending[] = [];
  private dropped = 0;

  private readonly maxDistancePx: number;
  private readonly maxAgeMs: number;

  constructor(maxDistancePx = 3, maxAgeMs = 300) {
    this.maxDistancePx = maxDistancePx;
    this.maxAgeMs = maxAgeMs;
  }

  addWeb(t: number, x: number, y: number): Pair | null {
    return this.add(this.web, this.native, { t, x, y }, (own, other) => other.t - own.t);
  }

  addNative(t: number, x: number, y: number): Pair | null {
    return this.add(this.native, this.web, { t, x, y }, (own, other) => own.t - other.t);
  }

  /** How many events aged out without a partner. */
  unmatched(): number {
    return this.dropped;
  }

  private add(
    own: Pending[],
    other: Pending[],
    item: Pending,
    delay: (own: Pending, other: Pending) => number,
  ): Pair | null {
    for (const list of [own, other]) {
      const keep = list.filter((entry) => item.t - entry.t <= this.maxAgeMs);
      this.dropped += list.length - keep.length;
      list.splice(0, list.length, ...keep);
    }
    let best = -1;
    let bestDistance = this.maxDistancePx;
    other.forEach((entry, index) => {
      const distance = Math.hypot(entry.x - item.x, entry.y - item.y);
      if (distance < bestDistance || (best < 0 && distance <= this.maxDistancePx)) {
        best = index;
        bestDistance = distance;
      }
    });
    if (best < 0) {
      own.push(item);
      return null;
    }
    const [partner] = other.splice(best, 1);
    return { delayMs: delay(item, partner), distancePx: bestDistance };
  }
}

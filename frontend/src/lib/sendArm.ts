// The press rules of Send to machine's armed press (`specs/0134-send-and-start` SS9, SS13; UX notes
// P4, P12 to P15): what a press does, when the arm ends, and which keys count. The timings are
// those of `deleteArm.ts`. Pure, so the tests load it without a bundler.

/** What a press of the page's button does. */
export type SendPress = "ignore" | "arm" | "send" | "sendAndStart";

/** What the button knows at a press; `fresh` reads the will-it-start rule over a fresh status. */
export interface PressFacts {
  /** Enabled and not busy. */
  pressable: boolean;
  /** The name the maker saw says "and start". */
  starts: boolean;
  /** Armed, and past the 400 ms guard when `confirming`. */
  armed: boolean;
  fresh: () => boolean;
}

/** A press: a name that only sends only sends; a first "start" press arms when a fresh status
 * still starts (else nothing, P12); the confirming press starts only when a fresh status still
 * starts (else it only sends, P13). A press never does more than its name says. */
export function sendPress(facts: PressFacts): SendPress {
  if (!facts.pressable) return "ignore";
  if (!facts.starts) return "send";
  if (!facts.armed) return facts.fresh() ? "arm" : "ignore";
  return facts.fresh() ? "sendAndStart" : "send";
}

/** Whether an armed button disarms now (SS13, P15): its name is no longer "start", it turned
 * busy or disabled, or the job it would send changed since the arming press. */
export function armEnds(armed: boolean, starts: boolean, pressable: boolean, job: number, armedJob: number): boolean {
  return armed && (!starts || !pressable || job !== armedJob);
}

/** Whether a keydown must not click the button (P14): a repeat of a held `Enter` or `Space`. */
export function ignoredKey(key: string, repeat: boolean): boolean {
  return repeat && (key === "Enter" || key === " ");
}

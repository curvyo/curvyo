// The Pencil bar's values for the app run (`specs/0158-pen-display` slice 1, UX notes §2, §4):
// kept across New and Open, back to the defaults at the next start, never in the document. The
// session reads them at the next stroke; the frontend also remembers whether a pen was seen and
// whether its last stroke lacked pressure (the dimming and the "No pen pressure" state).

/** A pressure curve as the session names it. */
export type PressureCurveName = "soft" | "linear" | "firm";

export interface PencilSettingsState {
  /** Smoothing in percent, 0 to 100 (the Freehand slider's scale: percent / 100 = mm). */
  smoothing: number;
  simulate: boolean;
  /** Min and Max in mm. */
  min: number;
  max: number;
  curve: PressureCurveName;
  /** A pen was seen in this run. */
  penSeen: boolean;
  /** The last pen stroke had no pressure. */
  noPressure: boolean;
}

export const PENCIL_DEFAULTS: PencilSettingsState = {
  smoothing: 40,
  simulate: false,
  min: 0.2,
  max: 2,
  curve: "linear",
  penSeen: false,
  noPressure: false,
};

/** The ranges of Min and Max in mm (criterion 4). */
export const MIN_RANGE: readonly [number, number] = [0.05, 20];
export const MAX_RANGE: readonly [number, number] = [0.1, 50];

const clamp = (value: number, [low, high]: readonly [number, number]) =>
  Number.isNaN(value) ? low : Math.min(high, Math.max(low, value));

/** A new Min: clamped, and a Max below it raised to it, so the value just set is kept. */
export function withMin(state: PencilSettingsState, mm: number): PencilSettingsState {
  const min = clamp(mm, MIN_RANGE);
  return { ...state, min, max: Math.max(state.max, min) };
}

/** A new Max: clamped, and never below Min (raised to it). */
export function withMax(state: PencilSettingsState, mm: number): PencilSettingsState {
  return { ...state, max: Math.max(clamp(mm, MAX_RANGE), state.min) };
}

/** Curve, Min and Max are dimmed while the next stroke cannot use them: Simulate pressure off and
 * no pen seen, or the last pen stroke without pressure. */
export function widthsDimmed(state: PencilSettingsState): boolean {
  return !state.simulate && (!state.penSeen || state.noPressure);
}

/** The state after a finished stroke: `pen` when it came from a pen, `hadPressure` the session's
 * answer (1 yes, 0 no, -1 none yet). */
export function afterStroke(state: PencilSettingsState, pen: boolean, hadPressure: number): PencilSettingsState {
  if (!pen || hadPressure < 0) return state;
  return { ...state, penSeen: true, noPressure: hadPressure === 0 };
}

let state: PencilSettingsState = PENCIL_DEFAULTS;
const listeners = new Set<() => void>();

export function pencilSettings(): PencilSettingsState {
  return state;
}

export function setPencilSettings(next: PencilSettingsState): void {
  if (next !== state) {
    state = next;
    listeners.forEach((listener) => listener());
  }
}

export function subscribePencilSettings(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The session calls the bar's values reach it by. */
export interface PencilSettingsSink {
  set_pencil_smoothing(mm: number): boolean;
  set_pencil_simulate(on: boolean): void;
  set_pencil_min_mm(mm: number): void;
  set_pencil_max_mm(mm: number): void;
  set_pencil_curve(name: string): boolean;
}

/** Hands the values to a session (a new one after New or Open gets them at its first stroke). */
export function applyPencilSettings(session: PencilSettingsSink, values: PencilSettingsState = state): void {
  session.set_pencil_smoothing(values.smoothing / 100);
  session.set_pencil_simulate(values.simulate);
  session.set_pencil_min_mm(values.min);
  session.set_pencil_max_mm(values.max);
  session.set_pencil_curve(values.curve);
}

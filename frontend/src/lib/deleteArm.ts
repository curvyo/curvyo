// The arm and disarm rules of the inline delete confirm (`specs/0081-inline-delete-confirm`
// criteria 4 to 6 and 16): one armed trash at a time, a second press after the guard removes.

/** A second press this soon after arming is ignored (a double-click never removes). */
export const ARM_GUARD_MS = 400;
/** An armed trash disarms by itself this long after the arming press. */
export const ARM_TIMEOUT_MS = 4000;

/** The armed trash's target id (`null` when none is armed) and when it was armed (ms). */
export interface ArmState {
  armed: string | null;
  at: number;
}

export const IDLE: ArmState = { armed: null, at: 0 };

/** A press on the trash of `target` at `now`: arms it, ignores it inside the guard, or removes. */
export function pressTrash(state: ArmState, target: string, now: number): { state: ArmState; remove: boolean } {
  if (state.armed !== target || now - state.at >= ARM_TIMEOUT_MS) {
    return { state: { armed: target, at: now }, remove: false };
  }
  if (now - state.at < ARM_GUARD_MS) {
    return { state, remove: false };
  }
  return { state: IDLE, remove: true };
}

/** Back to no trash armed. */
export function disarm(): ArmState {
  return IDLE;
}

/** Whether the arm survives focus landing in the zone `zone` (the armed row's or cell's id;
 * `null` or `undefined` when focus lands outside every zone or on nothing). */
export function focusKeepsArm(armed: string, zone: string | null | undefined): boolean {
  return zone === armed;
}

/**
 * The Shift, Ctrl and Alt state the Select tool's marquee and lasso follow
 * (`specs/0014-advanced-selection/`: a running gesture must follow a modifier key
 * with the pointer at rest).
 *
 * The flags of a keyboard event (`shiftKey` and its siblings) are not reliable
 * on the event of the modifier key itself: some engines (WebKitGTK, the Tauri
 * webview on Linux) report the key's own flag as it was before the key
 * changed, so a Shift keydown says `shiftKey: false` and the keyup still says
 * `true`. The flags of any other key's event, and of every pointer event, are
 * right. A modifier also has two keys (left and right Shift), and releasing
 * one while the other is down leaves it held.
 *
 * So the state is tracked, not read: the keys that are down are kept by
 * `code` (`ShiftLeft`, `ShiftRight`, ...), a modifier is held while any of
 * its keys is down, and an event whose flags are reliable corrects the
 * record (a flag reported false forgets the keys of that modifier, one
 * reported true with no key known remembers an unknown key). A window blur or
 * a hidden page forgets everything: the key-up will never arrive.
 */
export interface HeldModifiers {
  shift: boolean;
  /** Ctrl, or Cmd on macOS. */
  ctrl: boolean;
  alt: boolean;
}

/** The parts of a `KeyboardEvent` the tracker reads. */
export interface KeyEventLike {
  type: string;
  key: string;
  code: string;
  location?: number;
  shiftKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
}

type Modifier = keyof HeldModifiers;

const MODIFIER_OF_KEY: Record<string, Modifier | undefined> = {
  Shift: "shift",
  Control: "ctrl",
  Meta: "ctrl",
  Alt: "alt",
};

/** Stands for a held key the tracker never saw go down. */
const UNKNOWN_KEY = "unknown";

export class ModifierTracker {
  private chordDown = false;

  /**
   * Whether Ctrl (Cmd) was pressed together with another, non-modifier key, as in Ctrl+Z, and has
   * not been released since. The Ctrl badges (plus and minus) announce what a Ctrl click would
   * do; during a keyboard chord they are noise. A pointer press ends the chord, so a Ctrl click or
   * drag after Ctrl+Z still shows them.
   */
  get chord(): boolean {
    return this.chordDown;
  }

  /** A pointer button went down: Ctrl now belongs to the click, not to a key chord. */
  pointerPressed(): void {
    this.chordDown = false;
  }

  private readonly down: Record<Modifier, Set<string>> = {
    shift: new Set(),
    ctrl: new Set(),
    alt: new Set(),
  };

  /** What is held now. */
  get held(): HeldModifiers {
    return {
      shift: this.down.shift.size > 0,
      ctrl: this.down.ctrl.size > 0,
      alt: this.down.alt.size > 0,
    };
  }

  /** A keyboard event; returns what is held after it. */
  keyEvent(event: KeyEventLike): HeldModifiers {
    const changed = MODIFIER_OF_KEY[event.key];
    if (changed !== undefined) {
      // Which physical key: `code` tells left from right; a synthetic event
      // without one is told apart by `key` and `location`.
      const id = event.code || `${event.key}:${event.location ?? 0}`;
      if (event.type === "keydown") {
        this.down[changed].add(id);
      } else if (event.type === "keyup") {
        this.down[changed].delete(id);
        // A key-up of a key never seen going down (its keydown went
        // elsewhere): the modifier was held only by a key that was never
        // seen, and that one just went up.
        const keys = this.down[changed];
        if (keys.size === 1 && keys.has(UNKNOWN_KEY)) {
          keys.clear();
        }
      }
    }
    if (changed === undefined && event.type === "keydown" && (event.ctrlKey || event.metaKey)) {
      this.chordDown = true;
    }
    // The flags of the modifiers this event did not change are right.
    this.resync(
      {
        shift: event.shiftKey,
        ctrl: event.ctrlKey || event.metaKey,
        alt: event.altKey,
      },
      changed,
    );
    return this.settle();
  }

  /** The flags of a pointer event, which are always right: they correct the
   * record, so a key-up that never arrived is forgotten at the next pointer
   * event. Returns what is held after it. */
  pointerFlags(flags: HeldModifiers): HeldModifiers {
    this.resync(flags, undefined);
    return this.settle();
  }

  /** The window lost focus or the page was hidden: no key-up will come. */
  reset(): HeldModifiers {
    this.chordDown = false;
    for (const keys of Object.values(this.down)) {
      keys.clear();
    }
    return this.held;
  }

  /** A chord ends with Ctrl; returns what is held. */
  private settle(): HeldModifiers {
    const held = this.held;
    if (!held.ctrl) {
      this.chordDown = false;
    }
    return held;
  }

  private resync(flags: HeldModifiers, except: Modifier | undefined): void {
    for (const modifier of ["shift", "ctrl", "alt"] as const) {
      if (modifier === except) {
        continue;
      }
      if (!flags[modifier]) {
        this.down[modifier].clear();
      } else if (this.down[modifier].size === 0) {
        this.down[modifier].add(UNKNOWN_KEY);
      }
    }
  }
}

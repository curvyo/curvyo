/**
 * Whether Ctrl (Cmd on a Mac) is held, window-wide, as a store the palette swatches subscribe to
 * (`specs/0089` UX notes §27.12): a Ctrl+click excludes, so a hovered swatch shows the Exclude
 * glyph while Ctrl is down.
 *
 * The state comes from the editor's [`ModifierTracker`], fed by every key and pointer event of
 * the window: WebKitGTK (the Linux desktop webview) reports `ctrlKey: true` on the Control key's
 * own key-up, so the event's flag cannot tell a release; the tracker goes by the key. A window
 * blur or a hidden page forgets the keys, as no key-up follows.
 */
import { ModifierTracker, type KeyEventLike } from "./keyModifiers.ts";

/** The parts of `window` and `document` the store listens on, so a node test can hand in its own. */
export interface ModifierEventSources {
  window: Pick<EventTarget, "addEventListener" | "removeEventListener">;
  document: Pick<EventTarget, "addEventListener" | "removeEventListener"> & { visibilityState: string };
}

/** The flags of a pointer event. */
interface PointerFlags {
  shiftKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
}

const KEY_EVENTS = ["keydown", "keyup"] as const;
const POINTER_EVENTS = ["pointermove", "pointerdown", "pointerup"] as const;

/** A store of the held Ctrl over `sources`: it listens while it has subscribers. */
export function heldCtrlStore(sources: () => ModifierEventSources) {
  const tracker = new ModifierTracker();
  const listeners = new Set<() => void>();
  let held = false;
  let attached: ModifierEventSources | null = null;
  const settle = (ctrl: boolean) => {
    if (held !== ctrl) {
      held = ctrl;
      listeners.forEach((listen) => listen());
    }
  };
  const onKey = (event: Event) => settle(tracker.keyEvent(event as unknown as KeyEventLike).ctrl);
  const onPointer = (event: Event) => {
    const flags = event as unknown as PointerFlags;
    settle(tracker.pointerFlags({ shift: flags.shiftKey, ctrl: flags.ctrlKey || flags.metaKey, alt: flags.altKey }).ctrl);
  };
  const onBlur = () => settle(tracker.reset().ctrl);
  const onVisibility = () => {
    if (attached?.document.visibilityState === "hidden") {
      onBlur();
    }
  };
  const attach = () => {
    const on = sources();
    KEY_EVENTS.forEach((type) => on.window.addEventListener(type, onKey, true));
    POINTER_EVENTS.forEach((type) => on.window.addEventListener(type, onPointer, true));
    on.window.addEventListener("blur", onBlur);
    on.document.addEventListener("visibilitychange", onVisibility);
    attached = on;
  };
  const detach = () => {
    const on = attached;
    if (!on) {
      return;
    }
    KEY_EVENTS.forEach((type) => on.window.removeEventListener(type, onKey, true));
    POINTER_EVENTS.forEach((type) => on.window.removeEventListener(type, onPointer, true));
    on.window.removeEventListener("blur", onBlur);
    on.document.removeEventListener("visibilitychange", onVisibility);
    attached = null;
    tracker.reset();
    held = false;
  };
  return {
    subscribe(listen: () => void): () => void {
      if (listeners.size === 0) {
        attach();
      }
      listeners.add(listen);
      return () => {
        listeners.delete(listen);
        if (listeners.size === 0) {
          detach();
        }
      };
    },
    get: (): boolean => held,
  };
}

/** The page's store. */
export const pageCtrl = heldCtrlStore(() => ({ window, document }));

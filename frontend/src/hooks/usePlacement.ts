import { useCallback, useEffect, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";

/** One field of the click-to-create placement chip, as Rust shows it. */
export interface PlacementFieldState {
  label: string;
  name: string;
  text: string;
  placeholder: string;
  suffix: string;
  invalid: boolean;
  /** The size field: its label is the r/⌀ button. */
  size: boolean;
}

/** One modifier check of the chip: Shift or Ctrl as a switch (`0052` criterion 32). */
export interface PlacementCheckState {
  label: string;
  shift: boolean;
  on: boolean;
}

/** The placement chip, read from `WasmSession.placement_chip()` (`specs/0052-click-to-create-
 * shapes/` criteria 11, 12, 17, 20, 23, 29). Positions are canvas-relative CSS pixels. */
export interface PlacementChipState {
  groupName: string;
  linked: boolean;
  frozen: boolean;
  fields: PlacementFieldState[];
  checks: PlacementCheckState[];
  /** The refused field's index after an Enter or a click, or -1. */
  errorField: number;
  errorMessage: string;
  a: { x: number; y: number };
  pointer: { x: number; y: number };
}

export interface PlacementApi {
  chip: PlacementChipState | null;
  /** The live region's text: the latest sentence and a counter, so a repeat is spoken again. */
  announcement: { text: string; id: number };
  /** Whether the placement started from a touch: the chip takes focus only when tapped. */
  touchStarted: boolean;
  /** Re-reads the chip; cheap, called after every input that may change it. */
  refresh: () => void;
  /** Tells the session the pointer kind of the press about to be forwarded. */
  notePointer: (pointerType: string) => void;
  edit: (index: number, text: string) => void;
  toggleDiameter: () => void;
  /** Enter: places, or leaves the chip open with the refusal. */
  place: () => boolean;
  /** A click on a modifier check: flips the same state as a Shift or Ctrl tap. */
  toggleModifier: (shift: boolean) => void;
  /** A key-down in the chip or on the canvas: a Shift or Ctrl tap toggles (criterion 32). */
  keyDown: (event: { key: string; shiftKey: boolean; ctrlKey: boolean; metaKey: boolean; repeat: boolean }) => void;
  cancel: () => void;
}

function readChip(session: WasmSession): PlacementChipState | null {
  const raw = session.placement_chip();
  if (!raw) {
    return null;
  }
  const fields: PlacementFieldState[] = [];
  for (let i = 0; i < raw.field_count(); i += 1) {
    fields.push({
      label: raw.label(i),
      name: raw.name(i),
      text: raw.text(i),
      placeholder: raw.placeholder(i),
      suffix: raw.suffix(i),
      invalid: raw.invalid(i),
      size: raw.is_size(i),
    });
  }
  const checks: PlacementCheckState[] = [];
  for (let i = 0; i < raw.check_count(); i += 1) {
    checks.push({ label: raw.check_label(i), shift: raw.check_is_shift(i), on: raw.check_on(i) });
  }
  const chip: PlacementChipState = {
    groupName: raw.group_name(),
    linked: raw.linked,
    frozen: raw.frozen,
    fields,
    checks,
    errorField: raw.error_field(),
    errorMessage: raw.error_message(),
    a: { x: raw.a_x, y: raw.a_y },
    pointer: { x: raw.pointer_x, y: raw.pointer_y },
  };
  raw.free();
  return chip;
}

function sameChip(a: PlacementChipState | null, b: PlacementChipState | null): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * The click-to-create placement's host side: the chip's state, its calls into the session, the
 * live region, and the press listener that cancels it on a press outside the canvas, the chip
 * and the active tool's bar, or on a secondary press on the canvas (`adrs.md` decision 6).
 */
export function usePlacement(
  getSession: () => WasmSession | null,
  onChange: () => void,
  containerRef: React.RefObject<HTMLDivElement | null>,
): PlacementApi {
  const [chip, setChip] = useState<PlacementChipState | null>(null);
  const [announcement, setAnnouncement] = useState({ text: "", id: 0 });
  const touchRef = useRef(false);
  const [touchStarted, setTouchStarted] = useState(false);

  const refresh = useCallback(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    const next = readChip(session);
    setChip((previous) => {
      if (previous === null && next !== null) {
        setTouchStarted(touchRef.current);
      }
      return sameChip(previous, next) ? previous : next;
    });
    const said = session.take_placement_announcement();
    if (said) {
      setAnnouncement((previous) => ({ text: said, id: previous.id + 1 }));
    }
  }, [getSession]);

  const notePointer = useCallback(
    (pointerType: string) => {
      touchRef.current = pointerType === "touch";
      getSession()?.set_touch_input(touchRef.current);
    },
    [getSession],
  );

  const edit = useCallback(
    (index: number, text: string) => {
      getSession()?.placement_edit(index, text);
      onChange();
    },
    [getSession, onChange],
  );

  const toggleDiameter = useCallback(() => {
    getSession()?.placement_toggle_diameter();
    onChange();
  }, [getSession, onChange]);

  const place = useCallback(() => {
    const placed = getSession()?.placement_place() ?? false;
    onChange();
    return placed;
  }, [getSession, onChange]);

  const toggleModifier = useCallback(
    (shift: boolean) => {
      getSession()?.placement_toggle_modifier(shift);
      onChange();
    },
    [getSession, onChange],
  );

  const keyDown = useCallback(
    (event: { key: string; shiftKey: boolean; ctrlKey: boolean; metaKey: boolean; repeat: boolean }) => {
      const session = getSession();
      if (
        session?.placement_key_down(
          event.key,
          event.shiftKey,
          event.ctrlKey || event.metaKey,
          event.repeat,
        )
      ) {
        onChange();
      }
    },
    [getSession, onChange],
  );

  const cancel = useCallback(() => {
    getSession()?.cancel_placement();
    onChange();
  }, [getSession, onChange]);

  const running = chip !== null;
  useEffect(() => {
    if (!running) {
      return undefined;
    }
    // Capture phase: the cancel runs before the pressed control's own action (UX notes 5).
    const onPress = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Element)) {
        return;
      }
      if (target.closest("[data-placement-chip]") || target.closest("[data-placement-keeps]")) {
        return;
      }
      const onCanvas = target instanceof HTMLCanvasElement && containerRef.current?.contains(target);
      if (onCanvas && event.button !== 2) {
        // The session decides a primary press on the canvas (criteria 4, 5, 22); the middle
        // button pans (criterion 9).
        return;
      }
      cancel();
    };
    window.addEventListener("pointerdown", onPress, true);
    return () => window.removeEventListener("pointerdown", onPress, true);
  }, [running, cancel, containerRef]);

  // Every end of the placement returns the focus to the canvas (criteria 6, 7, 27).
  const wasRunning = useRef(false);
  useEffect(() => {
    if (wasRunning.current && !running) {
      // A press on a panel control keeps its own focus; the chip's field, being gone, does not.
      const focused = document.activeElement;
      const container = containerRef.current;
      if (!focused || focused === document.body || container?.contains(focused)) {
        container?.focus();
      }
    }
    wasRunning.current = running;
  }, [running, containerRef]);

  return {
    chip,
    announcement,
    touchStarted,
    refresh,
    notePointer,
    edit,
    toggleDiameter,
    place,
    toggleModifier,
    keyDown,
    cancel,
  };
}

import { useCallback, useEffect, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";

/** One field of the Pen entry chip, as Rust shows it. */
export interface PenEntryFieldState {
  label: string;
  name: string;
  suffix: string;
  text: string;
}

/** The Pen entry chip, read from `WasmSession.pen_entry_chip()`
 * (`specs/0072-pen-angle-snap-and-typed-values/` criteria 13, 15, 16). Positions are
 * canvas-relative CSS pixels. */
export interface PenEntryChipState {
  groupName: string;
  frozen: boolean;
  fields: PenEntryFieldState[];
  last: { x: number; y: number };
  pointer: { x: number; y: number };
  /** The typed point the rubber band ends on, or `null` with a zero length. */
  end: { x: number; y: number } | null;
}

export interface PenEntryApi {
  chip: PenEntryChipState | null;
  /** The live region's text and a counter, so a repeat is spoken again (criterion 34). */
  announcement: { text: string; id: number };
  /** Bumped when Enter was refused: the chip selects the Length text again (criterion 26). */
  refused: number;
  /** Re-reads the chip; cheap, called after every input that may change it. */
  refresh: () => void;
  edit: (index: number, text: string) => void;
  /** Enter in a field: adds the node, or keeps the chip open with the notice. */
  place: () => void;
  cancel: () => void;
}

const FIELD_COUNT = 2;

function readChip(session: WasmSession): PenEntryChipState | null {
  const raw = session.pen_entry_chip();
  if (!raw) {
    return null;
  }
  const fields: PenEntryFieldState[] = [];
  for (let i = 0; i < FIELD_COUNT; i += 1) {
    fields.push({ label: raw.label(i), name: raw.name(i), suffix: raw.suffix(i), text: raw.text(i) });
  }
  const chip: PenEntryChipState = {
    groupName: raw.group_name(),
    frozen: raw.frozen,
    fields,
    last: { x: raw.last_x, y: raw.last_y },
    pointer: { x: raw.pointer_x, y: raw.pointer_y },
    end: Number.isFinite(raw.end_x) ? { x: raw.end_x, y: raw.end_y } : null,
  };
  raw.free();
  return chip;
}

/**
 * The Pen entry chip's host side: the chip's state, its calls into the session, the live region,
 * the notices, and the press listener that cancels it on a press outside the canvas and the chip
 * (`specs/0072-pen-angle-snap-and-typed-values/adrs.md` decision 6). A press on the canvas is the
 * session's: it places the typed node when frozen, or closes the chip and draws as usual.
 */
export function usePenEntry(
  getSession: () => WasmSession | null,
  onChange: () => void,
  showNotice: (text: string) => void,
  containerRef: React.RefObject<HTMLDivElement | null>,
): PenEntryApi {
  const [chip, setChip] = useState<PenEntryChipState | null>(null);
  const [announcement, setAnnouncement] = useState({ text: "", id: 0 });
  const [refused, setRefused] = useState(0);

  const refresh = useCallback(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    const next = readChip(session);
    setChip((previous) => (JSON.stringify(previous) === JSON.stringify(next) ? previous : next));
    const said = session.take_pen_entry_announcement();
    if (said) {
      setAnnouncement((previous) => ({ text: said, id: previous.id + 1 }));
    }
    const notice = session.take_pen_entry_notice();
    if (notice) {
      showNotice(notice);
    }
  }, [getSession, showNotice]);

  const edit = useCallback(
    (index: number, text: string) => {
      getSession()?.pen_entry_edit(index, text);
      onChange();
    },
    [getSession, onChange],
  );

  const place = useCallback(() => {
    const placed = getSession()?.pen_entry_place() ?? false;
    if (!placed) {
      setRefused((count) => count + 1);
    }
    onChange();
  }, [getSession, onChange]);

  const cancel = useCallback(() => {
    getSession()?.cancel_pen_entry();
    onChange();
  }, [getSession, onChange]);

  const open = chip !== null;
  useEffect(() => {
    if (!open) {
      return undefined;
    }
    // Capture phase: the cancel runs before the pressed control's own action (criterion 22).
    const onPress = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Element) || target.closest("[data-pen-entry-chip]")) {
        return;
      }
      if (target instanceof HTMLCanvasElement && containerRef.current?.contains(target)) {
        return;
      }
      cancel();
    };
    window.addEventListener("pointerdown", onPress, true);
    return () => window.removeEventListener("pointerdown", onPress, true);
  }, [open, cancel, containerRef]);

  // Every end of the chip returns the focus to the canvas (UX notes, "Focus").
  const wasOpen = useRef(false);
  useEffect(() => {
    if (wasOpen.current && !open) {
      const focused = document.activeElement;
      const container = containerRef.current;
      if (!focused || focused === document.body || container?.contains(focused)) {
        container?.focus();
      }
    }
    wasOpen.current = open;
  }, [open, containerRef]);

  return { chip, announcement, refused, refresh, edit, place, cancel };
}

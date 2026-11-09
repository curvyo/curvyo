import { useCallback, useMemo } from "react";
import type { PickRow } from "@/components/InlinePickList";
import type { WasmSession } from "@/lib/editorSession";

/** One laser page Place on… offers, as the session lists it. */
interface PlaceOnTarget {
  id: string;
  name: string;
  width_mm: number;
  height_mm: number;
  /** The size in the display unit, as the Place page rows show it. */
  size: string;
}

function isTarget(value: unknown): value is PlaceOnTarget {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.id === "string" &&
    typeof v.name === "string" &&
    typeof v.width_mm === "number" &&
    typeof v.height_mm === "number" &&
    typeof v.size === "string"
  );
}

/** The rows of the session's `place_on_targets` JSON; none for anything else. */
export function placeOnRows(text: string): PickRow[] {
  try {
    const parsed: unknown = JSON.parse(text);
    return Array.isArray(parsed)
      ? parsed.filter(isTarget).map((t) => ({ id: t.id, name: t.name, detail: t.size }))
      : [];
  } catch {
    return [];
  }
}

/**
 * Place on… in the Select bar (`specs/0100-bring-content-in` criterion 25): the laser pages, and
 * the command that makes linked clones of the selection on the chosen one, then focuses the
 * canvas. No rows hides the button.
 */
export function usePlaceOn(editor: {
  getSession: () => WasmSession | null;
  sync: () => void;
  syncRevision: number;
  containerRef: { current: HTMLElement | null };
}) {
  const { getSession, sync, syncRevision, containerRef } = editor;
  const rows = useMemo(() => {
    void syncRevision;
    return placeOnRows(getSession()?.place_on_targets() ?? "[]");
  }, [getSession, syncRevision]);
  const pick = useCallback(
    (id: string) => {
      const session = getSession();
      if (!session) return;
      if (session.place_on(id)) {
        sync();
        containerRef.current?.focus({ preventScroll: true });
      }
    },
    [getSession, sync, containerRef],
  );
  return { rows, onPick: pick };
}

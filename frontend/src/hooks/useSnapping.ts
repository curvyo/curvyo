import { useCallback, useMemo } from "react";

import type { WasmSession } from "@/lib/editorSession";

/** Snapping toolbar state from the session. */
export interface SnappingToolbarView {
  enabled: boolean;
  suspended: boolean;
  hasDrag: boolean;
  objects: boolean;
  guides: boolean;
  nodes: boolean;
  page: boolean;
  capturePx: number;
  releasePx: number;
}

/** Snapping API for the toolbar. */
export interface SnappingApi {
  view: SnappingToolbarView;
  /** Toggles the snapping enabled switch. Returns the new state. */
  toggle: () => boolean;
  /** Sets which target kinds contribute. */
  setTargets: (objects: boolean, guides: boolean, nodes: boolean, page: boolean) => boolean;
  /** Updates capture and release distances (screen pixels). */
  setZones: (capturePx: number, releasePx: number) => boolean;
}

/** Reads the current snapping state from the session. */
function readToolbar(session: WasmSession | null): SnappingToolbarView {
  if (!session) {
    return {
      enabled: false,
      suspended: false,
      hasDrag: false,
      objects: true,
      guides: true,
      nodes: true,
      page: true,
      capturePx: 8,
      releasePx: 14,
    };
  }
  const raw = session.snapping_toolbar();
  const view: SnappingToolbarView = {
    enabled: raw.enabled,
    suspended: raw.suspended,
    hasDrag: raw.has_drag,
    objects: raw.objects,
    guides: raw.guides,
    nodes: raw.nodes,
    page: raw.page,
    capturePx: raw.capture_px,
    releasePx: raw.release_px,
  };
  raw.free();
  return view;
}

interface EditorHandle {
  getSession: () => WasmSession | null;
  syncRevision: number;
}

/**
 * Snapping state and commands (`specs/0060-snapping`). The snapshot is re-read
 * whenever the session says its UI state changed (`syncRevision`).
 */
export function useSnapping(editor: EditorHandle): SnappingApi {
  const { getSession, syncRevision } = editor;
  const view = useMemo(() => readToolbar(getSession()), [getSession, syncRevision]);

  const toggle = useCallback(() => {
    const session = getSession();
    if (!session) {
      return false;
    }
    const newState = session.toggle_snapping();
    return newState;
  }, [getSession]);

  const setTargets = useCallback(
    (objects: boolean, guides: boolean, nodes: boolean, page: boolean): boolean => {
      const session = getSession();
      if (!session) {
        return false;
      }
      // We need a method to set targets individually; currently we have set_snapping with all fields.
      // For now, we'll read current settings and update only targets.
      const settings = session.snapping_settings();
      const changed = session.set_snapping(
        settings.enabled,
        objects,
        guides,
        nodes,
        page,
        settings.capture_px,
        settings.release_px,
      );
      settings.free();
      return changed;
    },
    [getSession],
  );

  const setZones = useCallback(
    (capturePx: number, releasePx: number): boolean => {
      const session = getSession();
      if (!session) {
        return false;
      }
      const settings = session.snapping_settings();
      const changed = session.set_snapping(
        settings.enabled,
        settings.objects,
        settings.guides,
        settings.nodes,
        settings.page,
        capturePx,
        releasePx,
      );
      settings.free();
      return changed;
    },
    [getSession],
  );

  return {
    view,
    toggle,
    setTargets,
    setZones,
  };
}
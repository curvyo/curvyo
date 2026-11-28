// The link between the main window and the pen window in both pages (`specs/0158-pen-display`
// slice 2a, ADR 0096 A10): shuttles bundles every frame; in the main window it opens the pen
// window, holds it around fate-deciding dialogs and saves on its request; in the pen window it
// opens every project that arrives as a fresh session.
import { useEffect, useRef, useState } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import { bridge, closePenWindow, isTauri, openPenWindow } from "@/platform/host";
import { isPeerGone, linkFrame } from "@/lib/windowLink";
import {
  PEN_WINDOW_FAILED,
  penWindowState,
  setPenWindowPress,
  setPenWindowState,
} from "@/lib/penWindowStore";

/** No first frame of the pen window within this long is a failure (criterion 1). */
const OPEN_TIMEOUT_MS = 10_000;
/** The failure notice shows this long. */
const FAILED_NOTICE_MS = 8_000;

/** What the main window's link needs from the project. */
export interface LinkProject {
  /** A fate-deciding dialog is open: the unsaved check, or an Open or Save as dialog. */
  fateOpen: boolean;
  /** Save, as Ctrl+S in the main window. */
  save: () => void;
}

/** The main window's half: the button's states, the hold and the pen window's save. */
export function useMainWindowLink(editor: EditorSession, project: LinkProject): void {
  const projectRef = useRef(project);
  useEffect(() => {
    projectRef.current = project;
  });
  const timer = useRef<number | null>(null);
  const { getSession, addFrameHook, sync, showHistoryNotice } = editor;

  useEffect(() => {
    if (!isTauri) return;
    const stopTimer = () => {
      if (timer.current !== null) window.clearTimeout(timer.current);
      timer.current = null;
    };
    const fail = () => {
      stopTimer();
      void closePenWindow().catch(() => undefined);
      getSession()?.link_end();
      setPenWindowState("closed");
      showHistoryNotice(PEN_WINDOW_FAILED, FAILED_NOTICE_MS, "canvas", "alert");
    };
    void bridge
      .watch((bytes) => {
        const session = getSession();
        if (!session) return;
        session.link_receive(bytes);
        if (isPeerGone(bytes)) {
          stopTimer();
          setPenWindowState("closed");
        } else if (penWindowState() === "opening") {
          stopTimer();
          setPenWindowState("open");
        }
      })
      .catch(() => undefined);
    setPenWindowPress(() => {
      const state = penWindowState();
      if (state === "opening") return;
      if (state === "open") {
        void openPenWindow().catch(() => undefined);
        return;
      }
      setPenWindowState("opening");
      getSession()?.link_start("main");
      timer.current = window.setTimeout(fail, OPEN_TIMEOUT_MS);
      openPenWindow().catch(fail);
    });
    const removeHook = addFrameHook((session) => {
      const frame = linkFrame(session, (bytes) => void bridge.send(bytes).catch(() => undefined));
      if (frame.changed) sync();
      if (frame.save) projectRef.current.save();
      if (frame.notice) showHistoryNotice(frame.notice);
    });
    return () => {
      stopTimer();
      removeHook();
      setPenWindowPress(() => undefined);
    };
  }, [getSession, addFrameHook, sync, showHistoryNotice]);

  // Hold and Release around the main window's fate-deciding dialogs (criterion 13).
  const { fateOpen } = project;
  useEffect(() => {
    const bytes = getSession()?.link_hold(fateOpen);
    if (bytes) void bridge.send(bytes).catch(() => undefined);
  }, [fateOpen, getSession]);
}

/** The pen window's half: starts the link with the remembered Follow view, opens each arriving
 * project as a fresh session and reports the link's state for the banner; `save` asks the main
 * window to save. */
export function usePenWindowLink(
  editor: EditorSession,
  fitInsets: readonly [number, number],
  follow: boolean,
): { state: string; save: () => void } {
  const [state, setState] = useState("waiting");
  const opening = useRef(false);
  const followAtStart = useRef(follow);
  const { getSession, addFrameHook, sync, showHistoryNotice, openProject } = editor;

  useEffect(() => {
    void bridge.watch((bytes) => getSession()?.link_receive(bytes)).catch(() => undefined);
    const removeHook = addFrameHook((session) => {
      if (opening.current) return;
      if (session.link_state() === "none") {
        // The first fit of the project uses the free canvas of the pen layout (criterion 22).
        session.view_fit_page(fitInsets[0], fitInsets[1]);
        session.link_start("pen");
        session.link_set_follow(followAtStart.current);
      }
      const frame = linkFrame(session, (bytes) => void bridge.send(bytes).catch(() => undefined));
      setState(frame.state);
      if (frame.notice) showHistoryNotice(frame.notice);
      if (frame.changed) sync();
      if (frame.project) {
        opening.current = true;
        void openProject(frame.project).finally(() => {
          opening.current = false;
          sync();
        });
      }
    });
    return removeHook;
  }, [getSession, addFrameHook, sync, showHistoryNotice, openProject, fitInsets]);

  return { state, save: () => getSession()?.link_request_save() };
}

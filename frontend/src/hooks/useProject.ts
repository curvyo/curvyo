import { useCallback, useEffect, useRef, useState } from "react";

import type { DocumentPanelApi } from "@/hooks/useDocumentPanel";
import type { EditorSession } from "@/hooks/useEditorSession";
import type { PanelOpenApi } from "@/hooks/usePanelOpen";
import type { OpenBytesPayload, OpenErrorPayload, OpenPayload, SaveOutcome } from "@/lib/projectState";
import {
  baseName,
  type ReplacingKind,
  projectTitle,
  savedNotice,
} from "@/lib/projectText";
import { isMac } from "@/lib/historyText";
import { createBusyGate } from "@/lib/busyGate";
import { templateOutcome } from "@/lib/guideText";
import { invoke, isTauri, listen, PLATFORM } from "@/platform/host";

/** The browser build on a Mac uses Cmd as well. */
const MAC = isMac(navigator.platform);

/** A command of the File tab: the names `curvyo-ui-core`'s chord table uses. */
export type FileCommandName = "new" | "open" | "save" | "save-as" | "close" | "quit";

/** What a replacing action will do once the maker has answered the check. */
type Action =
  | { kind: "new" }
  | { kind: "open" }
  | { kind: "close" }
  | { kind: "quit" }
  | { kind: "handover"; path: string; bytes: number[] };

/** Who asked: a button of the File tab, or something outside it (a chord, the window close, the
 * operating system handing over a file). */
type Origin = { button: FileCommandName } | "outside";

/** The inline check of unsaved changes while it shows (`specs/0051-sidebar-and-no-menu/`
 * criteria 25 to 28). */
export interface InlineCheck {
  /** Changes with every request, so the block starts over (the 400 ms before Discard, the focus). */
  id: number;
  kind: ReplacingKind;
  origin: Origin;
  /** The view to return to on Cancel when the check was raised from outside the File tab. */
  previous: { tab: string; open: boolean } | null;
}

interface PendingCheck extends InlineCheck {
  action: Action;
}

/** The open project: the file it was saved to or opened from, if any. */
export interface ProjectInfo {
  fileName: string | null;
  path: string | null;
}

export interface ProjectApi {
  info: ProjectInfo;
  /** The project has changes that no saved file holds. */
  dirty: boolean;
  /** The check on screen, or `null`. */
  check: InlineCheck | null;
  /** A button of the File tab was pressed. */
  press: (command: FileCommandName) => void;
  /** The Template button (`specs/0046-document-shapes` criterion 19): the file dialog, then the
   * file's outlines as guides. It never replaces the project and raises no check. */
  loadTemplate: () => void;
  /** Save in the check: the action goes on after a successful save. */
  saveFromCheck: () => void;
  /** Discard in the check: the action goes on and the changes are dropped. */
  discard: () => void;
  /** Cancel or Escape in the check. */
  cancel: () => void;
}

interface ProjectDeps {
  editor: EditorSession;
  panelOpen: PanelOpenApi;
  panel: DocumentPanelApi;
  /** The existing error dialogs; their text is the host's or the session's. */
  onOpenError: (message: string) => void;
  onSaveError: (message: string) => void;
}

const SAVE_FAILED = "This project couldn't be saved.";
const DEFAULT_FILE_NAME = "Untitled.curvyo";

/** Whether keyboard focus is in a field whose edit commits when it loses focus. */
function isTextField(element: Element | null): element is HTMLElement {
  return element instanceof HTMLElement && element.matches('input, textarea, [contenteditable=""], [contenteditable="true"]');
}

/**
 * The open project in the page (`specs/0051-sidebar-and-no-menu/`, `adrs.md` decisions 3, 5, 7):
 * its file, the mirror of the session's dirty flag that the title, the File tab and the host
 * follow, and New, Open, Save, Save as, Close and Quit. `press` (the buttons), the file chords,
 * the window close and a file the operating system hands over all end in the
 * one place that reads the dirty state and raises the inline check. The rules (the chord table
 * and its gate, the dirty flag) are the session's; this hook runs the commands and keeps the host
 * told.
 */
export function useProject(deps: ProjectDeps): ProjectApi {
  const depsRef = useRef(deps);
  useEffect(() => {
    depsRef.current = deps;
  });

  const [info, setInfoState] = useState<ProjectInfo>({ fileName: null, path: null });
  const infoRef = useRef(info);
  const setInfo = useCallback((next: ProjectInfo) => {
    infoRef.current = next;
    setInfoState(next);
  }, []);

  const [dirty, setDirty] = useState(false);
  const dirtyRef = useRef(false);

  const [check, setCheckState] = useState<PendingCheck | null>(null);
  const checkRef = useRef<PendingCheck | null>(null);
  const setCheck = useCallback((next: PendingCheck | null) => {
    checkRef.current = next;
    setCheckState(next);
  }, []);
  const checkCounter = useRef(0);

  /** An Open or Save dialog is up, or a write is running: its answer belongs to the project that
   * was open when it started, so nothing that ends that project starts meanwhile. */
  const busy = useRef(createBusyGate()).current;
  const quittingRef = useRef(false);

  /** Re-reads the session's dirty flag and, when it changed, tells the host and the page title.
   * Runs every frame, before the paint of the frame in which a commit landed, so the title and the
   * dot change with the canvas (criterion 31). */
  const refreshDirty = useCallback(() => {
    const next = depsRef.current.editor.getSession()?.is_dirty() ?? false;
    if (next === dirtyRef.current) {
      return;
    }
    dirtyRef.current = next;
    setDirty(next);
    void invoke("set_dirty", { dirty: next }).catch(() => undefined);
    if (!isTauri) {
      document.title = projectTitle(infoRef.current.fileName, next);
    }
  }, []);

  useEffect(() => {
    // The next frame is scheduled first: a read that throws must not end dirty tracking, without
    // which the host is never told and a window close is never held.
    let frame = requestAnimationFrame(function tick() {
      frame = requestAnimationFrame(tick);
      refreshDirty();
    });
    // A hidden or minimised window gets no frames, and a commit just before it hid would never be
    // reported: the change of visibility reads once more, and a slow timer covers an engine that
    // does not tell.
    const onVisibility = () => refreshDirty();
    document.addEventListener("visibilitychange", onVisibility);
    const timer = window.setInterval(refreshDirty, 1000);
    return () => {
      cancelAnimationFrame(frame);
      document.removeEventListener("visibilitychange", onVisibility);
      window.clearInterval(timer);
    };
  }, [refreshDirty]);

  // The browser's tab title follows the file name too; the desktop host sets the window title.
  useEffect(() => {
    if (!isTauri) {
      document.title = projectTitle(info.fileName, dirty);
    }
  }, [info.fileName, dirty]);

  // The one popup the page cannot avoid: reloading or closing it with unsaved changes. It is
  // registered only while dirty, in the desktop build too (a WebView2 reload is seen by no host
  // event), and Quit switches it off first.
  useEffect(() => {
    if (!dirty) {
      return undefined;
    }
    const onUnload = (event: BeforeUnloadEvent) => {
      if (quittingRef.current) {
        return;
      }
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", onUnload);
    return () => window.removeEventListener("beforeunload", onUnload);
  }, [dirty]);

  // ---- the actions -------------------------------------------------------------------------

  /** Parses `bytes` as the new live session and only on success tells the host to treat `path` as
   * the project's path (`0001` criterion 7: a refusal never touches the open project). */
  const openBytes = useCallback(
    async (path: string, bytes: number[]) => {
      const { editor, onOpenError } = depsRef.current;
      try {
        await editor.openProject(new Uint8Array(bytes));
      } catch (error) {
        onOpenError(String(error));
        return;
      }
      await invoke("confirm_project_opened", { path }).catch(() => {
        /* The window title falls behind, but the document itself opened fine. */
      });
      setInfo({ fileName: baseName(path), path });
    },
    [setInfo],
  );

  const openFlow = useCallback(async () => {
    if (!busy.enter()) {
      return;
    }
    try {
      const picked = await invoke<OpenPayload | null>("pick_project");
      if (!picked) {
        return;
      }
      if (picked.kind === "io_error") {
        depsRef.current.onOpenError(picked.message);
        return;
      }
      await openBytes(picked.path, picked.bytes);
    } catch (error) {
      depsRef.current.onOpenError(String(error));
    } finally {
      busy.leave();
    }
  }, [busy, openBytes]);

  const loadTemplate = useCallback(async () => {
    if (!busy.enter()) {
      return;
    }
    const { editor, onOpenError } = depsRef.current;
    try {
      const picked = await invoke<OpenPayload | null>("pick_project");
      if (!picked) {
        return;
      }
      if (picked.kind === "io_error") {
        onOpenError(picked.message);
        return;
      }
      const shown = templateOutcome(editor.loadTemplate(new Uint8Array(picked.bytes)));
      if ("notice" in shown) {
        editor.showHistoryNotice(shown.notice);
      } else {
        onOpenError(shown.error);
      }
    } catch (error) {
      onOpenError(String(error));
    } finally {
      busy.leave();
    }
  }, [busy]);

  const resetProject = useCallback(async () => {
    // The host hears "clean" only once the new session exists: if the attach fails, the project is
    // still the old one.
    await depsRef.current.editor.newProject();
    await invoke("reset_project").catch(() => undefined);
    setInfo({ fileName: null, path: null });
  }, [setInfo]);

  const quit = useCallback(async () => {
    quittingRef.current = true;
    try {
      await invoke("quit_app");
    } catch {
      quittingRef.current = false;
    }
  }, []);

  const runAction = useCallback(
    async (action: Action) => {
      switch (action.kind) {
        case "new":
        case "close":
          await resetProject();
          break;
        case "open":
          await openFlow();
          break;
        case "handover":
          await openBytes(action.path, action.bytes);
          break;
        case "quit":
          await quit();
          break;
      }
    },
    [openBytes, openFlow, quit, resetProject],
  );

  /** Save and Save as: `"saved"` once the bytes are written, `"cancelled"` when the maker closed
   * the dialog, `"failed"` after the error dialog was raised. */
  const save = useCallback(
    async (saveAs: boolean): Promise<"saved" | "cancelled" | "failed"> => {
      if (!busy.enter()) {
        return "cancelled";
      }
      const { editor, onSaveError } = depsRef.current;
      try {
        let bytes: Uint8Array;
        try {
          bytes = editor.packProject();
        } catch {
          onSaveError(SAVE_FAILED);
          return "failed";
        }
        const outcome = await invoke<SaveOutcome>("save_project_bytes", {
          bytes: Array.from(bytes),
          saveAs,
          fileName: infoRef.current.fileName ?? DEFAULT_FILE_NAME,
        });
        if (outcome.kind === "cancelled") {
          return "cancelled";
        }
        if (outcome.kind === "failed") {
          onSaveError(outcome.message);
          return "failed";
        }
        editor.markSaved();
        refreshDirty();
        setInfo({ fileName: outcome.file_name, path: outcome.path });
        editor.showHistoryNotice(savedNotice(outcome.file_name, !isTauri));
        return "saved";
      } catch (error) {
        onSaveError(typeof error === "string" ? error : SAVE_FAILED);
        return "failed";
      } finally {
        busy.leave();
      }
    },
    [busy, refreshDirty, setInfo],
  );

  // ---- the check ---------------------------------------------------------------------------

  /** The one place that reads the dirty state: a clean project runs the action, a dirty one
   * raises the check in the File tab (criteria 25 to 28). */
  const request = useCallback(
    (action: Action, origin: Origin) => {
      // A dialog or a write is running: this action waits for the maker to ask again.
      if (busy.isBusy()) {
        return;
      }
      // A commit of this very frame counts: the flag is read now, not at the last frame.
      refreshDirty();
      if (!dirtyRef.current) {
        void runAction(action);
        return;
      }
      const { panelOpen, panel } = depsRef.current;
      const inFileTab = panelOpen.isOpen() && panel.view.activeTab === "file";
      // A second request keeps the view the first one found; Cancel returns to it.
      const previous =
        checkRef.current?.previous ??
        (inFileTab ? null : { tab: panel.view.activeTab, open: panelOpen.isOpen() });
      checkCounter.current += 1;
      setCheck({ id: checkCounter.current, kind: action.kind, origin, previous, action });
      if (!inFileTab) {
        // Not the maker's choice: the saved hidden or shown state stays as it was.
        panelOpen.setOpen(true, false);
        panel.pressTab("file");
      }
    },
    [busy, refreshDirty, runAction, setCheck],
  );

  const saveFromCheck = useCallback(async () => {
    const pending = checkRef.current;
    if (!pending) {
      return;
    }
    const result = await save(false);
    if (result === "saved") {
      // A second request may have replaced the first while the dialog was up.
      const current = checkRef.current ?? pending;
      setCheck(null);
      await runAction(current.action);
    } else if (result === "failed") {
      // A failed write drops the action: the project stays open and dirty.
      setCheck(null);
    }
    // A cancelled dialog leaves the block showing and the action pending.
  }, [runAction, save, setCheck]);

  const discard = useCallback(() => {
    const pending = checkRef.current;
    if (!pending || busy.isBusy()) {
      return;
    }
    setCheck(null);
    void runAction(pending.action);
  }, [busy, runAction, setCheck]);

  const cancel = useCallback(() => {
    const pending = checkRef.current;
    if (!pending) {
      return;
    }
    setCheck(null);
    const { panelOpen, panel, editor } = depsRef.current;
    if (pending.previous) {
      if (!panel.pressTab(pending.previous.tab)) {
        panel.pressTab("document");
      }
      if (!pending.previous.open) {
        panelOpen.setOpen(false, false);
      }
    }
    // The buttons come back with the focus on the one that was pressed, or on the canvas.
    window.setTimeout(() => {
      const origin = pending.origin;
      const button =
        origin !== "outside" && depsRef.current.panelOpen.isOpen()
          ? document.querySelector<HTMLElement>(`[data-file-command="${origin.button}"]`)
          : null;
      (button ?? editor.containerRef.current)?.focus();
    }, 0);
  }, [setCheck]);

  // The check lives in the File tab. Leaving the tab or hiding the body drops it without
  // writing anything, as every inline confirm does when the maker moves on.
  const inFileTab = deps.panelOpen.open && deps.panel.view.activeTab === "file";
  useEffect(() => {
    if (check && !inFileTab) {
      setCheck(null);
    }
  }, [check, inFileTab, setCheck]);

  // ---- the commands ------------------------------------------------------------------------

  const run = useCallback(
    (command: FileCommandName, origin: Origin) => {
      if (checkRef.current && (command === "save" || command === "save-as")) {
        // Ctrl+S and Ctrl+Shift+S while the check shows press its Save.
        void saveFromCheck();
        return;
      }
      switch (command) {
        case "new":
          request({ kind: "new" }, origin);
          break;
        case "open":
          request({ kind: "open" }, origin);
          break;
        case "close":
          request({ kind: "close" }, origin);
          break;
        case "quit":
          if (isTauri) {
            request({ kind: "quit" }, origin);
          }
          break;
        case "save":
          void save(false);
          break;
        case "save-as":
          void save(true);
          break;
      }
    },
    [request, save, saveFromCheck],
  );

  const press = useCallback(
    (command: FileCommandName) => run(command, { button: command }),
    [run],
  );

  // The file chords: one listener for every focus. The table, the typed-character match and the
  // gate are the session's; a text field commits its edit when it loses focus, and then the
  // action runs, so the saved file has the typed value (criteria 32 and 33).
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const { editor } = depsRef.current;
      const session = editor.getSession();
      if (!session) {
        return;
      }
      // An error dialog is up: the keys wait for it.
      if (document.querySelector('[role="alertdialog"]')) {
        return;
      }
      const primary = MAC ? event.metaKey : event.ctrlKey;
      const blocked = event.isComposing || document.documentElement.hasAttribute("aria-busy");
      const name = session.file_chord(
        event.key,
        event.shiftKey,
        primary,
        event.altKey,
        event.repeat,
        blocked,
        PLATFORM,
      );
      if (name === "") {
        return;
      }
      event.preventDefault();
      const field = isTextField(document.activeElement) ? document.activeElement : null;
      if (field) {
        field.blur();
        editor.containerRef.current?.focus();
      }
      run(name as FileCommandName, "outside");
      // A save leaves the maker where they typed; the other commands end or replace the project.
      if (field?.isConnected && (name === "save" || name === "save-as")) {
        field.focus();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [run]);

  // What the host sends: a file the operating system hands to the running app, the close of the
  // window while there are unsaved changes. A page that starts (a reload included) holds an empty
  // project, so the host forgets the path and dirty flag of the one before, and only then hands
  // over a file that came with the launch.
  useEffect(() => {
    invoke("reset_project")
      .catch(() => undefined)
      .then(() => invoke<OpenPayload | null>("take_pending_open"))
      .then((pending) => {
        if (!pending) {
          return;
        }
        if (pending.kind === "bytes") {
          request({ kind: "handover", path: pending.path, bytes: pending.bytes }, "outside");
        } else {
          depsRef.current.onOpenError(pending.message);
        }
      })
      .catch(() => {
        /* Nothing pending is indistinguishable from an unreachable host. */
      });
    const unlisteners = [
      listen<OpenBytesPayload>("open-bytes", (event) =>
        request(
          { kind: "handover", path: event.payload.path, bytes: event.payload.bytes },
          "outside",
        ),
      ),
      listen<OpenErrorPayload>("open-error", (event) =>
        depsRef.current.onOpenError(event.payload.message),
      ),
      listen("close-requested", () => request({ kind: "quit" }, "outside")),
    ];
    return () => {
      for (const unlisten of unlisteners) {
        void unlisten.then((stop) => stop());
      }
    };
  }, [request]);

  return {
    info,
    dirty,
    check,
    press,
    loadTemplate: () => void loadTemplate(),
    saveFromCheck: () => void saveFromCheck(),
    discard,
    cancel,
  };
}

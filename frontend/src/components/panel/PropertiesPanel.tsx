import { panelHandlesEscape } from "@/lib/panelEscape";
import { useCallback, useEffect, useId, useRef } from "react";

import { DocumentSection } from "@/components/panel/DocumentSection";
import { FileTab } from "@/components/panel/FileTab";
import { OffsetSection } from "@/components/panel/OffsetSection";
import { HistoryTab } from "@/components/panel/HistoryTab";
import { PanelStrip } from "@/components/panel/PanelStrip";
import { StyleSection } from "@/components/panel/StyleSection";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { Tooltip } from "@/components/ui/tooltip";
import type { DocumentPanelApi } from "@/hooks/useDocumentPanel";
import type { EditorSession } from "@/hooks/useEditorSession";
import type { PanelOpenApi } from "@/hooks/usePanelOpen";
import type { ProjectApi } from "@/hooks/useProject";
import { useBackgroundPanel } from "@/hooks/useBackgroundPanel";
import { useFormats } from "@/hooks/useFormats";
import { useHistoryScope } from "@/hooks/useHistoryScope";
import { useStylePanel } from "@/hooks/useStylePanel";
import { appVersion, fileCommandTooltip } from "@/lib/editorSession";
import { isTauri, PLATFORM } from "@/platform/host";

interface PropertiesPanelProps {
  editor: EditorSession;
  document: DocumentPanelApi;
  /** Whether the body is shown; owned by the app so a request from outside the panel can show it. */
  panelOpen: PanelOpenApi;
  /** The open project: the File tab shows it and the strip carries its dirty dot. */
  project: ProjectApi;
}

/**
 * The right-docked properties panel (`specs/0007-stroke-and-fill-styling` criteria 38, 39;
 * `specs/0051-sidebar-and-no-menu/` criteria 1 to 14): a 32 px strip of icon tabs on the edge next
 * to the canvas and, to its right, the 280 px body, full height down to the status bar, a sibling of
 * the canvas region and never an overlay. The collapse control in the strip hides the body; the
 * strip stays. A hidden body is out of the tab order and the canvas region takes its width; the
 * document does not move on screen because the resize keeps the view's top-left origin.
 * `Shift+Ctrl+F` (Style, or Document with nothing selected), `Shift+Ctrl+D` (Document) and
 * `Shift+Ctrl+H` (History) show the body, choose the tab and move focus to its first control; they
 * never hide it and are ignored during a canvas drag.
 *
 * The header row (the active tab's name and the subject line) does not scroll; the body below it
 * does (`specs/0043-properties-tabs/` criteria 18, 19). Which tab is active, and when it switches,
 * is the session's rule.
 */
export function PropertiesPanel({
  editor,
  document: doc,
  panelOpen,
  project,
}: PropertiesPanelProps) {
  const { open, setOpen } = panelOpen;
  const panelId = useId();
  const asideRef = useRef<HTMLElement>(null);
  /** A shortcut asked for the first control: it is focused once the chosen
   * tab's body has rendered (and the panel has opened). */
  const focusAfterShortcut = useRef(false);
  const panel = useStylePanel(editor);
  const background = useBackgroundPanel(editor);
  const { getSession } = editor;
  const formats = useFormats(getSession, doc);
  const { shortcut } = doc;
  const returnFocus = () => editor.containerRef.current?.focus();
  const picking = panel.view.pickTarget !== "" || background.view.picking;
  const endPick = () => {
    panel.endPick();
    background.endPick();
  };

  /** Focus lands on the first control (Stroke Paint), or on the panel itself
   * when it is empty. */
  const focusFirstControl = useCallback(() => {
    const aside = asideRef.current;
    if (!aside) {
      return;
    }
    const first =
      aside.querySelector<HTMLElement>("[data-first-focus]") ??
      aside.querySelector<HTMLElement>("input, button");
    const target = first ?? aside;
    // A control focused by the shortcut shows the focus ring even if the
    // webview does not count script focus as keyboard focus; the mark goes
    // when the focus does.
    target.focus({ focusVisible: true } as FocusOptions);
    if (document.activeElement !== target) {
      // The target handed focus on (the History list to its current row).
      return;
    }
    target.setAttribute("data-keyboard-focus", "");
    target.addEventListener("blur", () => target.removeAttribute("data-keyboard-focus"), {
      once: true,
    });
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      if (
        !(event.shiftKey && (event.ctrlKey || event.metaKey)) ||
        (key !== "f" && key !== "d" && key !== "h")
      ) {
        return;
      }
      event.preventDefault();
      if (getSession()?.pointer_is_down()) {
        return;
      }
      focusAfterShortcut.current = true;
      shortcut(key === "f" ? "style" : key === "h" ? "history" : "document");
      setOpen(true);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [getSession, shortcut, setOpen]);

  useEffect(() => {
    if (open && focusAfterShortcut.current) {
      focusAfterShortcut.current = false;
      focusFirstControl();
    }
  }, [open, doc.view, focusFirstControl]);

  // A control that had keyboard focus can leave the tree when the selection or
  // the tool changes (the panel becomes empty or shows the Document section):
  // focus goes to the canvas, not to the page body (`0017` criterion 1).
  const focusInside = useRef(false);
  const content = doc.view.content;
  useEffect(() => {
    const aside = asideRef.current;
    if (focusInside.current && aside && !aside.contains(window.document.activeElement)) {
      focusInside.current = false;
      returnFocus();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [content, panel.view.scopeKey]);

  const historyScope = useHistoryScope(
    getSession,
    editor.syncRevision,
    editor.selectionCount,
    doc.view.content === "history",
  );
  const objectSubject = historyScope.scope === "object" ? historyScope.info : null;
  const historyTotal = doc.view.content === "history" ? historyCount(editor) : 0;
  const subject =
    doc.view.content === "file"
      ? project.dirty
        ? "Unsaved changes"
        : ""
      : doc.view.content === "history"
      ? `${historyTotal} ${historyTotal === 1 ? "step" : "steps"}`
      : doc.view.content === "document"
      ? doc.view.presets.subject
      : doc.view.content === "style"
        ? panel.view.subject
        : "";
  const activeLabel = doc.view.tabs.find((tab) => tab.name === doc.view.activeTab)?.label ?? "";
  /** A press on a tab (or Enter, Space on a focused one) shows the body with that tab. */
  const activate = (name: string) => {
    setOpen(true);
    doc.pressTab(name);
  };
  return (
    <Tabs
      value={doc.view.activeTab}
      onValueChange={activate}
      orientation="vertical"
      // Hidden, the arrows only move the focus; Enter or Space opens the focused tab.
      activationMode={open ? "automatic" : "manual"}
      className="flex h-full shrink-0"
    >
      <PanelStrip
        tabs={doc.view.tabs}
        active={doc.view.activeTab}
        open={open}
        bodyId={panelId}
        fileDirty={project.dirty}
        onToggle={(event) => {
          // A press on the collapse control ends picking too (`0040` UX notes).
          if (picking) {
            endPick();
          }
          setOpen(!open);
          // `detail` is 0 for keyboard activation, 1 or more for a click.
          if (event.detail > 0) {
            returnFocus();
          }
        }}
        onActivate={activate}
        onReturnFocus={returnFocus}
      />
      <aside
        id={panelId}
        ref={asideRef}
        aria-label="Properties"
        tabIndex={-1}
        hidden={!open}
        onFocus={() => {
          focusInside.current = true;
        }}
        onBlur={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
            focusInside.current = false;
          }
        }}
        onMouseDown={(event) => {
          // A press on dead space (the heading, a label) must not leave the
          // focus on the panel, where the tool letters would do nothing: it
          // goes back to the canvas. A control keeps the press.
          if (
            !(event.target as HTMLElement).closest(
              "input, button, [role]:not([role=tabpanel]), select",
            )
          ) {
            event.preventDefault();
            returnFocus();
          }
        }}
        onPointerDownCapture={(event) => {
          // A press anywhere in the panel other than the eyedropper button ends
          // picking and writes nothing (`0017` criterion 26).
          if (picking && !(event.target as HTMLElement).closest("[data-eyedropper]")) {
            endPick();
          }
        }}
        onKeyDown={(event) => {
          // Escape in the panel (`0017` criterion 60): a running drag is
          // reverted first (`usePreviewGesture` takes the key before it gets
          // here); a field restores itself and returns focus; otherwise focus
          // returns to the canvas, unless a field already handled the key (its
          // `preventDefault`), as the hex field does for the slider it came from.
          // It never clears the selection and never reaches the canvas.
          if (panelHandlesEscape(event) && event.currentTarget.contains(event.target as Node)) {
            event.preventDefault();
            // Picking ends first and keeps the focus where it is.
            if (picking) {
              endPick();
              return;
            }
            returnFocus();
          }
        }}
        className="properties-panel flex h-full w-[280px] flex-col overflow-hidden bg-[var(--panel-bg)] outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
      >
        <div className="box-content flex h-6 shrink-0 items-center justify-between gap-2 px-3 pt-3 pb-2">
          <h2 className="shrink-0 text-sm font-normal text-[var(--toolbar-icon)]">{activeLabel}</h2>
          {doc.view.content === "history" && objectSubject ? (
            <ObjectSubject kind={objectSubject.kind} id={objectSubject.id} fullId={objectSubject.fullId} />
          ) : (
            <PanelSubject text={subject} />
          )}
        </div>
        {/* Keyed on the tab: the scroll position starts at the top on every
            tab change (criterion 19). */}
        <div
          key={doc.view.activeTab}
          className={
            doc.view.content === "history"
              ? // The one tab with an inner scroll area, the list: the header, the buttons
                // and the footer stay in view (`0020` criterion 43).
                "flex min-h-0 flex-1 flex-col px-3 pb-3"
              : "properties-panel-body min-h-0 flex-1 px-3 pb-3"
          }
        >
          <TabsContent
            value={doc.view.activeTab}
            tabIndex={-1}
            className={
              doc.view.content === "history"
                ? "flex min-h-0 flex-1 flex-col outline-none"
                : "outline-none"
            }
          >
            {doc.view.content === "file" ? (
              <FileTab
                project={project}
                desktop={isTauri}
                version={appVersion()}
                tooltipFor={(command) => fileCommandTooltip(command, PLATFORM)}
                onReturnFocus={returnFocus}
              />
            ) : doc.view.content === "document" ? (
              <DocumentSection
                document={doc}
                formats={formats}
                background={background}
                onReturnFocus={returnFocus}
              />
            ) : doc.view.content === "style" ? (
              <>
                {editor.offset.open && (
                  <OffsetSection
                    offset={editor.offset}
                    onDone={(byKey) => {
                      if (!byKey) {
                        returnFocus();
                        return;
                      }
                      // After the state sync has re-rendered the Select bar and removed the
                      // section: two frames, so the focus lands on the button that stays.
                      requestAnimationFrame(() =>
                        requestAnimationFrame(() => {
                          const button = window.document.querySelector<HTMLElement>(
                            "[data-offset-button]",
                          );
                          if (button) {
                            button.focus();
                          } else {
                            returnFocus();
                          }
                        }),
                      );
                    }}
                  />
                )}
                <StyleSection panel={panel} onReturnFocus={returnFocus} />
              </>
            ) : doc.view.content === "history" ? (
              <HistoryTab
                editor={editor}
                shown={open}
                onReturnFocus={returnFocus}
                historyScope={historyScope}
              />
            ) : null}
          </TabsContent>
        </div>
      </aside>
    </Tabs>
  );
}

/** The subject line of the Object scope: the kind (it shortens with an ellipsis) and the first
 * 6 characters of the id (never shortened); the tooltip has the kind and the full id as text
 * that can be selected (`specs/0041-object-history` criterion 2). */
function ObjectSubject({ kind, id, fullId }: { kind: string; id: string; fullId: string }) {
  return (
    <Tooltip
      side="left"
      content={
        <span className="select-text">
          {kind}, id {fullId}
        </span>
      }
    >
      <p className="flex min-w-0 max-w-[120px] gap-1 text-xs text-[var(--panel-muted-fg)]">
        <span className="min-w-0 truncate">{kind}</span>
        <span className="shrink-0 select-text font-mono">{id}</span>
      </p>
    </Tooltip>
  );
}

/** The subject line at the right of the header row: 12 px muted, one line, the
 * full text in a tooltip. */
function PanelSubject({ text }: { text: string }) {
  if (text === "") {
    return null;
  }
  return (
    <Tooltip side="left" content={text}>
      <p className="min-w-0 truncate text-xs text-[var(--panel-muted-fg)]">{text}</p>
    </Tooltip>
  );
}

/** The number of steps for the subject line of the History tab. */
function historyCount(editor: EditorSession): number {
  // `syncRevision` makes this read again after every change of the session.
  void editor.syncRevision;
  return editor.getSession()?.history_len() ?? 0;
}

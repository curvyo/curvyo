import { TracePresetMenu } from "@/components/TracePresetMenu";
import { SketchParameters } from "@/components/sketch/SketchParameters";
import type { SketchApi } from "@/hooks/useSketch";
import { SketchesPanel } from "@/components/sketch/SketchesPanel";
import { panelHandlesEscape } from "@/lib/panelEscape";
import { FontsTab } from "@/components/panel/FontsTab";
import { useTextLayout } from "@/hooks/useTextLayout";
import { focusCurrentFont, fontsSubject } from "@/lib/textText";
import { useCallback, useEffect, useId, useRef } from "react";

import { DocumentSection } from "@/components/panel/DocumentSection";
import { ExportSvgSection } from "@/components/panel/ExportSvgSection";
import { FileTab } from "@/components/panel/FileTab";
import { HistoryTab } from "@/components/panel/HistoryTab";
import { LayersTab } from "@/components/panel/LayersTab";
import { PanelStrip } from "@/components/panel/PanelStrip";
import { GridSection } from "@/components/panel/GridSection";
import { SnappingSection } from "@/components/panel/SnappingSection";
import { StyleSection } from "@/components/panel/StyleSection";
import { TraceInfoPage } from "@/components/panel/TraceInfoPage";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { Tooltip } from "@/components/ui/tooltip";
import type { DocumentPanelApi } from "@/hooks/useDocumentPanel";
import type { EditorSession } from "@/hooks/useEditorSession";
import type { PanelOpenApi } from "@/hooks/usePanelOpen";
import type { ProjectApi } from "@/hooks/useProject";
import { TRACE_INFO_TAB, type TraceInfoApi } from "@/hooks/useTraceInfo";
import { useBackgroundPanel } from "@/hooks/useBackgroundPanel";
import { useFormats } from "@/hooks/useFormats";
import { useHistoryScope } from "@/hooks/useHistoryScope";
import { useGrid } from "@/hooks/useGrid";
import { useSnapping } from "@/hooks/useSnapping";
import { useStylePanel } from "@/hooks/useStylePanel";
import { appVersion, fileCommandTooltip } from "@/lib/editorSession";
import { TRACE_INFO_TITLE } from "@/lib/traceText";
import { isTauri, PLATFORM } from "@/platform/host";

interface PropertiesPanelProps {
  sketch: SketchApi;
  editor: EditorSession;
  document: DocumentPanelApi;
  /** Whether the body is shown; owned by the app so a request from outside the panel can show it. */
  panelOpen: PanelOpenApi;
  /** The open project: the File tab shows it and the strip carries its dirty dot. */
  project: ProjectApi;
  /** The Trace tool's About Trace tab, last in the strip while the tool is active. */
  traceInfo: TraceInfoApi;
}

/**
 * The right-docked properties panel (`specs/0007-stroke-and-fill-styling` criteria 38, 39;
 * `specs/0051-sidebar-and-no-menu/` criteria 1 to 14): a 32 px strip of icon tabs on the edge next
 * to the canvas and, to its right, the 280 px body, full height down to the status bar, a sibling of
 * the canvas region and never an overlay. The collapse control in the strip hides the body; the
 * strip stays. A hidden body is out of the tab order and the canvas region takes its width; the
 * document does not move on screen because the resize keeps the view's top-left origin.
 * `Shift+Ctrl+F` (Style, or Document with nothing selected), `Shift+Ctrl+D` (Document) and
 * `Shift+Ctrl+H` (History) and `Shift+Ctrl+L` (Layers) show the body, choose the tab and move focus to its first control; they
 * never hide it and are ignored during a canvas drag.
 *
 * The header row (the active tab's name and the subject line) does not scroll; the body below it
 * does (`specs/0043-properties-tabs/` criteria 18, 19). Which tab is active, and when it switches,
 * is the session's rule.
 */
export function PropertiesPanel({
  sketch,
  editor,
  document: doc,
  panelOpen,
  project,
  traceInfo,
}: PropertiesPanelProps) {
  const { open, setOpen } = panelOpen;
  const panelId = useId();
  const asideRef = useRef<HTMLElement>(null);
  /** A shortcut asked for the first control: it is focused once the chosen
   * tab's body has rendered (and the panel has opened). */
  const focusAfterShortcut = useRef(false);
  const panel = useStylePanel(editor);
  const background = useBackgroundPanel(editor);
  const textLayout = useTextLayout(editor.getSession, editor.sync, editor.syncRevision);
  const { getSession } = editor;
  const formats = useFormats(getSession, doc);
  const snapping = useSnapping({ getSession, syncRevision: editor.syncRevision, sync:editor.sync });
  const grid = useGrid({ getSession, syncRevision: editor.syncRevision, sync: editor.sync });
  const { shortcut } = doc;
  // In text edit mode the keyboard goes back to the text being edited, so typing goes on after a
  // click on a control (customer 2026-10-26), as after the Text bar's controls.
  const returnFocus = () => {
    if (sketch.view.workbench === "vector" && editor.text.editing) {
      editor.text.sinkRef.current?.focus({ preventScroll: true });
    } else {
      editor.containerRef.current?.focus();
    }
  };
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
        (key !== "f" && key !== "d" && key !== "h" && key !== "l" && key !== "t")
      ) {
        return;
      }
      // Shift+Ctrl+T is the desktop's only: the browser keeps Chrome's key (`0074` criterion 15).
      if (key === "t" && !isTauri) {
        return;
      }
      event.preventDefault();
      if (getSession()?.pointer_is_down()) {
        return;
      }
      if (key === "t") {
        traceInfo.dismiss();
        shortcut("fonts");
        setOpen(true);
        focusCurrentFont();
        return;
      }
      traceInfo.dismiss();
      focusAfterShortcut.current = true;
      shortcut(
        key === "f" ? "style" : key === "h" ? "history" : key === "l" ? "layers" : "document",
      );
      setOpen(true);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [getSession, shortcut, setOpen, traceInfo]);

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
      : doc.view.content === "layers"
      ? (getSession()?.layers_subject() ?? "")
      : doc.view.content === "document"
      ? doc.view.presets.subject
      : doc.view.content === "style"
        ? panel.view.subject
        : doc.view.content === "fonts"
          ? fontsSubject(editor.text.fonts)
          : "";
  const activeLabel = doc.view.tabs.find((tab) => tab.name === doc.view.activeTab)?.label ?? "";
  // The About Trace tab is the frontend's, appended last so no other tab moves (UX notes 21.1).
  const tabs = traceInfo.available
    ? [...doc.view.tabs, { name: TRACE_INFO_TAB, label: TRACE_INFO_TITLE, tooltip: TRACE_INFO_TITLE, enabled: true }]
    : doc.view.tabs;
  const aboutActive = traceInfo.available && traceInfo.active;
  const activeTab = aboutActive ? TRACE_INFO_TAB : doc.view.activeTab;
  /** A press on a tab (or Enter, Space on a focused one) shows the body with that tab. */
  const activate = (name: string) => {
    if (name === TRACE_INFO_TAB) {
      traceInfo.choose();
      return;
    }
    traceInfo.dismiss();
    setOpen(true);
    doc.pressTab(name);
  };
  return (
    <div className="flex h-full shrink-0 flex-col">
    <div className={`bg-[var(--toolbar-bg)] ${open ? "px-[7px] py-1" : "w-8"}`}>
    <TracePresetMenu name="Workbench" align="end" widthClass={open ? "w-full" : "w-full !px-1 !gap-0"} current={sketch.view.workbench} edited={false}
      label={open ? (sketch.view.workbench === "sketch" ? "Sketch" : "Vector") : (sketch.view.workbench === "sketch" ? "S" : "V")}
      presets={[{key:"vector",label:"Vector",detail:""},{key:"sketch",label:"Sketch",detail:""}]}
      onPick={choice => {traceInfo.dismiss();sketch.switchWorkbench(choice.key as "vector" | "sketch");}} />
    </div>
    <Tabs
      value={activeTab}
      onValueChange={activate}
      orientation="vertical"
      // Hidden, the arrows only move the focus; Enter or Space opens the focused tab.
      activationMode={open ? "automatic" : "manual"}
      className="flex min-h-0 flex-1 shrink-0"
    >
      <PanelStrip
        tabs={tabs}
        active={activeTab}
        open={open}
        bodyId={panelId}
        fileDirty={project.dirty}
        desktop={isTauri}
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
        snappingEnabled={snapping.view.enabled}
        onToggleSnapping={snapping.toggle}
        gridShown={grid.view.shown}
        onToggleGrid={grid.toggle}
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
          {aboutActive ? (
            <h2 className="shrink-0 text-sm font-normal text-[var(--toolbar-icon)]">{TRACE_INFO_TITLE}</h2>
          ) : (
          <>
          <h2 className="shrink-0 text-sm font-normal text-[var(--toolbar-icon)]">{activeLabel}</h2>
          {doc.view.content === "history" && objectSubject ? (
            <ObjectSubject kind={objectSubject.kind} id={objectSubject.id} fullId={objectSubject.fullId} />
          ) : (
            <PanelSubject text={subject} />
          )}
          </>
          )}
        </div>
        {aboutActive ? (
          <div className="flex min-h-0 flex-1 flex-col px-3 pb-3">
            <TabsContent value={TRACE_INFO_TAB} tabIndex={-1} className="flex min-h-0 flex-1 flex-col outline-none">
              <TraceInfoPage onReturnFocus={returnFocus} />
            </TabsContent>
          </div>
        ) : (
        /* Keyed on the tab: the scroll position starts at the top on every
            tab change (criterion 19). */
        <div
          key={doc.view.activeTab}
          className={
            doc.view.content === "history" ||
            doc.view.content === "layers" ||
            doc.view.content === "fonts"
              ? // The tabs with an inner scroll area, the list: the header, the buttons
                // and the footer stay in view (`0020` criterion 43, `0050` criterion 3).
                "flex min-h-0 flex-1 flex-col px-3 pb-3"
              : "properties-panel-body min-h-0 flex-1 px-3 pb-3"
          }
        >
          <TabsContent
            value={doc.view.activeTab}
            tabIndex={-1}
            className={
              doc.view.content === "history" ||
              doc.view.content === "layers" ||
              doc.view.content === "fonts"
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
                exportSvg={<ExportSvgSection project={project} onReturnFocus={returnFocus} />}
              />
            ) : doc.view.content === "parameters" ? (
              <SketchParameters key={sketch.view.active ?? sketch.view.page} sketch={sketch} />
            ) : doc.view.content === "sketches" ? (
              <SketchesPanel key={`${sketch.view.page}:${sketch.view.active}`} sketch={sketch} />
            ) : doc.view.content === "snapping" ? (
              <SnappingSection snapping={snapping} onReturnFocus={returnFocus}/>
            ) : doc.view.content === "grid" ? (
              <GridSection grid={grid} disabled={snapping.view.hasDrag} onReturnFocus={returnFocus}/>
            ) : doc.view.content === "document" ? (
              <DocumentSection
                document={doc}
                formats={formats}
                background={background}
                onReturnFocus={returnFocus}
              />
            ) : doc.view.content === "style" ? (
              <StyleSection panel={panel} clones={editor.clones} onReturnFocus={returnFocus} />
            ) : doc.view.content === "layers" ? (
              <LayersTab
                getSession={getSession}
                syncRevision={editor.syncRevision}
                sync={editor.sync}
                shown={open}
                showNotice={editor.showHistoryNotice}
                onReturnFocus={returnFocus}
                onLinkedClone={() => editor.clones.apply("linked-clone", "layers")}
              />
            ) : doc.view.content === "fonts" ? (
              <FontsTab
                data={editor.text.fonts}
                onPick={editor.text.pickFont}
                onPickFamily={editor.text.pickFamily}
                onReturnFocus={() => {
                  if (sketch.view.workbench === "vector" && editor.text.editing) {
                    editor.text.sinkRef.current?.focus();
                  } else {
                    returnFocus();
                  }
                }}
                onShown={editor.text.listHostFonts}
                textsUsingFamily={editor.text.textsUsingFamily}
                layout={textLayout}
              />
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
        )}
      </aside>
    </Tabs>
    </div>
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
 * full text in a tooltip. It takes the rest of the header after the tabs (design
 * system, "Properties panel: tabs"), so "Compound path" fits at 800 px (`0054` UX
 * review); a longer text such as "Clone of Path 3f9a1c" is still cut with an ellipsis. */
function PanelSubject({ text }: { text: string }) {
  if (text === "") {
    return null;
  }
  return (
    <Tooltip side="left" content={text}>
      <p className="min-w-0 flex-1 truncate text-right text-xs text-[var(--panel-muted-fg)]">{text}</p>
    </Tooltip>
  );
}

/** The number of steps for the subject line of the History tab. */
function historyCount(editor: EditorSession): number {
  // `syncRevision` makes this read again after every change of the session.
  void editor.syncRevision;
  return editor.getSession()?.history_len() ?? 0;
}

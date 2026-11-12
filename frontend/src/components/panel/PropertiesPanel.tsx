import { Frame, TriangleAlert } from "lucide-react";
import { ManifestTab } from "@/components/workbench/ManifestTab";
import { OperationsSection } from "@/components/panel/OperationsSection";
import { FixtureJobsSection } from "@/components/panel/FixtureJobsSection";
import type { FixtureJobsApi } from "@/hooks/useFixtureJobs";
import { MaterialsSection } from "@/components/panel/MaterialsSection";
import type { MaterialsApi } from "@/hooks/useMaterials";
import type { OperationSettingsApi } from "@/hooks/useOperationSettings";
import type { FixturesApi } from "@/hooks/useFixtures";
import type { OperationsApi } from "@/hooks/useOperations";
import { ExtensionsGroup } from "@/components/panel/ExtensionsGroup";
import type { WorkbenchesApi } from "@/hooks/useWorkbenches";
import { contributionOf } from "@/workbenches/registry";
import { KindChip } from "@/components/KindChip";
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
import { WorkspaceSection } from "@/components/panel/WorkspaceSection";
import { HistoryTab } from "@/components/panel/HistoryTab";
import { LayersTab } from "@/components/panel/LayersTab";
import { PanelStrip } from "@/components/panel/PanelStrip";
import { GridSection } from "@/components/panel/GridSection";
import { SnappingSection } from "@/components/panel/SnappingSection";
import { CloneBlock } from "@/components/panel/CloneBlock";
import { StyleSection } from "@/components/panel/StyleSection";
import { TraceInfoPage } from "@/components/panel/TraceInfoPage";
import { ShareTab } from "@/components/panel/ShareTab";
import type { MembershipApi } from "@/hooks/useMembership";
import type { CollaborationApi } from "@/hooks/useCollaboration";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { Tooltip } from "@/components/ui/tooltip";
import type { DocumentPanelApi } from "@/hooks/useDocumentPanel";
import type { EditorSession } from "@/hooks/useEditorSession";
import type { PanelOpenApi } from "@/hooks/usePanelOpen";
import type { ProjectApi } from "@/hooks/useProject";
import type { WorkspaceApi } from "@/hooks/useWorkspace";
import type { WorkspaceSyncApi } from "@/hooks/useWorkspaceSync";
import { useMakeWorkspace } from "@/hooks/useMakeWorkspace";
import type { DevicesApi } from "@/hooks/useDevices";
import { syncedHeader } from "@/lib/devicesApi";
import { TRACE_INFO_TAB, type TraceInfoApi } from "@/hooks/useTraceInfo";
import { useBackgroundPanel } from "@/hooks/useBackgroundPanel";
import { useFormats } from "@/hooks/useFormats";
import { useHistoryScope } from "@/hooks/useHistoryScope";
import { useGrid } from "@/hooks/useGrid";
import { useTestGrid } from "@/hooks/useTestGrid";
import { useSnapping } from "@/hooks/useSnapping";
import { useStylePanel } from "@/hooks/useStylePanel";
import { appVersion, fileCommandTooltip } from "@/lib/editorSession";
import { TRACE_INFO_TITLE } from "@/lib/traceText";
import { isTauri, PLATFORM } from "@/platform/host";

interface PropertiesPanelProps {
  sketch: SketchApi;
  /** The workbench registry: the switcher row, a manufacturing's tab and the Extensions group. */
  workbenches: WorkbenchesApi;
  editor: EditorSession;
  document: DocumentPanelApi;
  /** Whether the body is shown; owned by the app so a request from outside the panel can show it. */
  panelOpen: PanelOpenApi;
  /** The open project: the File tab shows it and the strip carries its dirty dot. */
  project: ProjectApi;
  /** The open workspace (`specs/0095-workspace`): the File tab's Workspace section, desktop only. */
  workspace: WorkspaceApi;
  /** The workspace's sharing in the Share tab (`specs/0095-workspace` criteria 25 to 28). */
  workspaceSync: WorkspaceSyncApi;
  /** My devices (`specs/0096-own-devices-sync`). */
  devices: DevicesApi;
  /** The Trace tool's About Trace tab, last in the strip while the tool is active. */
  traceInfo: TraceInfoApi;
  /** Cooperative editing: the Share tab (`specs/0092-cooperative-editing`). */
  collab: CollaborationApi;
  /** Roles, leave, add back and the notices (`specs/0097-membership-lifecycle`). */
  membership: MembershipApi;
  /** The laser operations of the active laser page (`specs/0101-laser-operations`). */
  operations: OperationsApi;
  /** The laser fixtures of the active laser page (`specs/0102-laser-fixtures`). */
  fixtures: FixturesApi;
  /** The fixture jobs of the active laser page (`specs/0103-fixture-jobs`). */
  fixtureJobs: FixtureJobsApi;
  /** The Settings block of the selected operations (`specs/0028-material-test-library`). */
  operationSettings: OperationSettingsApi;
  /** The Materials section (`specs/0028-material-test-library`). */
  materials: MaterialsApi;
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
  workbenches,
  editor,
  document: doc,
  panelOpen,
  project,
  workspace,
  workspaceSync,
  devices,
  traceInfo,
  collab,
  membership,
  operations,
  fixtures,
  fixtureJobs,
  operationSettings,
  materials,
}: PropertiesPanelProps) {
  const { open, setOpen } = panelOpen;
  // The Share tab mark clears once its body was shown for 1 s, never by hovering (0097 2.1).
  const shareShown = open && doc.view.content === "share";
  const { unread, markRead } = membership;
  useEffect(() => {
    if (!shareShown || !unread) {
      return undefined;
    }
    const timer = window.setTimeout(markRead, 1000);
    return () => window.clearTimeout(timer);
  }, [shareShown, unread, markRead]);
  // The removed-device alert shows in My devices and in the File tab's Workspace section; one OK
  // dismisses both, and the workspace's own removed block with them (0096 ux-notes 7.2).
  const devicesForTabs: DevicesApi = {
    ...devices,
    dismissRemoved: () => {
      devices.dismissRemoved();
      workspace.dismissRemoved();
    },
  };
  const makeWorkspace = useMakeWorkspace(project, workspace);
  const panelId = useId();
  const asideRef = useRef<HTMLElement>(null);
  /** A shortcut asked for the first control: it is focused once the chosen
   * tab's body has rendered (and the panel has opened). */
  const focusAfterShortcut = useRef(false);
  const panel = useStylePanel(editor);
  const background = useBackgroundPanel(editor);
  const textLayout = useTextLayout(editor.getSession, editor.sync, editor.syncRevision);
  const { getSession } = editor;
  // "On Drawing 2. Double-click to edit." while the selection is move-only (`specs/0106`).
  // `syncRevision` re-renders the panel after every session change.
  const outOfContext = getSession()?.out_of_context_line() ?? "";
  const placedPage = getSession()?.placed_page_block() ?? "";
  const formats = useFormats(getSession, doc);
  const snapping = useSnapping({ getSession, syncRevision: editor.syncRevision, sync:editor.sync });
  const grid = useGrid({ getSession, syncRevision: editor.syncRevision, sync: editor.sync });
  // The test grid of the Operations tab (`specs/0028-material-test-library`).
  const testGrid = useTestGrid({ getSession, syncRevision: editor.syncRevision, sync: editor.sync });
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
  const manufacturing = workbenches.view;
  const contribution = contributionOf(workbenches.list, manufacturing?.process);
  const page = workbenches.list.page;
  // The kind chip sits in the header of the page-scoped tabs only (`specs/0098` criterion 6).
  const pageScoped = ["document", "style", "layers", "snapping", "grid", "sketches", "parameters"].includes(
    doc.view.content,
  );
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
      ? page.name
      : doc.view.content === "style"
        ? panel.view.subject
        : doc.view.content === "fonts"
          ? fontsSubject(editor.text.fonts)
          : doc.view.content === "share"
            ? collab.view.subject
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
        shareMark={membership.unread}
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
          {pageScoped && page.id ? (
            <span
              role="img"
              aria-label={`${page.word} page`}
              className="shrink-0 rounded-md px-1.5 py-0.5 text-[var(--toolbar-icon)]"
              style={{ border: "1px solid color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
            >
              <KindChip hidden size={12} kind={{ icon: page.icon, word: page.word, off: page.off }} />
            </span>
          ) : null}
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
                // Operations is not one: its column scrolls as a whole, so the Fixture jobs
                // section, the summary and the View switch stay reachable (`0103` ux-notes 3.2).
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
                extensions={<ExtensionsGroup workbenches={workbenches} />}
                workspace={
                  // The browser holds synced workspaces only (`specs/0096-own-devices-sync`
                  // criterion 35): the section is shown once it holds one.
                  isTauri || workspace.open || workspace.recents.length > 0 ? (
                    <WorkspaceSection
                      workspace={workspace}
                      sharedLine={syncedHeader(devicesForTabs, collab.view) ?? collab.view.workspace?.headerLine}
                      documentLine={collab.view.workspace?.openInWorkspace ? collab.view.workspace.documentLine : undefined}
                      devicesRemoved={devicesForTabs.view?.removed ? devicesForTabs.dismissRemoved : undefined}
                      onReturnFocus={returnFocus}
                    />
                  ) : undefined
                }
              />
            ) : doc.view.content === "share" ? (
              <ShareTab
                collab={collab}
                desktop={isTauri}
                makeWorkspace={makeWorkspace}
                visible={open}
                onReturnFocus={returnFocus}
                devices={devicesForTabs}
                membership={membership}
                workspace={
                  isTauri || workspace.open
                    ? {
                        open: workspace.open,
                        name: workspace.name,
                        documents: workspace.view.documents,
                        sync: workspaceSync,
                      }
                    : undefined
                }
              />
            ) : doc.view.content === "parameters" ? (
              <SketchParameters key={sketch.view.active ?? sketch.view.page} sketch={sketch} />
            ) : doc.view.content === "sketches" ? (
              <SketchesPanel key={`${sketch.view.page}:${sketch.view.active}`} sketch={sketch} pages={doc.view.pages} />
            ) : doc.view.content === "snapping" ? (
              <SnappingSection snapping={snapping} fixtures={fixtures} onReturnFocus={returnFocus}/>
            ) : doc.view.content === "grid" ? (
              <GridSection grid={grid} disabled={snapping.view.hasDrag} onReturnFocus={returnFocus}/>
            ) : doc.view.content === "document" ? (
              <DocumentSection
                document={doc}
                formats={formats}
                background={background}
                workbenches={workbenches}
                machine={
                  manufacturing && contribution ? (
                    <ManifestTab
                      contribution={contribution}
                      pages={doc.view.pages}
                      view={manufacturing}
                      workbenches={workbenches}
                      unit={doc.view.unit}
                      fixtures={fixtures}
                      onReturnFocus={returnFocus}
                    />
                  ) : page.off ? (
                    <SwitchedOffNotice word={page.word.replace(/ \(off\)$/, "")} />
                  ) : null
                }
                onReturnFocus={returnFocus}
              />
            ) : doc.view.content === "style" ? (
              outOfContext !== "" ? (
                // A move-only selection: the one muted line, every section hidden (`specs/0106`
                // criterion 11, UX notes §3).
                <div className="flex flex-col gap-4">
                  <p
                    role="note"
                    title={outOfContext}
                    className="mt-2 truncate text-xs text-[var(--panel-muted-fg)]"
                  >
                    {outOfContext}
                  </p>
                  {/* Linked clones out of context: the reduced block, select buttons only. */}
                  <CloneBlock clones={editor.clones} moveOnly onReturnFocus={returnFocus} onPressedAt={() => {}} />
                </div>
              ) : (
                <>
                  <PlacedPageBlock text={placedPage} />
                  {/* A placed page has no style of its own: no Stroke, no Fill (`specs/0106`
                      criterion 12). */}
                  {placedPage === "" ? (
                    <StyleSection panel={panel} clones={editor.clones} onReturnFocus={returnFocus} />
                  ) : null}
                </>
              )
            ) : doc.view.content === "operations" ? (
              <OperationsSection
                api={operations}
                selectionCount={editor.selectionCount}
                onReturnFocus={returnFocus}
                fixtureRunning={fixtureJobs.view?.running ?? 0}
                settings={operationSettings}
                onEditMaterial={materials.edit}
                materials={<MaterialsSection api={materials} onReturnFocus={returnFocus} />}
                grid={testGrid}
                materialsApi={materials}
                fixtureJobs={<FixtureJobsSection api={fixtureJobs} selectFixture={fixtures.select} />}
              />
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

/** The notice on a laser page whose extension is switched off (`specs/0098` criterion 18 b), in
 * place of the Machine and Fixtures blocks. */
function SwitchedOffNotice({ word }: { word: string }) {
  return (
    <div
      role="status"
      className="flex items-start gap-2 rounded-md p-2 text-xs text-[var(--toolbar-icon)]"
      style={{
        background: "var(--toolbar-bg)",
        border: "1px solid color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
      }}
    >
      <TriangleAlert size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="shrink-0" />
      {`${word} workbench is switched off`}
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

/** A selected placed page's Style block (`specs/0100` criterion 17): "Placed page of Coaster."
 * and no buttons; nothing otherwise. */
function PlacedPageBlock({ text }: { text: string }) {
  if (!text) return null;
  return (
    <div role="group" aria-label="Placed page" className="flex items-start gap-1 text-sm text-[var(--toolbar-icon)]">
      <Frame size={16} aria-hidden className="mt-0.5 shrink-0" />
      <span>{text}</span>
    </div>
  );
}

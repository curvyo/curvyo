import { Check, FilePlus, Folder, FolderOpen, FolderPlus, History, Trash2, Undo2, X } from "lucide-react";
import { GLYPH } from "@/components/ui/looks";
import { useCallback, useEffect, useRef, useState } from "react";

import { WorkspaceDeleted } from "@/components/panel/WorkspaceDeleted";
import { WorkspaceNameField } from "@/components/panel/WorkspaceNameField";
import { WorkspaceTree } from "@/components/panel/WorkspaceTree";
import { Tooltip } from "@/components/ui/tooltip";
import type { WorkspaceApi, WorkspaceSlot } from "@/hooks/useWorkspace";
import { type RecentWorkspaceData, type WorkspaceRowData, targetFolder } from "@/lib/workspaceRows";
import { devices_removed_alert, workspace_tree_words_json } from "@/wasm-bindings/curvyo_editor_wasm.js";
import { Button } from "@/components/ui/button";
import { AlertLine } from "@/components/ui/notice";

/** The fixed words of a shared workspace's tree, formatted in Rust; read on first use, since the
 * wasm module is not ready while this module loads. */
let treeWords: { noDeleted: string } | null = null;
function words(): { noDeleted: string } {
  treeWords ??= JSON.parse(workspace_tree_words_json()) as { noDeleted: string };
  return treeWords;
}

/** Delete ignores presses for this long after the confirm appears, as the File tab's Discard. */
const DELETE_ARM_MS = 400;

const ICON =
  "flex size-7 shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] pointer-coarse:size-9";

interface WorkspaceSectionProps {
  workspace: WorkspaceApi;
  /** The header line of a shared workspace ("Shared, 2 people", "Receiving 7 of 20"), from the
   * Share view; absent while the workspace is local. */
  sharedLine?: string;
  /** "Shared through workspace Box job" for an open document of a synced or shared workspace
   * (`specs/0105` criterion 2). */
  documentLine?: string;
  /** This device was removed from my devices: the alert shows here too, and this dismisses
   * both (`specs/0096-own-devices-sync` ux-notes 7.2). */
  devicesRemoved?: () => void;
  /** After a mouse press the keyboard goes back to the canvas. */
  onReturnFocus: () => void;
}

/**
 * The File tab's Workspace section (`specs/0095-workspace` criteria 1, 4 to 21; `ux-notes.md`
 * sections 1, 2, 3, 10 and 11): the header with the workspace's name and its four buttons, the
 * empty state with Open workspace… and the recent workspaces, the tree, the inline name field of a
 * new entry and the action slot. Desktop only.
 */
export function WorkspaceSection({ workspace, sharedLine, documentLine, devicesRemoved, onReturnFocus }: WorkspaceSectionProps) {
  const [focusedRow, setFocusedRow] = useState<WorkspaceRowData | null>(null);
  const rootDrop = useRef<HTMLParagraphElement>(null);
  const sectionRef = useRef<HTMLElement>(null);
  const onFocusRow = useCallback((row: WorkspaceRowData | null) => setFocusedRow(row), []);
  const focusTree = useCallback(() => {
    const row =
      sectionRef.current?.querySelector<HTMLElement>('[role="row"][tabindex="0"]') ??
      sectionRef.current?.querySelector<HTMLElement>("[data-ws-new]");
    row?.focus({ focusVisible: true } as FocusOptions);
  }, []);

  // After Open workspace the focus goes to the first row, or New document (ux-notes section 11).
  const name = workspace.name;
  useEffect(() => {
    if (name !== "") {
      requestAnimationFrame(focusTree);
    }
  }, [focusTree, name]);

  const startNew = (mode: "new-document" | "new-folder") =>
    workspace.startNaming({ mode, folder: targetFolder(focusedRow), error: "" });

  return (
    <section ref={sectionRef} aria-label="Workspace" className="mt-3 flex flex-col">
      {devicesRemoved || workspace.removedText !== "" ? (
        // One block: this device's removal from my devices replaces the workspace's own text.
        <div className="mb-2 flex items-start gap-2">
          <p role="alert" className="min-w-0 flex-1 text-xs text-[var(--toolbar-icon)]">
            {devicesRemoved ? devices_removed_alert() : workspace.removedText}
          </p>
          <Button icon={Check} onClick={devicesRemoved ?? workspace.dismissRemoved} className="min-w-[52px]">
            OK
          </Button>
        </div>
      ) : null}
      <div className="flex min-h-7 items-center gap-1">
        {workspace.open ? (
          <div className="min-w-0 flex-1">
            <p ref={rootDrop} title={workspace.name} className="truncate text-sm text-[var(--toolbar-icon)]">
              {workspace.name}
            </p>
            <p role="status" className="truncate text-xs text-[var(--panel-muted-fg)]">
              {sharedLine ?? workspace.view.header}
            </p>
            {documentLine ? <p className="truncate text-xs text-[var(--panel-muted-fg)]">{documentLine}</p> : null}
          </div>
        ) : (
          <p className="flex-1 text-sm font-semibold text-[var(--toolbar-icon)]">Workspace</p>
        )}
        {workspace.open ? (
          <>
            <HeaderButton label="New document" data-ws-new="" onPress={() => startNew("new-document")}>
              <FilePlus {...GLYPH} aria-hidden />
            </HeaderButton>
            <HeaderButton label="New folder" onPress={() => startNew("new-folder")}>
              <FolderPlus {...GLYPH} aria-hidden />
            </HeaderButton>
            <HeaderButton label="Recent workspaces" pressed={workspace.showRecent} onPress={workspace.toggleRecent}>
              <History {...GLYPH} aria-hidden />
            </HeaderButton>
            <HeaderButton label="Close workspace" onPress={() => workspace.close(focusTree)}>
              <X {...GLYPH} aria-hidden />
            </HeaderButton>
          </>
        ) : null}
      </div>
      {!workspace.open ? (
        <div className="mt-2 flex flex-col">
          <Button icon={FolderOpen}
            onClick={(event) => {
              workspace.pickWorkspace();
              if (event.detail > 0) {
                onReturnFocus();
              }
            }}
             className="w-full"
          >
Open workspace…
</Button>
          {workspace.refusal !== "" ? (
            <AlertLine className="mt-1">
              {workspace.refusal}
            </AlertLine>
          ) : null}
          <RecentList workspace={workspace} />
        </div>
      ) : workspace.showRecent ? (
        <div
          className="mt-2"
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              event.stopPropagation();
              workspace.toggleRecent();
            }
          }}
        >
          <RecentList workspace={workspace} />
        </div>
      ) : (
        <div className="mt-2 flex flex-col">
          {workspace.naming && workspace.naming.mode !== "rename" ? (
            <div className="mb-1 flex items-start gap-2 pl-1">
              <span aria-hidden className="mt-1 flex text-[var(--toolbar-icon)]">
                {workspace.naming.mode === "new-folder" ? (
                  <FolderPlus {...GLYPH} />
                ) : (
                  <FilePlus {...GLYPH} />
                )}
              </span>
              <WorkspaceNameField
                label={workspace.naming.mode === "new-folder" ? "Name of the new folder" : "Name of the new document"}
                initial="Untitled"
                error={workspace.naming.error}
                onCommit={workspace.commitName}
                onCancel={() => {
                  workspace.cancelNaming();
                  focusTree();
                }}
              />
            </div>
          ) : null}
          <WorkspaceTree
            workspace={workspace}
            onFocusRow={onFocusRow}
            onReturnFocus={onReturnFocus}
            rootDropRef={rootDrop}
          />
          <ActionSlot workspace={workspace} slot={workspace.slot} refocus={focusTree} />
          {sharedLine ? <WorkspaceDeleted workspace={workspace} emptyText={words().noDeleted} /> : null}
        </div>
      )}
    </section>
  );
}

function HeaderButton({
  label,
  pressed,
  onPress,
  children,
  ...rest
}: {
  label: string;
  pressed?: boolean;
  onPress: () => void;
  children: React.ReactNode;
  "data-ws-new"?: string;
}) {
  return (
    <Tooltip side="left" content={label}>
      <button type="button" aria-label={label} aria-pressed={pressed} onClick={onPress} className={ICON} {...rest}>
        {children}
      </button>
    </Tooltip>
  );
}

/** The recent workspaces (criterion 20, ux-notes section 2.3). */
function RecentList({ workspace }: { workspace: WorkspaceApi }) {
  if (workspace.recents.length === 0) {
    return null;
  }
  return (
    <ul aria-label="Recent workspaces" className="mt-2 flex flex-col">
      {workspace.recents.map((recent: RecentWorkspaceData, index) => (
        <li key={recent.path} className="flex h-10 items-center gap-1 pointer-coarse:h-11">
          <Tooltip side="left" content={recent.path}>
            <button
              type="button"
              aria-label={recent.found ? `${recent.name}, ${recent.path}` : `${recent.name}, not found`}
              onClick={(event) => {
                if (!recent.found) {
                  event.currentTarget.parentElement?.querySelector<HTMLElement>("[data-ws-remove]")?.focus();
                  return;
                }
                workspace.openRecent(index);
              }}
              className="flex h-full min-w-0 flex-1 items-center gap-2 rounded-[5px] px-1 text-left outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              style={{ opacity: recent.found ? 1 : 0.55 }}
            >
              <span aria-hidden className="flex text-[var(--toolbar-icon)]">
                <Folder {...GLYPH} />
              </span>
              <span className="flex min-w-0 flex-col">
                <span className="truncate text-sm text-[var(--toolbar-icon)]">{recent.name}</span>
                <span className="truncate text-xs text-[var(--panel-muted-fg)]" style={{ direction: "rtl", textAlign: "left" }}>
                  {recent.found ? recent.path : "Not found"}
                </span>
              </span>
            </button>
          </Tooltip>
          <Tooltip side="left" content={`Remove ${recent.name} from recent`}>
            <button
              type="button"
              data-ws-remove=""
              aria-label={`Remove ${recent.name} from recent`}
              onClick={() => workspace.removeRecent(index)}
              className={ICON}
            >
              <X {...GLYPH} aria-hidden />
            </button>
          </Tooltip>
        </li>
      ))}
    </ul>
  );
}

/** The action slot under the tree: the delete confirm, the move notice with Undo move, a status
 * or an alert (ux-notes sections 3.2 and 3.3). */
function ActionSlot({
  workspace,
  slot,
  refocus,
}: {
  workspace: WorkspaceApi;
  slot: WorkspaceSlot | null;
  refocus: () => void;
}) {
  if (!slot) {
    return null;
  }
  if (slot.kind === "confirm") {
    return <DeleteConfirm key={slot.id} workspace={workspace} text={slot.text} refocus={refocus} />;
  }
  if (slot.kind === "ask") {
    return <AskConfirm key={slot.id} workspace={workspace} slot={slot} refocus={refocus} />;
  }
  if (slot.kind === "undo") {
    return (
      <div className="mt-2 flex items-center gap-2">
        <p role="status" className="min-w-0 flex-1 text-xs text-[var(--toolbar-icon)]">
          {slot.text}
        </p>
        <Button icon={Undo2} onClick={workspace.undoMove} className="min-w-[76px]">
          Undo move
        </Button>
      </div>
    );
  }
  return (
    <p
      role={slot.kind === "alert" ? "alert" : "status"}
      className={`mt-2 text-xs ${slot.kind === "alert" ? "text-[var(--field-invalid)]" : "text-[var(--toolbar-icon)]"}`}
    >
      {slot.text}
    </p>
  );
}

/** An inline question of the action slot: the sentence, Cancel (focused) and its action. Cancel
 * and Escape put the focus back where it was when the question came, else on the tree. */
function AskConfirm({
  workspace,
  slot,
  refocus,
}: {
  workspace: WorkspaceApi;
  slot: Extract<WorkspaceSlot, { kind: "ask" }>;
  refocus: () => void;
}) {
  // Read during the first render, before Cancel takes the focus.
  const [trigger] = useState(() => (document.activeElement instanceof HTMLElement ? document.activeElement : null));
  const cancel = () => {
    workspace.dismissSlot();
    if (trigger?.isConnected) {
      trigger.focus();
    } else {
      refocus();
    }
  };
  return (
    <div
      role="group"
      aria-label={slot.text}
      className="mt-2"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          cancel();
        }
      }}
    >
      <p className="text-xs text-[var(--toolbar-icon)]">{slot.text}</p>
      <div className="mt-2 flex gap-2">
        <Button icon={X} autoFocus onClick={cancel} className="min-w-[76px]">
          Cancel
        </Button>
        <Button icon={Check}
          onClick={() => {
            workspace.dismissSlot();
            slot.run();
            refocus();
          }} className="min-w-[76px]"
        >
          {slot.action}
        </Button>
      </div>
    </div>
  );
}

/** The inline confirm of Delete: the sentence, Cancel (focused) and Delete. */
function DeleteConfirm({ workspace, text, refocus }: { workspace: WorkspaceApi; text: string; refocus: () => void }) {
  const cancel = useRef<HTMLButtonElement>(null);
  const [armed, setArmed] = useState(false);
  useEffect(() => {
    cancel.current?.focus();
    const timer = window.setTimeout(() => setArmed(true), DELETE_ARM_MS);
    return () => window.clearTimeout(timer);
  }, []);
  const close = () => {
    workspace.dismissSlot();
    refocus();
  };
  return (
    <div
      role="group"
      aria-label={text}
      className="mt-2"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          close();
        }
      }}
    >
      <p className="text-xs text-[var(--toolbar-icon)]">{text}</p>
      <div className="mt-2 flex gap-2">
        <Button icon={X} ref={cancel} onClick={close} className="min-w-[76px]">
          Cancel
        </Button>
        <Button variant="destructive" icon={Trash2}
          aria-disabled={!armed}
          onClick={() => {
            if (armed) {
              workspace.confirmDelete(refocus);
            }
          }} className="min-w-[76px] shrink-0"
        >
          Delete
        </Button>
      </div>
    </div>
  );
}

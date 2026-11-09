// Make a workspace for this file (`specs/0105-sharing-concept` criteria 7 to 9, ADR 0044 decision
// 6): the Share tab's button for a saved file outside any workspace, its inline confirm naming the
// new folder, then the file saved, moved into the folder and opened there as a Local workspace.
// The folder rule and the move are `curvyo-app`'s; this runs the two host commands.

import { useCallback, useState } from "react";

import type { ProjectApi } from "@/hooks/useProject";
import type { WorkspaceApi } from "@/hooks/useWorkspace";
import type { WorkspaceAnswer } from "@/lib/workspaceRows";
import { invoke, isTauri } from "@/platform/host";
import { workspace_open_refusal } from "@/wasm-bindings/curvyo_editor_wasm.js";

export interface MakeWorkspaceApi {
  /** A saved file is open outside any workspace on the desktop: the button is offered. */
  offered: boolean;
  /** The browser, which makes no workspaces: only the line says so. */
  browser: boolean;
  /** The folder the inline confirm names, while it is shown. */
  folder: string | null;
  /** The file's name the confirm says moves. */
  fileName: string;
  /** Why the last attempt failed, or empty. */
  failure: string;
  ask: () => void;
  confirm: () => void;
  cancel: () => void;
}

export function useMakeWorkspace(project: ProjectApi, workspace: WorkspaceApi): MakeWorkspaceApi {
  const [folder, setFolder] = useState<string | null>(null);
  const [failure, setFailure] = useState("");
  const offered = isTauri && !workspace.open && project.info.path !== null;

  const ask = useCallback(() => {
    setFailure("");
    void invoke<string>("workspace_folder_for_file")
      .then(setFolder)
      .catch((code: unknown) => setFailure(workspace_open_refusal(String(code))));
  }, []);

  const confirm = useCallback(() => {
    void (async () => {
      // No change is lost: an unsaved file is saved where it is first.
      if (project.dirty && (await project.saveNow()) !== "saved") {
        return;
      }
      const made = await invoke<{ answer: WorkspaceAnswer; path: string[] }>("make_workspace_for_file").catch(
        (code: unknown) => {
          setFailure(workspace_open_refusal(String(code)));
          return null;
        },
      );
      setFolder(null);
      if (made) {
        workspace.adoptMade(made.answer, made.path);
      }
    })();
  }, [project, workspace]);

  return {
    offered,
    browser: !isTauri,
    folder,
    fileName: project.info.fileName ?? "",
    failure,
    ask,
    confirm,
    cancel: () => setFolder(null),
  };
}

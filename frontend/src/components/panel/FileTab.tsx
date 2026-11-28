import { type ReactNode, useEffect, useId, useRef, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import type { FileCommandName, InlineCheck, ProjectApi } from "@/hooks/useProject";
import { RecoveryBlock } from "@/components/panel/RecoveryBlock";
import type { RecoveryApi } from "@/hooks/useRecovery";
import { TEMPLATE_NAME } from "@/lib/guideText";

/** The name of the Import SVG… button and its tooltip (UX notes: the control's name only). */
import { checkSentence, projectName, projectStatus } from "@/lib/projectText";
import { Button } from "@/components/ui/button";
import { Eraser, FilePlus, FolderOpen, LayoutTemplate, type LucideIcon, Power, Save, SaveAll, X } from "lucide-react";
import { AlertLine } from "@/components/ui/notice";

/** Discard ignores presses for this long after the check appears (a double-click on the button
 * that raised it cannot confirm). */
const DISCARD_ARM_MS = 400;


interface FileTabProps {
  project: ProjectApi;
  /** The desktop build: it has Save as, Quit and the file's path; the browser build downloads. */
  desktop: boolean;
  /** The application's version for the last line. */
  version: string;
  /** The tooltip of a button: its words and the chord this build has for it. */
  tooltipFor: (command: FileCommandName) => string;
  /** After a mouse press the keyboard goes back to the canvas. */
  onReturnFocus: () => void;
  /** Export SVG… and its switches (`specs/0024-svg-import-export` criterion 147), its own section
   * below the button block; the panel passes `ExportSvgSection`. */
  exportSvg?: ReactNode;
  /** The Credits block (`specs/0138` criterion 4), 12 px below the export options. */
  credits?: ReactNode;
  /** The Extensions group (`specs/0093` UX notes §7), at the end of the tab. */
  extensions?: ReactNode;
  /** The Workspace section (`specs/0095-workspace` `ux-notes.md` section 1), appended below Export
   * SVG and above Quit; the desktop build only. */
  workspace?: ReactNode;
  /** Diagnostics at the end of the tab, above the version (`specs/0158-pen-display` slice 0: the
   * Pen probe's button). */
  diagnostics?: ReactNode;
  /** The copies a crash left (`specs/0136-autosave-and-restore` criterion 4): a block above the
   * project block while any waits. */
  recovery?: RecoveryApi;
}

/** What the buttons need besides the project. */
type ButtonProps = Pick<FileTabProps, "project" | "desktop" | "tooltipFor" | "onReturnFocus">;

/**
 * The File tab (`specs/0051-sidebar-and-no-menu/` criteria 15, 16, 24 to 26): the project block,
 * the file actions as buttons, Quit on the desktop and the application's version. A replacing
 * action on a project with unsaved changes swaps the six buttons for the inline check, in
 * place and at the same height, so nothing below moves. No popup, no menu, no dialog of its own.
 */
export function FileTab({
  project,
  desktop,
  version,
  tooltipFor,
  onReturnFocus,
  exportSvg,
  credits,
  extensions,
  workspace,
  recovery,
  diagnostics,
}: FileTabProps) {
  const { info, dirty, check } = project;
  const buttonProps = { project, desktop, tooltipFor, onReturnFocus };
  return (
    <div className="flex flex-col">
      {recovery ? <RecoveryBlock recovery={recovery} onReturnFocus={onReturnFocus} /> : null}
      <ProjectBlock
        fileName={info.fileName}
        path={info.path}
        dirty={dirty}
        missing={info.missing ?? false}
        deleted={info.deletedStatus ?? ""}
        desktop={desktop}
        failure={recovery?.failure ?? null}
      />
      <div className="mt-3 h-[100px]">
        {check ? (
          <UnsavedCheck key={check.id} check={check} fileName={info.fileName} project={project} />
        ) : (
          <Buttons {...buttonProps} />
        )}
      </div>
      {exportSvg}
      {credits}
      {workspace}
      {desktop ? (
        <>
          <div
            aria-hidden
            className="my-4 h-px"
            style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
          />
          <CommandButton command="quit" label="Quit" icon={Power} {...buttonProps} />
        </>
      ) : null}
      {extensions}
      {diagnostics}
      <p className={`${desktop ? "mt-3" : "mt-4"} text-xs text-[var(--panel-muted-fg)]`}>
        {`Curvyo ${version}`}
      </p>
    </div>
  );
}

function ProjectBlock({
  fileName,
  path,
  dirty,
  missing,
  deleted,
  desktop,
  failure,
}: {
  failure: string | null;
  fileName: string | null;
  path: string | null;
  dirty: boolean;
  missing: boolean;
  deleted: string;
  desktop: boolean;
}) {
  const name = (
    <p className="truncate text-sm text-[var(--toolbar-icon)]">{projectName(fileName)}</p>
  );
  return (
    <div className="min-w-0">
      {desktop && path !== null ? (
        <Tooltip side="left" content={path}>
          {name}
        </Tooltip>
      ) : (
        name
      )}
      <p className="text-xs text-[var(--panel-muted-fg)]">{projectStatus(fileName, dirty, missing, deleted)}</p>
      {failure ? (
        // The failed write of the recovery copy (`specs/0136` criterion 13), until the next success.
        <AlertLine>
          {failure}
        </AlertLine>
      ) : null}
    </div>
  );
}

/** The buttons: New and Open, Save and Save as, Close and Template. The browser build has no Save
 * as. */
function Buttons(props: ButtonProps) {
  const { desktop } = props;
  const button = (command: FileCommandName, label: string, icon: LucideIcon) => (
    <CommandButton command={command} label={label} icon={icon} {...props} />
  );
  return (
    <div className="flex flex-col gap-2">
      <div className="flex gap-2">
        {button("new", "New", FilePlus)}
        {button("open", "Open…", FolderOpen)}
      </div>
      <div className="flex gap-2">
        {button("save", "Save", Save)}
        {desktop ? button("save-as", "Save as…", SaveAll) : null}
      </div>
      <div className="flex gap-2">
        {button("close", "Close", X)}
        <TemplateButton {...props} />
      </div>
    </div>
  );
}

/** The Template button (`specs/0046-document-shapes` criterion 19): beside Close, an outline text
 * button like its neighbours. Its tooltip is its name; it has no chord. */
function TemplateButton({ project, onReturnFocus }: ButtonProps) {
  return (
    <Tooltip side="left" content={TEMPLATE_NAME}>
      <Button icon={LayoutTemplate}
        data-file-command="template"
        onClick={(event) => {
          project.loadTemplate();
          if (event.detail > 0) {
            onReturnFocus();
          }
        }} className="min-w-[118px]"
      >
        {TEMPLATE_NAME}
      </Button>
    </Tooltip>
  );
}

function CommandButton({
  command,
  label,
  icon,
  project,
  tooltipFor,
  onReturnFocus,
}: ButtonProps & { command: FileCommandName; label: string; icon: LucideIcon }) {
  return (
    <Tooltip side="left" content={tooltipFor(command)}>
      <Button
        icon={icon}
        data-file-command={command}
        onClick={(event) => {
          project.press(command);
          // `detail` is 0 for keyboard activation, 1 or more for a click. A check that this press
          // raises takes the focus itself.
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
        className="min-w-[118px]"
      >
        {label}
      </Button>
    </Tooltip>
  );
}

/** The inline confirm, three-button variant: the sentence and Cancel, Discard and Save. */
function UnsavedCheck({
  check,
  fileName,
  project,
}: {
  check: InlineCheck;
  fileName: string | null;
  project: ProjectApi;
}) {
  const cancel = useRef<HTMLButtonElement>(null);
  const [armed, setArmed] = useState(false);
  useEffect(() => {
    cancel.current?.focus();
    const timer = window.setTimeout(() => setArmed(true), DISCARD_ARM_MS);
    return () => window.clearTimeout(timer);
  }, []);
  const sentence = checkSentence(fileName, check.kind, check.subject);
  const sentenceId = useId();
  return (
    <div
      role="group"
      aria-label={sentence}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          // Escape closes the block first; the panel's own Escape (focus to the canvas) waits for
          // the next press.
          event.preventDefault();
          event.stopPropagation();
          project.cancel();
        }
      }}
    >
      <p id={sentenceId} className="text-xs text-[var(--toolbar-icon)]">
        {sentence}
      </p>
      <div className="mt-2 flex gap-2">
        <Button icon={X} size="bar"
          ref={cancel}
          aria-describedby={sentenceId}
          onClick={project.cancel}
        >
          Cancel
        </Button>
        <Button variant="destructive" icon={Eraser} size="bar"
          aria-disabled={!armed}
          onClick={() => {
            if (armed) {
              project.discard();
            }
          }}
        >
          Discard
        </Button>
        <Button variant="primary" icon={Save} size="bar"
          onClick={project.saveFromCheck}
        >
          Save
        </Button>
      </div>
    </div>
  );
}

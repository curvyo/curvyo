import { useEffect, useId, useRef, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import type { FileCommandName, InlineCheck, ProjectApi } from "@/hooks/useProject";
import { TEMPLATE_NAME } from "@/lib/guideText";
import { checkSentence, projectName, projectStatus } from "@/lib/projectText";

/** Discard ignores presses for this long after the check appears (a double-click on the button
 * that raised it cannot confirm). */
const DISCARD_ARM_MS = 400;

const BUTTON =
  "h-7 shrink-0 rounded-[5px] border text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9";
const OUTLINE =
  "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]";

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
}

/** What the buttons need besides the project. */
type ButtonProps = Pick<FileTabProps, "project" | "desktop" | "tooltipFor" | "onReturnFocus">;

/**
 * The File tab (`specs/0051-sidebar-and-no-menu/` criteria 15, 16, 24 to 26): the project block,
 * the file actions as buttons, Quit on the desktop and the application's version. A replacing
 * action on a project with unsaved changes swaps the five buttons for the inline check, in
 * place and at the same height, so nothing below moves. No popup, no menu, no dialog of its own.
 */
export function FileTab({ project, desktop, version, tooltipFor, onReturnFocus }: FileTabProps) {
  const { info, dirty, check } = project;
  const buttonProps = { project, desktop, tooltipFor, onReturnFocus };
  return (
    <div className="flex flex-col">
      <ProjectBlock fileName={info.fileName} path={info.path} dirty={dirty} desktop={desktop} />
      <div className="mt-3 h-[100px]">
        {check ? (
          <UnsavedCheck key={check.id} check={check} fileName={info.fileName} project={project} />
        ) : (
          <Buttons {...buttonProps} />
        )}
      </div>
      {desktop ? (
        <>
          <div
            aria-hidden
            className="my-4 h-px"
            style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
          />
          <CommandButton command="quit" label="Quit" {...buttonProps} />
        </>
      ) : null}
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
  desktop,
}: {
  fileName: string | null;
  path: string | null;
  dirty: boolean;
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
      <p className="text-xs text-[var(--panel-muted-fg)]">{projectStatus(fileName, dirty)}</p>
    </div>
  );
}

/** The buttons: New and Open, Save and Save as, Close and Template. The browser build has no Save
 * as. */
function Buttons(props: ButtonProps) {
  const { desktop } = props;
  const button = (command: FileCommandName, label: string) => (
    <CommandButton command={command} label={label} {...props} />
  );
  return (
    <div className="flex flex-col gap-2">
      <div className="flex gap-2">
        {button("new", "New")}
        {button("open", "Open…")}
      </div>
      <div className="flex gap-2">
        {button("save", "Save")}
        {desktop ? button("save-as", "Save as…") : null}
      </div>
      <div className="flex gap-2">
        {button("close", "Close")}
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
      <button
        type="button"
        data-file-command="template"
        onClick={(event) => {
          project.loadTemplate();
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
        className={`${BUTTON} ${OUTLINE} w-[118px]`}
      >
        {TEMPLATE_NAME}
      </button>
    </Tooltip>
  );
}

function CommandButton({
  command,
  label,
  project,
  tooltipFor,
  onReturnFocus,
}: ButtonProps & { command: FileCommandName; label: string }) {
  return (
    <Tooltip side="left" content={tooltipFor(command)}>
      <button
        type="button"
        data-file-command={command}
        onClick={(event) => {
          project.press(command);
          // `detail` is 0 for keyboard activation, 1 or more for a click. A check that this press
          // raises takes the focus itself.
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
        className={`${BUTTON} ${OUTLINE} w-[118px]`}
      >
        {label}
      </button>
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
  const sentence = checkSentence(fileName, check.kind);
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
        <button
          ref={cancel}
          type="button"
          aria-describedby={sentenceId}
          onClick={project.cancel}
          className={`${BUTTON} ${OUTLINE} w-[76px]`}
        >
          Cancel
        </button>
        <button
          type="button"
          aria-disabled={!armed}
          onClick={() => {
            if (armed) {
              project.discard();
            }
          }}
          className={`${BUTTON} w-[76px] border-[var(--destructive)] bg-[var(--background)] text-[var(--destructive)] hover:bg-[var(--destructive)] hover:text-white focus-visible:bg-[var(--destructive)] focus-visible:text-white aria-disabled:hover:bg-[var(--background)] aria-disabled:hover:text-[var(--destructive)] aria-disabled:focus-visible:bg-[var(--background)] aria-disabled:focus-visible:text-[var(--destructive)]`}
        >
          Discard
        </button>
        <button
          type="button"
          onClick={project.saveFromCheck}
          className={`${BUTTON} w-[76px] border-[var(--toolbar-icon-active-bg)] bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] hover:brightness-110`}
        >
          Save
        </button>
      </div>
    </div>
  );
}

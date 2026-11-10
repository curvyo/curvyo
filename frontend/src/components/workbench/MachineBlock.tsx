import {
  BookmarkCheck,
  BookmarkPlus,
  ChevronDown,
  ChevronUp,
  RotateCcw,
  TriangleAlert,
} from "lucide-react";
import { type ReactNode, useEffect, useId, useRef, useState } from "react";

import { CornerControl } from "@/components/CornerControl";
import { ToolbarSwitch } from "@/components/ToolbarSwitch";
import { Tooltip } from "@/components/ui/tooltip";
import { MachineProfileList } from "@/components/workbench/MachineProfileList";
import { ARM_TRASH, ARM_ZONE, useDeleteArm } from "@/hooks/useDeleteArm";
import type { UnitSymbol } from "@/hooks/useDocumentPanel";
import type { MachineView, WorkbenchesApi } from "@/hooks/useWorkbenches";
import { typedNumber } from "@/lib/machineBlockKeys";
import { setMachineWriteFailureHandler } from "@/lib/machineProfilesFile";
import { formatWorkArea } from "@/lib/workAreaText";

const ICON_BUTTON =
  "inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

const FIELD =
  "flex h-7 items-center gap-1 rounded-[5px] border border-[var(--field-border)] bg-white px-2 text-sm focus-within:ring-2 focus-within:ring-[var(--editor-accent)]";

/** The write-failure alert (UX notes §7). */
const WRITE_FAILED = "Could not save the profile. Nothing was changed.";
/** The armed Update profile text. */
const ARMED_UPDATE = "Update profile. Overwrites the saved profile.";
/** Why the connection fields are read-only. */
const NO_CONNECTION = "Only saved profiles have a connection.";
/** How long a notice stays. */
const NOTICE_MS = 3000;

interface Notice {
  text: string;
  anchor: string;
  alert: boolean;
}

interface MachineBlockProps {
  label: string;
  machine: MachineView;
  workbenches: WorkbenchesApi;
  /** The document's display unit, for the read-only work area line. */
  unit?: UnitSymbol;
}

/**
 * The Machine block of a laser manufacturing (`specs/0026-machine-profile` UX notes §2 to §7): the
 * profile picker with Save as profile, the state row, the inline profile list, the copy's fields
 * (each edit one undo step) and the connection of the chosen saved profile (library, no undo).
 * Host-drawn from the session's view; the extension only places it (`adrs.md` note 4).
 */
export function MachineBlock({ label, machine, workbenches, unit = "mm" }: MachineBlockProps) {
  const api = workbenches.machine;
  const [open, setOpen] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);
  const profileButton = useRef<HTMLButtonElement>(null);
  const update = useDeleteArm();
  const copy = machine.copy;

  const show = (text: string, anchor: string, alert = false) => {
    if (text !== "") {
      setNotice({ text, anchor, alert });
    }
  };
  useEffect(() => {
    if (notice === null) return;
    const timer = window.setTimeout(() => setNotice(null), NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notice]);
  useEffect(() => {
    setMachineWriteFailureHandler(() => show(WRITE_FAILED, "save", true));
    return () => setMachineWriteFailureHandler(null);
  }, []);
  // A press on the canvas closes the list (UX notes §3).
  useEffect(() => {
    if (!open) return;
    const onPointer = (event: PointerEvent) => {
      if ((event.target as Element | null)?.tagName === "CANVAS") setOpen(false);
    };
    window.addEventListener("pointerdown", onPointer, true);
    return () => window.removeEventListener("pointerdown", onPointer, true);
  }, [open]);

  const backToButton = () => {
    setOpen(false);
    window.requestAnimationFrame(() => profileButton.current?.focus());
  };
  const noticeAt = (anchor: string) =>
    notice?.anchor === anchor ? <FieldNotice notice={notice} /> : null;

  return (
    <section aria-label={label} className="flex flex-col gap-2">
      <h3 className="text-sm text-[var(--toolbar-icon)]">{label}</h3>
      {machine.library_notices.length > 0 ? (
        <div
          role="status"
          className="flex items-start gap-2 rounded-md p-2 text-xs text-[var(--toolbar-icon)]"
          style={{
            background: "var(--toolbar-bg)",
            border: "1px solid color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
          }}
        >
          <TriangleAlert size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="shrink-0" />
          <span className="flex flex-col">
            {machine.library_notices.map((text) => (
              <span key={text}>{text}</span>
            ))}
          </span>
        </div>
      ) : null}
      <div className="relative flex h-7 items-center gap-1">
        <Tooltip side="left" content="Machine profile">
          <button
            ref={profileButton}
            type="button"
            aria-expanded={open}
            aria-label={`Machine profile, ${copy.name}`}
            className="flex h-7 min-w-0 flex-1 items-center justify-between gap-1 rounded-md border border-[var(--field-border)] bg-white px-2 text-sm outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            onClick={() => setOpen((was) => !was)}
          >
            <span className="truncate">{copy.name}</span>
            {open ? (
              <ChevronUp size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="shrink-0" />
            ) : (
              <ChevronDown size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="shrink-0" />
            )}
          </button>
        </Tooltip>
        <Tooltip side="left" content="Save as profile">
          <button
            type="button"
            aria-label="Save as profile"
            className={ICON_BUTTON}
            onClick={() => show(api.saveAs(), "save")}
          >
            <BookmarkPlus size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          </button>
        </Tooltip>
        {noticeAt("save")}
      </div>
      {machine.name_notice ? (
        <p role="status" className="text-xs text-[var(--toolbar-icon)]">
          {machine.name_notice}
        </p>
      ) : null}
      {machine.state ? (
        <div className="flex h-7 items-center justify-between gap-1" {...{ [ARM_ZONE]: "update" }}>
          <span role="status" className="text-xs text-[var(--toolbar-icon)]">
            {machine.state === "differs" ? "Differs from profile" : "Not in your profiles"}
          </span>
          {machine.state === "differs" ? (
            <span className="flex gap-1">
              {machine.can_update ? (
                <Tooltip side="left" content={update.armed ? ARMED_UPDATE : "Update profile"}>
                  <button
                    type="button"
                    aria-label={update.armed ? ARMED_UPDATE : "Update profile"}
                    {...{ [ARM_TRASH]: "update" }}
                    className={update.armed ? ICON_BUTTON.replace("hover:bg-[var(--editor-accent-hover)]", "bg-[var(--destructive)] text-white") : ICON_BUTTON}
                    onClick={() =>
                      update.press("update", ARMED_UPDATE, () => {
                        show(api.update(), "save");
                        profileButton.current?.focus();
                        return "";
                      })
                    }
                  >
                    <BookmarkCheck size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                  </button>
                </Tooltip>
              ) : null}
              <Tooltip side="left" content="Reset from profile">
                <button
                  type="button"
                  aria-label="Reset from profile"
                  className={ICON_BUTTON}
                  onClick={() => {
                    api.reset();
                    profileButton.current?.focus();
                  }}
                >
                  <RotateCcw size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                </button>
              </Tooltip>
            </span>
          ) : null}
          <span role="status" className="sr-only">
            {update.status}
          </span>
        </div>
      ) : null}
      {open ? (
        <MachineProfileList
          rows={machine.profiles}
          onPick={(id) => {
            if (!machine.profiles.find((row) => row.id === id)?.chosen) {
              api.pick(id);
            }
            backToButton();
          }}
          onDuplicate={(id) => api.duplicate(id)}
          onRename={(id, name) => show(api.rename(id, name), "save")}
          onDelete={(id) => api.remove(id)}
          onWorkArea={(id, width, height) => workbenches.setProfileWorkArea(id, width, height)}
          onClose={backToButton}
        />
      ) : null}
      <div className="flex flex-col gap-2">
        {/* The work area is the machine's: read-only text on the page (`specs/0098` criterion
            13); it is typed only in a user profile of the list. */}
        <div className="flex h-7 items-center justify-between">
          <span className="text-xs text-[var(--panel-muted-fg)]">Work area</span>
          <span className="text-[13px] tabular-nums text-[var(--toolbar-icon)]">
            {formatWorkArea(copy.width, copy.height, unit)}
          </span>
        </div>
        <CornerControl label="Origin corner" corner={copy.origin} onChange={(corner) => api.setOrigin(corner)} />
        <NumberField
          label="Max feed"
          unit="mm/min"
          accessibleName="Max feed (mm/min)"
          value={copy.max_feed}
          notice={noticeAt("feed")}
          onCommit={(value) => show(api.setMaxFeed(value), "feed")}
        />
        <NumberField
          label="Max power"
          accessibleName="Max power (S-max)"
          value={copy.max_power}
          notice={noticeAt("power")}
          onCommit={(value) => show(api.setMaxPower(value), "power")}
        />
        <div className="flex h-7 items-center justify-between">
          <ToolbarSwitch
            label="Laser mode"
            tooltip="Laser mode"
            checked={copy.laser_mode}
            onCheckedChange={(on) => api.setLaserMode(on)}
          />
        </div>
      </div>
      <ConnectionGroup
        machine={machine}
        notice={noticeAt("connection")}
        onCommit={(address, port, baud) => show(api.setConnection(address, port, baud), "connection")}
      />
    </section>
  );
}

function FieldNotice({ notice }: { notice: Notice }) {
  return (
    <div
      role={notice.alert ? "alert" : "status"}
      className="pointer-events-none absolute right-0 top-full z-20 mt-1 w-max max-w-[244px] rounded-lg p-2 text-xs"
      style={{
        background: "var(--toolbar-bg)",
        color: notice.alert ? "var(--field-invalid)" : "var(--toolbar-icon)",
        boxShadow: notice.alert
          ? "0 0 0 1px var(--field-invalid), var(--panel-elevation-shadow)"
          : "var(--panel-elevation-shadow)",
      }}
    >
      {notice.text}
    </div>
  );
}

/** Enter or blur commits, Escape reverts; the parent keys the field on its value. */
function useDraft(initial: string) {
  const [text, setText] = useState(initial);
  const [seen, setSeen] = useState(initial);
  if (seen !== initial) {
    // The shown value changed (a commit, undo or another tab): the draft follows it in place, so the
    // input keeps its focus.
    setSeen(initial);
    setText(initial);
  }
  return { text, setText, revert: () => setText(initial) };
}

function TextField({
  label,
  value,
  notice,
  readOnly = false,
  describedBy,
  onCommit,
}: {
  label: string;
  value: string;
  notice?: ReactNode;
  readOnly?: boolean;
  describedBy?: string;
  onCommit: (text: string) => void;
}) {
  const draft = useDraft(value);
  const commit = () => {
    if (draft.text === value) return;
    onCommit(draft.text);
    draft.revert();
  };
  return (
    <div className="relative">
      <label className={`${FIELD} ${readOnly ? "opacity-70" : ""}`}>
        <span className="shrink-0 text-[var(--toolbar-icon)]">{label}</span>
        <input
          aria-label={label}
          aria-describedby={describedBy}
          readOnly={readOnly}
          value={draft.text}
          onChange={(event) => draft.setText(event.target.value)}
          onBlur={readOnly ? undefined : commit}
          onKeyDown={(event) => {
            if (readOnly) return;
            if (event.key === "Enter") {
              event.preventDefault();
              commit();
            } else if (event.key === "Escape") {
              event.preventDefault();
              draft.revert();
            }
          }}
          className="min-w-0 flex-1 bg-transparent text-right outline-none"
        />
      </label>
      {notice}
    </div>
  );
}

function numberText(value: number): string {
  return Number.isFinite(value) ? String(Number(value.toFixed(3))) : "";
}

function NumberField({
  label,
  unit,
  accessibleName,
  value,
  notice,
  readOnly = false,
  describedBy,
  onCommit,
}: {
  label: string;
  unit?: string;
  accessibleName: string;
  value: number;
  notice?: ReactNode;
  readOnly?: boolean;
  describedBy?: string;
  onCommit: (value: number) => void;
}) {
  const draft = useDraft(numberText(value));
  const commit = () => {
    const typed = typedNumber(draft.text);
    if (typed === null) {
      draft.revert();
    } else if (typed !== value) {
      onCommit(typed);
      draft.revert();
    }
  };
  return (
    <div className="relative">
      <Tooltip side="left" content={label}>
        <label className={`${FIELD} ${readOnly ? "opacity-70" : ""}`}>
          <span className="shrink-0 text-[var(--toolbar-icon)]">{label}</span>
          <input
            aria-label={accessibleName}
            aria-describedby={describedBy}
            inputMode="decimal"
            readOnly={readOnly}
            value={draft.text}
            onChange={(event) => draft.setText(event.target.value)}
            onBlur={readOnly ? undefined : commit}
            onKeyDown={(event) => {
              if (readOnly) return;
              if (event.key === "Enter") {
                event.preventDefault();
                commit();
              } else if (event.key === "Escape") {
                event.preventDefault();
                draft.revert();
              }
            }}
            className="min-w-0 flex-1 bg-transparent text-right tabular-nums outline-none"
          />
          {unit ? (
            <span aria-hidden className="shrink-0 text-xs text-[var(--panel-muted-fg)]">
              {unit}
            </span>
          ) : null}
        </label>
      </Tooltip>
      {notice}
    </div>
  );
}

/** The connection of the chosen saved profile (UX notes §2 item 6): a library write on commit, no
 * undo; read-only with its reason for the built-in and a profile not in the library. */
function ConnectionGroup({
  machine,
  notice,
  onCommit,
}: {
  machine: MachineView;
  notice: ReactNode;
  onCommit: (address: string, port: string, baud: number) => void;
}) {
  const reason = useId();
  const connection = machine.connection;
  const readOnly = connection === null;
  const address = connection?.connector_address ?? "";
  const port = connection?.controller_port ?? "";
  const baud = connection?.baud_rate ?? 115200;
  const describedBy = readOnly ? reason : undefined;
  return (
    <section aria-label="Connection" className="mt-1 flex flex-col gap-2">
      <h3 className="text-sm text-[var(--toolbar-icon)]">Connection</h3>
      {readOnly ? (
        <p id={reason} className="text-xs text-[var(--panel-muted-fg)]">
          {NO_CONNECTION}
        </p>
      ) : null}
      <TextField
        label="Connector address"
        value={address}
        readOnly={readOnly}
        describedBy={describedBy}
        onCommit={(text) => onCommit(text, port, baud)}
      />
      <TextField
        label="Controller port"
        value={port}
        readOnly={readOnly}
        describedBy={describedBy}
        onCommit={(text) => onCommit(address, text, baud)}
      />
      <NumberField
        label="Baud rate"
        accessibleName="Baud rate"
        value={baud}
        readOnly={readOnly}
        describedBy={describedBy}
        notice={notice}
        onCommit={(value) => onCommit(address, port, value)}
      />
    </section>
  );
}

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
import { PanelSwitch } from "@/components/ui/switch";
import { Tooltip } from "@/components/ui/tooltip";
import { ConnectorGroup } from "@/components/workbench/ConnectorGroup";
import { ARMED_ACCENT_BUTTON, FIELD, ICON_BUTTON } from "@/components/workbench/machineStyles";
import { MachineProfileList } from "@/components/workbench/MachineProfileList";
import { MachineSizeOffer } from "@/components/workbench/MachineSizeOffer";
import { ARM_TRASH, ARM_ZONE, useDeleteArm } from "@/hooks/useDeleteArm";
import type { UnitSymbol } from "@/hooks/useDocumentPanel";
import type { MachineView, WorkbenchesApi } from "@/hooks/useWorkbenches";
import type { LibrarySyncApi } from "@/lib/devicesApi";
import { typedNumber } from "@/lib/machineBlockKeys";
import { setMachineWriteFailureHandler } from "@/lib/machineProfilesFile";
import { formatWorkArea } from "@/lib/workAreaText";

/** The write-failure alert (UX notes §7). */
const WRITE_FAILED = "Could not save the profile. Nothing was changed.";
/** The armed Update profile text. */
const ARMED_UPDATE = "Update profile. Overwrites the saved profile.";
/** Why the connection fields are read-only. */
const NO_CONNECTION = "Only saved profiles have a connection.";
/** The write-failure alert after Use machine size: the copy and page did change (`specs/0131` UX
 * notes §4). */
const SIZE_WRITE_FAILED = "Could not save the profile.";
/** How long after Use machine size a write failure is its own. */
const SIZE_WRITE_WINDOW_MS = 5000;
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
  /** The library sync's toggles (`specs/0116-library-sync`); absent where the sync is not
   * wired, `view` null while this device is in no group. */
  library?: LibrarySyncApi;
  /** After a mouse click on a switch the keyboard goes back to the canvas. */
  onReturnFocus?: () => void;
}

/**
 * The Machine block of a laser manufacturing (`specs/0026-machine-profile` UX notes §2 to §7): the
 * profile picker with Save as profile, the state row, the inline profile list, the copy's fields
 * (each edit one undo step) and the connection of the chosen saved profile (library, no undo).
 * Host-drawn from the session's view; the extension only places it (`adrs.md` note 4).
 */
export function MachineBlock({ label, machine, workbenches, unit = "mm", library, onReturnFocus = () => {} }: MachineBlockProps) {
  const api = workbenches.machine;
  const [open, setOpen] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);
  const profileButton = useRef<HTMLButtonElement>(null);
  const sizePressedAt = useRef(Number.NEGATIVE_INFINITY);
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
    setMachineWriteFailureHandler(() => {
      const own = Date.now() - sizePressedAt.current < SIZE_WRITE_WINDOW_MS;
      show(own ? SIZE_WRITE_FAILED : WRITE_FAILED, "save", true);
    });
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
  const focusProfile = () => profileButton.current?.focus();
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
      <p className="text-xs text-[var(--panel-muted-fg)]">Fixture sets in profile: {machine.fixture_sets_in_profile}</p>
      {machine.name_notice ? (
        <p role="status" className="text-xs text-[var(--toolbar-icon)]">
          {machine.name_notice}
        </p>
      ) : null}
      {/* The state row keeps its 28 px place on every laser page, empty while the copy equals its
          profile, so nothing below it moves under the pointer after a press (`specs/0029` P31). */}
      {machine.state === null || machine.state === undefined ? (
        <div aria-hidden data-machine-state-row="" className="h-7 shrink-0" />
      ) : (
        <div data-machine-state-row="" className="flex h-7 items-center justify-between gap-1" {...{ [ARM_ZONE]: "update" }}>
          <span role="status" className="text-xs text-[var(--toolbar-icon)]">
            {machine.state === "differs" ? "Differs from profile" : "Not in your profiles"}
          </span>
          {/* Reset from profile first with the save mark, Update profile last with the
              counter-clockwise arrow (`specs/0130` criterion 2a, Frage 260 B); Update is hidden
              for the built-in, so Reset then holds the rightmost slot. */}
          {machine.state === "differs" ? (
            <span className="flex gap-1">
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
                  <BookmarkCheck size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                </button>
              </Tooltip>
              {machine.can_update ? (
                <Tooltip side="left" content={update.armed ? ARMED_UPDATE : "Update profile"}>
                  <button
                    type="button"
                    aria-label={update.armed ? ARMED_UPDATE : "Update profile"}
                    {...{ [ARM_TRASH]: "update" }}
                    className={update.armed ? ARMED_ACCENT_BUTTON : ICON_BUTTON}
                    onClick={() =>
                      update.press("update", ARMED_UPDATE, () => {
                        show(api.update(), "save");
                        profileButton.current?.focus();
                        return "";
                      })
                    }
                  >
                    <RotateCcw size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                  </button>
                </Tooltip>
              ) : null}
            </span>
          ) : null}
          <span role="status" className="sr-only">
            {update.status}
          </span>
        </div>
      )}
      {open ? (
        <MachineProfileList
          rows={machine.profiles}
          library={library}
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
        <MachineSizeOffer
          profileId={machine.connection === null ? null : (machine.profiles.find((row) => row.chosen)?.id ?? null)}
          address={machine.connection?.connector_address.trim() ?? ""}
          unit={unit}
          api={api}
          width={copy.width}
          height={copy.height}
          onFocusProfile={focusProfile}
          onUsed={() => {
            sizePressedAt.current = Date.now();
          }}
        />
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
        {/* The numbers a new operation takes without a material (`specs/0150` criterion 1, W1, W2). */}
        <NumberField
          label="Default power"
          unit="%"
          accessibleName="Default power (percent)"
          value={copy.default_power}
          notice={noticeAt("default_power")}
          onCommit={(value) => show(api.setDefaultPower(value), "default_power")}
        />
        <NumberField
          label="Default speed"
          unit="mm/min"
          accessibleName="Default speed (mm/min)"
          value={copy.default_speed}
          notice={noticeAt("default_speed")}
          onCommit={(value) => show(api.setDefaultSpeed(value), "default_speed")}
        />
        <div className={SWITCH_ROW}>
          <ToolbarSwitch
            label="Laser mode"
            tooltip="Laser mode"
            checked={copy.laser_mode}
            onCheckedChange={(on) => api.setLaserMode(on)}
          />
        </div>
        {/* The Focus point mode's pulse (`specs/0118` 35 a, W60 to W62): after Laser mode, which
            greys it, GRBL profiles only. */}
        {copy.output === "GRBL" ? (
          <FocusPulseFields
            copy={copy}
            off={copy.laser_mode ? null : "Laser mode is off"}
            onFocusPulse={(on) => api.setFocusPulse(on)}
            onPower={(value) => show(api.setPulsePower(value), "pulse-power")}
            onLength={(value) => show(api.setPulseLength(value), "pulse-length")}
            noticeAt={noticeAt}
            onReturnFocus={onReturnFocus}
          />
        ) : null}
        {/* The capability Z offset reads (`specs/0111` criterion 22): one undo step on the page's
            profile copy; default off. */}
        <div className={SWITCH_ROW}>
          <PanelSwitch
            label="Motorised Z axis"
            checked={copy.motorised_z === true}
            onCheckedChange={(on) => api.setMotorisedZ(on)}
            onReturnFocus={onReturnFocus}
          />
        </div>
        {/* Air assist (`specs/0029` criterion 25): one undo step on the page's profile copy. */}
        <div className={SWITCH_ROW}>
          <PanelSwitch
            label="Air assist"
            checked={copy.air_assist === true}
            onCheckedChange={(on) => api.setAirAssist(on)}
            onReturnFocus={onReturnFocus}
          />
        </div>
        {/* The output format, plain text while there is one (`specs/0029` P23, criterion 39). */}
        <div className="flex h-7 items-center justify-between">
          <span className="text-sm text-[var(--toolbar-icon)]">Output</span>
          <span className="text-[13px] text-[var(--panel-muted-fg)]">{copy.output}</span>
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

/**
 * Focus pulse, Pulse power and Pulse length (`specs/0118` 35 a, W60 to W62): copy fields, one undo
 * step per commit, clamped on commit with the built notice. While the copy's Laser mode is off all
 * three are greyed in place at 40 % with the cause, their values kept; Power and Length stay
 * editable while the pulse is off (W61).
 */
function FocusPulseFields({
  copy,
  off,
  onFocusPulse,
  onPower,
  onLength,
  noticeAt,
  onReturnFocus,
}: {
  copy: { focus_pulse: boolean; pulse_power: number; pulse_length: number };
  off: string | null;
  onFocusPulse: (on: boolean) => void;
  onPower: (value: number) => void;
  onLength: (value: number) => void;
  noticeAt: (anchor: string) => ReactNode;
  onReturnFocus: () => void;
}) {
  const causeId = useId();
  const greyed = off !== null;
  return (
    <>
      <div className={`${SWITCH_ROW} ${greyed ? "opacity-40" : ""}`}>
        <Tooltip side="left" content={greyed ? <Cause name="Focus pulse" cause={off} /> : "Focus pulse"}>
          <span className="w-full">
            <PanelSwitch
              label="Focus pulse"
              checked={copy.focus_pulse === true}
              onCheckedChange={(on) => {
                if (!greyed) onFocusPulse(on);
              }}
              onReturnFocus={onReturnFocus}
              extra={greyed ? { "aria-disabled": "true", "aria-describedby": causeId } : undefined}
            />
          </span>
        </Tooltip>
      </div>
      <NumberField
        label="Pulse power"
        unit="%"
        accessibleName="Pulse power (percent of max power)"
        value={copy.pulse_power}
        notice={noticeAt("pulse-power")}
        greyed={off}
        describedBy={greyed ? causeId : undefined}
        onCommit={onPower}
      />
      <NumberField
        label="Pulse length"
        unit="ms"
        accessibleName="Pulse length (ms)"
        value={copy.pulse_length}
        notice={noticeAt("pulse-length")}
        greyed={off}
        describedBy={greyed ? causeId : undefined}
        onCommit={onLength}
      />
      {greyed ? (
        <span id={causeId} className="sr-only">
          {off}
        </span>
      ) : null}
    </>
  );
}

/** A greyed control's tooltip: its name, and the cause as the second line. */
function Cause({ name, cause }: { name: string; cause: string }) {
  return (
    <>
      <div>{name}</div>
      <div className="text-muted-foreground">{cause}</div>
    </>
  );
}

/** A switch row: the label at the left, the switch at the row's right edge (`specs/0111` 6.3, P31). */
const SWITCH_ROW = "flex h-7 items-center justify-between [&>label]:w-full [&>label]:justify-between";

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
  greyed = null,
  describedBy,
  onCommit,
}: {
  label: string;
  unit?: string;
  accessibleName: string;
  value: number;
  notice?: ReactNode;
  readOnly?: boolean;
  /** Greyed in place with this cause (`aria-disabled`, 40 %): the value shows, typing changes
   * nothing (`specs/0118` W62). */
  greyed?: string | null;
  describedBy?: string;
  onCommit: (value: number) => void;
}) {
  const draft = useDraft(numberText(value));
  if (greyed !== null) {
    return (
      <div className="relative">
        <Tooltip side="left" content={<Cause name={label} cause={greyed} />}>
          <label className={`${FIELD} opacity-40`}>
            <span className="shrink-0 text-[var(--toolbar-icon)]">{label}</span>
            <input
              aria-label={accessibleName}
              aria-describedby={describedBy}
              aria-disabled="true"
              readOnly
              value={numberText(value)}
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
      <ConnectorGroup
        profileId={readOnly ? null : (machine.profiles.find((row) => row.chosen)?.id ?? null)}
        address={address.trim()}
        reasonId={readOnly ? reason : undefined}
      />
    </section>
  );
}

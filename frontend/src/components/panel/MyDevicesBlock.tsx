// The Share tab's My devices block (`specs/0096-own-devices-sync` ux-notes 2 to 9): the rows,
// and where the decision rows, the workspace pick, the pairing and backup blocks go.

import { ChevronDown, ChevronRight, Globe, LogIn, LogOut, Monitor, MonitorSmartphone, Unlink } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";

import { ALERT, APPLY, ARMED, ArmLines, BUTTON, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import { useArm } from "@/hooks/useArm";
import { EntryField } from "@/components/panel/EntryField";
import { DecisionRow, PickWorkspaces } from "@/components/panel/DeviceChoices";
import { BackupLines, KeyBackupRow, RestoreButton } from "@/components/panel/KeyBackupBlock";
import { JoinerBlock, OpenerBlock } from "@/components/panel/PairingBlocks";
import type { DevicesApi, DeviceRow } from "@/hooks/useDevices";
import { type KeyBackupApi, useKeyBackup } from "@/hooks/useKeyBackup";
import {
  devices_browser_text,
  devices_removed_alert,
  devices_text,
  key_backup_text,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

interface MyDevicesBlockProps {
  devices: DevicesApi;
  /** The tab's server address (the Server field). */
  server: string;
  /** The frozen-room warning of the open synced workspace, appended to the pairing's
   * workspace choice (`specs/0097` ux-notes 3.2), or empty. */
  frozenWarning?: string;
  /** Why pairing is unavailable ("Set a server address.", ...), or empty. */
  reason: string;
  /** After Enter or Escape in the rename field: the keyboard goes back to the canvas. */
  onReturnFocus: () => void;
}

const GLYPH = { size: 16, strokeWidth: 1.5, absoluteStrokeWidth: true, "aria-hidden": true } as const;

/** One device's row (ux-notes 2.1): name (F2 renames the own one), fingerprint, last seen or its
 * sync state, the detail lines per synced workspace behind a disclosure, and Remove or Leave. */
function DeviceRowView({
  row,
  devices,
  backup,
  onRemoved,
  onReturnFocus,
}: {
  row: DeviceRow;
  devices: DevicesApi;
  backup: KeyBackupApi;
  /** A Remove went through: the block moves the focus on. */
  onRemoved: (member: string) => void;
  onReturnFocus: () => void;
}) {
  const arm = useArm(`device:${row.member || "own"}`);
  const [open, setOpen] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const action = useRef<HTMLButtonElement>(null);
  const first = useRef<HTMLButtonElement | null>(null);
  const returnTo = useRef<"action" | "first" | null>(null);
  useEffect(() => {
    if (!renaming && returnTo.current) {
      (returnTo.current === "action" ? action.current : (first.current ?? action.current))?.focus();
      returnTo.current = null;
    }
  });
  const details = row.details ?? [];
  const name = renaming ? (
    <EntryField
      label="Device name"
      shown={row.label}
      mixed={false}
      width={180}
      align="left"
      autoFocus
      messages={{ empty: "Enter a name.", long: "Use at most 24 characters." }}
      onSubmit={(text) => {
        if (text.trim() === row.label) {
          return "unchanged";
        }
        const refusal = devices.setDeviceName(text);
        return refusal === "" ? "committed" : refusal === "Enter a name." ? "invalid:empty" : "invalid:long";
      }}
      onClose={() => {
        returnTo.current = "first";
        setRenaming(false);
      }}
      onReturnFocus={onReturnFocus}
    />
  ) : (
    <p
      className="truncate text-sm text-[var(--toolbar-icon)]"
      title={row.label}
      onDoubleClick={() => setRenaming(row.own)}
    >
      {row.label}
      {row.own ? <span className={`${MUTED} ml-1`}>this device</span> : null}
    </p>
  );
  const text = (
    <div className="flex min-w-0 flex-1 flex-col text-left">
      {name}
      {row.fingerprint ? <p className="font-mono text-xs text-[var(--panel-muted-fg)]">{row.fingerprint}</p> : null}
    </div>
  );
  const Glyph = row.browser || (row.own && devices.browser) ? Globe : Monitor;
  const lines = [
    ...(row.seen ? [row.seen] : []),
    ...(row.own ? devices.browserLines : []),
  ];
  return (
    <li
      className={`flex flex-col ${row.fingerprint ? "min-h-[56px] pointer-coarse:min-h-[60px]" : ""}`}
      aria-busy={row.removing || undefined}
      onKeyDown={(event) => {
        if (event.key === "F2" && row.own && !renaming) {
          event.preventDefault();
          setRenaming(true);
        }
      }}
    >
      <div className="flex min-h-[40px] items-center gap-2 py-1">
        {details.length > 0 && !renaming ? (
          <button
            ref={first}
            type="button"
            data-row-first={row.member || "own"}
            aria-expanded={open}
            className="flex min-w-0 flex-1 items-center gap-2 rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            onClick={() => setOpen(!open)}
          >
            {open ? (
              <ChevronDown {...GLYPH} className="shrink-0 text-[var(--toolbar-icon)]" />
            ) : (
              <ChevronRight {...GLYPH} className="shrink-0 text-[var(--toolbar-icon)]" />
            )}
            <Glyph {...GLYPH} className="shrink-0 text-[var(--toolbar-icon)]" />
            {text}
          </button>
        ) : (
          <>
            <span aria-hidden className="w-4 shrink-0" />
            <Glyph {...GLYPH} className="shrink-0 text-[var(--toolbar-icon)]" />
            {text}
          </>
        )}
        {row.removing ? (
          <span aria-hidden className="w-7 shrink-0" />
        ) : (
          <span {...arm.zone} className="flex">
            <button
              {...arm.control}
              ref={action}
              type="button"
              data-row-first={details.length > 0 ? undefined : row.member || "own"}
              title={row.own ? "Leave my devices" : "Remove"}
              aria-label={row.own ? "Leave my devices" : `Remove ${row.label}`}
              className={`${BUTTON} w-7 ${arm.armed ? ARMED : `${OUTLINE} border-transparent`}`}
              onClick={() =>
                arm.press(() => {
                  const refusal = row.own ? devices.leave() : devices.remove(row.member);
                  if (refusal === "") {
                    if (row.own) {
                      // A passkey backup of a device that left is deleted (ADR 0040 decision 13).
                      backup.forget();
                    } else {
                      onRemoved(row.member);
                    }
                  }
                  return refusal;
                })
              }
            >
              {row.own ? <LogOut {...GLYPH} /> : <Unlink {...GLYPH} />}
            </button>
          </span>
        )}
      </div>
      {lines.map((line, index) => (
        <p key={line} role={index === 0 && row.seen ? "status" : undefined} aria-live={index === 0 && row.seen ? "polite" : undefined} className={`${MUTED} pl-12`}>
          {line}
        </p>
      ))}
      {open && details.length > 0 ? (
        <ul aria-label={`Workspaces of ${row.label}`} className="flex flex-col pl-6">
          {details.map((detail) => (
            <li key={detail.name} className="flex h-6 items-center justify-between gap-2">
              <span className="truncate text-sm text-[var(--toolbar-icon)]" title={detail.name}>
                {detail.name}
              </span>
              <span role="status" aria-live="polite" className={`${MUTED} shrink-0`}>
                {detail.state}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
      <ArmLines arm={arm} />
    </li>
  );
}

/**
 * The Share tab's My devices block (`specs/0096-own-devices-sync` ux-notes section 2): the
 * removed alert, the device list, the one-device warning, and Add a device / Join my devices
 * with their pairing blocks.
 */
export function MyDevicesBlock({ devices: given, server, reason, onReturnFocus, frozenWarning = "" }: MyDevicesBlockProps) {
  const backup = useKeyBackup(given);
  // A passkey backup is a state of the own row (ADR 0040 decision 13).
  const devices = backup.backedUp ? { ...given, browserLines: [key_backup_text("backed-up"), ...given.browserLines] } : given;
  const { view, pairing } = devices;
  const [joining, setJoining] = useState(false);
  const block = useRef<HTMLElement>(null);
  const addButton = useRef<HTMLButtonElement>(null);
  const joinButton = useRef<HTMLButtonElement>(null);
  /** Where the focus goes after the next render: a row (member, "own"), or a button. */
  const focusNext = useRef<string | null>(null);

  const reasonId = useId();
  const paired = view !== null && !view.removed;
  const joinReason = reason || devices.pairReason;
  // A browser joins a group but does not start one (ADR 0040 note).
  const addReason = joinReason || (devices.browser && !paired ? devices_browser_text("cannot-start") : "");
  const rows: DeviceRow[] =
    view && view.rows.length > 0
      ? view.rows
      : [{ member: "", label: devices.deviceName, fingerprint: "", own: true, seen: "" }];
  const openerBlock = pairing?.opener ? pairing.view : null;
  const joinerView = pairing && !pairing.opener ? pairing.view : null;
  // The device just paired by this one, whose workspaces are chosen now (ux-notes 3.1).
  const picking = devices.justPaired;

  // Focus rules (ux-notes 10): a closed pairing block returns to the button that opened it; the
  // new device's first row after its pairing; after a Remove the next row, else the previous,
  // else Add a device.
  const opened = openerBlock !== null || picking !== "" ? "add" : joining ? "join" : "";
  const wasOpen = useRef(opened);
  const wasPaired = useRef(paired);
  useEffect(() => {
    if (wasOpen.current !== "" && opened === "") {
      focusNext.current = wasOpen.current === "join" && paired && !wasPaired.current ? "row:own" : wasOpen.current;
    }
    wasOpen.current = opened;
    wasPaired.current = paired;
    const target = focusNext.current;
    if (target === null) {
      return;
    }
    focusNext.current = null;
    const button = target === "add" ? addButton.current : target === "join" ? joinButton.current : null;
    const row = target.startsWith("row:")
      ? block.current?.querySelector<HTMLElement>(`[data-row-first="${target.slice(4)}"]`)
      : null;
    (button ?? row ?? addButton.current)?.focus();
  });
  const removed = (member: string) => {
    const index = rows.findIndex((r) => r.member === member);
    const next = rows[index + 1] ?? rows[index - 1];
    focusNext.current = next ? `row:${next.member || "own"}` : "add";
  };

  return (
    <section
      ref={block}
      aria-label="My devices"
      className="flex flex-col gap-2 border-b border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] pb-3"
    >
      <div className="flex items-baseline justify-between gap-2">
        <p className="text-sm font-semibold text-[var(--toolbar-icon)]">My devices</p>
        <span className={MUTED}>{paired ? view.count : devices_text("only-this-device")}</span>
      </div>
      {view?.removed ? (
        <div className="flex flex-col gap-2">
          <p role="alert" className={ALERT}>
            {devices_removed_alert()}
          </p>
          <button
            type="button"
            className={`${BUTTON} ${OUTLINE} w-[60px]`}
            onClick={() => {
              focusNext.current = "join";
              devices.dismissRemoved();
            }}
          >
            OK
          </button>
        </div>
      ) : null}
      <ul aria-label="My devices" className="flex flex-col">
        {rows.map((row) =>
          paired ? (
            <DeviceRowView
              key={row.member || "own"}
              row={row}
              devices={devices}
              backup={backup}
              onRemoved={removed}
              onReturnFocus={onReturnFocus}
            />
          ) : (
            <li key="own" className="flex min-h-[40px] items-center gap-2 py-1">
              <span aria-hidden className="w-4 shrink-0" />
              {devices.browser ? (
                <Globe {...GLYPH} className="shrink-0 text-[var(--toolbar-icon)]" />
              ) : (
                <Monitor {...GLYPH} className="shrink-0 text-[var(--toolbar-icon)]" />
              )}
              <p className="truncate text-sm text-[var(--toolbar-icon)]">
                {row.label}
                <span className={`${MUTED} ml-1`}>this device</span>
              </p>
            </li>
          ),
        )}
      </ul>
      {paired && view.renewal ? (
        <p role="status" className={MUTED}>
          {view.renewal}
        </p>
      ) : null}
      {paired
        ? (view.alerts ?? []).map((alert) => (
            <p key={alert} role="alert" className={ALERT}>
              {alert}
            </p>
          ))
        : null}
      {paired
        ? devices.decisions.map((decision) => (
            <DecisionRow key={decision.id} devices={devices} id={decision.id} name={decision.name} text={decision.text} />
          ))
        : null}
      {devices.syncProgress ? (
        <p role="status" className={MUTED}>
          {devices.syncProgress}
        </p>
      ) : null}
      {paired
        ? devices.syncAlerts.map((alert) => (
            <p key={alert} role="alert" className={ALERT}>
              {alert}
            </p>
          ))
        : null}
      {paired && devices.browser && !openerBlock ? <KeyBackupRow backup={backup} /> : null}
      {openerBlock ? (
        <OpenerBlock devices={devices} view={openerBlock} server={server} />
      ) : picking !== "" ? (
        <PickWorkspaces
          label={picking}
          frozenWarning={frozenWarning}
          offered={devices.offered}
          synced={view ? view.workspaces.filter((w) => w.on).map((w) => w.id) : []}
          onDone={(chosen) => {
            devices.clearJustPaired();
            if (chosen.length > 0) {
              devices.syncChosen(chosen);
            }
          }}
        />
      ) : joining && !paired ? (
        <JoinerBlock devices={devices} view={joinerView} server={server} onClose={() => setJoining(false)} />
      ) : (
        <div className="flex flex-col gap-1">
          <div className="flex flex-wrap gap-2">
            <button
              ref={addButton}
              type="button"
              aria-disabled={addReason !== ""}
              aria-describedby={addReason ? reasonId : undefined}
              className={`${BUTTON} ${APPLY} w-[148px]`}
              onClick={() => {
                if (addReason === "") {
                  devices.addDevice();
                }
              }}
            >
              <MonitorSmartphone {...GLYPH} />
              Add a device
            </button>
            {paired ? null : (
              <button
                ref={joinButton}
                type="button"
                aria-disabled={joinReason !== ""}
                aria-describedby={joinReason ? reasonId : undefined}
                className={`${BUTTON} ${OUTLINE} w-[148px]`}
                onClick={() => {
                  if (joinReason === "") {
                    setJoining(true);
                  }
                }}
              >
                <LogIn {...GLYPH} />
                Join my devices
              </button>
            )}
          </div>
          {!paired && devices.browser ? (
            <>
              <RestoreButton backup={backup} server={server} reason={joinReason} reasonId={reasonId} />
              <BackupLines backup={backup} />
            </>
          ) : null}
          {addReason ? (
            <p id={reasonId} className={MUTED}>
              {joinReason || (paired ? "" : addReason)}
            </p>
          ) : null}
        </div>
      )}
    </section>
  );
}

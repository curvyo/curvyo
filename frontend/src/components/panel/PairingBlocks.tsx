// The pairing blocks of My devices (`specs/0096-own-devices-sync` ux-notes 3 and 4): the old
// device's code and compare step, and the new device's join block, with the check words.

import { FolderOpen } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";

import { ALERT, APPLY, BUTTON, FIELD, MUTED, OUTLINE, SURFACE } from "@/components/panel/devicesUi";
import type { DevicesApi, PairingView } from "@/hooks/useDevices";
import { formatPairCode, pairCodeComplete } from "@/lib/pairCode";
import {
  devices_waiting_for_other,
  server_shown,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** The four check words in a 2 x 2 grid (ux-notes 3, step 2). */
function CheckWords({ view }: { view: PairingView }) {
  const words = view.words ?? [];
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs text-[var(--toolbar-icon)]">Check words</span>
      <div className="grid grid-cols-2 gap-2">
        {words.map((word) => (
          <span key={word} className={`${SURFACE} px-2 py-1 text-base font-semibold text-[var(--toolbar-icon)]`}>
            {word}
          </span>
        ))}
      </div>
      <p className={MUTED}>{view.must_match}</p>
    </div>
  );
}

/** The old device's pairing block (ux-notes 3): code, compare, pairing, or the alert. */
export function OpenerBlock({ devices, view, server }: { devices: DevicesApi; view: PairingView; server: string }) {
  const group = useRef<HTMLDivElement>(null);
  useEffect(() => {
    group.current?.focus();
  }, [view.phase]);
  const escape = (event: React.KeyboardEvent) => {
    if (event.key === "Escape" && (view.phase === "code" || view.phase === "compare" || view.phase === "connecting")) {
      event.preventDefault();
      event.stopPropagation();
      devices.cancelPairing(view.phase === "compare");
    }
  };
  if (view.phase === "failed") {
    return (
      <div className="flex flex-col gap-2">
        <p role="alert" className={ALERT}>
          {view.text}
        </p>
        <div className="flex flex-wrap gap-2">
          <button type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={devices.closePairing}>
            Close
          </button>
          <button
            type="button"
            className={`${BUTTON} ${APPLY} w-[100px]`}
            onClick={() => {
              devices.closePairing();
              devices.addDevice();
            }}
          >
            New code
          </button>
        </div>
      </div>
    );
  }
  if (view.phase === "compare") {
    const name = `${view.heading ?? ""}. Check words: ${(view.words ?? []).join(", ")}.`;
    return (
      <div ref={group} tabIndex={-1} role="group" aria-label={name} className="flex flex-col gap-2 outline-none" onKeyDown={escape}>
        <p className="truncate text-sm text-[var(--toolbar-icon)]">{view.heading}</p>
        <p className="font-mono text-xs text-[var(--panel-muted-fg)]">{view.fingerprint}</p>
        <CheckWords view={view} />
        <div className="flex flex-wrap gap-2">
          <button type="button" className={`${BUTTON} ${APPLY} w-[76px]`} onClick={devices.pair}>
            Pair
          </button>
          <button type="button" className={`${BUTTON} ${OUTLINE} w-[110px]`} onClick={() => devices.cancelPairing(true)}>
            Words differ
          </button>
        </div>
      </div>
    );
  }
  if (view.phase === "pairing" || view.phase === "paired") {
    return (
      <p role="status" aria-busy={view.phase === "pairing"} className={MUTED}>
        {view.phase === "pairing" ? view.text : ""}
      </p>
    );
  }
  return (
    <div ref={group} tabIndex={-1} className="flex flex-col gap-2 outline-none" onKeyDown={escape}>
      <span className="text-xs text-[var(--toolbar-icon)]">Pairing code</span>
      <div role="group" aria-label={view.spelled ?? "Pairing code"}>
        <p
          aria-hidden
          className={`${SURFACE} select-all py-2 text-center font-mono text-2xl font-semibold tracking-[0.08em] text-[var(--toolbar-icon)]`}
        >
          {view.code ?? "...."}
        </p>
      </div>
      <p className={MUTED}>Server {server_shown(server)}</p>
      {view.valid ? (
        <p aria-hidden className={MUTED}>
          {view.valid}
        </p>
      ) : null}
      <p role="status" aria-live="polite" className="sr-only">
        {view.announce ?? ""}
      </p>
      <p role="status" aria-live="polite" className={MUTED}>
        {view.wrong ? view.wrong : devices_waiting_for_other()}
      </p>
      <button type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={() => devices.cancelPairing(false)}>
        Cancel
      </button>
    </div>
  );
}

/** The new device's join block (ux-notes 4.2 to 4.5). */
export function JoinerBlock({
  devices,
  view,
  server,
  onClose,
}: {
  devices: DevicesApi;
  view: PairingView | null;
  server: string;
  onClose: () => void;
}) {
  const [code, setCode] = useState("");
  const [name, setName] = useState(devices.deviceName);
  const reasonId = useId();
  const alertId = useId();
  const codeField = useRef<HTMLInputElement>(null);
  const connectButton = useRef<HTMLButtonElement>(null);
  const group = useRef<HTMLDivElement>(null);
  const waiting = view !== null && view.phase !== "failed";
  const problem = view?.phase === "failed" ? (view.problem ?? "") : "";
  const locked = problem === "locked";
  const reason = locked
    ? ""
    : !pairCodeComplete(code)
      ? "Enter the code."
      : name.trim() === ""
        ? "Enter a name."
        : "";
  const ready = reason === "" && !locked;
  // A refusal: a locked or expired code is cleared, a wrong one stays selected (4.5).
  const [seenProblem, setSeenProblem] = useState("");
  if (problem !== seenProblem) {
    setSeenProblem(problem);
    if (problem === "locked" || problem === "expired") {
      setCode("");
    }
  }
  useEffect(() => {
    if (problem === "wrong") {
      codeField.current?.select();
    }
    if (problem !== "") {
      codeField.current?.focus();
    }
  }, [problem]);
  // Waiting and compare: the focus goes to the group, so the words are read (4.3).
  const phase = view?.phase ?? "";
  useEffect(() => {
    if (phase === "compare" || phase === "joining" || phase === "connecting") {
      group.current?.focus();
    }
  }, [phase]);
  const connect = () => {
    if (!ready || waiting) {
      return;
    }
    if (name.trim() !== devices.deviceName) {
      devices.setDeviceName(name);
    }
    devices.joinDevices(code);
  };
  const close = () => {
    if (view) {
      devices.cancelPairing(false);
    }
    onClose();
  };
  if (view && (view.phase === "compare" || view.phase === "joining" || view.phase === "connecting")) {
    const label =
      view.phase === "compare" ? `${view.heading ?? ""}. Check words: ${(view.words ?? []).join(", ")}.` : "Connecting...";
    return (
      <div
        ref={group}
        tabIndex={-1}
        role="group"
        aria-label={label}
        className="flex flex-col gap-2 outline-none"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            close();
          }
        }}
      >
        <p role="status" aria-live="polite" className="text-sm text-[var(--toolbar-icon)]">
          {view.phase === "compare" ? view.heading : "Connecting..."}
        </p>
        {view.phase === "compare" ? <CheckWords view={view} /> : null}
        <div className="flex flex-wrap gap-2">
          <button type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={close}>
            Cancel
          </button>
          {view.phase === "compare" ? (
            <button
              type="button"
              className={`${BUTTON} ${OUTLINE} w-[110px]`}
              onClick={() => {
                devices.cancelPairing(true);
                onClose();
              }}
            >
              Words differ
            </button>
          ) : null}
        </div>
      </div>
    );
  }
  return (
    <div
      className="flex flex-col gap-2"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          close();
        }
      }}
    >
      <label className="flex flex-col gap-1">
        <span className="text-xs text-[var(--toolbar-icon)]">Pairing code</span>
        <input
          ref={codeField}
          autoFocus
          autoComplete="off"
          spellCheck={false}
          autoCapitalize="characters"
          readOnly={locked}
          className={`${FIELD} font-mono text-base`}
          value={code}
          aria-invalid={problem !== ""}
          aria-describedby={problem !== "" ? alertId : undefined}
          onChange={(event) => setCode(formatPairCode(event.target.value))}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              connect();
            }
          }}
        />
      </label>
      {problem !== "" ? (
        <div className="flex flex-col gap-1">
          <p id={alertId} role="alert" className={ALERT}>
            {view?.text}
          </p>
          {problem === "lost" || problem === "unreachable" ? (
            <button type="button" className={`${BUTTON} ${OUTLINE} w-[90px]`} onClick={connect}>
              Try again
            </button>
          ) : null}
        </div>
      ) : null}
      <p className={MUTED}>Server {server_shown(server)}</p>
      <label className="flex flex-col gap-1">
        <span className="text-xs text-[var(--toolbar-icon)]">Device name</span>
        <input
          className={FIELD}
          value={name}
          maxLength={24}
          aria-invalid={name.trim() === ""}
          onChange={(event) => setName(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              connectButton.current?.focus();
            }
          }}
        />
      </label>
      {devices.browser ? null : (
        <div className="flex flex-col gap-1">
          <span className="text-xs text-[var(--toolbar-icon)]">Synced workspaces folder</span>
          <p className={`${MUTED} truncate`} title={devices.syncedFolder.path || "~/Curvyo"}>
            {devices.syncedFolder.path || "~/Curvyo"}
          </p>
          <button type="button" className={`${BUTTON} ${OUTLINE} w-[148px]`} onClick={devices.chooseSyncedFolder}>
            <FolderOpen size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            Change folder...
          </button>
          {devices.syncedFolder.refusal ? (
            <p role="alert" className={ALERT}>
              {devices.syncedFolder.refusal}
            </p>
          ) : null}
        </div>
      )}
      <div className="flex flex-wrap gap-2">
        <button type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={close}>
          Cancel
        </button>
        <button
          ref={connectButton}
          type="button"
          aria-disabled={!ready}
          aria-describedby={ready ? undefined : locked ? alertId : reasonId}
          className={`${BUTTON} ${APPLY} w-[90px]`}
          onClick={connect}
        >
          Connect
        </button>
      </div>
      {reason ? (
        <p id={reasonId} className={MUTED}>
          {reason}
        </p>
      ) : null}
    </div>
  );
}

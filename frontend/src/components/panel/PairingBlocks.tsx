// The pairing blocks of My devices (`specs/0096-own-devices-sync` ux-notes 3 and 4): the old
// device's code and compare step, and the new device's join block, with the check words.

import { FolderOpen, KeyRound, Plug, RefreshCw, X } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";

import { FIELD, MUTED, SURFACE } from "@/components/panel/devicesUi";
import type { DevicesApi, PairingView } from "@/hooks/useDevices";
import { formatPairCode, pairCodeComplete } from "@/lib/pairCode";
import {
  devices_waiting_for_other,
  server_shown,
} from "@/wasm-bindings/curvyo_editor_wasm.js";
import { Button } from "@/components/ui/button";
import { AlertLine } from "@/components/ui/notice";

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
        <AlertLine>
          {view.text}
        </AlertLine>
        <div className="flex flex-wrap gap-2">
          <Button icon={X} className="min-w-[76px]" onClick={devices.closePairing}>
            Close
          </Button>
          <Button variant="primary" icon={RefreshCw} className="min-w-[100px]"
            onClick={() => {
              devices.closePairing();
              devices.addDevice();
            }}
          >
            New code
          </Button>
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
          <Button variant="primary" icon={KeyRound} className="min-w-[76px]" onClick={devices.pair}>
            Pair
          </Button>
          <Button icon={X} className="min-w-[110px]" onClick={() => devices.cancelPairing(true)}>
            Words differ
          </Button>
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
      <Button icon={X} className="min-w-[76px]" onClick={() => devices.cancelPairing(false)}>
        Cancel
      </Button>
    </div>
  );
}

/** The new device's join block (ux-notes 4.2 to 4.5). */
export function JoinerBlock({
  devices,
  view,
  server,
  onClose,
  code: given,
}: {
  devices: DevicesApi;
  view: PairingView | null;
  server: string;
  onClose: () => void;
  /** The code typed into the Join field (`specs/0105` criterion 16): no field of its own. */
  code?: string;
}) {
  const [typed, setCode] = useState("");
  const code = given ?? typed;
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
          <Button icon={X} className="min-w-[76px]" onClick={close}>
            Cancel
          </Button>
          {view.phase === "compare" ? (
            <Button icon={X} className="min-w-[110px]"
              onClick={() => {
                devices.cancelPairing(true);
                onClose();
              }}
            >
              Words differ
            </Button>
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
      {given === undefined ? (
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
      ) : null}
      {problem !== "" ? (
        <div className="flex flex-col gap-1">
          <AlertLine id={alertId}>
            {view?.text}
          </AlertLine>
          {problem === "lost" || problem === "unreachable" ? (
            <Button icon={RefreshCw} className="min-w-[90px]" onClick={connect}>
              Try again
            </Button>
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
          <Button icon={FolderOpen}  className="min-w-[148px]" onClick={devices.chooseSyncedFolder}>
Change folder...
</Button>
          {devices.syncedFolder.refusal ? (
            <AlertLine>
              {devices.syncedFolder.refusal}
            </AlertLine>
          ) : null}
        </div>
      )}
      <div className="flex flex-wrap gap-2">
        <Button icon={X} className="min-w-[76px]" onClick={close}>
          Cancel
        </Button>
        <Button variant="primary" icon={Plug}
          ref={connectButton}
          aria-disabled={!ready}
          aria-describedby={ready ? undefined : locked ? alertId : reasonId} className="min-w-[90px]"
          onClick={connect}
        >
          Connect
        </Button>
      </div>
      {reason ? (
        <p id={reasonId} className={MUTED}>
          {reason}
        </p>
      ) : null}
    </div>
  );
}

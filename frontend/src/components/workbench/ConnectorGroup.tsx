import { CircleAlert, KeyRound, PlugZap, Unlink } from "lucide-react";
import { type ReactNode, type RefObject, useEffect, useRef, useState } from "react";

import { ARMED_DESTRUCTIVE_BUTTON, GLYPH, ICON_BUTTON, SMALL } from "@/components/workbench/machineStyles";
import { SymbolButton } from "@/components/workbench/SymbolButton";
import { Tooltip } from "@/components/ui/tooltip";
import { type ConnectorLinkModel, useConnectorLink } from "@/hooks/useConnectorLink";
import { useArm } from "@/hooks/useArm";
import {
  type Line,
  codeComplete,
  controllerRows,
  fingerprintLines,
  pairRowShown,
  trustLabel,
  typedCode,
} from "@/lib/connectorText";
import { isTauri } from "@/platform/host";

interface Props {
  /** The chosen saved profile's id; `null` for the built-in profile or a profile not in the library. */
  profileId: string | null;
  address: string;
  /** The id of the "Only saved profiles have a connection." line, when it is shown. */
  reasonId?: string;
}

/**
 * The Connector group of the Machine block (`specs/0064-machine-connector` UX notes §2 to §10;
 * `specs/0118` UX notes §9): test, pair and forget the connector, and what its controller
 * reported. The job buttons live in the Operations tab's Machine controls. Desktop only.
 */
export function ConnectorGroup(props: Props) {
  // A new profile or address starts from empty state.
  return <ConnectorGroupFor key={`${props.profileId ?? ""}|${props.address}`} {...props} />;
}

function ConnectorGroupFor({ profileId, address, reasonId }: Props) {
  const desktop = isTauri;
  const usable = desktop && profileId !== null;
  const model = useConnectorLink(profileId, address, usable);
  const ready = usable && address !== "";
  // After pairing the focus goes to Test connection (W11).
  const testButton = useRef<HTMLButtonElement | null>(null);
  return (
    <section aria-label="Connector" className="mt-1 flex flex-col gap-1">
      <div className="flex h-7 items-center justify-between">
        <h4 className="text-sm text-[var(--toolbar-icon)]">Connector</h4>
        {desktop ? (
          <SymbolButton
            name="Test connection"
            enabled={ready && model.busy === null}
            busy={model.busy === "test"}
            describedBy={usable ? undefined : reasonId}
            buttonRef={(node) => {
              testButton.current = node;
            }}
            onPress={model.test}
          >
            <PlugZap {...GLYPH} />
          </SymbolButton>
        ) : null}
      </div>
      {!desktop ? (
        <p className="text-xs text-[var(--panel-muted-fg)]">Needs the desktop app</p>
      ) : usable ? (
        <ConnectorBody model={model} address={address} testButton={testButton} />
      ) : null}
    </section>
  );
}

function ConnectorBody({
  model,
  address,
  testButton,
}: {
  model: ConnectorLinkModel;
  address: string;
  testButton: RefObject<HTMLButtonElement | null>;
}) {
  // Where the focus goes when a button unmounts: the Code field after Forget.
  const [focusCode, setFocusCode] = useState(false);
  if (address === "") {
    return <p className="text-xs text-[var(--panel-muted-fg)]">No connector address</p>;
  }
  const showPair = pairRowShown({ paired: model.paired !== null, noStore: model.noStore, maintenance: model.maintenance });
  return (
    <>
      <LinkLine line={model.line} alert={model.alert} />
      {model.keyChanged ? (
        <KeyBlock rows={[["Stored key", model.keyChanged.stored], ["Presented key", model.keyChanged.presented]]} />
      ) : null}
      {model.paired ? <TrustBlock model={model} onForget={() => setFocusCode(true)} /> : null}
      {showPair ? (
        <PairRow model={model} focusOnMount={focusCode} onPaired={() => testButton.current?.focus({ preventScroll: true })} />
      ) : null}
      {model.paired ? <ControllerBlock model={model} /> : null}
    </>
  );
}

function LinkLine({ line, alert }: { line: Line | null; alert: string }) {
  return (
    <>
      <p
        role="status"
        className={`flex items-start gap-1 text-xs [overflow-wrap:anywhere] ${line?.failure ? "text-[var(--destructive)]" : "text-[var(--toolbar-icon)]"} ${line === null ? "-mb-1" : ""}`}
      >
        {line?.failure ? <CircleAlert {...SMALL} className="mt-0.5 shrink-0" /> : null}
        <span className={line?.text === "Testing" || line?.text === "Pairing" ? "text-[var(--panel-muted-fg)]" : ""}>
          {line?.text}
        </span>
      </p>
      <p role="alert" className="sr-only">
        {alert}
      </p>
    </>
  );
}

function KeyBlock({ rows }: { rows: [string, string][] }) {
  return (
    <dl className="grid grid-cols-[80px_minmax(0,1fr)] gap-x-1 gap-y-1 text-xs">
      {rows.map(([label, fingerprint]) => (
        <KeyRow key={label} label={label} fingerprint={fingerprint} />
      ))}
    </dl>
  );
}

function KeyRow({ label, fingerprint, children }: { label: string; fingerprint: string; children?: ReactNode }) {
  const [first, second] = fingerprintLines(fingerprint);
  return (
    <>
      <dt className="text-[var(--panel-muted-fg)]">{label}</dt>
      <dd className="m-0 flex items-start justify-between gap-1 text-[var(--toolbar-icon)]">
        <span className="select-text font-mono tabular-nums">
          {first}
          <br />
          {second}
        </span>
        {children}
      </dd>
    </>
  );
}

function TrustBlock({ model, onForget }: { model: ConnectorLinkModel; onForget: () => void }) {
  const arm = useArm("forget-connector");
  const paired = model.paired;
  if (paired === null) return null;
  const name = arm.armed ? "Forget pairing. A new code is needed." : "Forget pairing";
  return (
    <dl
      aria-label={trustLabel(paired, model.session)}
      className="grid grid-cols-[80px_minmax(0,1fr)] gap-x-1 gap-y-1 text-xs"
      {...arm.zone}
    >
      <dt className="self-center text-[var(--panel-muted-fg)]">Paired with</dt>
      <dd className="m-0 flex items-center justify-between gap-1">
        <span className="min-w-0 truncate text-[var(--toolbar-icon)]" title={paired.connectorName}>
          {paired.connectorName}
        </span>
        <Tooltip side="left" content={name}>
          <button
            type="button"
            aria-label={name}
            {...arm.control}
            className={arm.armed ? ARMED_DESTRUCTIVE_BUTTON : ICON_BUTTON}
            onClick={() =>
              arm.press(() => {
                onForget();
                model.forget();
                return "";
              })
            }
          >
            <Unlink {...GLYPH} />
          </button>
        </Tooltip>
      </dd>
      {model.session?.ownerLabel ? (
        <>
          <dt className="text-[var(--panel-muted-fg)]">Belongs to</dt>
          <dd className="m-0 min-w-0 truncate text-[var(--toolbar-icon)]" title={model.session.ownerLabel}>
            {model.session.ownerLabel}
          </dd>
        </>
      ) : null}
      {model.session ? (
        <>
          <dt className="text-[var(--panel-muted-fg)]">This app</dt>
          <dd className="m-0 whitespace-nowrap text-[var(--toolbar-icon)]">{model.session.role}</dd>
        </>
      ) : null}
      <KeyRow label="Key" fingerprint={paired.fingerprint} />
    </dl>
  );
}

function PairRow({ model, focusOnMount, onPaired }: { model: ConnectorLinkModel; focusOnMount: boolean; onPaired: () => void }) {
  const [code, setCode] = useState("");
  const field = useRef<HTMLInputElement>(null);
  const pairing = model.busy === "pair";
  const enabled = codeComplete(code) && model.busy === null;
  useEffect(() => {
    if (focusOnMount) field.current?.focus();
  }, [focusOnMount]);
  const press = async () => {
    if (!enabled) return;
    const ok = await model.pair(code);
    if (ok) {
      onPaired();
      setCode("");
    } else {
      field.current?.focus();
      field.current?.select();
    }
  };
  return (
    <div className="flex items-center gap-1">
      <label className="flex h-7 w-[212px] items-center gap-1 rounded-[5px] border border-[var(--field-border)] bg-white px-2 focus-within:ring-2 focus-within:ring-[var(--editor-accent)]">
        <span className="shrink-0 text-xs text-[var(--panel-muted-fg)]">Code</span>
        <input
          ref={field}
          aria-label="Code"
          autoCapitalize="characters"
          autoComplete="off"
          spellCheck={false}
          readOnly={pairing}
          value={code}
          onChange={(event) => setCode(typedCode(event.target.value))}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void press();
            } else if (event.key === "Escape") {
              event.preventDefault();
              setCode("");
            }
          }}
          className="min-w-0 flex-1 bg-transparent text-right font-mono text-sm outline-none"
        />
      </label>
      <SymbolButton name="Pair" enabled={enabled} busy={pairing} onPress={() => void press()}>
        <KeyRound {...GLYPH} />
      </SymbolButton>
    </div>
  );
}

/** The Controller block (`specs/0127` UX notes section 2): what the connector's controller
 * reported; text only, no control. Without a controller one "Controller" row names why
 * (`specs/0118` W10); the settings warnings sit in the Machine controls footer (W9). */
function ControllerBlock({ model }: { model: ConnectorLinkModel }) {
  const status = model.status;
  if (status === null) return null;
  if (status.state.name === "NoController") {
    const failure = status.detail !== null && status.detail !== undefined && status.detail !== "";
    return (
      <div role="group" aria-label="Controller" className="mt-1 flex flex-col gap-1">
        <dl className="grid select-text grid-cols-[80px_minmax(0,1fr)] gap-x-1 gap-y-1 text-xs">
          <dt className="text-[var(--panel-muted-fg)]">Controller</dt>
          <dd
            role="status"
            className={`m-0 [overflow-wrap:anywhere] ${failure ? "text-[var(--destructive)]" : "text-[var(--toolbar-icon)]"}`}
          >
            {failure ? status.detail : "No controller"}
          </dd>
        </dl>
      </div>
    );
  }
  const info = model.controller;
  if (info === null) return null;
  const rows = controllerRows(info, status);
  return (
    <div role="group" aria-label="Controller" className="mt-1 flex flex-col gap-1">
      <dl className="grid select-text grid-cols-[80px_minmax(0,1fr)] gap-x-1 gap-y-1 text-xs">
        {rows.map(([label, value]) => (
          <div key={label} className="contents">
            <dt className="text-[var(--panel-muted-fg)]">{label}</dt>
            <dd aria-live="off" className="m-0 tabular-nums text-[var(--toolbar-icon)] [overflow-wrap:anywhere]">
              {value}
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

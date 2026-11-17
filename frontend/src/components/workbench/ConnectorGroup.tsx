import {
  CircleAlert,
  KeyRound,
  LoaderCircle,
  Pause,
  Play,
  PlugZap,
  Square,
  TriangleAlert,
  Unlink,
  Upload,
} from "lucide-react";
import { type KeyboardEvent, type ReactNode, useEffect, useRef, useState } from "react";

import { ICON_BUTTON } from "@/components/workbench/machineStyles";
import { Tooltip } from "@/components/ui/tooltip";
import { type Busy, type ConnectorModel, useConnector } from "@/hooks/useConnector";
import { useArm } from "@/hooks/useArm";
import {
  type Line,
  codeComplete,
  fingerprintLines,
  formatCount,
  formatElapsed,
  hasProgress,
  jobButtons,
  jobStateLine,
  pairRowShown,
  typedCode,
} from "@/lib/connectorText";
import { isTauri } from "@/platform/host";

const GLYPH = { size: 16, strokeWidth: 1.5, absoluteStrokeWidth: true, "aria-hidden": true } as const;
const SMALL = { size: 12, strokeWidth: 1.5, absoluteStrokeWidth: true, "aria-hidden": true } as const;
const OFF = "aria-disabled:opacity-40 aria-disabled:hover:bg-transparent";

interface Props {
  /** The chosen saved profile's id; `null` for the built-in profile or a profile not in the library. */
  profileId: string | null;
  address: string;
  /** The id of the "Only saved profiles have a connection." line, when it is shown. */
  reasonId?: string;
}

/**
 * The Connector group of the Machine block (`specs/0064-machine-connector` UX notes §2 to §10):
 * test, pair, forget, send job and the job controls for the connector on the machine. Desktop only.
 */
export function ConnectorGroup(props: Props) {
  // A new profile or address starts from empty state.
  return <ConnectorGroupFor key={`${props.profileId ?? ""}|${props.address}`} {...props} />;
}

function ConnectorGroupFor({ profileId, address, reasonId }: Props) {
  const desktop = isTauri;
  const usable = desktop && profileId !== null;
  const model = useConnector(profileId, address, usable);
  const ready = usable && address !== "";
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
            onPress={model.test}
          >
            <PlugZap {...GLYPH} />
          </SymbolButton>
        ) : null}
      </div>
      {!desktop ? (
        <p className="text-xs text-[var(--panel-muted-fg)]">Needs the desktop app</p>
      ) : usable ? (
        <ConnectorBody model={model} address={address} />
      ) : null}
    </section>
  );
}

function ConnectorBody({ model, address }: { model: ConnectorModel; address: string }) {
  // Where the focus goes when a button unmounts: the Code field after Forget, Send job after Pair.
  const [focusCode, setFocusCode] = useState(false);
  const [focusSend, setFocusSend] = useState(false);
  if (address === "") {
    return <p className="text-xs text-[var(--panel-muted-fg)]">No connector address</p>;
  }
  const showPair = pairRowShown({ paired: model.paired !== null, noStore: model.noStore });
  return (
    <>
      <LinkLine line={model.line} alert={model.alert} />
      {model.keyChanged ? (
        <KeyBlock rows={[["Stored key", model.keyChanged.stored], ["Presented key", model.keyChanged.presented]]} />
      ) : null}
      {model.paired ? <TrustBlock model={model} onForget={() => { setFocusCode(true); setFocusSend(false); }} /> : null}
      {showPair ? <PairRow model={model} focusOnMount={focusCode} onPaired={() => setFocusSend(true)} /> : null}
      {model.paired ? <JobSection model={model} focusFirst={focusSend} /> : null}
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

function TrustBlock({ model, onForget }: { model: ConnectorModel; onForget: () => void }) {
  const arm = useArm("forget-connector");
  const paired = model.paired;
  if (paired === null) return null;
  const name = arm.armed ? "Forget pairing. A new code is needed." : "Forget pairing";
  return (
    <dl
      aria-label={`Paired with ${paired.connectorName}, key ${paired.fingerprint}`}
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
            className={arm.armed ? ICON_BUTTON.replace("hover:bg-[var(--editor-accent-hover)]", "bg-[var(--destructive)] text-white") : ICON_BUTTON}
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
      <KeyRow label="Key" fingerprint={paired.fingerprint} />
    </dl>
  );
}

function PairRow({ model, focusOnMount, onPaired }: { model: ConnectorModel; focusOnMount: boolean; onPaired: () => void }) {
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

function JobSection({ model, focusFirst }: { model: ConnectorModel; focusFirst: boolean }) {
  return (
    <>
      <JobToolbar model={model} focusFirst={focusFirst} />
      <JobBlock model={model} />
    </>
  );
}

function JobToolbar({ model, focusFirst }: { model: ConnectorModel; focusFirst: boolean }) {
  const buttons = jobButtons(model.status?.state ?? null);
  const idle = model.busy === null;
  const items: { key: string; name: string; enabled: boolean; busy: Busy; icon: ReactNode; press: () => void }[] = [
    { key: "send", name: "Send job...", enabled: buttons.send && idle, busy: "send", icon: <Upload {...GLYPH} />, press: model.send },
    {
      key: "start",
      name: buttons.startName,
      enabled: buttons.start && idle,
      busy: buttons.startName === "Resume" ? "resume" : "start",
      icon: <Play {...GLYPH} />,
      press: () => model.command(buttons.startName === "Resume" ? "resume" : "start"),
    },
    { key: "pause", name: "Pause", enabled: buttons.pause && idle, busy: "pause", icon: <Pause {...GLYPH} />, press: () => model.command("pause") },
    { key: "stop", name: "Stop", enabled: buttons.stop && idle, busy: "stop", icon: <Square {...GLYPH} />, press: () => model.command("stop") },
  ];
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const [rover, setRover] = useState(0);
  useEffect(() => {
    // After pairing the focus goes to Send job..., the first stop.
    if (focusFirst) refs.current[0]?.focus({ preventScroll: true });
  }, [focusFirst]);
  const move = (event: KeyboardEvent, at: number) => {
    const target =
      event.key === "ArrowLeft" ? Math.max(0, at - 1)
      : event.key === "ArrowRight" ? Math.min(items.length - 1, at + 1)
      : event.key === "Home" ? 0
      : event.key === "End" ? items.length - 1
      : null;
    if (target === null) return;
    event.preventDefault();
    setRover(target);
    refs.current[target]?.focus();
  };
  return (
    <div role="toolbar" aria-label="Job" className="flex gap-1">
      {items.map((item, at) => (
        <SymbolButton
          key={item.key}
          name={item.name}
          enabled={item.enabled}
          busy={model.busy === item.busy}
          tabIndex={rover === at ? 0 : -1}
          buttonRef={(node) => {
            refs.current[at] = node;
          }}
          onFocus={() => setRover(at)}
          onKeyDown={(event) => move(event, at)}
          onPress={item.press}
        >
          {item.icon}
        </SymbolButton>
      ))}
    </div>
  );
}

function JobBlock({ model }: { model: ConnectorModel }) {
  const { status, held, sendingName, refusal } = model;
  const sending = sendingName !== null && model.busy === "send";
  const line = sending
    ? ({ text: `Sending ${sendingName}`, failure: false } as Line)
    : refusal ?? (status ? jobStateLine(status, held) : null);
  if (line === null) return null;
  const alarm = status?.state.name === "Alarm" && refusal === null && !sending;
  const bar = status !== null && !sending && refusal === null && hasProgress(status);
  const lines = status?.job?.lines ?? 0;
  const done = status?.job?.line ?? 0;
  const name = sending ? sendingName : held?.name;
  const elapsed = status?.state.name === "Running" || status?.state.name === "Paused" || status?.state.name === "Done";
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-start justify-between gap-1 text-xs">
        <p
          role={alarm ? "alert" : "status"}
          className={`flex items-start gap-1 [overflow-wrap:anywhere] ${line.failure ? "text-[var(--destructive)]" : "text-[var(--toolbar-icon)]"}`}
        >
          {alarm ? <TriangleAlert {...SMALL} className="mt-0.5 shrink-0" /> : line.failure ? <CircleAlert {...SMALL} className="mt-0.5 shrink-0" /> : null}
          <span>{line.text}</span>
        </p>
        {elapsed && status?.job ? (
          <span className="shrink-0 tabular-nums text-[var(--panel-muted-fg)]">
            <span className="sr-only">Elapsed </span>
            {formatElapsed(status.job.elapsed_ms)}
          </span>
        ) : null}
      </div>
      {bar ? (
        <div
          role="progressbar"
          aria-label="Job progress"
          aria-valuemin={0}
          aria-valuemax={lines}
          aria-valuenow={done}
          aria-valuetext={`${formatCount(done)} of ${formatCount(lines)} lines`}
          className="h-1.5 w-[244px] max-w-full overflow-hidden rounded-full border border-[var(--field-border)] bg-white"
        >
          <div
            className={`h-full ${alarm || line.failure ? "bg-[var(--destructive)]" : "bg-[var(--editor-accent)]"}`}
            style={{ width: `${lines === 0 ? 0 : Math.min(100, (done / lines) * 100)}%` }}
          />
        </div>
      ) : null}
      <div aria-live="off" className="flex items-center justify-between gap-1 text-xs">
        <JobName name={name ?? ""} />
        {bar ? (
          <span className="shrink-0 tabular-nums text-[var(--toolbar-icon)]">
            {formatCount(done)} / {formatCount(lines)} lines
          </span>
        ) : null}
      </div>
    </div>
  );
}

/** The job's name, cut in the middle so the extension stays (UX notes §8). */
function JobName({ name }: { name: string }) {
  const dot = name.lastIndexOf(".");
  const [stem, extension] = dot > 0 ? [name.slice(0, dot), name.slice(dot)] : [name, ""];
  return (
    <span className="flex min-w-0 text-[var(--panel-muted-fg)]" title={name || undefined}>
      <span className="truncate">{stem}</span>
      <span className="shrink-0">{extension}</span>
    </span>
  );
}

interface SymbolButtonProps {
  name: string;
  enabled: boolean;
  busy?: boolean;
  describedBy?: string;
  tabIndex?: number;
  buttonRef?: (node: HTMLButtonElement | null) => void;
  onFocus?: () => void;
  onKeyDown?: (event: KeyboardEvent) => void;
  onPress: () => void;
  children: ReactNode;
}

/** A 28 px symbol button; when it does not apply it is greyed and keeps its place, its tab stop and
 * its tooltip, and a press does nothing (UX notes §3). */
function SymbolButton({ name, enabled, busy = false, describedBy, tabIndex, buttonRef, onFocus, onKeyDown, onPress, children }: SymbolButtonProps) {
  return (
    <Tooltip side="left" content={name}>
      <button
        type="button"
        ref={buttonRef}
        aria-label={name}
        aria-disabled={!enabled}
        aria-busy={busy || undefined}
        aria-describedby={describedBy}
        tabIndex={tabIndex}
        className={`${ICON_BUTTON} ${OFF}`}
        onFocus={onFocus}
        onKeyDown={onKeyDown}
        onClick={() => {
          if (enabled) onPress();
        }}
      >
        {busy ? <LoaderCircle {...GLYPH} className="animate-spin" /> : children}
      </button>
    </Tooltip>
  );
}

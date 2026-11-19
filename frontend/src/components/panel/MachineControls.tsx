import { CircleAlert, FileUp, House, LockOpen, Pause, Play, Square, TriangleAlert } from "lucide-react";
import { type KeyboardEvent, type ReactNode, type RefObject, useEffect, useId, useRef, useState } from "react";

import { GLYPH, SMALL } from "@/components/workbench/machineStyles";
import { SymbolButton } from "@/components/workbench/SymbolButton";
import { type MachineControlsModel, useMachineControls } from "@/hooks/useMachineControls";
import { formatCount, formatElapsed, jobButtons } from "@/lib/connectorText";
import { type ControlsLine, afterHoming, controlsLine, countsName, footerNotice, homeButton } from "@/lib/machineControls";
import { isTauri } from "@/platform/host";
import { controller_warnings } from "@/wasm-bindings/curvyo_editor_wasm.js";

interface Props {
  profileId: string;
  address: string;
  /** The page profile's Laser mode and Max power, for the settings warning lines (W9). */
  profile: ProfileValues | null;
}

interface ProfileValues {
  laserMode: boolean;
  maxPower: number;
}

/**
 * The Machine controls footer of a laser page's Operations tab (`specs/0118-machine-control`
 * UX notes sections 2 to 11): the settings warning, the job block and, as its bottom row, the
 * toolbar Home | Send job..., Start / Resume, Pause, Stop. Everything that comes and goes sits
 * above the toolbar, so the toolbar never moves.
 */
export function MachineControls(props: Props) {
  // A new profile or address starts from empty state.
  return <MachineControlsFor key={`${props.profileId}|${props.address}`} {...props} />;
}

function MachineControlsFor({ profileId, address, profile }: Props) {
  const heading = useId();
  const model = useMachineControls(profileId, address, isTauri);
  const notice = footerNotice(model.entry, isTauri);
  const paired = isTauri && model.entry.link === "paired";
  return (
    <section aria-labelledby={heading} data-machine-controls className="flex shrink-0 flex-col gap-2 px-3 pb-3">
      <div
        aria-hidden
        className="my-0.5 h-px w-full shrink-0"
        style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
      />
      <h3 id={heading} className="text-[11px] leading-4 font-semibold text-[var(--panel-muted-fg)]">
        Machine controls
      </h3>
      {notice !== null ? (
        <p className={`text-xs ${notice.failure ? "text-[var(--destructive)]" : "text-[var(--panel-muted-fg)]"}`}>
          {notice.text}
        </p>
      ) : paired ? (
        <>
          <Warnings model={model} profile={profile} />
          <ControlsBody model={model} />
        </>
      ) : null}
    </section>
  );
}

/** The Controller block's two settings warnings, where Start is pressed (W9). */
function Warnings({ model, profile }: { model: MachineControlsModel; profile: ProfileValues | null }) {
  const info = model.entry.controller;
  const shown = info !== null && model.entry.status !== null && model.entry.status.state.name !== "NoController";
  const warnings = shown && profile !== null
    ? controller_warnings(profile.laserMode, profile.maxPower, info.settings?.laser_mode ?? undefined, info.settings?.s_max ?? undefined)
    : [];
  return (
    <div role="status" className="flex flex-col gap-1 text-xs text-[var(--toolbar-icon)] empty:-mb-2">
      {warnings.map((text) => (
        <p key={text} className="flex items-start gap-1">
          <TriangleAlert {...SMALL} className="mt-0.5 shrink-0" />
          <span>{text}</span>
        </p>
      ))}
    </div>
  );
}

function ControlsBody({ model }: { model: MachineControlsModel }) {
  // Unlock leaves the tree when the alarm clears; a keyboard user who pressed it lands on the
  // toolbar's current button instead of the page body.
  const toolbar = useRef<HTMLDivElement>(null);
  const unlockHadFocus = useRef(false);
  const status = model.entry.status;
  const inAlarm = status?.state.name === "Alarm";
  useEffect(() => {
    if (inAlarm || !unlockHadFocus.current) return;
    unlockHadFocus.current = false;
    if (document.activeElement === null || document.activeElement === document.body) {
      toolbar.current?.querySelector<HTMLButtonElement>('[tabindex="0"]')?.focus();
    }
  }, [inAlarm]);
  // The state word after a homing is said once (W7).
  const before = useRef(status);
  const [spoken, setSpoken] = useState("");
  useEffect(() => {
    const words = afterHoming(before.current, status, model.entry.held);
    before.current = status;
    setSpoken(words ?? "");
  }, [status, model.entry.held]);
  // The live regions stay in the tree while paired, so a line that appears with the job block
  // ("Homing" after Idle) is still read, and a polite line that turns into an alert is read once.
  const line = controlsLine(model.entry, model.own);
  return (
    <>
      <JobBlock model={model} line={line} onUnlockFocus={() => (unlockHadFocus.current = true)} />
      <p role="status" className="sr-only">
        {line !== null && !line.alerts ? line.text : spoken}
      </p>
      <p role="alert" className="sr-only">
        {line?.alerts ? line.text : ""}
      </p>
      <Toolbar model={model} toolbarRef={toolbar} />
    </>
  );
}

interface Item {
  key: string;
  name: string;
  enabled: boolean;
  busy: boolean;
  cause?: string | null;
  icon: ReactNode;
  press: () => void;
}

function Toolbar({ model, toolbarRef }: { model: MachineControlsModel; toolbarRef: RefObject<HTMLDivElement | null> }) {
  const { entry } = model;
  const buttons = jobButtons(entry.status?.state ?? null);
  const idle = entry.busy === null;
  const home = homeButton(entry);
  const resume = buttons.startName === "Resume";
  const items: Item[] = [
    { key: "home", name: "Home", enabled: home.enabled, busy: home.busy, cause: home.cause, icon: <House {...GLYPH} />, press: () => model.command("home") },
    { key: "send", name: "Send job...", enabled: buttons.send && idle, busy: entry.busy === "send", icon: <FileUp {...GLYPH} />, press: model.send },
    {
      key: "start",
      name: buttons.startName,
      enabled: buttons.start && idle,
      busy: entry.busy === (resume ? "resume" : "start"),
      icon: <Play {...GLYPH} />,
      press: () => model.command(resume ? "resume" : "start"),
    },
    { key: "pause", name: "Pause", enabled: buttons.pause && idle, busy: entry.busy === "pause", icon: <Pause {...GLYPH} />, press: () => model.command("pause") },
    { key: "stop", name: "Stop", enabled: buttons.stop && idle, busy: entry.busy === "stop", icon: <Square {...GLYPH} />, press: () => model.command("stop") },
  ];
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const [rover, setRover] = useState(0);
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
    <div ref={toolbarRef} role="toolbar" aria-label="Machine" className="flex gap-1">
      {items.map((item, at) => (
        <div key={item.key} className={at === 0 ? "mr-1 flex" : "flex"}>
          <SymbolButton
            name={item.name}
            enabled={item.enabled}
            busy={item.busy}
            cause={item.cause}
            side="top"
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
        </div>
      ))}
    </div>
  );
}

function JobBlock({ model, line, onUnlockFocus }: { model: MachineControlsModel; line: ControlsLine | null; onUnlockFocus: () => void }) {
  const { entry, own } = model;
  if (line === null) return null;
  const status = entry.status;
  const name = countsName(entry, own);
  const stateName = status?.state.name;
  const elapsed = line.bar !== null && (stateName === "Running" || stateName === "Paused" || stateName === "Done");
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-start justify-between gap-1 text-xs">
        <p
          className={`flex items-start gap-1 [overflow-wrap:anywhere] ${line.alarm ? "pt-1.5" : ""} ${line.failure ? "text-[var(--destructive)]" : "text-[var(--toolbar-icon)]"}`}
        >
          {line.alarm ? <TriangleAlert {...SMALL} className="mt-0.5 shrink-0" /> : line.failure ? <CircleAlert {...SMALL} className="mt-0.5 shrink-0" /> : null}
          <span>{line.text}</span>
        </p>
        {line.alarm ? (
          <SymbolButton
            name="Unlock"
            enabled={entry.busy === null}
            busy={entry.busy === "unlock"}
            side="top"
            onFocus={onUnlockFocus}
            onPress={() => model.command("unlock")}
          >
            <LockOpen {...GLYPH} />
          </SymbolButton>
        ) : null}
        {elapsed && status?.job ? (
          <span className="shrink-0 tabular-nums text-[var(--panel-muted-fg)]">
            <span className="sr-only">Elapsed </span>
            {formatElapsed(status.job.elapsed_ms)}
          </span>
        ) : null}
      </div>
      {line.bar !== null ? (
        <div
          role="progressbar"
          aria-label="Job progress"
          aria-valuemin={0}
          aria-valuemax={line.bar.lines}
          aria-valuenow={line.bar.line}
          aria-valuetext={`${formatCount(line.bar.line)} of ${formatCount(line.bar.lines)} lines`}
          className="h-1.5 w-[244px] max-w-full overflow-hidden rounded-full border border-[var(--field-border)] bg-white"
        >
          <div
            className={`h-full ${line.bar.failure ? "bg-[var(--destructive)]" : "bg-[var(--editor-accent)]"}`}
            style={{ width: `${line.bar.lines === 0 ? 0 : Math.min(100, (line.bar.line / line.bar.lines) * 100)}%` }}
          />
        </div>
      ) : null}
      <div aria-live="off" className="flex items-center justify-between gap-1 text-xs">
        <JobName name={name ?? ""} />
        {line.bar !== null ? (
          <span className="shrink-0 tabular-nums text-[var(--toolbar-icon)]">
            {formatCount(line.bar.line)} / {formatCount(line.bar.lines)} lines
          </span>
        ) : null}
      </div>
    </div>
  );
}

/** The job's name, cut in the middle so the extension stays (`0064` UX notes §8). */
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

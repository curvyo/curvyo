// The Pen probe page (`specs/0158-pen-display` slice 0, ADR 0094): the webview's Pointer Event
// values and the native pen samples side by side, with the measurements between them, a scratch
// area drawing both traces and a button that copies the session as plain text. Nothing is stored
// or sent (criterion 4); the host reader runs only while this page is open.
import { type ReactNode, useEffect, useLayoutEffect, useRef, useState } from "react";

import { ToolbarSwitch } from "@/components/ToolbarSwitch";
import { type PenProbeApi, usePenProbe } from "@/hooks/usePenProbe";
import { closePenProbe, openPenProbe, usePenProbeOpen } from "@/lib/penProbeStore";
import { PEN_WINDOW_LABELS, pressPenWindow, setPenViewShown, usePenViewShown, usePenWindowState } from "@/lib/penWindowStore";
import { isTauri } from "@/platform/host";

const BUTTON =
  "h-7 shrink-0 rounded-[5px] border px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9 border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]";

const ms = (value: number | null): string => (value === null ? "-" : `${value.toFixed(1)} ms`);
const px = (value: number | null): string => (value === null ? "-" : `${value.toFixed(1)} px`);

function Rows({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="min-w-0 flex-1" aria-label={title}>
      <h3 className="mb-1 text-sm text-[var(--toolbar-icon)]">{title}</h3>
      <dl className="grid grid-cols-[max-content_1fr] gap-x-3 gap-y-0.5 text-sm tabular-nums">{children}</dl>
    </section>
  );
}

function Row({ name, value }: { name: string; value: string }) {
  return (
    <>
      <dt className="text-[var(--panel-muted-fg)]">{name}</dt>
      <dd className="min-w-0 break-words">{value}</dd>
    </>
  );
}

const two = (value: number | null): string => (value === null ? "none" : value.toFixed(2));

function WebColumn({ probe }: { probe: PenProbeApi }) {
  const { web } = probe.snapshot;
  return (
    <Rows title="Webview (Pointer Events)">
      <Row name="Pointer type" value={web.seen ? web.pointerType : "no event yet"} />
      <Row name="Pressure" value={web.pressureShown ? two(web.pressure) : "none"} />
      <Row name="Peak pressure" value={web.pressureShown || web.peakPressure > 0 ? `${two(web.peakPressure)} (${web.distinctPressures} values)` : "none"} />
      <Row name="Tilt X / Y" value={web.seen ? `${web.tiltX}° / ${web.tiltY}°` : "-"} />
      <Row name="Eraser end" value={web.eraser ? "yes" : "no"} />
      <Row name="Barrel button" value={web.barrel ? "pressed" : "no"} />
      <Row name="Events per second" value={`${web.eventsPerSecond.toFixed(0)} (last event stood for ${web.coalescedLast})`} />
    </Rows>
  );
}

function NativeColumn({ probe }: { probe: PenProbeApi }) {
  const { native } = probe.snapshot;
  if (probe.route === "checking") {
    return <Rows title="Native pen (app)"><Row name="Status" value="checking" /></Rows>;
  }
  if (probe.route === "unavailable") {
    return <Rows title="Native pen (app)"><Row name="Status" value="not available in this build" /></Rows>;
  }
  const tilt = native.tilt ? `${native.tilt.x.toFixed(2)} / ${native.tilt.y.toFixed(2)}` : "none";
  return (
    <Rows title="Native pen (app)">
      <Row name="Device" value={native.samples > 0 ? native.device || "unknown" : "no pen sample yet (a mouse shows nothing here)"} />
      <Row name="Pen end" value={native.end ? `${native.end} (${native.phase})` : "-"} />
      <Row name="Pressure" value={two(native.pressure)} />
      <Row name="Peak pressure" value={two(native.peakPressure)} />
      <Row name="Tilt X / Y" value={tilt} />
      <Row name="Eraser seen" value={native.eraserSeen ? "yes" : "no"} />
      <Row name="Barrel buttons" value={`${native.barrel.toString(2).padStart(2, "0")}${native.barrelSeen ? " (seen)" : ""}`} />
      <Row name="Samples per second" value={`${native.samplesPerSecond.toFixed(0)} (${native.dropped} dropped)`} />
    </Rows>
  );
}

function Measurements({ probe }: { probe: PenProbeApi }) {
  const { delay, position, frame, native } = probe.snapshot;
  return (
    <Rows title="Native against webview">
      <Row name="Delay, last 5 s" value={`median ${ms(delay.medianMs)}, largest ${ms(delay.maxMs)}`} />
      <Row name="Delay, session" value={`median ${ms(delay.sessionMedianMs)}, largest ${ms(delay.sessionMaxMs)} (${delay.pairs} pairs)`} />
      <Row name="Age in the app" value={ms(native.ageMs)} />
      <Row name="Position difference" value={`median ${px(position.medianPx)}, largest ${px(position.maxPx)}`} />
      <Row name="Frame time, last 5 s" value={`median ${ms(frame.medianMs)}, largest ${ms(frame.maxMs)}`} />
    </Rows>
  );
}

/** Copies `text`; the clipboard API where the page may use it, else a selected text area. */
async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const area = document.createElement("textarea");
    area.value = text;
    document.body.appendChild(area);
    area.select();
    const done = document.execCommand("copy");
    area.remove();
    return done;
  }
}

function Actions({ probe }: { probe: PenProbeApi }) {
  const [copied, setCopied] = useState<"idle" | "copied" | "failed">("idle");
  return (
    <div className="flex flex-wrap items-center gap-3">
      {probe.route === "native" ? (
        <ToolbarSwitch
          label="Motion compression"
          tooltip="GDK merges queued motion events to one per frame; off keeps every pen sample and gives the webview all of them"
          checked={probe.compression}
          onCheckedChange={probe.setCompression}
        />
      ) : null}
      <button type="button" className={BUTTON} onClick={probe.clear}>
        Clear
      </button>
      <button
        type="button"
        className={BUTTON}
        onClick={() => {
          void copyText(probe.report()).then((done) => setCopied(done ? "copied" : "failed"));
        }}
      >
        Copy report
      </button>
      <span role="status" className="text-xs text-[var(--panel-muted-fg)]">
        {copied === "copied" ? "Copied. Nothing was stored or sent." : copied === "failed" ? "Could not copy." : ""}
      </span>
    </div>
  );
}

/** The File tab's entry to the page (**Proposal**, `specs/0158-pen-display/plan.md`). */
export function PenProbeButton() {
  return (
    <button type="button" onClick={openPenProbe} className={`${BUTTON} mt-2 w-[118px]`}>
      Pen probe…
    </button>
  );
}

/** The File tab's Pen display section (`specs/0158-pen-display` slice 2a criterion 1, 2b criterion
 * 6, UX 9.1, 10.5): the pen window's one button, whose label is its state, the "Show pen view"
 * switch while the pen window is open, then the Pen probe. Desktop only. */
export function PenDisplaySection() {
  const state = usePenWindowState();
  const viewShown = usePenViewShown();
  return (
    <section aria-label="Pen display" className="mt-4 flex flex-col">
      {isTauri ? (
        <button
          type="button"
          onClick={pressPenWindow}
          aria-busy={state === "opening" ? true : undefined}
          className={`${BUTTON} w-[244px]`}
        >
          {PEN_WINDOW_LABELS[state]}
        </button>
      ) : null}
      {isTauri && state === "open" ? (
        <button
          type="button"
          role="switch"
          aria-checked={viewShown}
          onClick={() => setPenViewShown(!viewShown)}
          className="mt-2 flex h-6 w-[244px] items-center gap-2 rounded-[5px] text-left text-xs text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9"
        >
          <span
            aria-hidden
            className={`relative inline-block h-[14px] w-6 shrink-0 rounded-full border ${
              viewShown
                ? "border-[var(--toolbar-icon-active-bg)] bg-[var(--toolbar-icon-active-bg)]"
                : "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent"
            }`}
          >
            <span
              className={`absolute top-[2px] h-2 w-2 rounded-full transition-[left] motion-reduce:transition-none ${
                viewShown ? "left-[12px] bg-[var(--toolbar-icon-active-fg)]" : "left-[2px] bg-[var(--toolbar-icon)]"
              }`}
            />
          </span>
          Show pen view
        </button>
      ) : null}
      <PenProbeButton />
    </section>
  );
}

/** The page while it is open (a fresh session each time); Escape and Close leave it. */
export function PenProbe() {
  return usePenProbeOpen() ? <PenProbePage /> : null;
}

function PenProbePage() {
  const canvas = useRef<HTMLCanvasElement | null>(null);
  const probe = usePenProbe(canvas);
  useLayoutEffect(() => {
    if (canvas.current) {
      canvas.current.width = canvas.current.clientWidth;
      canvas.current.height = canvas.current.clientHeight;
    }
  }, []);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        closePenProbe();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Pen probe"
      className="fixed inset-0 z-50 flex flex-col gap-3 overflow-auto bg-[var(--panel-bg)] p-4 text-[var(--toolbar-icon)]"
    >
      <header className="flex items-center justify-between gap-3">
        <div>
          <h2 className="text-base">Pen probe</h2>
          <p className="text-xs text-[var(--panel-muted-fg)]">
            Draw on the area below with the pen. Blue dots are the webview, dark dots the native pen; the dot size is the pressure. Nothing is stored or sent.
          </p>
        </div>
        <button type="button" autoFocus className={BUTTON} onClick={closePenProbe}>
          Close
        </button>
      </header>
      <div className="flex flex-wrap gap-6">
        <WebColumn probe={probe} />
        <NativeColumn probe={probe} />
        <Measurements probe={probe} />
      </div>
      <Actions probe={probe} />
      <canvas
        ref={canvas}
        aria-label="Scratch area"
        className="min-h-[200px] w-full flex-1 touch-none rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] bg-white"
      />
    </div>
  );
}

// The Pen probe page (`specs/0158-pen-display` slice 0, ADR 0094): the webview's Pointer Event
// values and the native pen samples side by side, with the measurements between them, a scratch
// area drawing both traces and a button that copies the session as plain text. Nothing is stored
// or sent (criterion 4); the host reader runs only while this page is open.
import { type ReactNode, useEffect, useLayoutEffect, useRef, useState } from "react";

import { type PenProbeApi, usePenProbe } from "@/hooks/usePenProbe";
import { closePenProbe, openPenProbe, usePenProbeOpen } from "@/lib/penProbeStore";
import { Button } from "@/components/ui/button";
import { Activity, Copy, Eraser, X } from "lucide-react";
import { ToggleButton } from "@/components/ui/toggle-button";
import { SECTION_HEADING } from "@/components/ui/looks";


const ms = (value: number | null): string => (value === null ? "-" : `${value.toFixed(1)} ms`);
const px = (value: number | null): string => (value === null ? "-" : `${value.toFixed(1)} px`);

function Rows({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="min-w-0 flex-1" aria-label={title}>
      <h3 className={`mb-1 ${SECTION_HEADING}`}>{title}</h3>
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
        <ToggleButton icon={Activity} look="bar"
          label="Motion compression"
          tooltip="GDK merges queued motion events to one per frame; off keeps every pen sample and gives the webview all of them"
          pressed={probe.compression}
          onPressedChange={probe.setCompression}
        />
      ) : null}
      <Button icon={Eraser} onClick={probe.clear}>
        Clear
      </Button>
      <Button icon={Copy}
        onClick={() => {
          void copyText(probe.report()).then((done) => setCopied(done ? "copied" : "failed"));
        }}
      >
        Copy report
      </Button>
      <span role="status" className="text-xs text-[var(--panel-muted-fg)]">
        {copied === "copied" ? "Copied. Nothing was stored or sent." : copied === "failed" ? "Could not copy." : ""}
      </span>
    </div>
  );
}

/** The File tab's entry to the page (**Proposal**, `specs/0158-pen-display/plan.md`). */
export function PenProbeButton() {
  return (
    <Button icon={Activity} onClick={openPenProbe} className="mt-4 min-w-[118px]">
      Pen probe…
    </Button>
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
    // One sheet on the app's dialog look (`docs/ui-elements.md` §19, 0164 F58): an 8 px surface
    // with the panel elevation over a dimmed window, 16 px from every edge, the title 14 px
    // semibold, the actions as shared buttons.
    <div className="fixed inset-0 z-50 flex bg-black/10 p-4">
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Pen probe"
        className="m-auto flex h-full max-h-[900px] w-full max-w-[1200px] flex-col gap-3 overflow-auto rounded-[8px] bg-[var(--panel-bg)] p-4 text-[var(--toolbar-icon)] shadow-[var(--panel-elevation-shadow)]"
      >
        <header className="flex items-start justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold">Pen probe</h2>
            <p className="text-xs text-[var(--panel-muted-fg)]">
              Draw on the area below with the pen. Blue dots are the webview, dark dots the native pen; the dot size is the pressure. Nothing is stored or sent.
            </p>
          </div>
          <Button icon={X} autoFocus onClick={closePenProbe}>
            Close
          </Button>
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
          className="min-h-[200px] w-full flex-1 touch-none rounded-[5px] border border-[var(--field-border)] bg-white"
        />
      </div>
    </div>
  );
}

// Dev-only performance probe (`specs/0126-performance-pass` phase 0): counts the render loop and
// the page-frame loop per second and drives the measuring scenarios with synthetic events.
// Compiled out unless the frontend is built with `VITE_CURVYO_PERF=ws://127.0.0.1:<port>`
// (`tools/perf/measure.py` does that); a normal build has no probe, no socket and no UI.
import type { WasmSession } from "@/lib/editorSession";
import { frameView } from "@/lib/frameState";
import { ABLATIONS, type Ablation, ablationInput, applyAblation } from "@/lib/perfAblate";

const target: string | undefined = import.meta.env.VITE_CURVYO_PERF;
export const perfEnabled = typeof target === "string" && target.length > 0;

type Counters = {
  built: number; submitted: number; changed: number; sites: Record<string, number>; renderMs: number; renderMax: number;
  pagesTicks: number; pagesChanged: number; pagesMs: number; pagesMax: number;
  /** Main-thread time of named sections (input handlers, render-loop phases, chrome ticks):
   * count, summed ms and worst ms. */
  spans: Record<string, [number, number, number]>;
  /** Renders of the components that follow the view (function bodies run). */
  renders: Record<string, number>;
  /** Intervals between two animation frames, ms (the probe's own `requestAnimationFrame` loop). */
  intervals: number[];
  /** Long tasks (`PerformanceObserver` 'longtask', or the gap detector below): count and ms. */
  longTasks: number; longMs: number;
  /** Timer gaps over 50 ms (a 4 ms `setTimeout` chain that came back late): count and ms. */
  gaps: number; gapMs: number;
  /** View inputs (wheel, pan moves) and the ms from the first unapplied one to the `render()`
   * that drew it, one entry per drawn frame that had inputs waiting. */
  inputs: number; inputToDraw: number[];
  /** Page-frame commits inside the `PageFrames` animation-frame tick that read the new pages
   * (painted with the canvas's draw of that frame) and after it, in a later task (painted a frame
   * later). */
  pagesInFrame: number; pagesLater: number;
};
const zero = (): Counters => ({
  built: 0, submitted: 0, changed: 0, sites: {}, renderMs: 0, renderMax: 0, pagesTicks: 0, pagesChanged: 0, pagesMs: 0, pagesMax: 0,
  spans: {}, renders: {}, intervals: [], longTasks: 0, longMs: 0, gaps: 0, gapMs: 0, inputs: 0, inputToDraw: [], pagesInFrame: 0, pagesLater: 0,
});
let counters = zero();
let scenario = "boot";
let lastGeneration = -1;
let lastSubmitted = 0;
let socket: WebSocket | null = null;
let scenariosStarted = false;
const queue: string[] = [];
// The session whose drawn frames are timed by phase (`WasmSession::take_frame_phases`).
let timed: WasmSession | null = null;
let second = 0;

/** What the runner asked for on connecting: the full scenario list, or one ablation variant
 * (`measure.py --ablate`) with its warm-up and measured seconds. */
type Order = { mode: "full" } | { mode: "ablate"; variant: string; warmup: number; seconds: number };
let receiveOrder: (order: Order) => void = () => {};
const order = new Promise<Order>((resolve) => {
  receiveOrder = resolve;
});

function send(message: object) {
  if (!perfEnabled) return;
  const text = JSON.stringify(message);
  if (socket?.readyState === WebSocket.OPEN) socket.send(text);
  else queue.push(text);
}

function start() {
  if (!perfEnabled || socket) return;
  // An error that stops the render loop or the scenarios (a wasm binding older than the page,
  // say) names itself in the runner's table instead of leaving zero counters.
  window.addEventListener("error", (event) => send({ event: "error", scenario, message: `${event.message} ${event.filename}:${event.lineno} ${event.error instanceof Error ? event.error.stack : ""}` }));
  window.addEventListener("unhandledrejection", (event) => send({ event: "error", scenario, message: event.reason instanceof Error ? `${event.reason} ${event.reason.stack}` : String(event.reason) }));
  socket = new WebSocket(target as string);
  socket.onopen = () => {
    for (const text of queue.splice(0)) socket?.send(text);
  };
  socket.onmessage = (event) => receiveOrder(JSON.parse(String(event.data)) as Order);
  // The animation-frame cadence the page really gets, whatever the render loop does.
  let lastFrame = 0;
  const frames = (now: number) => {
    if (lastFrame > 0) counters.intervals.push(Math.round((now - lastFrame) * 10) / 10);
    lastFrame = now;
    requestAnimationFrame(frames);
  };
  requestAnimationFrame(frames);
  // Long tasks: WebKit has no 'longtask' entries (checked at start, sent as `observer`), so a
  // 4 ms timer chain that comes back more than 50 ms late counts a main-thread stall either way.
  const longTaskSupported = typeof PerformanceObserver !== "undefined" && (PerformanceObserver.supportedEntryTypes ?? []).includes("longtask");
  send({ event: "observer", longtask: longTaskSupported, entryTypes: typeof PerformanceObserver !== "undefined" ? PerformanceObserver.supportedEntryTypes ?? [] : [] });
  if (longTaskSupported) {
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        counters.longTasks += 1;
        counters.longMs += entry.duration;
      }
    }).observe({ type: "longtask", buffered: false });
  }
  let lastTimer = performance.now();
  const gapTimer = () => {
    const now = performance.now();
    const late = now - lastTimer;
    if (late > 50) {
      counters.gaps += 1;
      counters.gapMs += late;
    }
    lastTimer = now;
    setTimeout(gapTimer, 4);
  };
  setTimeout(gapTimer, 4);
  setInterval(flush, 1000);
}

/** Sends the counters since the last flush (tagged with the running scenario) and starts anew. */
function flush() {
  // Frames, ms of draw list, pictures, upload and submit, layers and vertices (summed).
  const phases = timed ? Array.from(timed.take_frame_phases()) : [];
  send({ event: "second", scenario, second: second++, ...counters, phases });
  counters = zero();
}

/** Start of a render-loop tick: the time stamp `renderTick` closes (0 when the probe is off). */
export function perfStart(): number {
  return perfEnabled ? performance.now() : 0;
}

/** End of a render-loop tick. `generation` and `submitted` are the frame generation and the
 * frames the session submitted to the GPU so far (`WasmSession::frame_state`, -1 and 0 without a
 * session): a tick whose generation differs from the previous one counts as changed, and every
 * submit the counter grew by counts, those of a resize included. `site` hands over the source
 * lines of the session's changes (`WasmSession::take_change_sites`), read only when it changed. */
export function renderTick(t0: number, generation: number, submitted: number, site: () => string) {
  if (!perfEnabled) return;
  const ms = performance.now() - t0;
  counters.built += 1;
  // A new or opened session starts its counter at 0 again.
  counters.submitted += submitted >= lastSubmitted ? submitted - lastSubmitted : submitted;
  lastSubmitted = submitted;
  if (generation !== lastGeneration) {
    lastGeneration = generation;
    counters.changed += 1;
    for (const line of site().split("\n")) {
      const space = line.lastIndexOf(" ");
      if (space > 0) {
        const at = line.slice(0, space);
        counters.sites[at] = (counters.sites[at] ?? 0) + Number(line.slice(space + 1));
      }
    }
  }
  counters.renderMs += ms;
  counters.renderMax = Math.max(counters.renderMax, ms);
}

/** Main-thread time of the section that started at `t0` (`perfStart`). */
export function perfSpan(name: string, t0: number) {
  if (!perfEnabled) return;
  const ms = performance.now() - t0;
  const span = (counters.spans[name] ??= [0, 0, 0]);
  span[0] += 1;
  span[1] += ms;
  span[2] = Math.max(span[2], ms);
}

/** A render of a component that follows the view. */
export function perfRender(name: string) {
  if (!perfEnabled) return;
  counters.renders[name] = (counters.renders[name] ?? 0) + 1;
}

// The time of the first view input the canvas has not drawn yet (0: none waiting).
let inputWaiting = 0;

/** A view input (a wheel event, a pan move) reached the session. */
export function perfInput() {
  if (!perfEnabled) return;
  counters.inputs += 1;
  if (inputWaiting === 0) inputWaiting = performance.now();
}

/** The render loop drew a frame: the inputs waiting for it are on the canvas now. */
export function perfDrawn() {
  if (!perfEnabled || inputWaiting === 0) return;
  counters.inputToDraw.push(Math.round((performance.now() - inputWaiting) * 10) / 10);
  inputWaiting = 0;
}

// Whether a `PageFrames` tick is running right now.
let inPagesTick = false;

/** A `PageFrames` tick starts or ends (`running`). */
export function perfPagesTick(running: boolean) {
  inPagesTick = running;
}

/** `PageFrames` committed new pages: inside its tick or in a later task. */
export function perfPagesCommitted() {
  if (!perfEnabled) return;
  if (inPagesTick) counters.pagesInFrame += 1;
  else counters.pagesLater += 1;
}

/** End of a PageFrames tick; `changed` when its pages JSON differed from the previous one. */
export function pagesTick(t0: number, changed: boolean) {
  if (!perfEnabled) return;
  const ms = performance.now() - t0;
  counters.pagesTicks += 1;
  if (changed) counters.pagesChanged += 1;
  counters.pagesMs += ms;
  counters.pagesMax = Math.max(counters.pagesMax, ms);
}

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

/** A page of 5 000 stroked ellipses (about 10 curves each), as one SVG: the stand-in for the
 * whale and dolphin fixtures, which are trace inputs and no loadable documents. */
function generatedSvg(nodes: number): string {
  const columns = 100;
  const paths: string[] = [];
  for (let i = 0; i < nodes; i += 1) {
    const x = 20 + (i % columns) * 10;
    const y = 20 + Math.floor(i / columns) * 10;
    paths.push(`<path d="M${x} ${y} c3 -4 6 -4 8 0 s-2 5 -8 4 s-2 -3 0 -4z" fill="none" stroke="#000" stroke-width="0.4"/>`);
  }
  const height = 40 + Math.ceil(nodes / columns) * 10;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1040" height="${height}" viewBox="0 0 1040 ${height}">${paths.join("")}</svg>`;
}

async function every(ms: number, seconds: number, step: (index: number) => void) {
  const total = Math.round((seconds * 1000) / ms);
  for (let index = 0; index < total; index += 1) {
    step(index);
    await sleep(ms);
  }
}

/** The canvas's place in device pixels of the window (left, top, width, height), which the
 * runner crops its screenshots to. */
function where(canvas: HTMLCanvasElement) {
  const box = canvas.getBoundingClientRect();
  const ratio = window.devicePixelRatio;
  return [box.left, box.top, box.width, box.height].map((value) => Math.round(value * ratio));
}

/** The empty page's pan: middle button held, left and right at a constant 8 px per 16 ms tick,
 * each tick a burst of 8 moves of 1 px, as a 500 Hz mouse sends (`buttons` 0: the same moves as a
 * hover, no press). `tick` runs after each burst. */
async function sweep(canvas: HTMLCanvasElement, seconds: number, buttons: 0 | 4, tick: (index: number, phase: number) => void = () => {}) {
  const box = canvas.getBoundingClientRect();
  const ex = box.left + box.width / 2;
  const ey = box.top + box.height / 2;
  const step = 8;
  const half = 40;
  const at = (index: number) => {
    const phase = index % (2 * half);
    const offset = phase < half ? phase : 2 * half - phase;
    return { clientX: ex - (half / 2) * step + offset * step, clientY: ey, pointerId: 1, pointerType: "mouse", isPrimary: true, bubbles: true, cancelable: true };
  };
  if (buttons) canvas.dispatchEvent(new PointerEvent("pointerdown", { ...at(0), button: 1, buttons }));
  await every(16, seconds, (index) => {
    const from = at(Math.max(index - 1, 0)).clientX;
    const to = at(index);
    for (let part = 1; part <= 8; part += 1) {
      canvas.dispatchEvent(new PointerEvent("pointermove", { ...to, clientX: from + ((to.clientX - from) * part) / 8, button: -1, buttons }));
    }
    tick(index, index % (2 * half));
  });
  if (buttons) canvas.dispatchEvent(new PointerEvent("pointerup", { ...at(0), button: 1, buttons: 0 }));
}

/** The sizes that decide the canvas's cost: its backing store and CSS box, the pixel ratio and
 * the window. */
function sizes(canvas: HTMLCanvasElement) {
  return {
    backing: [canvas.width, canvas.height],
    css: [canvas.clientWidth, canvas.clientHeight],
    dpr: window.devicePixelRatio,
    window: [window.innerWidth, window.innerHeight],
    screen: [window.screen.width, window.screen.height],
  };
}

/** One ablation run: the variant applied, then one continuous empty-page pan (or hover, or
 * nothing), its first `warmup` seconds as scenario `warmup`, the rest as `measure`. */
async function runAblation(canvas: HTMLCanvasElement, session: WasmSession, refit: () => void, variant: string, warmup: number, seconds: number) {
  const known = (ABLATIONS as readonly string[]).includes(variant);
  const chosen = (known ? variant : "baseline") as Ablation;
  const applied = known ? applyAblation(chosen, canvas, refit) : `unknown variant ${variant}, ran baseline`;
  // Two frames: the resize and the hidden parts reach the screen before the clock starts.
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const view = () => frameView(session.frame_state());
  send({ event: "ablation", variant, applied, ...sizes(canvas) });
  const input = ablationInput(chosen);
  const switchTo = (name: string) => {
    flush();
    scenario = name;
    send({ event: "scenario", name, state: "start", view: view() });
  };
  switchTo("warmup");
  const ticks = Math.round((warmup * 1000) / 16);
  const marks = (index: number) => {
    if (index === ticks) switchTo("measure");
  };
  if (input === "none") {
    await sleep(warmup * 1000);
    switchTo("measure");
    await sleep(seconds * 1000);
  } else {
    await sweep(canvas, warmup + seconds, input === "pan" ? 4 : 0, marks);
  }
  flush();
  send({ event: "scenario", name: "measure", state: "end", view: view(), ...sizes(canvas) });
  scenario = "between";
  send({ event: "done" });
}

async function run(canvas: HTMLCanvasElement, getSession: () => WasmSession | null, refit: () => void) {
  while (!getSession()) await sleep(200);
  await sleep(1500);
  const session = getSession() as WasmSession;
  session.set_frame_phase_timing(true);
  timed = session;
  const asked = await order;
  if (asked.mode === "ablate") {
    await runAblation(canvas, session, refit, asked.variant, asked.warmup, asked.seconds);
    return;
  }
  // The view (origin x, origin y, scale) at a scenario's start and end: a zoom or pan that left
  // it unchanged did not reach the session.
  const view = () => frameView(session.frame_state());
  const mark = async (name: string, body: () => Promise<void>) => {
    scenario = name;
    send({ event: "scenario", name, state: "start", view: view(), canvas: where(canvas) });
    await body();
    send({ event: "scenario", name, state: "end", view: view(), canvas: where(canvas) });
    scenario = "between";
    await sleep(1500);
  };
  await mark("idle-empty", () => sleep(15_000));
  const empty = canvas.getBoundingClientRect();
  const ex = empty.left + empty.width / 2;
  const ey = empty.top + empty.height / 2;
  // The empty page panned left and right at a constant 8 px per move, 60 moves per second, so a
  // screenshot taken mid-sweep (`snap`) shows how far the canvas's page trails the DOM page frame:
  // the offset over 8 px is the lag in moves (frames). The sweep keeps the page's left edge in view.
  // Bursts: a real mouse or touchpad sends several events per frame (here 8 moves of 1 px and 4
  // wheel events of a quarter notch per 16 ms tick), which the per-event cost has to survive.
  await mark("empty-pan", () =>
    sweep(canvas, 5, 4, (index, phase) => {
      if (phase === 20 || phase === 60) {
        send({ event: "snap", scenario: "empty-pan", index, step: phase < 40 ? 8 : -8, canvas: where(canvas) });
      }
    }),
  );
  // The still page after the pan: the offset between canvas and frame at rest.
  send({ event: "snap", scenario: "empty-rest", index: 0, step: 0, canvas: where(canvas) });
  await sleep(1000);
  // The empty page, zoomed as `wheel-zoom` zooms the 5 000-path page.
  await mark("empty-zoom", () =>
    every(16, 5, (index) => {
      const sign = Math.floor(index / 30) % 2 === 0 ? -1 : 1;
      for (let part = 0; part < 4; part += 1) {
        canvas.dispatchEvent(new WheelEvent("wheel", { deltaY: (sign * 100) / 4, clientX: ex, clientY: ey, ctrlKey: true, bubbles: true, cancelable: true }));
      }
    }),
  );
  const outcome = session.import_svg(new TextEncoder().encode(generatedSvg(5000)), "perf-5000.svg");
  send({ event: "loaded", outcome, document: "generated 5000 paths" });
  session.fit_document();
  await sleep(2000);
  const rect = canvas.getBoundingClientRect();
  const cx = rect.left + rect.width / 2;
  const cy = rect.top + rect.height / 2;
  await mark("idle-5000", () => sleep(30_000));
  // Wheel zoom: 60 events per second, 30 notches in, 30 out, around the page centre.
  await mark("wheel-zoom", () =>
    every(16, 5, (index) => {
      const sign = Math.floor(index / 30) % 2 === 0 ? -1 : 1;
      canvas.dispatchEvent(new WheelEvent("wheel", { deltaY: sign * 100, clientX: cx, clientY: cy, ctrlKey: true, bubbles: true, cancelable: true }));
    }),
  );
  // Pan drag: middle button held, the pointer on a circle of 150 px, 60 moves per second.
  await mark("pan-drag", async () => {
    const at = (angle: number) => ({ clientX: cx + 150 * Math.cos(angle), clientY: cy + 150 * Math.sin(angle), pointerId: 1, pointerType: "mouse", isPrimary: true, bubbles: true, cancelable: true });
    canvas.dispatchEvent(new PointerEvent("pointerdown", { ...at(0), button: 1, buttons: 4 }));
    await every(16, 5, (index) => canvas.dispatchEvent(new PointerEvent("pointermove", { ...at(index / 30), button: -1, buttons: 4 })));
    canvas.dispatchEvent(new PointerEvent("pointerup", { ...at(0), button: 1, buttons: 0 }));
  });
  send({ event: "done" });
}

/** Called once by the editor hook when the canvas exists; a no-op in a normal build. `refit`
 * sizes the canvas's backing store again (after an ablation changed its pixel ratio). */
export function startPerfScenarios(canvas: HTMLCanvasElement | null, getSession: () => WasmSession | null, refit: () => void) {
  if (!perfEnabled || !canvas || scenariosStarted) return;
  scenariosStarted = true;
  start();
  void run(canvas, getSession, refit);
}

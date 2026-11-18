// Dev-only performance probe (`specs/0126-performance-pass` phase 0): counts the render loop and
// the page-frame loop per second and drives the measuring scenarios with synthetic events.
// Compiled out unless the frontend is built with `VITE_CURVYO_PERF=ws://127.0.0.1:<port>`
// (`tools/perf/measure.py` does that); a normal build has no probe, no socket and no UI.
import type { WasmSession } from "@/lib/editorSession";
import { frameView } from "@/lib/frameState";

const target: string | undefined = import.meta.env.VITE_CURVYO_PERF;
export const perfEnabled = typeof target === "string" && target.length > 0;

type Counters = { built: number; submitted: number; changed: number; sites: Record<string, number>; renderMs: number; renderMax: number; pagesTicks: number; pagesChanged: number; pagesMs: number; pagesMax: number };
const zero = (): Counters => ({ built: 0, submitted: 0, changed: 0, sites: {}, renderMs: 0, renderMax: 0, pagesTicks: 0, pagesChanged: 0, pagesMs: 0, pagesMax: 0 });
let counters = zero();
let scenario = "boot";
let lastGeneration = -1;
let lastSubmitted = 0;
let socket: WebSocket | null = null;
let scenariosStarted = false;
const queue: string[] = [];

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
  let second = 0;
  setInterval(() => {
    send({ event: "second", scenario, second: second++, ...counters });
    counters = zero();
  }, 1000);
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

async function run(canvas: HTMLCanvasElement, getSession: () => WasmSession | null) {
  while (!getSession()) await sleep(200);
  await sleep(1500);
  const session = getSession() as WasmSession;
  // The view (origin x, origin y, scale) at a scenario's start and end: a zoom or pan that left
  // it unchanged did not reach the session. `canvas` is the canvas in device pixels of the
  // window, which the runner crops its screenshots to.
  const view = () => frameView(session.frame_state());
  const where = () => {
    const box = canvas.getBoundingClientRect();
    const ratio = window.devicePixelRatio;
    return [box.left, box.top, box.width, box.height].map((value) => Math.round(value * ratio));
  };
  const mark = async (name: string, body: () => Promise<void>) => {
    scenario = name;
    send({ event: "scenario", name, state: "start", view: view(), canvas: where() });
    await body();
    send({ event: "scenario", name, state: "end", view: view(), canvas: where() });
    scenario = "between";
    await sleep(1500);
  };
  await mark("idle-empty", () => sleep(15_000));
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

/** Called once by the editor hook when the canvas exists; a no-op in a normal build. */
export function startPerfScenarios(canvas: HTMLCanvasElement | null, getSession: () => WasmSession | null) {
  if (!perfEnabled || !canvas || scenariosStarted) return;
  scenariosStarted = true;
  start();
  void run(canvas, getSession);
}

// Dev-only performance probe (`specs/0126-performance-pass` phase 0): counts the render loop and
// the page-frame loop per second and drives the measuring scenarios with synthetic events.
// Compiled out unless the frontend is built with `VITE_CURVYO_PERF=ws://127.0.0.1:<port>`
// (`tools/perf/measure.py` does that); a normal build has no probe, no socket and no UI.
import type { WasmSession } from "@/lib/editorSession";

const target: string | undefined = import.meta.env.VITE_CURVYO_PERF;
export const perfEnabled = typeof target === "string" && target.length > 0;

type Counters = { built: number; submitted: number; changed: number; renderMs: number; renderMax: number; pagesTicks: number; pagesChanged: number; pagesMs: number; pagesMax: number };
const zero = (): Counters => ({ built: 0, submitted: 0, changed: 0, renderMs: 0, renderMax: 0, pagesTicks: 0, pagesChanged: 0, pagesMs: 0, pagesMax: 0 });
let counters = zero();
let scenario = "boot";
let lastView = "";
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

/** End of a render-loop tick. `view` is the view string the loop already read ("" without a
 * session): a frame whose view differs from the previous tick's counts as changed. */
export function renderTick(t0: number, rendered: boolean, view: string) {
  if (!perfEnabled) return;
  const ms = performance.now() - t0;
  counters.built += 1;
  if (rendered) counters.submitted += 1;
  if (view !== lastView) {
    lastView = view;
    counters.changed += 1;
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
  const mark = async (name: string, body: () => Promise<void>) => {
    scenario = name;
    send({ event: "scenario", name, state: "start" });
    await body();
    send({ event: "scenario", name, state: "end" });
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

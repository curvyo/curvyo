// Dev-only GL micro benchmarks of the perf build (`tools/perf/measure.py --gl-bench`): which
// part of a frame's present costs what in this webview, ours (the wgpu canvas) or the platform's
// (any WebGL, 2D or composited canvas at the same size). Inactive unless the frontend is built
// with `VITE_CURVYO_PERF` and the runner asked for the bench.
import type { WasmSession } from "@/lib/editorSession";
import { setPerfBackingScale } from "./perfAblate.ts";

/** The probe's message sink, its pan driver and the refit of the editor canvas. */
export type BenchHost = {
  send: (message: object) => void;
  /** A middle-button pan of the empty page for `seconds` of 16 ms ticks or until `stop`. */
  pan: (seconds: number, stop: () => boolean) => Promise<void>;
  refit: () => void;
};

/** One size of a bench: the backing store in device pixels and the CSS box it is shown in. */
export type BenchSize = { label: string; backing: [number, number]; css: [number, number] };

/** The sizes every bench runs at, from the editor canvas's backing store and CSS box: 1:1 at the
 * full, half and quarter size per side (the CSS box shrinks with the backing store), and the half
 * and quarter backing store stretched over the full box (as the `half-backing` ablation did), so
 * the scaling path shows apart from the pixel count. */
export function benchSizes(backing: [number, number], css: [number, number], stretched: boolean): BenchSize[] {
  const scaled = (f: number): [number, number] => [Math.max(1, Math.round(backing[0] * f)), Math.max(1, Math.round(backing[1] * f))];
  const box = (f: number): [number, number] => [css[0] * f, css[1] * f];
  const sizes: BenchSize[] = [
    { label: "1", backing: scaled(1), css: box(1) },
    { label: "1/2", backing: scaled(0.5), css: box(0.5) },
    { label: "1/4", backing: scaled(0.25), css: box(0.25) },
  ];
  if (stretched) {
    sizes.push({ label: "1/2 str", backing: scaled(0.5), css: box(1) }, { label: "1/4 str", backing: scaled(0.25), css: box(1) });
  }
  return sizes;
}

const SETTLE_MS = 500;
const BENCH_MS = 4000;
const nextFrame = () => new Promise<number>((resolve) => requestAnimationFrame(resolve));
const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

/** Runs `draw` once per animation frame: `SETTLE_MS` unrecorded (context creation, shader
 * compile, the first present), then `BENCH_MS` whose frame intervals are returned. */
async function frames(draw: (index: number) => void, onMeasureStart: () => void): Promise<number[]> {
  const intervals: number[] = [];
  let index = 0;
  const settleEnd = performance.now() + SETTLE_MS;
  while (performance.now() < settleEnd) {
    await nextFrame();
    draw(index++);
  }
  onMeasureStart();
  let last = await nextFrame();
  draw(index++);
  const end = last + BENCH_MS;
  while (last < end) {
    const now = await nextFrame();
    draw(index++);
    intervals.push(Math.round((now - last) * 10) / 10);
    last = now;
  }
  return intervals;
}

/** A scratch canvas over the editor canvas's top left corner, above everything. */
function scratchCanvas(at: DOMRect, size: BenchSize, corner: "left" | "right" = "left"): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.width = size.backing[0];
  canvas.height = size.backing[1];
  canvas.dataset.perfBench = "";
  const left = corner === "left" ? at.left : at.right - size.css[0];
  canvas.style.cssText = `position:fixed;left:${left}px;top:${at.top}px;width:${size.css[0]}px;height:${size.css[1]}px;z-index:2147483647;pointer-events:none`;
  document.body.appendChild(canvas);
  return canvas;
}

/** Frees a WebGL context at once (WebKit caps live contexts and drops the oldest, which would be
 * the editor's) and removes its canvas. */
function discard(canvas: HTMLCanvasElement, gl: WebGL2RenderingContext | null) {
  gl?.getExtension("WEBGL_lose_context")?.loseContext();
  canvas.remove();
}

/** The strings and limits of a WebGL2 context: what the webview really renders with. */
export function glStrings(gl: WebGL2RenderingContext) {
  const debug = gl.getExtension("WEBGL_debug_renderer_info");
  return {
    vendor: gl.getParameter(gl.VENDOR),
    renderer: gl.getParameter(gl.RENDERER),
    version: gl.getParameter(gl.VERSION),
    glsl: gl.getParameter(gl.SHADING_LANGUAGE_VERSION),
    unmaskedVendor: debug ? gl.getParameter(debug.UNMASKED_VENDOR_WEBGL) : "(no WEBGL_debug_renderer_info)",
    unmaskedRenderer: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : "(no WEBGL_debug_renderer_info)",
    maxSamples: gl.getParameter(gl.MAX_SAMPLES),
    attributes: gl.getContextAttributes(),
  };
}

/** The platform the page runs on: the user agent, a scratch WebGL2 context's strings, the
 * editor canvas's own context (the one wgpu created: read-only calls only) and wgpu's adapter. */
export function platform(editor: HTMLCanvasElement, session: WasmSession) {
  const scratch = document.createElement("canvas");
  const gl = scratch.getContext("webgl2");
  const scratchStrings = gl ? glStrings(gl) : null;
  discard(scratch, gl);
  // `getContext` on a canvas that already has a webgl2 context returns that same context.
  const own = editor.getContext("webgl2");
  return {
    userAgent: navigator.userAgent,
    scratch: scratchStrings,
    editor: own ? { ...glStrings(own), lost: own.isContextLost() } : null,
    wgpu: session.gpu_info(),
  };
}

const VERTEX = `#version 300 es
void main() {
  vec2 p[3] = vec2[3](vec2(-0.05, -0.05), vec2(0.05, -0.05), vec2(0.0, 0.05));
  gl_Position = vec4(p[gl_VertexID], 0.0, 1.0);
}`;
const FRAGMENT = `#version 300 es
precision mediump float;
uniform vec4 colour;
out vec4 result;
void main() { result = colour; }`;

/** A program drawing one tiny triangle in the uniform `colour` (no buffers: `gl_VertexID`). */
function tinyProgram(gl: WebGL2RenderingContext) {
  const program = gl.createProgram() as WebGLProgram;
  for (const [type, source] of [[gl.VERTEX_SHADER, VERTEX], [gl.FRAGMENT_SHADER, FRAGMENT]] as const) {
    const shader = gl.createShader(type) as WebGLShader;
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    gl.attachShader(program, shader);
  }
  gl.linkProgram(program);
  return { program, colour: gl.getUniformLocation(program, "colour") };
}

/** The wgpu GL backend's frame on WebGL (wgpu-hal `gles/web.rs`): draw into a multisampled colour
 * and depth target, resolve it into a single-sampled texture, blit that Y-flipped into the canvas. */
function wgpuShapedTargets(gl: WebGL2RenderingContext, width: number, height: number, samples: number) {
  const msaa = gl.createFramebuffer();
  gl.bindFramebuffer(gl.FRAMEBUFFER, msaa);
  const colour = gl.createRenderbuffer();
  gl.bindRenderbuffer(gl.RENDERBUFFER, colour);
  gl.renderbufferStorageMultisample(gl.RENDERBUFFER, samples, gl.RGBA8, width, height);
  gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.RENDERBUFFER, colour);
  const depth = gl.createRenderbuffer();
  gl.bindRenderbuffer(gl.RENDERBUFFER, depth);
  gl.renderbufferStorageMultisample(gl.RENDERBUFFER, samples, gl.DEPTH24_STENCIL8, width, height);
  gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.DEPTH_STENCIL_ATTACHMENT, gl.RENDERBUFFER, depth);
  const resolved = gl.createFramebuffer();
  gl.bindFramebuffer(gl.FRAMEBUFFER, resolved);
  const texture = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texStorage2D(gl.TEXTURE_2D, 1, gl.RGBA8, width, height);
  gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, texture, 0);
  gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  return { msaa, resolved };
}

/** The raw GL variants: context attributes and whether the frame takes wgpu's path. */
const GL_VARIANTS: { name: string; attributes: WebGLContextAttributes; wgpuPath: boolean; stretched: boolean }[] = [
  { name: "gl-opaque", attributes: { alpha: false, antialias: false, depth: false, preserveDrawingBuffer: false }, wgpuPath: false, stretched: true },
  { name: "gl-alpha-pma", attributes: { alpha: true, premultipliedAlpha: true, antialias: false, depth: false, preserveDrawingBuffer: false }, wgpuPath: false, stretched: false },
  { name: "gl-alpha-nopma", attributes: { alpha: true, premultipliedAlpha: false, antialias: false, depth: false, preserveDrawingBuffer: false }, wgpuPath: false, stretched: false },
  // The attributes wgpu creates its context with (only `antialias: false`; alpha, premultiplied
  // alpha and depth stay at their defaults, true).
  { name: "gl-wgpu-ctx", attributes: { antialias: false }, wgpuPath: false, stretched: false },
  { name: "gl-wgpu-path", attributes: { antialias: false }, wgpuPath: true, stretched: true },
];

/** One raw WebGL2 bench: a clear and one tiny draw per frame (the wgpu path: into the MSAA target,
 * resolved and blitted). Returns the frame intervals. */
async function glBench(at: DOMRect, size: BenchSize, attributes: WebGLContextAttributes, wgpuPath: boolean, samples: number, onStart: () => void) {
  const canvas = scratchCanvas(at, size);
  const gl = canvas.getContext("webgl2", attributes);
  if (!gl) {
    discard(canvas, null);
    return { intervals: [], note: "no webgl2 context" };
  }
  const [width, height] = size.backing;
  const { program, colour } = tinyProgram(gl);
  const count = Math.min(samples, gl.getParameter(gl.MAX_SAMPLES) as number);
  const targets = wgpuPath && count > 1 ? wgpuShapedTargets(gl, width, height, count) : null;
  gl.viewport(0, 0, width, height);
  const intervals = await frames((index) => {
    const shade = (index % 60) / 60;
    if (targets) gl.bindFramebuffer(gl.FRAMEBUFFER, targets.msaa);
    gl.clearColor(0.3, 0.3, 0.32, 1);
    gl.clearDepth(1);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    gl.useProgram(program);
    gl.uniform4f(colour, shade, 1 - shade, 0.5, 1);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    if (targets) {
      gl.bindFramebuffer(gl.READ_FRAMEBUFFER, targets.msaa);
      gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, targets.resolved);
      gl.blitFramebuffer(0, 0, width, height, 0, 0, width, height, gl.COLOR_BUFFER_BIT, gl.NEAREST);
      gl.bindFramebuffer(gl.READ_FRAMEBUFFER, targets.resolved);
      gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, null);
      gl.blitFramebuffer(0, height, width, 0, 0, 0, width, height, gl.COLOR_BUFFER_BIT, gl.NEAREST);
    }
  }, onStart);
  const note = `attrs ${JSON.stringify(gl.getContextAttributes())}${wgpuPath ? `, ${count}x MSAA + depth, resolve, flipped blit` : ""}`;
  discard(canvas, gl);
  return { intervals, note };
}

/** A 2D canvas filled whole once per frame, in a colour that changes every frame. */
async function fillBench(at: DOMRect, size: BenchSize, onStart: () => void) {
  const canvas = scratchCanvas(at, size);
  const context = canvas.getContext("2d") as CanvasRenderingContext2D;
  const intervals = await frames((index) => {
    context.fillStyle = `hsl(${(index * 6) % 360} 40% 50%)`;
    context.fillRect(0, 0, canvas.width, canvas.height);
  }, onStart);
  canvas.remove();
  return { intervals, note: "fillRect of the whole canvas" };
}

/** A 2D canvas painted once; per frame only its CSS transform changes (the compositor's cost). */
async function transformBench(at: DOMRect, size: BenchSize, onStart: () => void) {
  const canvas = scratchCanvas(at, size);
  const context = canvas.getContext("2d") as CanvasRenderingContext2D;
  context.fillStyle = "#777";
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.fillStyle = "#eee";
  context.fillRect(canvas.width / 4, canvas.height / 4, canvas.width / 2, canvas.height / 2);
  const intervals = await frames((index) => {
    canvas.style.transform = `translateX(${(index % 40) - 20}px)`;
  }, onStart);
  canvas.remove();
  return { intervals, note: "static canvas, translateX per frame" };
}

/** The frames the session submitted so far (`frame_state()[6]`). */
const submitted = (session: WasmSession) => session.frame_state()[6];

/** Pins the editor canvas's CSS box to `css` px (its parent sizes it by its content in places,
 * so a shrunk canvas would not grow back on its own) and refits its backing store. */
async function pinEditor(editor: HTMLCanvasElement, host: BenchHost, css: [number, number]) {
  editor.style.setProperty("width", `${css[0]}px`, "important");
  editor.style.setProperty("height", `${css[1]}px`, "important");
  host.refit();
  await nextFrame();
  await nextFrame();
  host.refit();
}

/** Our wgpu canvas at `size` (1:1: its pinned CSS box shrunk with it; stretched: the backing-store
 * factor of `perfAblate.ts` under the full box) while the empty page pans; with `second`, a plain
 * WebGL canvas of a quarter size in the top right corner draws in each frame too. */
async function editorBench(editor: HTMLCanvasElement, session: WasmSession, host: BenchHost, size: BenchSize, full: BenchSize, second: boolean, onStart: () => void) {
  const stretched = size.label.endsWith("str");
  setPerfBackingScale(stretched ? size.backing[0] / full.backing[0] : 1);
  await pinEditor(editor, host, size.css);
  const backing: [number, number] = [editor.width, editor.height];
  let extra: { canvas: HTMLCanvasElement; gl: WebGL2RenderingContext | null } | null = null;
  if (second) {
    const box = editor.getBoundingClientRect();
    const small: BenchSize = { label: "", backing: [Math.round(full.backing[0] / 4), Math.round(full.backing[1] / 4)], css: [full.css[0] / 4, full.css[1] / 4] };
    const canvas = scratchCanvas(box, small, "right");
    extra = { canvas, gl: canvas.getContext("webgl2", { alpha: false, antialias: false, depth: false }) };
  }
  let before = 0;
  let start = 0;
  // The pan's timers run about once per frame when frames are slow: it stops with the bench.
  let finished = false;
  const panning = host.pan(SETTLE_MS / 1000 + BENCH_MS / 1000 + 0.5, () => finished);
  const intervals = await frames((index) => {
    const gl = extra?.gl;
    if (gl) {
      const shade = (index % 60) / 60;
      gl.clearColor(shade, 0.4, 1 - shade, 1);
      gl.clear(gl.COLOR_BUFFER_BIT);
    }
  }, () => {
    before = submitted(session);
    start = performance.now();
    onStart();
  });
  const seconds = (performance.now() - start) / 1000;
  const submits = (submitted(session) - before) / Math.max(seconds, 0.001);
  finished = true;
  await panning;
  if (extra) discard(extra.canvas, extra.gl);
  setPerfBackingScale(1);
  await pinEditor(editor, host, full.css);
  const lost = editor.getContext("webgl2")?.isContextLost() ?? false;
  return { intervals, backing, submits, lost, note: `${second ? "+ a plain WebGL canvas (1/4) drawn each frame, " : ""}${submits.toFixed(1)} submits/s${lost ? ", EDITOR CONTEXT LOST" : ""}` };
}

/** The sample count wgpu chose (`sample count: N` of `gpu_info`), 4 when it is not there. */
function wgpuSamples(info: string): number {
  const match = /sample count: (\d+)/.exec(info);
  return match ? Number(match[1]) : 4;
}

/** The whole bench: the platform strings, then our canvas (first, while no scratch context has
 * been created and dropped), then the raw WebGL, 2D and compositor baselines, each at every size. */
export async function runGlBench(editor: HTMLCanvasElement, session: WasmSession, host: BenchHost, info: ReturnType<typeof platform>) {
  const backing: [number, number] = [editor.width, editor.height];
  const css: [number, number] = [editor.clientWidth, editor.clientHeight];
  const at = editor.getBoundingClientRect();
  const samples = wgpuSamples(info.wgpu);
  const run = async (name: string, size: BenchSize, body: (onStart: () => void) => Promise<{ intervals: number[]; note: string; backing?: [number, number] }>) => {
    await sleep(300);
    const result = await body(() => host.send({ event: "bench", state: "start", name, size: size.label }));
    host.send({ event: "bench", state: "end", name, size: size.label, backing: result.backing ?? size.backing, css: size.css, intervals: result.intervals, note: result.note });
  };
  const full = benchSizes(backing, css, false)[0];
  for (const size of benchSizes(backing, css, true)) {
    await run("wgpu-pan", size, (onStart) => editorBench(editor, session, host, size, full, false, onStart));
  }
  await run("wgpu-pan+gl", full, (onStart) => editorBench(editor, session, host, full, full, true, onStart));
  for (const variant of GL_VARIANTS) {
    for (const size of benchSizes(backing, css, variant.stretched)) {
      await run(variant.name, size, (onStart) => glBench(at, size, variant.attributes, variant.wgpuPath, samples, onStart));
    }
  }
  for (const size of benchSizes(backing, css, true)) {
    await run("2d-fill", size, (onStart) => fillBench(at, size, onStart));
  }
  for (const size of benchSizes(backing, css, false)) {
    await run("css-transform", size, (onStart) => transformBench(at, size, onStart));
  }
  host.send({ event: "platform-end", editorLost: editor.getContext("webgl2")?.isContextLost() ?? null });
}

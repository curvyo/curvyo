// Independent tester cases for `specs/0032-pen-tablet-input/specification.md`, page side, without
// a window: the sample batch of one pointer event (criteria 1, 2, 16), the readout text
// (criterion 15), the Pencil bar rendered on the server (criteria 14, 15) and the Pencil button in
// the tool rail (criterion 6). Real pointer delivery in a webview is NOT covered here.

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

import {
  pencilSampleValues,
  penReadoutText,
  type PencilPointerEvent,
  SAMPLE_VALUES,
} from "../src/lib/penInput.ts";

type Server = {
  ssrLoadModule(url: string): Promise<Record<string, unknown>>;
  close(): Promise<void>;
};
let server: Server;

before(async () => {
  const { createServer } = await import("vite");
  server = (await createServer({
    configFile: new URL("../vite.config.ts", import.meta.url).pathname,
    server: { middlewareMode: true },
    appType: "custom",
    logLevel: "silent",
  })) as unknown as Server;
});

after(async () => {
  await server.close();
});

const ORIGIN = { left: 20, top: 30 };

/** A pen event as the DOM delivers it, with fields the Pencil must not read. */
function domPen(x: number, y: number, pressure: number, extra: Record<string, unknown> = {}) {
  return {
    clientX: x + ORIGIN.left,
    clientY: y + ORIGIN.top,
    pressure,
    tiltX: 12,
    tiltY: -3,
    timeStamp: 100 + x,
    pointerId: 77,
    pointerType: "pen",
    buttons: 1,
    width: 1,
    height: 1,
    ...extra,
  } as PencilPointerEvent;
}

// ---------------------------------------------------------------- criteria 1, 2, 16

test("c2 a move with 5 coalesced samples gives 5 samples", () => {
  const coalesced = [1, 2, 3, 4, 5].map((i) => domPen(i, i * 2, i / 10));
  const event = { ...domPen(5, 10, 0.5), getCoalescedEvents: () => coalesced };
  const values = pencilSampleValues(event, ORIGIN, true);
  assert.equal(values.length, 5 * SAMPLE_VALUES);
  for (let i = 0; i < 5; i++) {
    const s = Array.from(values.slice(i * SAMPLE_VALUES, (i + 1) * SAMPLE_VALUES));
    assert.deepEqual(s, [i + 1, (i + 1) * 2, (i + 1) / 10, 12, -3, 101 + i]);
  }
});

test("c2 a move without getCoalescedEvents gives 1 sample", () => {
  const values = pencilSampleValues(domPen(3, 4, 0.25), ORIGIN, true);
  assert.equal(values.length, SAMPLE_VALUES);
  assert.deepEqual(Array.from(values), [3, 4, 0.25, 12, -3, 103]);
});

test("c1 pressure and tilt are passed exactly as delivered", () => {
  for (const [p, tx, ty] of [
    [0, 0, 0],
    [0.5, -90, 90],
    [1, 45.5, -12.25],
    [0.123456789, 1e-9, -1e-9],
  ]) {
    const v = pencilSampleValues(
      { ...domPen(0, 0, p), tiltX: tx, tiltY: ty } as PencilPointerEvent,
      ORIGIN,
      false,
    );
    assert.equal(v[2], p);
    assert.equal(v[3], tx);
    assert.equal(v[4], ty);
  }
});

test("c16 no pointer id or device field reaches the sample batch", () => {
  const values = Array.from(pencilSampleValues(domPen(1, 1, 0.4), ORIGIN, false));
  assert.equal(values.length, SAMPLE_VALUES);
  assert.ok(!values.includes(77), "the pointer id is not passed");
});

test("c2 a long coalesced batch (2000 samples) is kept whole", () => {
  const coalesced = Array.from({ length: 2000 }, (_, i) => domPen(i * 0.1, 0, 0.3));
  const event = { ...domPen(199.9, 0, 0.3), getCoalescedEvents: () => coalesced };
  assert.equal(pencilSampleValues(event, ORIGIN, true).length, 2000 * SAMPLE_VALUES);
});

// ---------------------------------------------------------------- criterion 15

test("c15 readout text", () => {
  assert.equal(penReadoutText(0.62, 12, -3), "Pen: pressure 0.62, tilt 12° / −3°");
  assert.equal(penReadoutText(0, 0, 0), "Pen: pressure 0.00, tilt 0° / 0°");
  assert.equal(penReadoutText(0.5, -90, 90), "Pen: pressure 0.50, tilt −90° / 90°");
  // Rounding -0.4 must not print "−0°".
  assert.equal(penReadoutText(0.5, -0.4, 0.4), "Pen: pressure 0.50, tilt 0° / 0°");
});

// ---------------------------------------------------------------- criterion 14 and 15: the bar

async function renderBar(bar: Record<string, unknown>): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const mod = await server.ssrLoadModule("/src/components/PencilToolbar.tsx");
  const noop = () => "unchanged";
  const element = react.createElement(mod.PencilToolbar as never, {
    bar: {
      penSeen: false,
      pressureSeen: false,
      readout: null,
      minimumMm: 0.2,
      maximumMm: 2,
      curve: "linear",
      ...bar,
    },
    onSetMinimum: noop,
    onSetMaximum: noop,
    onSetCurve: noop,
    onReturnFocus: noop,
  });
  return dom.renderToString(element as never);
}

test("c14 a mouse-only session shows no bar at all", async () => {
  assert.equal(await renderBar({}), "");
});

test("c15 a pen in range shows the readout but no pressure controls", async () => {
  const html = await renderBar({ penSeen: true, readout: "Pen: pressure 0.62, tilt 12° / −3°" });
  assert.ok(html.includes("Pen: pressure 0.62, tilt 12° / −3°"), html);
  assert.ok(!html.includes("Minimum width"), html);
  assert.ok(!html.includes("radiogroup"), html);
});

test("c14 after a pressure stroke: Minimum, Maximum (mm) and Curve with Linear checked", async () => {
  const html = await renderBar({ penSeen: true, pressureSeen: true, readout: null });
  assert.ok(html.includes("Minimum width"), html);
  assert.ok(html.includes("Maximum width"), html);
  assert.ok(html.includes("mm"), html);
  assert.ok(html.includes('role="radiogroup"'), html);
  for (const label of ["Soft", "Linear", "Firm"]) {
    assert.ok(html.includes(`>${label}<`), label);
  }
  const radios = [...html.matchAll(/<button[^>]*role="radio"[^>]*>([^<]*)</g)];
  assert.equal(radios.length, 3);
  const checked = radios.filter((m) => m[0].includes('aria-checked="true"')).map((m) => m[1]);
  assert.deepEqual(checked, ["Linear"]);
  // Min, Max and Curve come in that order (Tab order).
  assert.ok(html.indexOf("Minimum width") < html.indexOf("Maximum width"));
  assert.ok(html.indexOf("Maximum width") < html.indexOf("radiogroup"));
  // The values are shown: 0.2 and 2.
  assert.ok(/value="0\.2"/.test(html) || html.includes(">0.2<"), html);
});

test("c15 the readout is not a live region", async () => {
  const html = await renderBar({ penSeen: true, readout: "Pen: pressure 0.10, tilt 0° / 0°" });
  assert.ok(!html.includes("aria-live"), html);
});

// ---------------------------------------------------------------- criterion 6: the rail

async function renderRail(tool: string, selectionCount: number): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const mod = await server.ssrLoadModule("/src/components/ToolRail.tsx");
  const noop = () => undefined;
  const off = new Proxy(
    {},
    {
      get: (_t, key) => (key === "then" ? undefined : typeof key === "string" && /^(on|apply|run)/.test(key) ? noop : false),
    },
  );
  const element = react.createElement(mod.ToolRail as never, {
    tool,
    selectionCount,
    onSelect: noop,
    onReturnFocus: noop,
    commands: { booleans: { availability: off, apply: noop }, path: { availability: off, apply: noop }, notice: null },
    grouping: { availability: off, apply: noop },
  });
  return dom.renderToString(element as never);
}

test("c6 the Pencil is the seventh and last button of the Tools card", async () => {
  let html: string;
  try {
    html = await renderRail("pencil", 0);
  } catch (error) {
    assert.fail(`the rail did not render with stub commands: ${String(error)}`);
  }
  const names = [...html.matchAll(/aria-label="([^"(]+ tool) \([^"]*\)"/g)].map((m) => m[1]);
  assert.deepEqual(names.slice(0, 7), [
    "Select tool",
    "Pen tool",
    "Node tool",
    "Rectangle tool",
    "Ellipse tool",
    "Polygon/star tool",
    "Pencil tool",
  ]);
  const pencil = /<button[^>]*aria-label="Pencil tool \(P\)"[^>]*>/.exec(html)?.[0] ?? "";
  assert.ok(pencil.includes('aria-pressed="true"'), `active Pencil button: ${pencil}`);
  // With the Select tool and a selection the letter reads "Esc, P", as the other tools.
  const busy = await renderRail("select", 2);
  assert.ok(busy.includes('aria-label="Pencil tool (Esc, P)"'), "Esc, P while a selection exists");
  assert.ok(/<button[^>]*aria-label="Pencil tool \(Esc, P\)"[^>]*aria-pressed="false"/.test(busy));
});

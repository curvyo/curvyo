// Tester acceptance for specs/0117-centre-lines criterion 4: every refusal kind the session
// reports has its message (ux-notes.md), and the notice counts converted, kept and skipped.
import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { centreLinesDimmedTooltip, centreLinesNotice, centreLinesRefusalText } from "../src/lib/centreLinesText.ts";

const base = { kind: "applied", converted: 1, kept: 0, skipped: 0, unlinked: 0, nodes: 0 };

describe("centre lines refusals (tester, 0117 criterion 4)", () => {
  it("names each reason", () => {
    assert.equal(centreLinesRefusalText("text", 0), "Convert text to path first.");
    assert.equal(centreLinesRefusalText("image", 0), "Trace the image first.");
    assert.equal(centreLinesRefusalText("not_filled", 0), "Select filled shapes.");
    assert.equal(centreLinesRefusalText("too_large", 100001), "Too large: 100,001 nodes. The limit is 100,000.");
  });
  it("is silent when nothing is refused", () => {
    assert.equal(centreLinesRefusalText("none", 0), null);
    assert.equal(centreLinesRefusalText("applied", 0), null);
  });
  it("shows a refusal as an alert notice", () => {
    for (const kind of ["image", "not_filled"]) {
      const notice = centreLinesNotice({ ...base, kind });
      assert.equal(notice?.alert, true, kind);
      assert.equal(notice?.text, centreLinesRefusalText(kind, 0));
    }
  });
  it("counts a mixed selection", () => {
  // 0159 S7 (customer 2026-11-27): result notices carry no counts of what came out.
    assert.equal(centreLinesNotice({ ...base, kept: 1, skipped: 2 })?.text, "Centre lines: some shapes were skipped.");
  });
  it("writes the dimmed tooltip for text and image", () => {
    assert.equal(centreLinesDimmedTooltip("text", 0), "Find centre lines: convert text to path first.");
    assert.equal(centreLinesDimmedTooltip("image", 0), "Find centre lines: trace the image first.");
  });
});

// Options that are not numbers take their defaults at the binding (overlap 0.2, Thick parts 6 mm,
// optimise 0.75, keep off, offset 0.3 mm), so the result equals an Apply with those defaults.
// The binding takes five arguments: overlap, Thick parts in mm (it replaced the blob ratio,
// customer 2026-11-18), optimise, keep inner areas, offset.
describe("centre lines options that are not numbers (tester, 0117 criterion 15a)", () => {
  const ring =
    '<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="40mm" viewBox="0 0 40 40">' +
    '<path fill="#c8141e" fill-rule="evenodd" d="M31 20A11 11 0 1 0 9 20A11 11 0 1 0 31 20Z' +
    'M29 20A9 9 0 1 1 11 20A9 9 0 1 1 29 20Z"/>' +
    '<path fill="#c8141e" d="M2 36H38V38H2Z"/></svg>';
  type Args = [number, number, number, boolean, number];
  const run = async (args: Args) => {
    const { readFileSync } = await import("node:fs");
    const wasm = await import("../src/wasm-bindings/curvyo_editor_wasm.js");
    const bytes = readFileSync(new URL("../src/wasm-bindings/curvyo_editor_wasm_bg.wasm", import.meta.url));
    await wasm.default({ module_or_path: bytes });
    const session = new wasm.WasmSession(1n);
    session.import_begin("picker", false, 0, 0);
    session.import_add(new TextEncoder().encode(ring), "ring.svg");
    session.import_finish(true);
    const view = session.apply_centre_lines(...args, true);
    const out = { kind: view.kind, converted: view.converted, svg: session.export_svg(false, false, false, false) };
    session.free();
    return out;
  };
  it("maps NaN and infinities to the defaults, each option on its own", async () => {
    // Keep inner areas is on so that the offset is read (the ring has one hole).
    const defaults: Args = [0.2, 6, 0.75, true, 0.3];
    const want = await run(defaults);
    assert.equal(want.kind, "applied");
    assert.equal(want.converted, 2);
    // The comparison sees each option: another value gives another document.
    assert.notDeepEqual(await run([0.2, 6, 0.75, true, 1.0]), want, "offset is read");
    assert.notDeepEqual(await run([0.2, 6, 0.0, true, 0.3]), want, "optimise is read");
    for (const x of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY]) {
      for (const i of [0, 1, 2, 4]) {
        const args = [...defaults] as Args;
        args[i] = x;
        assert.deepEqual(await run(args), want, `option ${i} = ${x}`);
      }
      assert.deepEqual(await run([x, x, x, true, x]), want, `all options ${x}`);
    }
  });
});

// Tester 0117 criterion 15a: the bar's Optimise default is 75 % (UX notes table, spec).
describe("centre lines bar defaults (tester, 0117 criterion 15)", () => {
  it("opens Optimise at 75 %", async () => {
    const { CENTRE_LINES_DEFAULTS } = await import("../src/lib/centreLinesText.ts");
    assert.equal(CENTRE_LINES_DEFAULTS.optimise, 75);
    assert.equal(CENTRE_LINES_DEFAULTS.overlap, 20);
    assert.equal(CENTRE_LINES_DEFAULTS.thickParts, 6);
    assert.equal(CENTRE_LINES_DEFAULTS.keepInner, false);
    assert.equal(CENTRE_LINES_DEFAULTS.offset, 0.3);
  });
  it("reads an empty, blank or non-numeric field as no value (never as 0)", async () => {
    const { parseOption, fieldFeedback } = await import("../src/lib/centreLinesText.ts");
    for (const text of ["", "   ", "abc", "NaN", "Infinity"]) {
      assert.equal(parseOption(text, 0, 100), null, `"${text}"`);
      assert.equal(fieldFeedback(text, { label: "Optimise", min: 0, max: 100, suffix: "%" }).value, null, `"${text}"`);
    }
  });
});

// Tester 0117 criterion 15: the options bar is the only options UI (the app has no popups).
describe("centre lines options UI has no popup (tester, 0117 criterion 15)", () => {
  const sources = ["../src/components/CentreLinesButton.tsx", "../src/hooks/useCentreLines.ts", "../src/lib/centreLinesText.ts"];
  const forbidden = [
    /role=["'](dialog|alertdialog)["']/,
    /<dialog\b/,
    /showModal|showPopover/,
    /\bpopover\s*=\s*["']?(auto|manual)?/,
    /aria-haspopup/,
    /createPortal/,
    /\b(Dialog|Popover|AlertDialog)(Root|Trigger|Content|Portal|Overlay|Close|Anchor)?\b/,
  ];
  for (const path of sources) {
    it(`${path.split("/").pop()} has no dialog, popover or portal`, async () => {
      const { readFileSync } = await import("node:fs");
      const text = readFileSync(new URL(path, import.meta.url), "utf8");
      for (const pattern of forbidden) {
        assert.equal(pattern.test(text), false, `${pattern} found in ${path}`);
      }
    });
  }
  it("the bar is rendered in the top bar, not as an overlay", async () => {
    const { readFileSync } = await import("node:fs");
    const text = readFileSync(new URL("../src/components/CentreLinesButton.tsx", import.meta.url), "utf8");
    assert.match(text, /data-centre-lines-bar/);
    assert.doesNotMatch(text, /position:\s*fixed|fixed inset|\bfixed\b.*\binset-0/);
  });
});

// Tester 0117 Round B (customer 2026-11-18): Thick parts in mm is read by the binding; a value
// that keeps the 2 mm ring filled gives another document than the default (the NaN test above
// would not see a wrong mapping of argument 1 otherwise).
describe("centre lines Thick parts at the binding (tester, 0117 criterion 12)", () => {
  const ring =
    '<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="40mm" viewBox="0 0 40 40">' +
    '<path fill="#c8141e" fill-rule="evenodd" d="M31 20A11 11 0 1 0 9 20A11 11 0 1 0 31 20Z' +
    'M29 20A9 9 0 1 1 11 20A9 9 0 1 1 29 20Z"/></svg>';
  const run = async (thick: number) => {
    const { readFileSync } = await import("node:fs");
    const wasm = await import("../src/wasm-bindings/curvyo_editor_wasm.js");
    const bytes = readFileSync(new URL("../src/wasm-bindings/curvyo_editor_wasm_bg.wasm", import.meta.url));
    await wasm.default({ module_or_path: bytes });
    const session = new wasm.WasmSession(1n);
    session.import_begin("picker", false, 0, 0);
    session.import_add(new TextEncoder().encode(ring), "ring.svg");
    session.import_finish(true);
    const view = session.apply_centre_lines(0.2, thick, 0.75, false, 0.3, true);
    const out = { kind: view.kind, converted: view.converted, kept: view.kept, svg: session.export_svg(false, false, false, false) };
    session.free();
    return out;
  };
  it("keeps the 2 mm ring filled at 0.5 mm, converts it at 6 mm and at 100 mm (no limit)", async () => {
    const low = await run(0.5);
    const def = await run(6);
    const max = await run(100);
    assert.notDeepEqual(low.svg, def.svg, "Thick parts is read");
    assert.equal(def.kind, "applied");
    assert.equal(def.converted, 1);
    assert.deepEqual(max.svg, def.svg, "the 2 mm ring is under both limits");
    console.log(`0.5 mm: ${low.kind} converted ${low.converted} kept ${low.kept}`);
  });
});

// Tester 0117 criterion 15c: the info page's texts are the UX notes' table, row for row, in bar
// order, and the page renders them from that one module.
describe("centre lines info page equals the UX notes (tester, 0117 criterion 15c)", () => {
  it("has every row of the UX notes §3.2 table and nothing else", async () => {
    const { readFileSync } = await import("node:fs");
    const { CENTRE_INFO_ENTRIES, CENTRE_INFO_INTRO, CENTRE_INFO_TITLE } = await import("../src/lib/centreLinesText.ts");
    const notes = readFileSync(new URL("../../specs/0117-centre-lines/ux-notes.md", import.meta.url), "utf8");
    const section = notes.slice(notes.indexOf("### 3.2"), notes.indexOf("### 3.3"));
    const rows = [...section.matchAll(/^\| ([^|]+?) \| "([^"]+)" \|/gm)].map((m) => ({ term: m[1], text: m[2] }));
    assert.ok(rows.length >= 6, `rows found: ${rows.length}`);
    assert.deepEqual(rows[0], { term: "Intro", text: CENTRE_INFO_INTRO });
    assert.deepEqual(rows.slice(1), [...CENTRE_INFO_ENTRIES]);
    assert.equal(CENTRE_INFO_TITLE, "About centre lines");
    for (const { text } of rows) {
      assert.doesNotMatch(text, /ratio/i, "criterion 15c: no ratio");
    }
    const page = readFileSync(new URL("../src/components/panel/CentreInfoPage.tsx", import.meta.url), "utf8");
    assert.match(page, /CENTRE_INFO_ENTRIES\.map/);
    for (const { text } of rows) {
      assert.equal(page.includes(text), false, "no copy of a text in the page");
    }
  });
});

// Tester 0117 criterion 15 and UX notes §3.1: the sliders' table, row for row (label, unit,
// track, default, arrow / Shift / Ctrl steps, accessible name), and Home / End are the track's
// ends, never the typed maximum.
describe("centre lines sliders follow the UX notes table (tester, 0117 criterion 15)", () => {
  it("matches the table and steps by it", async () => {
    const { CENTRE_FIELDS, CENTRE_LINES_DEFAULTS, centreField, centreFieldStep, centreFieldValueAt } = await import("../src/lib/centreLinesText.ts");
    const want = [
      { key: "overlap", label: "Overlap", unit: "%", min: -50, max: 50, step: 1, bigStep: 10, fineStep: 1, name: "Overlap, percent of the line width", def: 20 },
      { key: "thickParts", label: "Thick parts", unit: "mm", min: 0.5, max: 20, step: 0.1, bigStep: 1, fineStep: 0.01, name: "Thick parts, millimetres", def: 6 },
      { key: "optimise", label: "Optimise", unit: "%", min: 0, max: 100, step: 1, bigStep: 10, fineStep: 1, name: "Optimise, percent", def: 75 },
      { key: "offset", label: "Offset", unit: "mm", min: -5, max: 5, step: 0.1, bigStep: 1, fineStep: 0.01, name: "Inner area offset, millimetres", def: 0.3 },
    ] as const;
    assert.deepEqual(CENTRE_FIELDS.map((f) => f.key), want.map((w) => w.key), "bar order");
    for (const w of want) {
      const f = centreField(w.key);
      for (const k of ["label", "unit", "min", "max", "step", "bigStep", "fineStep", "name"] as const) {
        assert.equal(f[k], w[k], `${w.key}.${k}`);
      }
      assert.equal((CENTRE_LINES_DEFAULTS as unknown as Record<string, number>)[w.key], w.def, `${w.key} default`);
      // Arrow, Shift+arrow and Ctrl+arrow from the default, both ways.
      for (const [grid, size] of [["normal", w.step], ["coarse", w.bigStep], ["fine", w.fineStep]] as const) {
        const up = centreFieldStep(f, w.def, 1, grid);
        const down = centreFieldStep(f, w.def, -1, grid);
        assert.ok(Math.abs(up - Math.min(f.typedMax, w.def + size)) < 1e-9, `${w.key} ${grid} up: ${up}`);
        assert.ok(Math.abs(down - Math.max(w.min, w.def - size)) < 1e-9, `${w.key} ${grid} down: ${down}`);
      }
      // Home and End: the track's ends.
      assert.equal(centreFieldValueAt(f, 0, "normal"), w.min, `${w.key} Home`);
      assert.equal(centreFieldValueAt(f, 1, "normal"), w.max, `${w.key} End`);
    }
    // Ten small steps equal one Shift step (no float drift).
    const thick = centreField("thickParts");
    let v = 6;
    for (let i = 0; i < 10; i += 1) v = centreFieldStep(thick, v, 1, "normal");
    assert.equal(v, 7);
  });
});

// Tester 0117 criterion 15 (no popups): the info page and the jobs add no dialog, popover or
// portal either.
describe("centre lines info page and jobs have no popup (tester, 0117 criterion 15)", () => {
  for (const path of ["../src/components/panel/CentreInfoPage.tsx", "../src/lib/centreLineJobs.ts"]) {
    it(`${path.split("/").pop()} has no dialog, popover or portal`, async () => {
      const { readFileSync } = await import("node:fs");
      const text = readFileSync(new URL(path, import.meta.url), "utf8");
      for (const pattern of [/role=["'](dialog|alertdialog)["']/, /<dialog\b/, /showModal|showPopover/, /createPortal/, /aria-haspopup/]) {
        assert.equal(pattern.test(text), false, `${pattern} in ${path}`);
      }
    });
  }
});

// Tester 0117 criterion 15b at the page's jobs (ADR 0063 amendment 2026-11-18, W): the newest
// values always win, new subjects are never lost to a pending overwrite, Apply gets the newest
// generation's result, and a fresh worker gets the subjects again.
describe("centre lines jobs keep the newest values (tester, 0117 criterion 15b)", () => {
  type Req = { type: string; id?: number; options?: number[]; wire?: Float64Array };
  const harness = async () => {
    const { CentreLineJobs } = await import("../src/lib/centreLineJobs.ts");
    const made: { got: Req[]; terminated: boolean; worker: { onmessage: ((e: { data: unknown }) => void) | null } }[] = [];
    const make = () => {
      const state = { got: [] as Req[], terminated: false, worker: null as unknown as { onmessage: ((e: { data: unknown }) => void) | null } };
      const worker = {
        onmessage: null as ((e: { data: unknown }) => void) | null,
        onerror: null,
        postMessage(message: Req) {
          state.got.push(message);
          if (message.type === "init") setTimeout(() => worker.onmessage?.({ data: { id: 0, kind: "ready", error: "" } }), 0);
        },
        terminate() {
          state.terminated = true;
        },
      };
      state.worker = worker;
      made.push(state);
      return worker;
    };
    const previews: { gen: number; wire: number[] }[] = [];
    const jobs = new CentreLineJobs(make as never, () => Promise.resolve({} as WebAssembly.Module), {
      preview: (gen: number, wire: Float64Array) => previews.push({ gen, wire: [...wire] }),
      progress: () => undefined,
      failed: () => undefined,
    });
    const reply = (i: number, data: unknown) => made[i].worker.onmessage?.({ data });
    const last = (i: number, type: string) => [...made[i].got].reverse().find((r) => r.type === type) as Req;
    return { jobs, made, previews, reply, last };
  };
  const tick = () => new Promise((resolve) => setTimeout(resolve, 5));
  const o = (overlap: number): [number, number, number, number, number] => [overlap, 6, 0.75, 0, 0.3];

  it("sends new subjects even when a newer request without subjects overwrites the pending one", async () => {
    const { jobs, made, reply, last, previews } = await harness();
    jobs.request(o(0.1), new Float64Array([1, 1]));
    await tick();
    const first = last(0, "centre-finish");
    // While the first runs: the selection changes (new subjects), then a slider moves on.
    jobs.request(o(0.2), new Float64Array([2, 2]));
    const newest = jobs.request(o(0.3));
    reply(0, { id: first.id, kind: "centre-preview", wire: new Float64Array([9]), ms: 1 });
    await tick();
    const subjects = made[0].got.filter((r) => r.type === "centre-subjects");
    assert.equal(subjects.length, 2, "the new subjects went to the worker");
    assert.deepEqual([...(subjects[1].wire ?? [])], [2, 2]);
    const second = last(0, "centre-finish");
    assert.deepEqual(second.options, o(0.3), "the newest values");
    assert.ok(made[0].got.indexOf(subjects[1]) < made[0].got.indexOf(second), "subjects before finish");
    reply(0, { id: second.id, kind: "centre-preview", wire: new Float64Array([8]), ms: 1 });
    assert.deepEqual(previews.map((p) => p.gen), [newest], "only the newest generation is shown");
  });

  it("gives Apply the newest generation's result, never an older one", async () => {
    const { jobs, reply, last } = await harness();
    jobs.request(o(0.1), new Float64Array([1, 1]));
    await tick();
    const first = last(0, "centre-finish");
    jobs.request(o(0.4));
    const result = jobs.result();
    reply(0, { id: first.id, kind: "centre-preview", wire: new Float64Array([1]), ms: 1 });
    await tick();
    const second = last(0, "centre-finish");
    assert.deepEqual(second.options, o(0.4));
    reply(0, { id: second.id, kind: "centre-preview", wire: new Float64Array([2]), ms: 1 });
    await tick();
    const asked = last(0, "centre-result");
    assert.equal(asked.id, second.id, "the result of the newest generation is asked for");
    reply(0, { id: asked.id, kind: "centre-result", wire: new Float64Array([42]) });
    assert.deepEqual([...((await result) ?? [])], [42]);
  });

  it("starts a fresh worker after a failure; the page closes the bar and a new press resends the subjects", async () => {
    const { jobs, made, reply, last, previews } = await harness();
    jobs.request(o(0.1), new Float64Array([5, 5]));
    await tick();
    reply(0, { id: last(0, "centre-finish").id, kind: "error", message: "trap" });
    assert.equal(made[0].terminated, true);
    const gen = jobs.request(o(0.2), new Float64Array([6, 6]));
    await tick();
    assert.equal(made.length, 2, "a fresh worker");
    const subjects = made[1].got.find((r) => r.type === "centre-subjects");
    assert.deepEqual([...(subjects?.wire ?? [])], [6, 6]);
    reply(1, { id: last(1, "centre-finish").id, kind: "centre-preview", wire: new Float64Array([3]), ms: 1 });
    assert.deepEqual(previews.map((p) => p.gen), [gen]);
  });
});

// Tester 0117 criterion 1 (customer 2026-11-18, point 6) and UX notes §1: Find centre lines is a
// group of the Node bar after Close path and before Edit width (also in the clone-only state), the
// last item of the Node context menu after a separator, and gone from the Select bar.
describe("centre lines entry points (tester, 0117 criterion 1)", () => {
  it("sits in the Node bar and menu, not in the Select bar", async () => {
    const { readFileSync } = await import("node:fs");
    const read = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");
    assert.doesNotMatch(read("../src/components/SelectToolbar.tsx"), /centre|CentreLines|Route\b/i, "not in the Select bar");
    const node = read("../src/components/NodeToolbar.tsx");
    const groups = [...node.matchAll(/role="group" aria-label="Centre lines"/g)].map((m) => m.index ?? 0);
    assert.equal(groups.length, 2, "the group in the bar and in the clone-only state");
    const bar = groups[1];
    assert.ok(node.lastIndexOf("<ClosePathGroup", bar) > 0 && node.lastIndexOf("<ClosePathGroup", bar) < bar, "after Close path");
    assert.ok(node.indexOf("{widthEdit}", bar) > bar, "before Edit width");
    const item = node.lastIndexOf("{centreLines?.shown ?");
    const tail = node.slice(item, node.indexOf("</ContextMenu.Content>", item));
    assert.match(tail, /^[^]*?<ContextMenu\.Separator[^]*?<ContextMenu\.Item[^]*?CENTRE_LINES_LABEL/, "separator, then the item");
    assert.equal((tail.match(/<ContextMenu\.Item\b/g) ?? []).length, 1, "nothing after it");
    assert.ok(node.indexOf("<NodeMenuItems", node.indexOf("function NodeContextMenu")) < item, "after the Node items");
    const { CENTRE_LINES_LABEL } = await import("../src/lib/centreLinesText.ts");
    assert.equal(CENTRE_LINES_LABEL, "Find centre lines…");
    assert.match(read("../src/components/CentreLinesButton.tsx"), /\bRoute\b/, "the Route glyph");
  });
});

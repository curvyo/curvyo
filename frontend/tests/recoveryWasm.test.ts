// The recovery copy through the real wasm surface (`specs/0136-autosave-and-restore` criteria 1, 5,
// 7): a step is seen, the worker's replica packs it, the record and the check read it, the
// assembled container opens as an unsaved project with the pages it had. Needs `npm run build:wasm`.

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

const BINDINGS = path.resolve(import.meta.dirname, "..", "src", "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");
const skipWasm = { skip: !existsSync(WASM) };

type Bindings = typeof import("../src/wasm-bindings/curvyo_editor_wasm.js") & {
  initSync: (options: { module: BufferSource }) => unknown;
};

async function load(): Promise<Bindings> {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as Bindings;
  bindings.initSync({ module: readFileSync(WASM) });
  return bindings;
}

test("a step is handed to the replica, packed, checked and restored as an unsaved project", skipWasm, async () => {
  const wasm = await load();
  const session = new wasm.WasmSession(11n);
  assert.equal(session.recovery_changed(), false, "a new project is no step");
  assert.ok(session.add_page_of_kind("sketch", "") !== "");
  assert.equal(session.recovery_changed(), true);
  assert.equal(session.recovery_changed(), false);

  const delta = session.recovery_export();
  assert.equal(delta.full, true, "the first hand-over seeds the replica");
  const writer = new wasm.WasmRecoveryWriter(12n);
  assert.equal(writer.apply(delta.full, delta.bytes), "");
  assert.equal(writer.apply(false, new Uint8Array([1, 2, 3])), "rejected");
  const packed = writer.pack();
  assert.ok(packed);
  assert.equal(packed.pages, 2);

  const record = wasm.recovery_record_text("00112233445566778899aabbccddeeff", "Poster.curvyo", "", 1_763_899_999_000, packed.pages);
  const checked = JSON.parse(wasm.recovery_check_entry(record, packed.bytes));
  assert.deepEqual([checked.name, checked.pages, checked.problem], ["Poster.curvyo", 2, null]);
  const damaged = JSON.parse(wasm.recovery_check_entry(record, packed.bytes.slice(0, 40)));
  assert.equal(damaged.problem, "damaged");
  const newer = JSON.parse(wasm.recovery_check_entry(record.replace('"format_version":1', '"format_version":9'), packed.bytes));
  assert.equal(newer.problem, "newer");

  const assembler = new wasm.WasmAssembler(packed.bytes);
  const container = assembler.finish();
  assert.ok(container);
  const restored = wasm.WasmSession.open_recovered(13n, container);
  assert.equal(restored.is_dirty(), true, "restored work is unsaved");
  assert.equal((JSON.parse(restored.pages_json()) as unknown[]).length, 2);
});

test("the schedule of the wasm surface follows the one rule", skipWasm, async () => {
  const wasm = await load();
  const schedule = new wasm.WasmRecoverySchedule();
  assert.equal(schedule.due(0, true), "nothing");
  schedule.step();
  assert.equal(schedule.due(0, true), "write");
  schedule.begin();
  schedule.written(0);
  schedule.step();
  assert.equal(schedule.due(1_000, true), "nothing");
  assert.equal(schedule.due(300_000, true), "write");
  schedule.begin();
  assert.equal(schedule.failed(300_000), true, "the first failure shows the notice");
  assert.equal(schedule.failure_line(), true);
});

test("the words of a row come from Rust", skipWasm, async () => {
  const wasm = await load();
  const at = { year: 2026, month: 11, day: 23, hour: 12, minute: 3 };
  const view = JSON.parse(
    wasm.recovery_row_view(JSON.stringify({ name: "Poster.curvyo", pages: 3, changed: true, problem: null }), JSON.stringify(at), JSON.stringify(at)),
  );
  assert.equal(view.line2, "12:03 · 3 pages");
  assert.equal(view.line3, "File changed since");
  assert.equal(view.restore_label, "Restore Poster.curvyo");
  assert.equal(wasm.recovery_help_line(undefined), "Kept in this browser");
  assert.equal(wasm.recovery_failure_sentence("disk-full"), "Recovery copy not written: the disk is full.");
  assert.equal(wasm.recovery_recovered_name("Poster.curvyo"), "Poster (recovered)");
});

// `specs/0091-trace-text-recognition` criterion 25: the lines are decided before the trace, the
// trace runs once, and one percent runs from 0 to 100 over the whole job, never backwards.

import { strict as assert } from "node:assert";
import { test } from "node:test";

import { JobProgress, traceWithText, type TextFlowRunner, type TextFlowSession } from "../src/lib/traceTextFlow.ts";

const pixels = () => ({ width: 2, height: 2, rgba: new Uint8Array(16), map: [1, 0, 0, 1, 0, 0] });

function standIns() {
  const calls: string[] = [];
  const runner: TextFlowRunner = {
    async regions(_p, _s, lines) {
      calls.push(`regions ${lines}`);
      return new Float64Array([0]);
    },
    async fit(_request, loadFaces, onProgress) {
      calls.push("fit");
      await loadFaces([]);
      for (const p of [0, 40, 100]) {
        onProgress?.(p);
      }
      return "{\"kind\":\"finished\",\"lines\":[]}";
    },
    async trace(_p, _s, onProgress, lines) {
      calls.push(`trace ${lines}`);
      for (const p of [0, 30, 60, 100]) {
        onProgress?.(p);
      }
      return { wire: new Float64Array([0]), seconds: 0 };
    },
  };
  const session: TextFlowSession = {
    trace_fit_request_for: () => "{\"lines\":[1]}",
    set_trace_fit: () => {
      calls.push("set fit");
      return true;
    },
    // The fit set no text for the ring: its line is no longer accepted.
    trace_ocr_lines: () => "decided",
  };
  return { calls, runner, session };
}

const fitInputs = { tolerance: 0.3, favourites: "[]", loadFaces: async () => [] };

test("0091 c25: the lines are decided first and the picture is traced once with them", async () => {
  const { calls, runner, session } = standIns();
  const seen: number[] = [];
  const progress = new JobProgress(["read", "fit", "trace"], (p) => seen.push(p));
  progress.stage("read")(100);
  const done = await traceWithText(session, runner, "7", pixels(), "{}", "read", fitInputs, progress, () => false);
  assert.deepEqual(calls, ["regions read", "fit", "set fit", "trace decided"]);
  assert.equal(calls.filter((c) => c.startsWith("trace")).length, 1, "one trace per run");
  assert.equal(done?.lines, "decided");
  // One percent: rising only, ending at 100.
  assert.ok(seen.every((p, i) => i === 0 || p > seen[i - 1]), `${seen}`);
  assert.equal(seen.at(-1), 100);
  // Reading 45, fit 25 and trace 25 of 95: the reading ends at 47 %.
  assert.ok(seen.includes(47), `the reading fills its own share first: ${seen}`);
});

test("0091 c25: a stop after the fit traces nothing", async () => {
  const { calls, runner, session } = standIns();
  let stopped = false;
  const stopping: TextFlowRunner = {
    ...runner,
    fit: async (...args) => {
      const r = await runner.fit(...args);
      stopped = true;
      return r;
    },
  };
  const progress = new JobProgress(["fit", "trace"], () => undefined);
  assert.equal(await traceWithText(session, stopping, "7", pixels(), "{}", "read", fitInputs, progress, () => stopped), null);
  assert.ok(!calls.some((c) => c.startsWith("trace")));
});

test("0091 c25: without text the job is the trace alone, still 0 to 100", () => {
  const seen: number[] = [];
  const progress = new JobProgress(["reduced", "trace"], (p) => seen.push(p));
  for (const p of [0, 50, 100]) {
    progress.stage("reduced")(p);
  }
  for (const p of [0, 50, 100]) {
    progress.stage("trace")(p);
  }
  progress.stage("fit")(100);
  assert.deepEqual(seen, [0, 8, 16, 58, 100]);
});

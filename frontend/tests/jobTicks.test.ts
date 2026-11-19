// The job ticks of `specs/0123-raster-engraving` criteria 12 and 37 (d) (ADR 0069 §12): one share
// of the pending job per animation frame, one sync when the job is built.
import { test } from "node:test";
import assert from "node:assert/strict";

import { jobTick, StaleMarkTimer, staleMark, type JobTickSession } from "../src/lib/jobTicks.ts";

/** A session whose job is built after `ticks` shares. */
function fake(ticks: number): JobTickSession & { words: boolean } {
  let left = ticks;
  return {
    words: false,
    show_stale_words() {
      this.words = left > 0;
    },
    job_pending: () => left > 0,
    advance_job: () => {
      if (left === 0) return false;
      left -= 1;
      return left === 0;
    },
  };
}

test("a pending job asks for another frame until it is built, then syncs once", () => {
  const session = fake(3);
  let syncs = 0;
  const sync = () => {
    syncs += 1;
  };
  assert.equal(jobTick(session, sync), true);
  assert.equal(jobTick(session, sync), true);
  assert.equal(syncs, 0);
  assert.equal(jobTick(session, sync), false);
  assert.equal(syncs, 1);
});

test("a job built by another read meanwhile: no frame, one sync", () => {
  let syncs = 0;
  assert.equal(
    jobTick(fake(0), () => {
      syncs += 1;
    }),
    false,
  );
  assert.equal(syncs, 1);
});

test("the 300 ms mark shows the stale words and syncs while the job is pending, else nothing", () => {
  let syncs = 0;
  const pending = fake(2);
  staleMark(pending, () => {
    syncs += 1;
  });
  assert.equal(pending.words, true);
  assert.equal(syncs, 1);
  const built = fake(0);
  staleMark(built, () => {
    syncs += 1;
  });
  assert.equal(built.words, false);
  assert.equal(syncs, 1);
});

test("the mark is armed once per pending period: syncs meanwhile never re-arm it", () => {
  const armed: Array<() => void> = [];
  const timer = new StaleMarkTimer(
    (fire) => armed.push(fire),
    () => {},
  );
  let fired = 0;
  // The effect runs after every sync while the job is pending, its own mark's sync included.
  for (let sync = 0; sync < 5; sync += 1) timer.arm(() => (fired += 1));
  assert.equal(armed.length, 1, "one mark for the pending period");
  armed[0]();
  timer.arm(() => (fired += 1));
  assert.equal(armed.length, 1, "the mark's own sync does not re-arm it");
  assert.equal(fired, 1);
  timer.reset();
  timer.arm(() => (fired += 1));
  assert.equal(armed.length, 2, "the next pending job gets its own mark");
});

test("a reset clears an armed mark", () => {
  const cleared: number[] = [];
  const timer = new StaleMarkTimer(
    () => 7,
    (id) => cleared.push(id),
  );
  timer.arm(() => {});
  timer.reset();
  timer.reset();
  assert.deepEqual(cleared, [7]);
});

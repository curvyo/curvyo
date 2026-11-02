import assert from "node:assert/strict";
import { test } from "node:test";

import {
  ANNOUNCE_DELAY_MS,
  AT_LEAST_ONE,
  ChangeQueue,
  FIELD_DEBOUNCE_MS,
  AT_LEAST_ZERO,
  ENTER_NUMBER,
  ENTER_WHOLE,
  ENTRY_FIELDS,
  IMAGES_TEXT,
  REFUSED_TEXT,
  WARN_TEXT,
  MORE_THAN_ZERO,
  PERCENT_RANGE,
  SWEEP_RANGE,
  TILE_EMPTY_RULE,
  TILE_WITH_OTHERS_RULE,
  type TilingRecipe,
  countText,
  entryText,
  freshSeed,
  noticeLines,
  parseEntry,
  parseRecipe,
  readField,
  tileRule,
  tileSlot,
  tilingAnnouncement,
  VARIANCE_BLOCKS,
  varianceSummary,
} from "../src/lib/tilingText.ts";

test("UX notes §2.2 to §2.4: each field rule and its messages", () => {
  assert.deepEqual(readField("5", "count"), { value: 5 });
  assert.deepEqual(readField("0", "count"), { error: AT_LEAST_ONE });
  assert.deepEqual(readField("2.5", "count"), { error: ENTER_WHOLE });
  assert.deepEqual(readField("100001", "count"), { value: 100001 }, "the bar shows the refusal");
  assert.deepEqual(readField("−2.5", "length"), { value: -2.5 });
  assert.deepEqual(readField("abc", "length"), { error: ENTER_NUMBER });
  assert.deepEqual(readField("0", "positive"), { error: MORE_THAN_ZERO });
  assert.deepEqual(readField("400", "sweep"), { error: SWEEP_RANGE });
  assert.deepEqual(readField("101", "shift"), { error: PERCENT_RANGE });
  assert.deepEqual(readField("-1", "range"), { error: AT_LEAST_ZERO });
  assert.deepEqual(readField("-1", "step"), { value: -1 });
  assert.deepEqual(readField("4294967296", "count"), { error: ENTER_WHOLE });
});

test("criterion 36: the count in the live region", () => {
  assert.equal(countText(1), "1 instance");
  assert.equal(countText(48), "48 instances");
  assert.equal(countText(10001), "10,001 instances");
});

test("UX notes §12.3: the notice slot names the limits, never the count", () => {
  const status = { count: 12, level: "fine" as const, images_copies: false, refusal: "", reopened: false };
  assert.deepEqual(noticeLines(status), []);
  assert.deepEqual(noticeLines({ ...status, count: 10001, level: "simplified" }), [WARN_TEXT]);
  assert.deepEqual(noticeLines({ ...status, images_copies: true }), [IMAGES_TEXT]);
  assert.deepEqual(noticeLines({ ...status, count: 100001, level: "refused", images_copies: true }), [IMAGES_TEXT, REFUSED_TEXT]);
  for (const line of noticeLines({ ...status, count: 10001, level: "simplified", images_copies: true })) {
    assert.doesNotMatch(line, /10,001/);
  }
});

test("UX notes §12.4: the entry boxes, their fields and prefill", () => {
  assert.deepEqual(ENTRY_FIELDS.centre.map((field) => field.name), ["Centre X, millimetres", "Centre Y, millimetres"]);
  assert.deepEqual(ENTRY_FIELDS.radius.map((field) => [field.label, field.rule]), [["r", "positive"]]);
  assert.deepEqual(ENTRY_FIELDS.start.map((field) => [field.name, field.width, field.labelWidth]), [["Start angle, degrees", 126, 44]]);
  assert.deepEqual(ENTRY_FIELDS.sweep.map((field) => [field.name, field.rule]), [["Sweep, degrees", "sweep"]]);
  assert.equal(entryText(-12.346), "−12.35");
  assert.equal(entryText(30), "30");
  assert.equal(entryText(-0.001), "0");
  assert.deepEqual(parseEntry('{"handle":"start","x":1,"y":2,"cx":3,"cy":4}'), { handle: "start", x: 1, y: 2, cx: 3, cy: 4 });
  assert.equal(parseEntry(""), null);
});

test("criterion 19: a fresh seed comes from the platform and is never 0", () => {
  assert.equal(freshSeed((into) => { into[0] = 0; }), 1);
  assert.equal(freshSeed((into) => { into[0] = 77; }), 77);
});

test("criteria 3 and 26: the Tile slot and its tooltip rule", () => {
  assert.equal(tileSlot(0), "nothing");
  assert.equal(tileSlot(1), "with-others");
  assert.equal(tileSlot(2), "tile");
  assert.equal(tileSlot(3), "edit");
  assert.equal(tileRule("nothing"), TILE_EMPTY_RULE);
  assert.equal(tileRule("with-others"), TILE_WITH_OTHERS_RULE);
});

test("criterion 15: the Variance tooltip names the active channels", () => {
  const recipe = parseRecipe(
    JSON.stringify({ rotation: [5, 0, 0], scale: [0, 0, 2], offset_x: [0, 0, 0], offset_y: [0, 0, 0], opacity: [0, 0, 0], hue: [0, 0, 0] }),
  ) as TilingRecipe;
  assert.equal(varianceSummary(recipe), "Variance: rotation, scale");
  assert.equal(parseRecipe(""), null);
});

test("UX notes §12.5 (2026-11-02): three blocks of two channels, Colour and Opacity together", () => {
  assert.deepEqual(
    VARIANCE_BLOCKS.map((block) => block.keys),
    [
      ["rotation", "scale"],
      ["offset_x", "offset_y"],
      ["hue", "opacity"],
    ],
  );
});

test("UX notes §2.1: field changes are sent once, 120 ms after the last keystroke", () => {
  let now = 0;
  const pending = new Map<number, { at: number; run: () => void }>();
  let next = 1;
  const timers = {
    set: (run: () => void, ms: number) => {
      pending.set(next, { at: now + ms, run });
      return next++;
    },
    clear: (id: number) => {
      pending.delete(id);
    },
  };
  const advance = (ms: number) => {
    now += ms;
    for (const [id, timer] of [...pending]) {
      if (timer.at <= now) {
        pending.delete(id);
        timer.run();
      }
    }
  };
  const sent: Partial<TilingRecipe>[] = [];
  let accept = true;
  const queue = new ChangeQueue<TilingRecipe>((change) => {
    sent.push(change);
    return accept;
  }, timers);
  queue.queue({ columns: 1 });
  advance(FIELD_DEBOUNCE_MS - 20);
  queue.queue({ columns: 12 });
  advance(FIELD_DEBOUNCE_MS - 1);
  assert.equal(sent.length, 0, "still typing");
  advance(1);
  assert.deepEqual(sent, [{ columns: 12 }]);
  let refused = false;
  accept = false;
  queue.queue({ rows: 3 }, () => {
    refused = true;
  });
  assert.equal(queue.flush(), false, "Apply sends what waits at once");
  assert.equal(refused, true, "the field is marked");
  queue.queue({ rows: 4 });
  queue.drop();
  advance(FIELD_DEBOUNCE_MS);
  assert.equal(sent.length, 2, "Cancel drops what waits");
});

test("UX notes §12.3: the live region names the count on entering", () => {
  assert.equal(tilingAnnouncement(48), "Tiling, 48 instances");
  assert.equal(tilingAnnouncement(1), "Tiling, 1 instance");
  assert.equal(ANNOUNCE_DELAY_MS, 500);
});

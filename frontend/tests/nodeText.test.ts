// Run with `npm test`. The sentences of `src/lib/nodeText.ts` against the wording of
// `specs/0053-node-marquee-selection/specification.md` (criteria 19, 23).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  joinReasonLines,
  joinReasonText,
  type NodeCountState,
  nodeCountText,
} from "../src/lib/nodeText.ts";

const base: NodeCountState = {
  joinBlock: 1,
  joinNotEnd: 0,
  selectedNodes: 0,
  totalNodes: 120,
  segmentSelected: false,
};

test("the status-bar count reads k of n, n, or Segment, n (criterion 23)", () => {
  assert.equal(nodeCountText({ ...base, selectedNodes: 5 }), "5 of 120 nodes");
  assert.equal(nodeCountText({ ...base, selectedNodes: 1, totalNodes: 4 }), "1 of 4 nodes");
  assert.equal(nodeCountText(base), "120 nodes");
  assert.equal(nodeCountText({ ...base, segmentSelected: true }), "Segment, 120 nodes");
  assert.equal(nodeCountText({ ...base, totalNodes: 0 }), null);
  assert.equal(
    nodeCountText({ ...base, selectedNodes: 123456, totalNodes: 999999 }),
    "123456 of 999999 nodes",
  );
});

test("the Join reason names the count or the non-end nodes (criterion 19)", () => {
  assert.equal(
    joinReasonText({ ...base, selectedNodes: 5 }),
    "Needs exactly two end nodes of open paths or outlines. 5 nodes selected.",
  );
  assert.equal(
    joinReasonText({ ...base, selectedNodes: 1 }),
    "Needs exactly two end nodes of open paths or outlines. 1 node selected.",
  );
  assert.equal(
    joinReasonText({ ...base, joinBlock: 2, joinNotEnd: 1, selectedNodes: 2 }),
    "Needs end nodes of open paths or outlines. 1 of 2 selected nodes is not an end node.",
  );
  assert.equal(joinReasonText({ ...base, joinBlock: 3, selectedNodes: 2 }), null);
  assert.equal(joinReasonText({ ...base, joinBlock: 0, selectedNodes: 2 }), null);
  assert.equal(joinReasonText(base), null, "nothing selected: no reason line");
});

test("the Join reason splits into a rule line and a count line", () => {
  assert.deepEqual(joinReasonLines({ ...base, selectedNodes: 5 }), [
    "Needs exactly two end nodes of open paths or outlines.",
    "5 nodes selected.",
  ]);
  assert.deepEqual(joinReasonLines({ ...base, joinBlock: 2, joinNotEnd: 1, selectedNodes: 2 }), [
    "Needs end nodes of open paths or outlines.",
    "1 of 2 selected nodes is not an end node.",
  ]);
  assert.equal(joinReasonLines(base), null);
});

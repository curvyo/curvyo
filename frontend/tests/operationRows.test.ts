// The Operations tab's row logic (`specs/0101-laser-operations` `ux-notes.md` sections 2 to 4,
// 6.2, 7, 9, 10): sections, accessible names, the summary lines, the reorder limit at the section
// line and the digit route. Pure functions over the decoded view (`operationsView.ts`).
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  decodeOperationsView,
  type OperationRow,
  type OperationsView,
} from "../src/lib/operationsView.ts";
import {
  ROW_JUMP,
  digitRow,
  HEADER_HEIGHT,
  ROW_HEIGHT,
  itemsText,
  listEntries,
  listGroups,
  neighbourIndex,
  nextFocusIndex,
  rowKey,
  rowName,
  summaryLines,
  tickPressAdds,
  assignWord,
  liveReadoutVisible,
  sectionRuns,
  insertionEdge,
  focusAfterRename,
  ghostTop,
  nameKeyAction,
} from "../src/lib/operationRows.ts";

function row(patch: Partial<OperationRow> & Pick<OperationRow, "name">): OperationRow {
  return {
    id: { peer: "1", counter: Math.floor(Math.random() * 1e6) },
    kind: "cut",
    section: "cutting",
    show: true,
    output: true,
    marginMm: 0,
    items: 1,
    tick: "none",
    ...patch,
  };
}

function view(rows: OperationRow[], extra: Partial<OperationsView> = {}): OperationsView {
  return {
    cutsLast: true,
    rows,
    summary: { operations: 0, running: 0, notAssigned: 0, runsWhileHidden: 0, outputOff: 0 },
    ...extra,
  };
}

// Customer 2026-11-10: no colour words (the colour test was removed with the colour).

test("items are counted in the singular for one", () => {
  assert.equal(itemsText(1), "1 item");
  assert.equal(itemsText(0), "0 items");
  assert.equal(itemsText(4), "4 items");
});

test("the accessible name carries the kind, the count and Output off", () => {
  assert.equal(rowName(row({ name: "Cut 1", kind: "cut", items: 4 })), "Cut 1, Cut, 4 items");
  assert.equal(
    rowName(row({ name: "Engrave + Outline 2", kind: "engrave_outline", items: 1, output: false })),
    "Engrave + Outline 2, Engrave + Outline, 1 item, Output off",
  );
  assert.equal(
    rowName(row({ name: "Plasma 1", kind: "unknown", section: null })),
    "Plasma 1, unknown operation",
  );
});

test("a row key is the node id text, unique per operation", () => {
  const a = row({ name: "a", id: { peer: "1", counter: 4 } });
  const b = row({ name: "b", id: { peer: "2", counter: 4 } });
  assert.notEqual(rowKey(a), rowKey(b));
  assert.equal(rowKey(a), "1:4");
});

test("with Cuts last on, two groups in order: Marking, then Cutting, both always present", () => {
  const groups = listGroups(
    view([
      row({ name: "Score 1", kind: "score", section: "marking" }),
      row({ name: "Cut 1", kind: "cut", section: "cutting" }),
    ]),
  );
  assert.deepEqual(
    groups.map((g) => [g.section, g.rows.map((r) => r.row.name)]),
    [
      ["marking", ["Score 1"]],
      ["cutting", ["Cut 1"]],
    ],
  );
  const empty = listGroups(view([row({ name: "Cut 1" })]));
  assert.deepEqual(
    empty.map((g) => [g.section, g.rows.length]),
    [
      ["marking", 0],
      ["cutting", 1],
    ],
  );
});

test("with Cuts last off, one flat group and the list index of each row", () => {
  const groups = listGroups({
    ...view([
      row({ name: "Cut 1", kind: "cut", section: "cutting" }),
      row({ name: "Score 1", kind: "score", section: "marking" }),
    ]),
    cutsLast: false,
  });
  assert.equal(groups.length, 1);
  assert.equal(groups[0].section, null);
  assert.deepEqual(
    groups[0].rows.map((r) => r.index),
    [0, 1],
  );
});

test("an unknown operation stays in the Marking group, in its own place", () => {
  const groups = listGroups(
    view([
      row({ name: "Plasma 1", kind: "unknown", section: null }),
      row({ name: "Cut 1", kind: "cut", section: "cutting" }),
    ]),
  );
  assert.deepEqual(groups[0].rows.map((r) => r.row.name), ["Plasma 1"]);
});

test("the summary reads the run line, then only the non-zero lines", () => {
  const texts = (v: OperationsView) => summaryLines(v).map((line) => line.text);
  assert.deepEqual(texts(view([])), ["Nothing runs. Assign items to an operation."]);
  const busy = view([row({ name: "Cut 1" }), row({ name: "Cut 2" })], {
    summary: { operations: 2, running: 9, notAssigned: 3, runsWhileHidden: 1, outputOff: 2 },
  });
  assert.deepEqual(texts(busy), [
    "Runs 2 operations, 9 items.",
    "3 items not assigned.",
    "1 item runs while hidden.",
    "2 items sit in operations with Output off.",
  ]);
  const plain = view([row({ name: "Cut 1" })], {
    summary: { operations: 1, running: 0, notAssigned: 0, runsWhileHidden: 0, outputOff: 0 },
  });
  assert.deepEqual(texts(plain), ["Runs 1 operation, 0 items."]);
  assert.deepEqual(
    summaryLines(busy).map((line) => line.count),
    [null, "notAssigned", "runsWhileHidden", "outputOff"],
    "the run line names no count, the others name theirs",
  );
});

test("a press on a mixed or empty tick adds the selection, a full tick removes it", () => {
  assert.equal(tickPressAdds("none"), true);
  assert.equal(tickPressAdds("some"), true);
  assert.equal(tickPressAdds("all"), false);
});

test("Alt+arrow moves one place inside the section and stops at the section line", () => {
  const rows = [
    row({ name: "Score 1", kind: "score", section: "marking" }),
    row({ name: "Engrave 1", kind: "engrave", section: "marking" }),
    row({ name: "Cut 1", kind: "cut", section: "cutting" }),
    row({ name: "Cut 2", kind: "cut", section: "cutting" }),
  ];
  const v = view(rows);
  assert.equal(neighbourIndex(v, 1, 1), null, "Engrave 1 down would cross into Cutting");
  assert.equal(neighbourIndex(v, 2, -1), null, "Cut 1 up crosses the section line");
  assert.equal(neighbourIndex(v, 0, 1), 1);
  assert.equal(neighbourIndex(v, 3, -1), 2);
  assert.equal(neighbourIndex(v, 0, -1), null, "no wrap at the top");
});

test("with Cuts last off any row moves to its neighbour, and an unknown row never moves", () => {
  const rows = [
    row({ name: "Cut 1", kind: "cut", section: "cutting" }),
    row({ name: "Score 1", kind: "score", section: "marking" }),
    row({ name: "Plasma 1", kind: "unknown", section: null }),
  ];
  const v = { ...view(rows), cutsLast: false };
  assert.equal(neighbourIndex(v, 0, 1), 1);
  assert.equal(neighbourIndex(v, 2, -1), null);
  assert.equal(neighbourIndex(v, 1, -1), 0);
});

test("the focus moves with the arrows, Home, End and five-row paging, never wrapping", () => {
  assert.equal(ROW_JUMP, 5);
  assert.equal(nextFocusIndex(10, 0, "ArrowDown"), 1);
  assert.equal(nextFocusIndex(10, 9, "ArrowDown"), 9);
  assert.equal(nextFocusIndex(10, 0, "ArrowUp"), 0);
  assert.equal(nextFocusIndex(10, 3, "Home"), 0);
  assert.equal(nextFocusIndex(10, 3, "End"), 9);
  assert.equal(nextFocusIndex(10, 3, "PageDown"), 8);
  assert.equal(nextFocusIndex(10, 7, "PageDown"), 9);
  assert.equal(nextFocusIndex(10, 2, "PageUp"), 0);
  assert.equal(nextFocusIndex(0, 0, "ArrowDown"), null);
  assert.equal(nextFocusIndex(10, 2, "a"), null);
});

test("digits 1 to 9 name the first nine rows; 0 and rows past nine have no digit", () => {
  const rows = Array.from({ length: 10 }, (_, i) => row({ name: `Cut ${i + 1}` }));
  const v = view(rows);
  assert.equal(digitRow(v, 1)?.name, "Cut 1");
  assert.equal(digitRow(v, 9)?.name, "Cut 9");
  assert.equal(digitRow(v, 0), null);
  assert.equal(digitRow(v, 10), null);
});

test("a decoded Rust state gives the same groups and lines as the fixture reads", () => {
  // The fixture the Rust side writes for the five kinds with Cuts last on (`operations_view`).
  const json = {
    cuts_last: true,
    rows: [
      {
        id: { peer: "1", counter: 1 },
        name: "Score 1",
        kind: "score",
        section: "marking",
        show: true,
        output: true,
        margin_mm: 0,
        items: 1,
        tick: "none",
      },
    ],
    summary: { operations: 1, running: 1, not_assigned: 0, runs_while_hidden: 0, output_off: 0 },
    items: { running: [], not_assigned: [], runs_while_hidden: [], output_off: [] },
    clone_members: [],
    view_mode: "operations",
  };
  const v = decodeOperationsView(json);
  assert.ok(v);
  assert.deepEqual(summaryLines(v).map((line) => line.text), ["Runs 1 operation, 1 item."]);
  assert.equal(listGroups(v)[0].rows.length, 1);
});

test("the flat entries carry a fixed height and a header per section; an open row is no taller (no details)", () => {
  const rows = [
    row({ name: "Score 1", kind: "score", section: "marking", id: { peer: "1", counter: 1 } }),
    row({ name: "Cut 1", kind: "cut", section: "cutting", id: { peer: "1", counter: 2 } }),
  ];
  const entries = listEntries(view(rows), "1:2");
  assert.deepEqual(
    entries.map((e) => [e.kind, e.height]),
    [
      ["header", HEADER_HEIGHT],
      ["row", ROW_HEIGHT],
      ["header", HEADER_HEIGHT],
      ["row", ROW_HEIGHT],
    ],
  );
  assert.equal(entries[3].kind === "row" && entries[3].index, 1);
  const flat = listEntries({ ...view(rows), cutsLast: false }, null);
  assert.deepEqual(
    flat.map((e) => e.kind),
    ["row", "row"],
  );
});

test("with Cuts last on, the visible entries split into section runs, each named by its section", () => {
  const rows = [
    row({ name: "Score 1", kind: "score", section: "marking", id: { peer: "1", counter: 1 } }),
    row({ name: "Engrave 1", kind: "engrave", section: "marking", id: { peer: "1", counter: 2 } }),
    row({ name: "Cut 1", kind: "cut", section: "cutting", id: { peer: "1", counter: 3 } }),
  ];
  const entries = listEntries(view(rows), null);
  const runs = sectionRuns(entries, true);
  assert.deepEqual(
    runs.map((r) => [r.section, r.entries.length]),
    [
      ["marking", 3],
      ["cutting", 2],
    ],
  );
  // A window that starts inside the Marking rows keeps the section of each row.
  const window = sectionRuns(entries.slice(2), true);
  assert.deepEqual(
    window.map((r) => [r.section, r.entries.length]),
    [
      ["marking", 1],
      ["cutting", 2],
    ],
  );
});

test("with Cuts last off there is one run with no section name", () => {
  const rows = [row({ name: "Cut 1", section: "cutting" }), row({ name: "Score 1", kind: "score", section: "marking" })];
  const runs = sectionRuns(listEntries({ ...view(rows), cutsLast: false }, null), false);
  assert.deepEqual(
    runs.map((r) => [r.section, r.entries.length]),
    [[null, 2]],
  );
});

test("every row is one 40 px height, an Output off row included (customer 2026-11-10: no taller row)", () => {
  const rows = [
    row({ name: "Cut 1", output: true, id: { peer: "1", counter: 1 } }),
    row({ name: "Cut 2", output: false, id: { peer: "1", counter: 2 } }),
  ];
  const heights = listEntries(view(rows), null).map((e) => (e.kind === "row" ? e.height : 0));
  assert.deepEqual(heights.filter((h) => h > 0), [ROW_HEIGHT, ROW_HEIGHT]);
  assert.equal(ROW_HEIGHT, 40);
});

test("the drop badge of a canvas drag over a row reads Assign to with the row's name", () => {
  assert.equal(assignWord("Cut 1"), "Assign to Cut 1");
});

test("the Δ move readout hides while the pointer is over an operation row", () => {
  const readout = { text: "Δ 3.0, 0.0 mm", second: null, third: null, thirdInvalid: false, x: 1, y: 2 };
  assert.equal(liveReadoutVisible(readout, false), true);
  assert.equal(liveReadoutVisible(readout, true), false);
  assert.equal(liveReadoutVisible(null, false), false);
});

test("the insertion line sits at the top edge of the landing row when it moves up, the bottom edge when it moves down (ux-notes 4.6)", () => {
  assert.equal(insertionEdge(2, 0), "top");
  assert.equal(insertionEdge(0, 2), "bottom");
  assert.equal(insertionEdge(1, 1), null, "no line where the row already is");
});

test("the ghost follows the pointer vertically and stays inside the list's box (ux-notes 4.6)", () => {
  // The grab point is 10 px below the row's top; the list spans y 100..300; the ghost is 40 px.
  assert.equal(ghostTop(150, 10, 100, 300, 40), 140);
  assert.equal(ghostTop(50, 10, 100, 300, 40), 100, "clamped to the top of the list");
  assert.equal(ghostTop(900, 10, 100, 300, 40), 260, "clamped so the ghost ends at the bottom");
});

test("a key on a row's name: Alt+arrows step, F2 renames, Delete arms (or removes when armed), Enter removes only when armed (ux-notes 4.4, 4.5, 9)", () => {
  assert.equal(nameKeyAction("ArrowUp", true, false), "step-up");
  assert.equal(nameKeyAction("ArrowDown", true, false), "step-down");
  assert.equal(nameKeyAction("F2", false, false), "rename");
  assert.equal(nameKeyAction("Delete", false, false), "arm");
  assert.equal(nameKeyAction("Delete", false, true), "remove", "Delete on the armed row removes");
  assert.equal(nameKeyAction("Enter", false, true), "remove", "Enter on the armed row removes");
  assert.equal(nameKeyAction("Enter", false, false), "none", "Enter on an idle row only selects it");
  assert.equal(nameKeyAction("ArrowDown", false, false), "none", "plain arrows move the focus, not the row");
});

test("after a rename ends by Enter or Escape the focus stays on the row; a blur commits and leaves the focus where the maker put it", () => {
  assert.equal(focusAfterRename("enter"), true);
  assert.equal(focusAfterRename("escape"), true);
  assert.equal(focusAfterRename("blur"), false);
});

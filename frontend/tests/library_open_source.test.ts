// Open source in the browser build (`specs/0137-document-library` criterion 7): a document of
// another stored workspace opens through that workspace and its page is activated by id; a page
// gone from the source gives the notice; a document gone gives the alert. Pure logic, the host
// calls stubbed.

import assert from "node:assert/strict";
import { test } from "node:test";

import { type OpenSourceDeps, openSource } from "../src/lib/libraryOpenSource.ts";

const record = { document: "Lid", page: "Front", page_id: "7-3", at: 0 };
const place = { source: "ws-2", path: ["Sides", "Lid.curvyo"], in_workspace: false, file: "Sides/Lid.curvyo" };

function deps(over: Partial<OpenSourceDeps>, log: string[]): OpenSourceDeps {
  return {
    scan: async () => 0,
    find: async () => place,
    desktop: false,
    isOpen: () => false,
    activateNow: () => true,
    activateWhenOpen: (r) => log.push(`wait ${r.page_id}`),
    openInWorkspace: (path) => log.push(`workspace ${path.join("/")}`),
    openStored: async (source, path) => {
      log.push(`stored ${source} ${path.join("/")}`);
      return true;
    },
    openPath: async (file) => {
      log.push(`path ${file}`);
    },
    pageGone: (r) => log.push(`page gone ${r.page}`),
    sourceGone: (r) => log.push(`source gone ${r.document}`),
    ...over,
  };
}

test("a browser document of another stored workspace opens through it and waits for its page", async () => {
  const log: string[] = [];
  await openSource(record, place, deps({}, log));
  assert.deepEqual(log, ["wait 7-3", "stored ws-2 Sides/Lid.curvyo"]);
});

test("the browser never calls the desktop's open of a path", async () => {
  const log: string[] = [];
  await openSource(record, place, deps({ openPath: async () => assert.fail("no host path") }, log));
  assert.ok(!log.some((line) => line.startsWith("path")));
});

test("a stored document that is gone gives the alert", async () => {
  const log: string[] = [];
  await openSource(record, place, deps({ openStored: async () => false }, log));
  assert.deepEqual(log, ["wait 7-3", "source gone Lid"]);
});

test("a source that no longer has the page opens where it was and the page-gone notice follows", async () => {
  const log: string[] = [];
  await openSource(record, place, deps({ find: async () => null }, log));
  assert.deepEqual(log, ["wait 7-3", "stored ws-2 Sides/Lid.curvyo"]);
});

test("the open document without the page says so at once, in both builds", async () => {
  for (const desktop of [false, true]) {
    const log: string[] = [];
    await openSource(
      record,
      { ...place, in_workspace: true },
      deps({ desktop, isOpen: () => true, activateNow: () => false }, log),
    );
    assert.deepEqual(log, ["page gone Front"]);
  }
});

test("nothing known gives the alert; the desktop opens a file through the host", async () => {
  const gone: string[] = [];
  await openSource(record, null, deps({ find: async () => null }, gone));
  assert.deepEqual(gone, ["source gone Lid"]);
  const desktop: string[] = [];
  await openSource(record, place, deps({ desktop: true }, desktop));
  assert.deepEqual(desktop, ["wait 7-3", "path Sides/Lid.curvyo"]);
});

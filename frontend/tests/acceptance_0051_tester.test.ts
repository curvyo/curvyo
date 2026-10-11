// Independent tester cases for `specs/0051-sidebar-and-no-menu/specification.md`, page side, without
// a window: the words and rules in `lib/` (criteria 10, 16, 19, 25, 30, 32, 34) and the strip and
// the File tab rendered on the server (criteria 2, 5, 6, 7, 15, 16, 23, 24, 25). Keyboard, focus,
// layout and persistence in a real webview are NOT covered here (see the report).

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

import { detectPlatform } from "../src/lib/platformKind.ts";
import {
  PANEL_COLLAPSED_KEY,
  readPanelCollapsed,
  writePanelCollapsed,
} from "../src/lib/panelCollapsed.ts";
import {
  baseName,
  checkSentence,
  projectName,
  projectStatus,
  projectTitle,
  savedNotice,
} from "../src/lib/projectText.ts";

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

// ---------------------------------------------------------------- criterion 30: the title

test("c30 the window title for every state, with the em dash and a leading asterisk", () => {
  assert.equal(projectTitle(null, false), "Curvyo");
  assert.equal(projectTitle(null, true), "*Curvyo");
  assert.equal(projectTitle("Poster.curvyo", false), "Poster.curvyo \u2014 Curvyo");
  assert.equal(projectTitle("Poster.curvyo", true), "*Poster.curvyo \u2014 Curvyo");
});

test("c30 the title walk of the spec: edit, save, edit, undo, reopen", () => {
  // *Curvyo, Curvyo, *Curvyo, *Curvyo (undo does not clean), Curvyo
  const steps: [string | null, boolean][] = [
    [null, true],
    ["a.curvyo", false],
    ["a.curvyo", true],
    ["a.curvyo", true],
    ["a.curvyo", false],
  ];
  assert.deepEqual(
    steps.map(([name, dirty]) => projectTitle(name, dirty).replace(/^\*?(a\.curvyo \u2014 )?/, "")),
    ["Curvyo", "Curvyo", "Curvyo", "Curvyo", "Curvyo"],
  );
});

test("c30 hostile names do not break the title", () => {
  for (const name of ["", " ", "*", "a\nb", "\u202eevil.curvyo", "x".repeat(100_000)]) {
    const title = projectTitle(name, true);
    assert.ok(title.startsWith("*"));
    assert.ok(title.endsWith("Curvyo"));
  }
});

// ---------------------------------------------------------------- criterion 16: project block

test("c16 name and status line", () => {
  assert.equal(projectName(null), "Untitled");
  assert.equal(projectName("Poster.curvyo"), "Poster.curvyo");
  assert.equal(projectStatus(null, true), "Unsaved changes");
  assert.equal(projectStatus("Poster.curvyo", true), "Unsaved changes");
  assert.equal(projectStatus("Poster.curvyo", false), "No changes since the last save");
  assert.equal(projectStatus(null, false), "Not saved yet");
});

// A path ending in a separator has an empty base name. Not reachable from the OS dialogs (they
// return files), so this is a todo, not a failure: the assertion is the real one.
test("c16 an empty file name still shows a name (hostile: empty string)", { todo: "defect D5: baseName('dir/') is empty and projectName shows nothing" }, () => {
  // A path such as "dir/" or "C:\\" has an empty base name; the block must not render nothing.
  assert.notEqual(projectName(baseName("dir/")), "", "empty base name shows as an empty project name");
});

test("baseName handles both separators and edge shapes", () => {
  assert.equal(baseName("/home/m/a.curvyo"), "a.curvyo");
  assert.equal(baseName("C:\\Users\\m\\a.curvyo"), "a.curvyo");
  assert.equal(baseName("a.curvyo"), "a.curvyo");
  assert.equal(baseName("mixed/dir\\a.curvyo"), "a.curvyo");
  assert.equal(baseName(""), "");
});

// ---------------------------------------------------------------- criterion 25: the sentence

test("c25 the sentence for every replacing action, named and untitled", () => {
  const tail = " Discarded changes cannot be brought back.";
  const cases: [string, string][] = [
    ["new", "start a new project"],
    ["open", "open another project"],
    ["handover", "open another project"],
    ["close", "close it"],
    ["quit", "quit"],
  ];
  for (const [kind, end] of cases) {
    assert.equal(
      checkSentence("Poster.curvyo", kind as never),
      `Poster.curvyo has unsaved changes. Save them before you ${end}?${tail}`,
    );
    assert.equal(
      checkSentence(null, kind as never),
      `This project has unsaved changes. Save them before you ${end}?${tail}`,
    );
  }
});

// ---------------------------------------------------------------- criterion 19: notice

test("c19 the saved notice, desktop and browser", () => {
  assert.equal(savedNotice("Poster.curvyo", false), "Saved Poster.curvyo.");
  assert.equal(savedNotice("poster.curvyo", true), "Downloaded poster.curvyo.");
});

// ---------------------------------------------------------------- criterion 10: persistence

function fakeStorage(initial: Record<string, string> = {}) {
  const data = new Map(Object.entries(initial));
  return {
    data,
    getItem: (key: string) => data.get(key) ?? null,
    setItem: (key: string, value: string) => {
      data.set(key, value);
    },
  };
}

test("c10 the key and the values", () => {
  assert.equal(PANEL_COLLAPSED_KEY, "curvyo.panel-collapsed");
  const storage = fakeStorage();
  assert.equal(readPanelCollapsed(storage), false, "first start shows the body");
  writePanelCollapsed(true, storage);
  assert.equal(storage.data.get("curvyo.panel-collapsed") !== undefined, true);
  assert.equal(readPanelCollapsed(storage), true);
  writePanelCollapsed(false, storage);
  assert.equal(readPanelCollapsed(storage), false);
});

test("c10 unreadable or unwritable storage means shown, silently", () => {
  const throwing = {
    getItem: () => {
      throw new Error("denied");
    },
    setItem: () => {
      throw new Error("denied");
    },
  };
  assert.equal(readPanelCollapsed(throwing), false);
  assert.doesNotThrow(() => writePanelCollapsed(true, throwing));
  assert.equal(readPanelCollapsed(undefined as never), false);
  assert.doesNotThrow(() => writePanelCollapsed(true, undefined as never));
  for (const junk of ["", "1", "TRUE", "yes", "null", "undefined", "{}", "true ", "\u0000"]) {
    assert.equal(readPanelCollapsed(fakeStorage({ [PANEL_COLLAPSED_KEY]: junk })), false, junk);
  }
});

// ---------------------------------------------------------------- platform detection

test("c32 platform kind: browser unless desktop; mac, windows, linux by navigator.platform", () => {
  assert.equal(detectPlatform("MacIntel", false), "browser");
  assert.equal(detectPlatform("Win32", false), "browser");
  assert.equal(detectPlatform("Linux x86_64", false), "browser");
  assert.equal(detectPlatform("MacIntel", true), "macos");
  assert.equal(detectPlatform("MacARM", true), "macos");
  assert.equal(detectPlatform("Win32", true), "windows");
  assert.equal(detectPlatform("Linux x86_64", true), "linux");
  assert.equal(detectPlatform("Linux armv8l", true), "linux");
  assert.equal(detectPlatform("FreeBSD amd64", true), "linux");
  assert.equal(detectPlatform("", true), "linux");
});

// ---------------------------------------------------------------- rendering helpers

async function renderStrip(opts: {
  open: boolean;
  active: string;
  fileDirty: boolean;
  styleEnabled?: boolean;
}): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const tabs = await server.ssrLoadModule("/src/components/ui/tabs.tsx");
  const strip = await server.ssrLoadModule("/src/components/panel/PanelStrip.tsx");
  const noop = () => undefined;
  const views = [
    { name: "file", label: "File", tooltip: "File", enabled: true },
    { name: "document", label: "Document", tooltip: "Document (Shift+Ctrl+D)", enabled: true },
    {
      name: "style",
      label: "Style",
      tooltip: "Style: select an object first",
      enabled: opts.styleEnabled ?? false,
    },
    { name: "history", label: "History", tooltip: "History (Shift+Ctrl+H)", enabled: true },
  ];
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(
      tabs.Tabs as never,
      { value: opts.active, orientation: "vertical" },
      react.createElement(strip.PanelStrip as never, {
        tabs: views,
        active: opts.active,
        open: opts.open,
        bodyId: "body",
        fileDirty: opts.fileDirty,
        onToggle: noop,
        onActivate: noop,
        onReturnFocus: noop,
      }),
    ) as never,
  );
  return dom.renderToString(element as never);
}

function elementsWithRole(html: string, role: string): string[] {
  return [...html.matchAll(new RegExp(`<[a-z]+ [^>]*role="${role}"[^>]*>`, "g"))].map((m) => m[0]);
}

const attr = (tag: string, name: string) => new RegExp(`${name}="([^"]*)"`).exec(tag)?.[1];

// ---------------------------------------------------------------- the strip

test("c2 c6 the tablist: vertical, named Panel, four tabs in the order File, Document, Style, History", async () => {
  const html = await renderStrip({ open: true, active: "document", fileDirty: false });
  const [list] = elementsWithRole(html, "tablist");
  assert.equal(attr(list, "aria-orientation"), "vertical");
  assert.equal(attr(list, "aria-label"), "Panel");
  const tabs = elementsWithRole(html, "tab");
  assert.deepEqual(
    tabs.map((tab) => attr(tab, "aria-label")),
    ["File", "Document", "Style", "History"],
  );
  for (const tab of tabs) {
    assert.ok(attr(tab, "aria-controls"), "aria-controls");
  }
});

test("c5 exactly one pressed tab with the body shown, none with it hidden; hidden does not forget", async () => {
  for (const active of ["file", "document", "history"]) {
    const shown = elementsWithRole(
      await renderStrip({ open: true, active, fileDirty: false }),
      "tab",
    );
    assert.equal(shown.filter((t) => attr(t, "aria-selected") === "true").length, 1, active);
    const hidden = elementsWithRole(
      await renderStrip({ open: false, active, fileDirty: false }),
      "tab",
    );
    assert.equal(hidden.filter((t) => attr(t, "aria-selected") === "true").length, 0, active);
  }
});

test("c5 the dimmed Style tab is disabled, aria-disabled, and an enabled one is neither", async () => {
  const dimmed = elementsWithRole(
    await renderStrip({ open: true, active: "document", fileDirty: false }),
    "tab",
  );
  const style = dimmed[2];
  assert.match(style, /disabled=""/);
  assert.equal(attr(style, "aria-disabled"), "true");
  const live = elementsWithRole(
    await renderStrip({ open: true, active: "document", fileDirty: false, styleEnabled: true }),
    "tab",
  );
  assert.doesNotMatch(live[2], /\sdisabled=/);
  for (const t of [live[0], live[1], live[3]]) {
    assert.doesNotMatch(t, /\sdisabled=/);
  }
});

test("c23 the dirty dot is on the File tab only, also with the body hidden, with the right name", async () => {
  for (const open of [true, false]) {
    const html = await renderStrip({ open, active: "document", fileDirty: true });
    assert.equal((html.match(/data-dirty-dot/g) ?? []).length, 1, `open=${open}`);
    const tabs = elementsWithRole(html, "tab");
    assert.equal(attr(tabs[0], "aria-label"), "File, unsaved changes");
    for (const other of tabs.slice(1)) {
      assert.doesNotMatch(other, /unsaved/);
    }
    // the dot lies inside the File button
    const fileButton = /<button[^>]*aria-label="File, unsaved changes"[^>]*>([\s\S]*?)<\/button>/.exec(html);
    assert.ok(fileButton && /data-dirty-dot/.test(fileButton[1]), "dot inside the File button");
  }
  const clean = await renderStrip({ open: true, active: "document", fileDirty: false });
  assert.equal((clean.match(/data-dirty-dot/g) ?? []).length, 0);
  assert.equal(attr(elementsWithRole(clean, "tab")[0], "aria-label"), "File");
});

test("c7 the collapse control: first button, named by what it does, aria-expanded and aria-controls", async () => {
  for (const open of [true, false]) {
    const html = await renderStrip({ open, active: "document", fileDirty: false });
    const first = /<button[^>]*>/.exec(html)![0];
    assert.equal(attr(first, "aria-label"), open ? "Hide panel" : "Show panel");
    assert.equal(attr(first, "aria-expanded"), String(open));
    assert.equal(attr(first, "aria-controls"), "body");
  }
});

test("c2 glyphs are 16 px with a 1.5 stroke, 28 px hit areas", async () => {
  const html = await renderStrip({ open: true, active: "document", fileDirty: false });
  const svgs = [...html.matchAll(/<svg[^>]*>/g)].map((m) => m[0]);
  assert.equal(svgs.length, 5, "collapse control plus four tabs");
  for (const svg of svgs) {
    assert.equal(attr(svg, "width"), "16");
    assert.equal(attr(svg, "height"), "16");
  }
  // 1.5 px absolute stroke: with `absoluteStrokeWidth` Lucide scales the stroke so 16 px shows 1.5
  for (const svg of svgs) {
    const w = Number(attr(svg, "stroke-width"));
    assert.ok(w > 1.5 && w < 3, `absolute stroke: attribute ${w}`);
  }
  assert.equal((html.match(/size-7/g) ?? []).length >= 5, true, "28 px hit areas");
  // no visible text in tabs
  for (const tab of elementsWithRole(html, "tab")) {
    assert.doesNotMatch(tab, />(File|Document|Style|History)</);
  }
});

// ---------------------------------------------------------------- the File tab

type Project = {
  info: { fileName: string | null; path: string | null };
  dirty: boolean;
  check: null | { id: number; kind: string; origin: unknown; previous: unknown };
  press: (c: string) => void;
  saveFromCheck: () => void;
  discard: () => void;
  cancel: () => void;
};

async function renderFile(opts: {
  desktop: boolean;
  project: Partial<Project> & { info?: Project["info"] };
}): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const file = await server.ssrLoadModule("/src/components/panel/FileTab.tsx");
  const noop = () => undefined;
  const project: Project = {
    info: { fileName: null, path: null },
    dirty: false,
    check: null,
    press: noop,
    saveFromCheck: noop,
    discard: noop,
    cancel: noop,
    ...opts.project,
  };
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(file.FileTab as never, {
      project,
      desktop: opts.desktop,
      version: "9.8.7",
      tooltipFor: (c: string) => `tip-${c}`,
      onReturnFocus: noop,
    }) as never,
  );
  return dom.renderToString(element as never);
}

const text = (html: string) => html.replace(/<[^>]+>/g, "|").replace(/\|+/g, "|");
const buttonLabels = (html: string) =>
  [...html.matchAll(/<button[^>]*>([^<]*)<\/button>/g)].map((m) => m[1]);

test("c15 desktop File tab: five buttons, then Quit, then the version", async () => {
  const html = await renderFile({ desktop: true, project: {} });
  assert.deepEqual(buttonLabels(html), ["New", "Open…", "Save", "Save as…", "Close", "Quit"]);
  assert.match(html, /Curvyo 9\.8\.7/);
  assert.ok(html.indexOf("Quit") < html.indexOf("Curvyo 9.8.7"));
});

test("c15 browser File tab: no Save as, no Quit, no divider", async () => {
  const html = await renderFile({ desktop: false, project: {} });
  assert.deepEqual(buttonLabels(html), ["New", "Open…", "Save", "Close"]);
  assert.doesNotMatch(html, /Quit|Save as/);
  assert.match(html, /Curvyo 9\.8\.7/);
});

test("c15 the buttons are never dimmed or hidden by the state of the project", async () => {
  for (const project of [
    {},
    { dirty: true },
    { info: { fileName: "a.curvyo", path: "/x/a.curvyo" } },
    { dirty: true, info: { fileName: "a.curvyo", path: "/x/a.curvyo" } },
  ]) {
    const html = await renderFile({ desktop: true, project });
    assert.deepEqual(buttonLabels(html), ["New", "Open…", "Save", "Save as…", "Close", "Quit"]);
    assert.doesNotMatch(html, /\sdisabled=/);
    assert.doesNotMatch(html, /aria-disabled/);
  }
});

test("c16 c23 the project block shows the name, the status line, and the path only on desktop", async () => {
  const named = { fileName: "Poster.curvyo", path: "/home/m/Poster.curvyo" };
  let html = await renderFile({ desktop: true, project: { info: named, dirty: true } });
  assert.match(text(html), /Poster\.curvyo\|Unsaved changes/);
  html = await renderFile({ desktop: true, project: { info: named, dirty: false } });
  assert.match(text(html), /Poster\.curvyo\|No changes since the last save/);
  html = await renderFile({ desktop: true, project: {} });
  assert.match(text(html), /Untitled\|Not saved yet/);
  html = await renderFile({ desktop: true, project: { dirty: true } });
  assert.match(text(html), /Untitled\|Unsaved changes/);
});

test("c24 c25 the inline check: a group named by its sentence, three buttons, no dialog/menu/listbox", async () => {
  const check = { id: 1, kind: "open", origin: "outside", previous: null };
  for (const desktop of [true, false]) {
    const html = await renderFile({
      desktop,
      project: { dirty: true, check, info: { fileName: "Poster.curvyo", path: "/x/Poster.curvyo" } },
    });
    const [group] = elementsWithRole(html, "group");
    assert.equal(
      attr(group, "aria-label"),
      "Poster.curvyo has unsaved changes. Save them before you open another project? Discarded changes cannot be brought back.",
    );
    const labels = buttonLabels(html);
    assert.deepEqual(labels.slice(0, 3), ["Cancel", "Discard", "Save"]);
    // the five buttons are replaced in place
    for (const gone of ["New", "Open…", "Save as…", "Close"]) {
      assert.ok(!labels.includes(gone), gone);
    }
    for (const role of ["dialog", "alertdialog", "menu", "listbox", "menuitem", "option"]) {
      assert.equal(elementsWithRole(html, role).length, 0, role);
    }
    assert.doesNotMatch(html, /<dialog/);
    // the block keeps the 100 px of the five buttons so nothing below moves
    assert.match(html, /h-\[100px\]/);
  }
});

test("c25 Discard starts disarmed (ignores presses for the first 400 ms)", async () => {
  const check = { id: 1, kind: "new", origin: "outside", previous: null };
  const html = await renderFile({ desktop: true, project: { dirty: true, check } });
  const discard = /<button[^>]*>Discard<\/button>/.exec(html)![0];
  assert.equal(attr(discard, "aria-disabled"), "true");
  const cancel = /<button[^>]*>Cancel<\/button>/.exec(html)![0];
  assert.doesNotMatch(cancel, /aria-disabled="true"/);
});

test("c24 a File tab without a check has no popup, menu, dialog or listbox role", async () => {
  const html = await renderFile({ desktop: true, project: {} });
  for (const role of ["dialog", "alertdialog", "menu", "listbox", "menuitem", "option"]) {
    assert.equal(elementsWithRole(html, role).length, 0, role);
  }
});

import { noticeDurationMs } from "../src/lib/historyText.ts";

// Criterion 19 says the "Saved <file name>." notice stays 3 s. The notice slot lengthens any text
// over 70 characters to 5 s (`0016` criterion 29), so a long file name gets 5 s.
test("c19 the saved notice stays 3 s whatever the file name", { todo: "defect D6: a file name over ~60 characters makes the notice last 5 s" }, () => {
  for (const name of ["a.curvyo", "x".repeat(80) + ".curvyo"]) {
    assert.equal(noticeDurationMs(savedNotice(name, false)), 3000, name);
  }
});

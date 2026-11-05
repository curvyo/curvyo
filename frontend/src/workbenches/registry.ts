// The workbench registry (ADR 0032 §3 as amended by ADR 0042 §7, `specs/0098-page-kinds`
// criterion 18): the one place that maps the workbench of a page kind to what the shell draws for
// it. Vector and Sketch are two implementations, every output page of any process is drawn by the
// one generic manufacturing entry; the shell never branches on a process. Pure data and
// functions, so the tests load it without a bundler.

/** A parameter of a schema-rendered block (`curvyo-extension-core` `Param`). */
export interface ManifestParam {
  key: string;
  label: string;
  tooltip: string;
  kind: { kind: "length"; min: number; max: number };
  default: { kind: "length"; value: number };
}

/** A block of a schema-rendered tab. */
export type ManifestBlock =
  | { kind: "machine_profile"; label: string }
  | { kind: "fixture_list"; label: string }
  | { kind: "notice"; id: string };

/** A workbench contribution of an extension's manifest. */
export interface ManifestWorkbenchContribution {
  kind: "workbench";
  id: string;
  label: string;
  icon: string;
  tabs: { id: string; label: string; icon: string; blocks: ManifestBlock[] }[];
  toolbox_cards: { label: string; commands: { id: string; label: string; icon: string }[] }[];
  fixture_kinds: { id: string; label: string; icon: string; params: ManifestParam[] }[];
}

/** One compiled-in extension's manifest, as the session sends it. */
export interface Manifest {
  id: string;
  name: string;
  version: string;
  licence: string;
  contributions: ManifestWorkbenchContribution[];
}

/** One kind of the Add page Kind switch (ADR 0042 §7): Drawing, Sketch, then one per enabled
 * extension manifest, with its word and glyph. */
export interface PageKindEntry {
  /** `drawing`, `sketch` or the process id of an output kind. */
  id: string;
  label: string;
  /** The Lucide name of the glyph. */
  icon: string;
  /** The registry kind whose tools a page of this kind shows. */
  workbench: string;
}

/** The active page as the header and status bar show it. */
export interface ActivePage {
  id: string;
  name: string;
  kind: string;
  word: string;
  icon: string;
  off: boolean;
}

/** The session's read of the page kinds (`workbenches_json`). */
export interface WorkbenchList {
  /** The registry kind of the active page's workbench: `vector`, `sketch` or `manufacturing`. */
  active: string;
  page: ActivePage;
  kinds: PageKindEntry[];
  extensions: { id: string; name: string; version: string; licence: string; enabled: boolean }[];
  manifests: Manifest[];
}

/** An empty list, before the session exists. */
export const NO_WORKBENCHES: WorkbenchList = {
  active: "vector",
  page: { id: "", name: "", kind: "drawing", word: "Drawing", icon: "PenTool", off: false },
  kinds: [],
  extensions: [],
  manifests: [],
};

/** What the shell draws for one workbench kind. `Ctx` is the app's context of the moment. */
export interface WorkbenchEntry<Ctx = unknown, Node = unknown> {
  kind: string;
  /** The tool rail. */
  rail: (ctx: Ctx) => Node;
  /** What floats over the canvas (bars, notices); nothing when absent. */
  overlay?: (ctx: Ctx) => Node;
}

const entries = new Map<string, WorkbenchEntry<never, never>>();

/** Registers what the shell draws for `entry.kind`; a later call for the same kind replaces it. */
export function registerWorkbench<Ctx, Node>(entry: WorkbenchEntry<Ctx, Node>): void {
  entries.set(entry.kind, entry as unknown as WorkbenchEntry<never, never>);
}

/** The registered drawing of `kind`, or `undefined` for a kind nothing registered. */
export function workbenchEntry<Ctx, Node>(kind: string): WorkbenchEntry<Ctx, Node> | undefined {
  return entries.get(kind) as WorkbenchEntry<Ctx, Node> | undefined;
}

/** The kinds the Kind switch offers, in the session's order: only those whose workbench a
 * registered entry draws (`specs/0098` criterion 18 a), so the shell never names a process. */
export function kindOptions(list: WorkbenchList): PageKindEntry[] {
  return list.kinds.filter((kind) => entries.has(kind.workbench));
}

/** The registry kind of the active page's workbench. */
export function activeKind(list: WorkbenchList): string {
  return list.active;
}

/** The workbench contribution of `process` among the enabled manifests. */
export function contributionOf(
  list: WorkbenchList,
  process: string | undefined,
): ManifestWorkbenchContribution | undefined {
  return list.manifests
    .find((manifest) => manifest.id === process)
    ?.contributions.find((contribution) => contribution.kind === "workbench");
}

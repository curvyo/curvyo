// The workbench registry (ADR 0032 §3, `specs/0093-laser-workbench-shell` criterion 11): the
// one place that maps a workbench kind to what the shell draws for it. Vector and Sketch are two
// implementations, every manufacturing of any process is drawn by the one generic manifest
// workbench; the shell never branches on a process. Pure data and functions, so the tests load it
// without a bundler.

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
  | { kind: "params"; label: string; params: ManifestParam[] }
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

/** One entry of the session's workbench list. */
export interface WorkbenchListEntry {
  /** `vector`, `sketch` or a manufacturing's full node id. */
  id: string;
  /** The registry kind that draws it. */
  kind: string;
  label: string;
  /** The process of a manufacturing. */
  process?: string;
  /** The glyph a manufacturing's process declares. */
  icon?: string;
  /** `false` for a manufacturing whose extension is off or unknown. */
  enabled: boolean;
}

/** The session's registry read (`workbenches_json`). */
export interface WorkbenchList {
  active: string;
  entries: WorkbenchListEntry[];
  add: { process: string; label: string; icon: string }[];
  extensions: { id: string; name: string; version: string; licence: string; enabled: boolean }[];
  manifests: Manifest[];
  disabled: { id: string; label: string }[];
}

/** An empty list, before the session exists. */
export const NO_WORKBENCHES: WorkbenchList = {
  active: "vector",
  entries: [],
  add: [],
  extensions: [],
  manifests: [],
  disabled: [],
};

/** What the shell draws for one workbench kind. `Ctx` is the app's context of the moment. */
export interface WorkbenchEntry<Ctx = unknown, Node = unknown> {
  kind: string;
  /** The switcher glyph's Lucide name for this entry. */
  icon: (entry: WorkbenchListEntry) => string;
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

/** One button of the switcher row. */
export interface SwitcherButton {
  id: string;
  label: string;
  icon: string;
  enabled: boolean;
  pressed: boolean;
}

/** The switcher's buttons in order (UX notes §1): every listed workbench a registered kind can
 * draw, in the session's order (Vector, Sketch, then the manufacturings in creation order). */
export function switcherButtons(list: WorkbenchList): SwitcherButton[] {
  return list.entries.flatMap((entry) => {
    const registered = entries.get(entry.kind);
    if (!registered) {
      return [];
    }
    return [
      {
        id: entry.id,
        label: entry.enabled ? entry.label : `${entry.label} (disabled)`,
        icon: registered.icon(entry),
        enabled: entry.enabled,
        pressed: entry.id === list.active,
      },
    ];
  });
}

/** The kind of the active workbench (`vector` when the list does not name it). */
export function activeKind(list: WorkbenchList): string {
  return list.entries.find((entry) => entry.id === list.active)?.kind ?? "vector";
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
